use anyhow::{bail, Result};
use isograph_config::{ConfigFileJavascriptModule, IsographProjectConfig, ISOGRAPH_FOLDER};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use std::fmt;
use std::path::{Path, PathBuf};
use swc_atoms::JsWord;
use swc_common::{errors::HANDLER, Mark, Span, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_utils::{prepend_stmts, quote_ident, ExprFactory};
use swc_ecma_visit::{noop_fold_type, Fold, FoldWith};
use swc_trace_macro::swc_trace;
use thiserror::Error;
use tracing::debug;

static OPERATION_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s*(entrypoint|field)\s*([^\.\s]+)\.([^\s\(]+)").unwrap());

pub fn compile_iso_literal_visitor<'a>(
    config: &'a IsographProjectConfig,
    filepath: &'a Path,
    root_dir: &'a Path,
    unresolved_mark: Option<Mark>,
) -> impl Fold + 'a {
    IsoLiteralCompilerVisitor {
        config,
        filepath,
        unresolved_mark,
        imports: vec![],
        root_dir,
    }
}

#[derive(Error, Clone, Debug, Eq, PartialEq)]
enum IsograthTransformError {
    #[error("Invalid iso tag usage. Expected 'entrypoint' or 'field'.")]
    InvalidIsoKeyword,

    #[error("Invalid iso tag usage. The iso function should be passed exactly one argument.")]
    IsoFnCallRequiresOneArg,

    #[error("Iso invocation require one parameter.")]
    IsoRequiresOneArg,

    #[error("Malformed iso literal. I hope the iso compiler failed to accept this literal!")]
    MalformedIsoLiteral,

    #[error("Only template literals are allowed in iso fragments.")]
    OnlyAllowedTemplateLiteral,

    #[error("Substitutions are not allowed in iso fragments.")]
    SubstitutionsNotAllowedInIsoFragments,
}

fn show_error(span: &Span, err: &IsograthTransformError) -> Result<(), anyhow::Error> {
    let msg = IsograthTransformError::to_string(err);

    HANDLER.with(|handler| {
        handler.struct_span_err(*span, &msg).emit();
    });
    bail!(msg)
}

#[derive(Debug, Clone)]
struct IsographImport {
    path: JsWord,
    item: JsWord,
    unresolved_mark: Option<Mark>,
}

impl IsographImport {
    fn as_module_item(&self) -> ModuleItem {
        ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
            span: Default::default(),
            specifiers: vec![ImportSpecifier::Default(ImportDefaultSpecifier {
                span: Default::default(),
                local: Ident {
                    span: self
                        .unresolved_mark
                        .map(|m| DUMMY_SP.apply_mark(m))
                        .unwrap_or_default(),
                    sym: self.item.clone(),
                    optional: false,
                },
            })],
            src: Box::new(self.path.clone().into()),
            type_only: false,
            with: None,
            phase: Default::default(),
        }))
    }
}

#[derive(Deserialize, Default, Debug, Clone, Copy, PartialEq)]
pub enum ArtifactType {
    #[default]
    Entrypoint,
    Field,
    Unknown,
}

impl fmt::Display for ArtifactType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtifactType::Entrypoint => f.write_str("entrypoint"),
            ArtifactType::Field => f.write_str("field"),
            ArtifactType::Unknown => f.write_str("unknown"),
        }
    }
}

impl From<&str> for ArtifactType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "entrypoint" => Self::Entrypoint,
            "field" => Self::Field,
            _ => Self::Unknown,
        }
    }
}

impl From<String> for ArtifactType {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

#[derive(Default, Debug, Clone)]
struct ValidIsographTemplateLiteral {
    pub field_type: String,
    pub field_name: String,
    pub artifact_type: ArtifactType,
}

impl ValidIsographTemplateLiteral {
    fn build_ident_expr_for_hoisted_import(
        &self,
        ident_name: &str,
        unresolved_mark: Option<Mark>,
    ) -> Expr {
        Expr::Ident(Ident {
            span: unresolved_mark
                .map(|m| DUMMY_SP.apply_mark(m))
                .unwrap_or_default(),
            sym: ident_name.into(),
            optional: false,
        })
    }

