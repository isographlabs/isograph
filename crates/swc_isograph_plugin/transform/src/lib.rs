#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::path::{Path, PathBuf};

use isograph_config::CompilerConfig;
use once_cell::sync::Lazy;
use regex::Regex;
use swc_atoms::JsWord;
use swc_common::{FileName, Mark, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_utils::{quote_ident, ExprFactory};
use swc_ecma_visit::{Fold, FoldWith};

#[derive(Debug, Clone)]
struct RelayImport {
    path: JsWord,
    item: JsWord,
    unresolved_mark: Option<Mark>,
}

impl RelayImport {
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

struct Isograph<'a> {
    // root_dir: PathBuf,
    // pages_dir: Option<PathBuf>,
    file_name: FileName,
    config: &'a CompilerConfig,
    // imports: Vec<RelayImport>,
    unresolved_mark: Option<Mark>,
}

struct IsographEntrypoint {
    pub field_type: String,
    pub field_name: String,
    pub artifact_type: String,
}

impl IsographEntrypoint {
    fn build_require_expr_from_path(path: &str, mark: Option<Mark>) -> Expr {
        Expr::Call(CallExpr {
            span: Default::default(),
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
        })
    }
    pub fn to_import_expr(
        self,
        config: &CompilerConfig,
        unresolved_mark: Option<Mark>,
    ) -> Option<Expr> {
        // const filename = path.state.filename;
        // const folder = pathModule.dirname(filename);
        // const cwd = path.state.cwd;
        // const artifactDirectory = pathModule.join(
        //   cwd,
        //   config.artifact_directory ?? config.project_root,
        // );

        // const fileToArtifactDir = pathModule.relative(folder, artifactDirectory);
        // const artifactDirToArtifact = `/__isograph/${type}/${field}/${artifactType}.ts`;
        // let fileToArtifact = pathModule.join(
        //   fileToArtifactDir,
        //   artifactDirToArtifact,
        // );
        panic!("todo")
    }

    fn path_for_artifact(
        &self,
        real_file_name: &Path,
        config: &CompilerConfig,
    ) -> Result<PathBuf, BuildRequirePathError> {
        let filename = format!(
            "/__isograph/{}/{}/{}.ts",
            self.field_type, self.field_name, self.artifact_type
        );
        let current_filename = "something".to_string();
        let current_cwd = PathBuf::new();
        todo!("wip");
        // let artifact_directory =
        // if let Some(artifact_directory) = config.artifact_directory {
        //     Ok(root_dir.join(artifact_directory).join(filename))
        // } else {
        //     Ok(real_file_name
        //         .parent()
        //         .unwrap()
        //         .join("__generated__")
        //         .join(filename))
        // }
    }

    // fn build_require_path(
    //     &mut self,
    //     operation_name: &str,
    // ) -> Result<PathBuf, BuildRequirePathError> {
    //     match &self.file_name {
    //         FileName::Real(real_file_name) => {
    //             self.path_for_artifact(real_file_name, operation_name)
    //         }
    //         _ => Err(BuildRequirePathError::FileNameNotReal),
    //     }
    // }
}
static OPERATION_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\\s*(entrypoint|field)\\s*([^\\.\\s]+)\\.([^\\s\\(]+)").unwrap());

fn parse_iso_call_arg_into_type(expr_or_spread: &ExprOrSpread) -> Option<IsographEntrypoint> {
    // Todo: maybe move to visitMut
    match *expr_or_spread.expr.clone() {
        Expr::Lit(Lit::Str(iso_entrypoint)) => {
            let capture_group = OPERATION_REGEX
                .captures_iter(&iso_entrypoint.value.as_str())
                .next();

            capture_group.map(|capture_group| IsographEntrypoint {
                field_type: capture_group[2].to_string(),
                field_name: capture_group[3].to_string(),
                artifact_type: "entrypoint".to_string(),
            })
        }
        _ => panic!("iso function can only be called with a literal argument"),
    }
}

impl<'a> Fold for Isograph<'a> {
    fn fold_expr(&mut self, expr: Expr) -> Expr {
        let expr = expr.fold_children_with(self);
        match &expr {
            Expr::Call(call_expr) => {
                if let Some(built_expr) = self.compile_call_statement(call_expr) {
                    built_expr
                } else {
                    expr
                }
            }
            _ => expr,
        }
    }
}

// TODO: This is really hacky.
fn unique_ident_name_from_operation_name(operation_name: &str) -> String {
    format!("__{}", operation_name)
}

#[derive(Debug)]
enum BuildRequirePathError {
    FileNameNotReal,
    ArtifactDirectoryExpected { file_name: String },
}

impl<'a> Isograph<'a> {
    fn compile_iso_call_statement(&mut self, call_expr: &CallExpr) -> Option<Expr> {
        let entrypoint = parse_iso_call_arg_into_type(&call_expr.args[0]);
        match entrypoint {
            Some(entrypoint) => entrypoint.to_import_expr(self.config, self.unresolved_mark),
            None => None,
        }
    }

    fn compile_call_statement(&mut self, call_expr: &CallExpr) -> Option<Expr> {
        // Maybe move to visitMut?
        let sub_expr = call_expr.callee.clone().expr();
        if let Some(boxed_expr) = sub_expr {
            if let Expr::Ident(ident) = *boxed_expr {
                if ident.sym == "iso" {
                    return self.compile_iso_call_statement(call_expr);
                }
            }
        }
        None
    }
}

pub fn isograph(
    config: &CompilerConfig,
    file_name: FileName,
    unresolved_mark: Option<Mark>,
) -> impl Fold + '_ {
    Isograph {
        file_name,
        config,
        unresolved_mark,
    }
}
