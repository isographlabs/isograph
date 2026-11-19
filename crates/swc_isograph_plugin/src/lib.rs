use isograph_config::ISOGRAPH_FOLDER;
use prelude::Postfix;
use regex::Regex;
use serde::Deserialize;
use std::{
    fmt,
    path::{Path, PathBuf},
    sync::LazyLock,
};
use swc_core::{
    atoms::Atom,
    common::{
        DUMMY_SP, Span, SyntaxContext, errors::HANDLER,
        plugin::metadata::TransformPluginMetadataContextKind,
    },
    ecma::{
        ast::*,
        utils::{ExprFactory, prepend_stmts, quote_ident},
        visit::{Fold, FoldWith, fold_pass, noop_fold_type},
    },
    plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},
};
use thiserror::Error;

static OPERATION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s*(entrypoint|field|pointer)\s*([^\.\s]+)\.([^\s\(]+)")
        .expect("OPERATION_REGEX is a valid regex")
});

/// The subset of `isograph.config.json` the transform reads.
#[derive(Deserialize, Debug)]
pub struct ProjectConfig {
    /// Relative to `root_dir`.
    pub output_directory: Option<PathBuf>,
}

/// The plugin options: `[["@isograph/plugin-swc", { root_dir, config }]]`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WasmConfig {
    /// The absolute path of the directory holding `isograph.config.json`.
    ///
    /// Unlike native env, in WASM we can't use env::current_dir
    /// as well as `/cwd` alias. current_dir cannot resolve to actual path,
    /// `/cwd` alias won't expand to `real` path but only gives ACCESS to the cwd as
    /// mounted path, which we can't use in this case.
    root_dir: PathBuf,
    config: ProjectConfig,
}

#[plugin_transform]
fn isograph_plugin_transform(
    program: Program,
    metadata: TransformPluginProgramMetadata,
) -> Program {
    match plugin_setup(metadata.reference()) {
        Ok(PluginSetup {
            wasm_config: WasmConfig { root_dir, config },
            file_name,
        }) => program.apply(compile_iso_literal_visitor(
            config.reference(),
            Path::new(file_name.as_str()),
            root_dir.as_path(),
            SyntaxContext::empty().apply_mark(metadata.unresolved_mark),
        )),
        Err(error) => {
            HANDLER.with(|handler| handler.err(error.to_string().as_str()));
            program
        }
    }
}

struct PluginSetup {
    wasm_config: WasmConfig,
    file_name: String,
}

fn plugin_setup(
    metadata: &TransformPluginProgramMetadata,
) -> Result<PluginSetup, PluginSetupError> {
    let json = metadata
        .get_transform_plugin_config()
        .ok_or(PluginSetupError::MissingPluginConfig)?;
    let wasm_config = serde_json::from_str(json.as_str())?;
    let file_name = metadata
        .get_context(TransformPluginMetadataContextKind::Filename.reference())
        .ok_or(PluginSetupError::MissingFileName)?;
    PluginSetup {
        wasm_config,
        file_name,
    }
    .wrap_ok()
}

#[derive(Error, Debug)]
enum PluginSetupError {
    #[error("The Isograph SWC plugin received no options. Pass {{ root_dir, config }}.")]
    MissingPluginConfig,

    #[error("The Isograph SWC plugin options are invalid: {0}")]
    InvalidPluginConfig(#[from] serde_json::Error),

    #[error("The Isograph SWC plugin requires the name of the file being transformed.")]
    MissingFileName,
}

pub fn compile_iso_literal_visitor<'a>(
    config: &'a ProjectConfig,
    filepath: &'a Path,
    root_dir: &'a Path,
    unresolved_ctxt: SyntaxContext,
) -> impl Pass + 'a {
    fold_pass(IsoLiteralPass {
        config,
        filepath,
        root_dir,
        unresolved_ctxt,
    })
}