    fn build_require_expr_from_path(&self, path: &str, mark: Option<Mark>) -> Expr {
        Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(Expr::Call(CallExpr {
                span: DUMMY_SP,
                callee: quote_ident!(
                    mark.map(|m| DUMMY_SP.apply_mark(m)).unwrap_or(DUMMY_SP),
                    "require"
                )
                .as_callee(),
                args: vec![Lit::Str(Str {
                    span: Default::default(),
                    value: JsWord::from(path),
                    raw: None,
                })
                .as_arg()],
                type_args: None,
            })),
            prop: MemberProp::Ident(Ident {
                sym: "default".into(),
                span: DUMMY_SP,
                optional: false,
            }),
        })
    }

    fn build_arrow_identity_expr(&self) -> Expr {
        Expr::Arrow(ArrowExpr {
            params: vec![Pat::Ident(Ident::new("x".into(), DUMMY_SP).into())],
            body: Box::new(Ident::new("x".into(), DUMMY_SP).into()),
            span: DUMMY_SP,
            is_async: Default::default(),
            is_generator: Default::default(),
            return_type: Default::default(),
            type_params: Default::default(),
        })
    }

    fn path_for_artifact(
        &self,
        real_filepath: &Path,
        config: &IsographProjectConfig,
        root_dir: &Path,
    ) -> Result<PathBuf, IsograthTransformError> {
        let folder = PathBuf::from(real_filepath.parent().unwrap());
        let cwd = PathBuf::from(root_dir);

        let artifact_directory = cwd
            .join(
                config
                    .artifact_directory
                    .as_ref()
                    .unwrap_or(&config.project_root),
            )
            .join(ISOGRAPH_FOLDER);
        let artifact_directory = artifact_directory.as_path();

        debug!("artifact_directory: {:#?}", artifact_directory);

        let file_to_artifact_dir = &pathdiff::diff_paths(artifact_directory, folder)
            .expect("Expected path to be diffable");

        let mut file_to_artifact = PathBuf::from(format!(
            "{}/{}/{}/{}.ts",
            file_to_artifact_dir.display(),
            self.field_type,
            self.field_name,
            self.artifact_type
        ));

        if cfg!(target_os = "windows") {
            file_to_artifact = PathBuf::from(format!("{}", file_to_artifact.display()).replace("\\", "/"));
        }
        
        if file_to_artifact.starts_with(ISOGRAPH_FOLDER) {
            file_to_artifact = PathBuf::from(format!("./{}", file_to_artifact.display()));
        }


        Ok(file_to_artifact)
    }
}

#[derive(Debug, Clone)]
struct IsoLiteralCompilerVisitor<'a> {
    root_dir: &'a Path,
    config: &'a IsographProjectConfig,
    filepath: &'a Path,
    imports: Vec<IsographImport>,
    unresolved_mark: Option<Mark>,
}

