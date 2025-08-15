use anyhow::{bail, Result};
use isograph_config::{ConfigFileJavascriptModule, IsographProjectConfig, ISOGRAPH_FOLDER};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
};
use swc_atoms::Atom;
use swc_core::{
    common::{
        errors::HANDLER, plugin::metadata::TransformPluginMetadataContextKind, Mark, Span, DUMMY_SP,
    },
    ecma::{
        ast::*,
        visit::{noop_fold_type, Fold, FoldWith},
    },
    plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},
};
use swc_ecma_utils::{prepend_stmts, quote_ident, ExprFactory};
use swc_trace_macro::swc_trace;

use thiserror::Error;
use tracing::debug;

static OPERATION_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s*(entrypoint|field|pointer)\s*([^\.\s]+)\.([^\s\(]+)").unwrap());

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WasmConfig {
    /// Unlike native env, in WASM we can't use env::current_dir
    /// as well as `/cwd` alias. current_dir cannot resolve to actual path,
    /// `/cwd` alias won't expand to `real` path but only gives ACCESS to the cwd as
    /// mounted path, which we can't use in this case.
    /// Must be an absolute path
    pub root_dir: PathBuf,
    pub config: IsographProjectConfig,
}

#[plugin_transform]
fn isograph_plugin_transform(
    program: Program,
    metadata: TransformPluginProgramMetadata,
) -> Program {
    let config: WasmConfig = serde_json::from_str(
        &metadata
            .get_transform_plugin_config()
            .expect("Failed to get plugin config for isograph"),
    )
    .unwrap_or_else(|e| panic!("Error parsing plugin config. Error: {e}"));

    let WasmConfig { root_dir, config } = config;

    debug!("Config: {:?}", config);

    let file_name = metadata.get_context(&TransformPluginMetadataContextKind::Filename);
    let file_name = file_name.as_deref().unwrap_or("unknown.js");

    let path = Path::new(file_name);

    let mut isograph = compile_iso_literal_visitor(
        &config,
        path,
        root_dir.as_path(),
        Some(metadata.unresolved_mark),
    );

    program.fold_with(&mut isograph)
}

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
        imports: BTreeMap::new(),
        root_dir,
    }
}

#[derive(Error, Clone, Debug, Eq, PartialEq)]
enum IsographTransformError {
    #[error("Invalid iso tag usage. Expected 'entrypoint', 'field' or 'pointer'.")]
    InvalidIsoKeyword,

    #[error("Invalid iso tag usage. The iso function should be passed exactly one argument.")]
    IsoFnCallRequiresOneArg,

    #[error("Iso invocation require one parameter.")]
    IsoRequiresOneArg,

    #[error("Only template literals are allowed in iso fragments.")]
    OnlyAllowedTemplateLiteral,

    #[error("Substitutions are not allowed in iso fragments.")]
    SubstitutionsNotAllowedInIsoFragments,
}

fn show_error(span: Span, err: &IsographTransformError) -> Result<(), anyhow::Error> {
    let msg = IsographTransformError::to_string(err);

    HANDLER.with(|handler| {
        handler.struct_span_err(span, &msg).emit();
    });
    bail!(msg)
}

#[derive(Debug, Clone)]
struct IsographImport {
    path: Atom,
    item: Atom,
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

#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
enum ArtifactType {
    Entrypoint,
    Field,
}

impl fmt::Display for ArtifactType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtifactType::Entrypoint => f.write_str("entrypoint"),
            ArtifactType::Field => f.write_str("field"),
        }
    }
}

impl From<&str> for ArtifactType {
    fn from(s: &str) -> Self {
        match s {
            "entrypoint" => Self::Entrypoint,
            "field" | "pointer" => Self::Field,
            _ => {
                panic!("Regex will not produce this case. This is indicative of a bug in Isograph.")
            }
        }
    }
}

fn build_ident_expr_for_hoisted_import(ident_name: &str, unresolved_mark: Option<Mark>) -> Expr {
    Expr::Ident(Ident {
        span: unresolved_mark
            .map(|m| DUMMY_SP.apply_mark(m))
            .unwrap_or_default(),
        sym: ident_name.into(),
        optional: false,
    })
}

#[derive(Debug, Clone)]
struct ValidIsographTemplateLiteral {
    pub field_type: String,
    pub field_name: String,
    pub artifact_type: ArtifactType,
}

impl ValidIsographTemplateLiteral {
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
                    value: Atom::from(path),
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