/// Picks the artifact reference from the program kind, then folds with a visitor.
struct IsoLiteralPass<'a> {
    root_dir: &'a Path,
    config: &'a ProjectConfig,
    filepath: &'a Path,
    unresolved_ctxt: SyntaxContext,
}

impl Fold for IsoLiteralPass<'_> {
    noop_fold_type!();

    fn fold_module(&mut self, module: Module) -> Module {
        module.fold_with(&mut self.visitor(ArtifactReference::Import(vec![])))
    }

    fn fold_script(&mut self, script: Script) -> Script {
        script.fold_with(&mut self.visitor(ArtifactReference::Require))
    }
}

impl<'a> IsoLiteralPass<'a> {
    fn visitor(&self, artifact_reference: ArtifactReference) -> IsoLiteralCompilerVisitor<'a> {
        IsoLiteralCompilerVisitor {
            root_dir: self.root_dir,
            config: self.config,
            filepath: self.filepath,
            artifact_reference,
            unresolved_ctxt: self.unresolved_ctxt,
        }
    }
}

/// How an entrypoint literal refers to its artifact.
enum ArtifactReference {
    /// Hoisted default imports, one per artifact path.
    Import(Vec<IsographImport>),
    /// `require(path).default` in place.
    Require,
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

    #[error(
        "Cannot compute the path from this file to the entrypoint artifact. root_dir and the file name must both be absolute."
    )]
    ArtifactPathUnavailable,
}

fn show_error(span: Span, err: &IsographTransformError) {
    HANDLER.with(|handler| {
        handler
            .struct_span_err(span, err.to_string().reference())
            .emit();
    });
}

#[derive(Debug, Clone)]
struct IsographImport {
    path: Atom,
    item: Atom,
    unresolved_ctxt: SyntaxContext,
}

impl IsographImport {
    fn as_module_item(&self) -> ModuleItem {
        ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
            span: Default::default(),
            specifiers: ImportSpecifier::Default(ImportDefaultSpecifier {
                span: Default::default(),
                local: Ident {
                    ctxt: self.unresolved_ctxt,
                    span: DUMMY_SP,
                    sym: self.item.clone(),
                    optional: false,
                },
            })
            .wrap_vec(),
            src: self.path.clone().to::<Str>().boxed(),
            type_only: false,
            with: None,
            phase: Default::default(),
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

impl ArtifactType {
    fn from_keyword(keyword: &str) -> Option<ArtifactType> {
        match keyword {
            "entrypoint" => ArtifactType::Entrypoint.wrap_some(),
            "field" | "pointer" => ArtifactType::Field.wrap_some(),
            _ => None,
        }
    }
}

fn build_ident_expr_for_hoisted_import(ident_name: &str, unresolved_ctxt: SyntaxContext) -> Expr {
    Expr::Ident(Ident {
        span: DUMMY_SP,
        sym: ident_name.to(),
        optional: false,
        ctxt: unresolved_ctxt,
    })
}

#[derive(Debug, Clone)]
struct ValidIsographTemplateLiteral {
    pub field_type: String,
    pub field_name: String,
    pub artifact_type: ArtifactType,
}

impl ValidIsographTemplateLiteral {
    fn build_require_expr_from_path(path: &str, unresolved_ctxt: SyntaxContext) -> Expr {
        Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Expr::Call(CallExpr {
                span: DUMMY_SP,
                callee: quote_ident!(unresolved_ctxt, "require").as_callee(),
                args: Lit::Str(Str {
                    span: Default::default(),
                    value: path.to(),
                    raw: None,
                })
                .as_arg()
                .wrap_vec(),
                type_args: None,
                ctxt: SyntaxContext::empty(),
            })
            .boxed(),
            prop: MemberProp::Ident(IdentName {
                sym: "default".to(),
                span: DUMMY_SP,
            }),
        })
    }

