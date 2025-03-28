#![allow(clippy::not_unsafe_ptr_arg_deref)]

use isograph_config::{CompilerConfig, JavascriptModule};
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::{Path, PathBuf};
use swc_atoms::JsWord;
use swc_common::{errors::HANDLER, Mark, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_utils::{prepend_stmts, quote_ident, ExprFactory};
use swc_ecma_visit::{noop_fold_type, Fold, FoldWith};
use swc_trace_macro::swc_trace;
use tracing::debug;

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
                        .unwrap_or(Default::default()),
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

#[derive(Debug, Clone)]
struct Isograph<'a> {
    // root_dir: PathBuf,
    // pages_dir: Option<PathBuf>,
    config: &'a CompilerConfig,
    filepath: &'a Path,
    imports: Vec<IsographImport>,
    unresolved_mark: Option<Mark>,
}

#[derive(Debug, Clone, PartialEq)]
struct IsographEntrypoint {
    pub field_type: String,
    pub field_name: String,
    pub artifact_type: String,
}

impl IsographEntrypoint {
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

    fn path_for_artifact(
        &self,
        real_filepath: &Path,
        config: &CompilerConfig,
    ) -> Result<PathBuf, BuildRequirePathError> {
        debug!("real_filepath: {:?}", real_filepath);
        let folder = PathBuf::from(real_filepath.parent().unwrap());
        let artifact_directory = config.artifact_directory.absolute_path.as_path();
        debug!("artifact_directory: {:#?}", artifact_directory);

        let file_to_artifact_dir = &pathdiff::diff_paths(artifact_directory, folder)
            .expect("Expected path to be diffable");
        debug!("file_to_artifact: {:#?}", file_to_artifact_dir);

        let mut file_to_artifact = PathBuf::from(format!(
            "{}/{}/{}/{}.ts",
            file_to_artifact_dir.display(),
            self.field_type,
            self.field_name,
            self.artifact_type
        ));

        #[cfg(target_os = "windows")]
        let file_to_artifact = file_to_artifact.replace("\\", "/");

        if file_to_artifact.starts_with("/") {
            file_to_artifact = PathBuf::from(format!(".{}", file_to_artifact.display()));
        }
        Ok(file_to_artifact)
    }
}

static OPERATION_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s*(entrypoint|field)\s*([^\.\s]+)\.([^\s\(]+)").unwrap());

fn parse_iso_call_arg_into_type(
    expr_or_spread: Option<&ExprOrSpread>,
) -> Option<IsographEntrypoint> {
    if let Some(ExprOrSpread { expr, .. }) = expr_or_spread {
        match &**expr {
            Expr::Tpl(Tpl { quasis, .. }) => OPERATION_REGEX
                .captures_iter(&*quasis[0].raw.trim())
                .next()
                .map(|capture_group| IsographEntrypoint {
                    artifact_type: capture_group[1].to_string(),
                    field_type: capture_group[2].to_string(),
                    field_name: capture_group[3].to_string(),
                }),
            _ => panic!("iso function can only be called with a literal argument"),
        }
    } else {
        panic!("iso function can only be called with an expression argument")
    }
}

#[derive(Debug)]
enum BuildRequirePathError {
    FileNameNotReal,
    ArtifactDirectoryExpected { filepath: String },
    InvalidArgs,
}

#[swc_trace]
impl<'a> Isograph<'a> {
    pub fn compile_import_statement(&mut self, entrypoint: &IsographEntrypoint) -> Expr {
        let file_to_artifact = entrypoint
            .path_for_artifact(self.filepath, self.config)
            .expect("Failed to get path for artifact.");

        debug!("gen expr for artifact: {}", file_to_artifact.display());

        debug!("options module: {:?}", self.config.options.module);

        match self.config.options.module {
            JavascriptModule::CommonJs => entrypoint.build_require_expr_from_path(
                &file_to_artifact.display().to_string(),
                self.unresolved_mark,
            ),
            JavascriptModule::EsModule => {
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
        iso_args: &Vec<ExprOrSpread>,
        fn_args: Option<&Vec<ExprOrSpread>>,
    ) -> Option<Expr> {
        let entrypoint = parse_iso_call_arg_into_type(iso_args.first())
            .expect("C: iso call argument must be a literal expression");

        if entrypoint.artifact_type == "entrypoint" {
            debug!("Entrypoint artifact type {:?}", entrypoint);
            Some(self.compile_import_statement(&entrypoint))
        } else if entrypoint.artifact_type == "field" {
            match fn_args {
                Some(iso_args) => {
                    if fn_args.iter().len() >= 1 {
                        if let Some(ExprOrSpread { expr: e, .. }) = iso_args.first() {
                            Some(e.as_ref().clone())
                        } else {
                            // show error
                            return None;
                        }
                    } else {
                        // show error
                        return None;
                    }
                }
                None => {
                    let e = Expr::Arrow(ArrowExpr {
                        params: vec![Pat::Ident(Ident::new("x".into(), DUMMY_SP).into())],
                        body: Box::new(Ident::new("x".into(), DUMMY_SP).into()),
                        span: DUMMY_SP,
                        is_async: Default::default(),
                        is_generator: Default::default(),
                        return_type: Default::default(),
                        type_params: Default::default(),
                    });

                    return Some(e);
                }
            }
        } else {
            // show error
            return None;
        }
    }
}

impl<'a> Fold for Isograph<'a> {
    noop_fold_type!();

    fn fold_expr(&mut self, expr: Expr) -> Expr {
        match &expr {
            Expr::Call(CallExpr {
                callee: Callee::Expr(callee),
                args,
                span,
                ..
            }) => match &**callee {
                Expr::Ident(ident) => {
                    debug!("Ident executed");
                    if ident.sym == "iso" {
                        debug!("found iso function ---.");
                        if let Some(build_expr) = self.compile_iso_call_statement(args, None) {
                            return build_expr;
                        }
                    }
                }
                Expr::Call(CallExpr {
                    callee: Callee::Expr(child_callee),
                    args: child_args,
                    ..
                }) => {
                    debug!("Call executed");
                    debug!("args: {:?}", args);
                    match &**child_callee {
                        Expr::Ident(ident) => {
                            if ident.sym == "iso" {
                                debug!("found iso function");
                                if let Some(build_expr) =
                                    self.compile_iso_call_statement(child_args, Some(args))
                                {
                                    return build_expr;
                                } else {
                                    HANDLER.with(|handler| {
                                        handler
                                            .struct_span_err(*span, "Invalid iso tag usage. The iso function should be passed at most one argument.")
                                            .emit();
                                        return;
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            },
            _ => {}
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

pub fn isograph<'a>(
    config: &'a CompilerConfig,
    filepath: &'a Path,
    unresolved_mark: Option<Mark>,
) -> impl Fold + 'a {
    Isograph {
        config,
        filepath,
        unresolved_mark,
        imports: vec![],
    }
}