    fn path_for_artifact(
        &self,
        real_filepath: &Path,
        config: &IsographProjectConfig,
        root_dir: &Path,
    ) -> Result<PathBuf, IsographTransformError> {
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
            // TODO a bug in the babel transform: https://github.com/isographlabs/isograph/issues/496
            "{}/{}/{}/{}.ts",
            file_to_artifact_dir.display(),
            self.field_type,
            self.field_name,
            self.artifact_type
        ));

        if cfg!(target_os = "windows") {
            file_to_artifact =
                PathBuf::from(format!("{}", file_to_artifact.display()).replace("\\", "/"));
        }

        // TODO Identify if this is needed
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
    imports: BTreeMap<(String, String), IsographImport>,
    unresolved_mark: Option<Mark>,
}

#[swc_trace]
impl IsoLiteralCompilerVisitor<'_> {
    fn parse_iso_template_literal(
        &self,
        expr_or_spread: &ExprOrSpread,
    ) -> Result<ValidIsographTemplateLiteral, IsographTransformError> {
        if let Expr::Tpl(Tpl { quasis, .. }) = &*expr_or_spread.expr {
            let first = if let Some((first, [])) = quasis.split_first() {
                first
            } else {
                return Err(IsographTransformError::SubstitutionsNotAllowedInIsoFragments);
            };

            return OPERATION_REGEX
                .captures_iter(first.raw.trim())
                .next()
                .map(|capture_group| {
                    debug!("capture_group {:?}", capture_group);
                    ValidIsographTemplateLiteral {
                        artifact_type: ArtifactType::from(&capture_group[1]),
                        field_type: capture_group[2].to_string(),
                        field_name: capture_group[3].to_string(),
                    }
                })
                .ok_or(IsographTransformError::InvalidIsoKeyword);
        }
        Err(IsographTransformError::OnlyAllowedTemplateLiteral)
    }

    fn handle_valid_isograph_entrypoint_literal(
        &mut self,
        iso_template_literal: ValidIsographTemplateLiteral,
    ) -> Expr {
        let file_to_artifact = iso_template_literal
            .path_for_artifact(self.filepath, self.config, self.root_dir)
            .expect("Failed to get path for artifact.");

        match self.config.options.module {
            ConfigFileJavascriptModule::CommonJs => iso_template_literal
                .build_require_expr_from_path(
                    &file_to_artifact.display().to_string(),
                    self.unresolved_mark,
                ),
            ConfigFileJavascriptModule::EsModule => {
                // TODO ensure `ident_name` is unique
                let ident_name = format!(
                    "_{}__{}",
                    iso_template_literal.field_type, iso_template_literal.field_name
                );

                // hoist import
                self.imports.insert(
                    (
                        iso_template_literal.field_type,
                        iso_template_literal.field_name,
                    ),
                    IsographImport {
                        path: file_to_artifact.display().to_string().into(),
                        item: ident_name.clone().into(),
                        unresolved_mark: self.unresolved_mark,
                    },
                );

                build_ident_expr_for_hoisted_import(&ident_name, self.unresolved_mark)
            }
        }
    }

    fn compile_iso_call_statement(
        &mut self,
        // iso(iso_args)(fn_args);
        iso_args: &[ExprOrSpread],
        fn_args: Option<&[ExprOrSpread]>,
    ) -> Result<Expr, IsographTransformError> {
        let first = if let Some((first, [])) = iso_args.split_first() {
            first
        } else {
            return Err(IsographTransformError::IsoRequiresOneArg);
        };

        let iso_template_literal = self.parse_iso_template_literal(first)?;

        match iso_template_literal.artifact_type {
            ArtifactType::Entrypoint => {
                Ok(self.handle_valid_isograph_entrypoint_literal(iso_template_literal))
            }
            ArtifactType::Field => {
                match fn_args {
                    Some(fn_args) => {
                        if let Some((first, [])) = fn_args.split_first() {
                            return Ok(first.expr.as_ref().clone());
                        }
                        // iso(...)(>args empty<) or iso(...)(first_arg, second_arg)
                        return Err(IsographTransformError::IsoFnCallRequiresOneArg);
                    }
                    // iso(...)>empty<
                    None => Ok(build_arrow_identity_expr()),
                }
            }
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
                                let _ = show_error(*span, &err);
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
                                    let _ = show_error(*child_span, &err);
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
            self.imports.values().map(|import| import.as_module_item()),
        );

        items
    }
}

fn build_arrow_identity_expr() -> Expr {
    Expr::Arrow(ArrowExpr {
        params: vec![Pat::Ident(Ident::new("x".into(), DUMMY_SP).into())],
        body: Box::new(Ident::new("x".into(), DUMMY_SP).into()),
        span: DUMMY_SP,
        is_async: false,
        is_generator: false,
        return_type: None,
        type_params: None,
    })
}