    fn path_for_artifact(
        &self,
        real_filepath: &Path,
        config: &ProjectConfig,
        root_dir: &Path,
    ) -> Result<PathBuf, IsographTransformError> {
        let folder = real_filepath
            .parent()
            .ok_or(IsographTransformError::ArtifactPathUnavailable)?;

        let artifact_directory = match config.output_directory.as_ref() {
            Some(output_directory) => root_dir.join(output_directory),
            None => root_dir.to_path_buf(),
        }
        .join(ISOGRAPH_FOLDER);

        // diff_paths returns the absolute path unchanged when `folder` is relative.
        let file_to_artifact_dir = pathdiff::diff_paths(artifact_directory, folder)
            .filter(|path| path.is_relative())
            .ok_or(IsographTransformError::ArtifactPathUnavailable)?;

        let mut file_to_artifact = PathBuf::from(format!(
            "{}/{}/{}/{}.ts",
            file_to_artifact_dir.display(),
            self.field_type,
            self.field_name,
            self.artifact_type
        ));

        if cfg!(target_os = "windows") {
            file_to_artifact =
                PathBuf::from(format!("{}", file_to_artifact.display()).replace('\\', "/"));
        }

        // A path that does not start with `../` would be a bare specifier.
        if !file_to_artifact.starts_with("..") {
            file_to_artifact = PathBuf::from(format!("./{}", file_to_artifact.display()));
        }

        file_to_artifact.wrap_ok()
    }
}

struct IsoLiteralCompilerVisitor<'a> {
    root_dir: &'a Path,
    config: &'a ProjectConfig,
    filepath: &'a Path,
    artifact_reference: ArtifactReference,
    unresolved_ctxt: SyntaxContext,
}

impl IsoLiteralCompilerVisitor<'_> {
    fn handle_valid_isograph_entrypoint_literal(
        &mut self,
        iso_template_literal: ValidIsographTemplateLiteral,
    ) -> Result<Expr, IsographTransformError> {
        let file_to_artifact =
            iso_template_literal.path_for_artifact(self.filepath, self.config, self.root_dir)?;

        match &mut self.artifact_reference {
            ArtifactReference::Require => {
                ValidIsographTemplateLiteral::build_require_expr_from_path(
                    file_to_artifact.display().to_string().reference(),
                    self.unresolved_ctxt,
                )
            }
            ArtifactReference::Import(imports) => {
                let path: Atom = file_to_artifact.display().to_string().to();
                let ident_name = format!(
                    "_{}__{}",
                    iso_template_literal.field_type, iso_template_literal.field_name
                );

                if !imports.iter().any(|import| import.path == path) {
                    imports.push(IsographImport {
                        path,
                        item: ident_name.clone().to(),
                        unresolved_ctxt: self.unresolved_ctxt,
                    });
                }

                build_ident_expr_for_hoisted_import(ident_name.reference(), self.unresolved_ctxt)
            }
        }
        .wrap_ok()
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
            return IsographTransformError::IsoRequiresOneArg.wrap_err();
        };

        let iso_template_literal = parse_iso_template_literal(first)?;

        match iso_template_literal.artifact_type {
            ArtifactType::Entrypoint => {
                self.handle_valid_isograph_entrypoint_literal(iso_template_literal)
            }
            ArtifactType::Field => {
                match fn_args {
                    Some(fn_args) => {
                        if let Some((first, [])) = fn_args.split_first() {
                            return first.expr.as_ref().clone().wrap_ok();
                        }
                        // iso(...)(>args empty<) or iso(...)(first_arg, second_arg)
                        IsographTransformError::IsoFnCallRequiresOneArg.wrap_err()
                    }
                    // iso(...)>empty<
                    None => build_arrow_identity_expr().wrap_ok(),
                }
            }
        }
    }
}