#[swc_trace]
impl IsoLiteralCompilerVisitor<'_> {
    fn parse_iso_call_arg_into_type(
        &self,
        expr_or_spread: Option<&ExprOrSpread>,
    ) -> Result<ValidIsographTemplateLiteral, IsograthTransformError> {
        if let Some(ExprOrSpread { expr, .. }) = expr_or_spread {
            if let Expr::Tpl(Tpl { quasis, .. }) = &**expr {
                if quasis.iter().len() != 1 {
                    return Err(IsograthTransformError::SubstitutionsNotAllowedInIsoFragments);
                }

                return OPERATION_REGEX
                    .captures_iter(quasis[0].raw.trim())
                    .next()
                    .map(|capture_group| {
                        debug!("capture_group {:?}", capture_group);
                        ValidIsographTemplateLiteral {
                            artifact_type: ArtifactType::from(capture_group[1].to_string()),
                            field_type: capture_group[2].to_string(),
                            field_name: capture_group[3].to_string(),
                        }
                    })
                    .ok_or(IsograthTransformError::InvalidIsoKeyword);
            }
        }
        Err(IsograthTransformError::OnlyAllowedTemplateLiteral)
    }

    pub fn compile_import_statement(&mut self, entrypoint: &ValidIsographTemplateLiteral) -> Expr {
        let file_to_artifact = entrypoint
            .path_for_artifact(self.filepath, self.config, self.root_dir)
            .expect("Failed to get path for artifact.");

        match self.config.options.module {
            ConfigFileJavascriptModule::CommonJs => entrypoint.build_require_expr_from_path(
                &file_to_artifact.display().to_string(),
                self.unresolved_mark,
            ),
            ConfigFileJavascriptModule::EsModule => {
                let ident_name = format!("_{}", entrypoint.field_name);

                // hoist import
                self.imports.push(IsographImport {
                    path: file_to_artifact.display().to_string().into(),
                    item: ident_name.clone().into(),
                    unresolved_mark: self.unresolved_mark,
                });
                entrypoint.build_ident_expr_for_hoisted_import(&ident_name, self.unresolved_mark)
            }
        }
    }

    fn compile_iso_call_statement(
        &mut self,
        // iso(iso_args)(fn_args);
        iso_args: &[ExprOrSpread],
        fn_args: Option<&[ExprOrSpread]>,
    ) -> Result<Expr, IsograthTransformError> {
        if iso_args.iter().len() != 1 {
            return Err(IsograthTransformError::IsoRequiresOneArg);
        }

        let entrypoint = self.parse_iso_call_arg_into_type(iso_args.first());

        match entrypoint {
            Err(e) => return Err(e),
            Ok(entrypoint) => match entrypoint.artifact_type {
                ArtifactType::Entrypoint => Ok(self.compile_import_statement(&entrypoint)),
                ArtifactType::Field => {
                    match fn_args {
                        Some(fn_args) => {
                            if fn_args.iter().len() == 1 {
                                if let Some(ExprOrSpread { expr: e, .. }) = fn_args.first() {
                                    return Ok(e.as_ref().clone());
                                }
                            }
                            // iso(...)(>args empty<) or iso(...)(first_arg, second_arg)
                            return Err(IsograthTransformError::IsoFnCallRequiresOneArg);
                        }
                        // iso(...)>empty<
                        None => Ok(entrypoint.build_arrow_identity_expr()),
                    }
                }
                ArtifactType::Unknown => {
                    return Err(IsograthTransformError::MalformedIsoLiteral);
                }
            },
        }
    }
}

impl Fold for IsoLiteralCompilerVisitor<'_> {
    noop_fold_type!();

    fn fold_expr(&mut self, expr: Expr) -> Expr {
        if let Expr::Call(CallExpr {
            callee: Callee::Expr(callee),
            args,
            span,
            ..
        }) = &expr
        {
            match &**callee {
                Expr::Ident(ident) => {
                    if ident.sym == "iso" {
                        match self.compile_iso_call_statement(args, None) {
                            Ok(build_expr) => {
                                // might have `iso` functions inside the build expr
                                let build_expr = build_expr.fold_children_with(self);
                                return build_expr;
                            }
                            Err(err) => {
                                let _ = show_error(span, &err);
                                // On error, we keep the same expression and fail showing the error
                                return expr;
                            }
                        }
                    }
                }
                Expr::Call(CallExpr {
                    callee: Callee::Expr(child_callee),
                    args: child_args,
                    span: child_span,
                    ..
                }) => {
                    if let Expr::Ident(ident) = &**child_callee {
                        if ident.sym == "iso" {
                            match self.compile_iso_call_statement(child_args, Some(args)) {
                                Ok(build_expr) => {
                                    // might have `iso` functions inside the build expr
                                    let build_expr = build_expr.fold_children_with(self);
                                    return build_expr;
                                }
                                Err(err) => {
                                    let _ = show_error(child_span, &err);
                                    // On error, we keep the same expression and fail showing the error
                                    return expr;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        expr.fold_children_with(self)
    }

    fn fold_module_items(&mut self, items: Vec<ModuleItem>) -> Vec<ModuleItem> {
        let mut items = items
            .into_iter()
            .map(|item| item.fold_children_with(self))
            .collect::<Vec<_>>();

        prepend_stmts(
            &mut items,
            self.imports.iter().map(|import| import.as_module_item()),
        );

        items
    }
}