fn parse_iso_template_literal(
    expr_or_spread: &ExprOrSpread,
) -> Result<ValidIsographTemplateLiteral, IsographTransformError> {
    if let Expr::Tpl(Tpl { quasis, .. }) = (*expr_or_spread.expr).reference() {
        let first = if let Some((first, [])) = quasis.split_first() {
            first
        } else {
            return IsographTransformError::SubstitutionsNotAllowedInIsoFragments.wrap_err();
        };

        return OPERATION_REGEX
            .captures_iter(first.raw.trim())
            .next()
            .and_then(|capture_group| {
                let artifact_type = ArtifactType::from_keyword(&capture_group[1])?;
                ValidIsographTemplateLiteral {
                    artifact_type,
                    field_type: capture_group[2].to_string(),
                    field_name: capture_group[3].to_string(),
                }
                .wrap_some()
            })
            .ok_or(IsographTransformError::InvalidIsoKeyword);
    }
    IsographTransformError::OnlyAllowedTemplateLiteral.wrap_err()
}

/// An `iso` invocation as it appears in the tree: `iso(iso_args)` bare, or
/// `iso(iso_args)(fn_args)` immediately called.
struct IsoCall<'a> {
    iso_args: &'a [ExprOrSpread],
    fn_args: Option<&'a [ExprOrSpread]>,
    /// The span of the `iso(...)` call itself, where errors are reported.
    span: Span,
}

fn iso_call(expr: &Expr) -> Option<IsoCall<'_>> {
    let Expr::Call(CallExpr {
        callee: Callee::Expr(callee),
        args,
        span,
        ..
    }) = expr
    else {
        return None;
    };
    match (**callee).reference() {
        Expr::Ident(ident) if ident.sym == "iso" => IsoCall {
            iso_args: args,
            fn_args: None,
            span: *span,
        }
        .wrap_some(),
        Expr::Call(CallExpr {
            callee: Callee::Expr(inner_callee),
            args: iso_args,
            span: iso_span,
            ..
        }) => match (**inner_callee).reference() {
            Expr::Ident(ident) if ident.sym == "iso" => IsoCall {
                iso_args,
                fn_args: args.as_slice().wrap_some(),
                span: *iso_span,
            }
            .wrap_some(),
            _ => None,
        },
        _ => None,
    }
}

impl Fold for IsoLiteralCompilerVisitor<'_> {
    noop_fold_type!();

    fn fold_expr(&mut self, expr: Expr) -> Expr {
        if let Some(IsoCall {
            iso_args,
            fn_args,
            span,
        }) = iso_call(expr.reference())
        {
            return match self.compile_iso_call_statement(iso_args, fn_args) {
                Ok(build_expr) => {
                    // might have `iso` functions inside the build expr
                    build_expr.fold_children_with(self)
                }
                Err(err) => {
                    show_error(span, err.reference());
                    // On error, we keep the same expression and fail showing the error
                    expr
                }
            };
        }

        expr.fold_children_with(self)
    }

    fn fold_module_items(&mut self, items: Vec<ModuleItem>) -> Vec<ModuleItem> {
        let mut items = items
            .into_iter()
            .map(|item| item.fold_children_with(self))
            .collect::<Vec<_>>();

        if let ArtifactReference::Import(imports) = self.artifact_reference.reference() {
            prepend_stmts(
                &mut items,
                imports.iter().map(|import| import.as_module_item()),
            );
        }

        items
    }
}

fn build_arrow_identity_expr() -> Expr {
    Expr::Arrow(ArrowExpr {
        params: Pat::Ident(
            Ident::new("x".to::<Atom>(), DUMMY_SP, SyntaxContext::empty()).to::<BindingIdent>(),
        )
        .wrap_vec(),
        body: Ident::new("x".to::<Atom>(), DUMMY_SP, SyntaxContext::empty())
            .to::<ArrowFunctionBody>()
            .boxed(),
        span: DUMMY_SP,
        is_async: false,
        is_generator: false,
        return_type: None,
        type_params: None,
        ctxt: SyntaxContext::empty(),
    })
}
