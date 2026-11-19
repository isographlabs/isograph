# Babel and SWC iso transforms

The Babel plugin and the SWC plugin rewrite `iso` calls at build time. `` iso(`entrypoint Query.HomeRoute`) `` becomes a reference to the generated entrypoint artifact. `` iso(`field Query.HomeRoute ...`)(fn) `` becomes `fn`. `` iso(`field ...`) `` with no call becomes `x => x`. `pointer` behaves as `field`.

Both are restored from i2 history (`180ec76f^` for Babel, `435f7352^` for SWC). i2's pre-deletion SWC plugin is closer to i2's standards than upstream main (`../isograph` at `79e2a3c0`): it already uses `Postfix`, `ArtifactType::from_keyword` instead of a panicking `From<&str>`, `iso_call`, and `SyntaxContext` instead of `Option<Mark>`. The pre-deletion Babel plugin is byte-identical to upstream main.

## Where each lives

- `libs/isograph-babel-plugin` (`@isograph/babel-plugin`), a pnpm workspace package. `pnpm-workspace.yaml` gains `libs/*`.
- `crates/swc_isograph_plugin`, its own Cargo workspace. The root `Cargo.toml` gains `exclude = ["crates/swc_isograph_plugin"]`.
- `libs/isograph-swc-plugin` (`@isograph/plugin-swc`), the npm package that ships `swc_isograph_plugin.wasm`, with a vitest test that runs the built wasm under `@swc/core`. It drops upstream's `@swc/counter` dependency, which nothing reads.

The SWC crate is its own workspace because `swc_core` with `ecma_plugin_transform` and `testing_transform` locks 217 packages, close to the root lock's 248. A root member would nearly double the root `Cargo.lock`, which every other branch edits. A clean host build of the crate with its tests takes 18 s, so build time is not the reason.

## SWC version

`swc_core = "84.2.0"`, the latest on crates.io. `@swc/core` 1.16.22 (latest on npm) is built from `swc_core` 84.0.1. Next.js 16.4.6 (latest) is built from `swc_core` 79. Since `@swc/core` 1.15.0, plugins built against `swc_core >= 47` with `--cfg=swc_ast_unknown` deserialize across host versions (swc.rs, "Compatibility"). A field deletion or a field type change in the AST still breaks a plugin. The plugin never matches an AST enum without a wildcard arm, so the `Unknown` variants the cfg adds need no new arms.

`.cargo/config.toml` gains the cfg for the wasm target:

```toml
# from .cargo/config.toml
[target.wasm32-wasip1]
rustflags = ["-C", "link-arg=--allow-undefined", "--cfg=swc_ast_unknown"]
```

## Config

Both transforms read `output_directory` from the config, relative to the config file's directory. When it is absent, the output directory is the config file's directory. The entrypoint artifact is `{output_directory}/__isograph/{Type}/{field}/entrypoint.ts`, upstream's layout with `output_directory` in place of `artifact_directory ?? project_root`.

`output_directory` is not on `IsographProjectConfig` yet. The artifact-generation work adds it. Neither transform deserializes `IsographProjectConfig`; each reads only `output_directory` and ignores the other keys. The compiler validates the config.

Dependency on artifact generation: the transforms assume (a) the artifact is under `{output_directory}/__isograph/`, not directly under `{output_directory}/`, and (b) an absent `output_directory` means the config file's directory. If artifact generation decides otherwise, both transforms change their path computation to match.

## Import or require

Upstream picks `import` or `require` from `options.module`. i2's config has no `options`. Both transforms pick from the file's own source type: a module gets a hoisted default import, a script gets `require(path).default`. A script cannot contain an `import` declaration, and a bundler converts module imports itself, so neither choice loses a case. Upstream's SWC plugin dropped hoisted imports in a script; this choice removes that case.

## What changes from the restored code

Babel (`libs/isograph-babel-plugin`):

- `compileTag.js`: the artifact directory is `config.config['output_directory'] ?? '.'` joined to the config directory. Import vs require reads `path.scope.getProgramParent().path.node.sourceType === 'module'` instead of `options.module`. A relative path that does not start with `.` gets a `./` prefix (upstream only fixed a leading `/`, so a file above the output directory imported `src/__isograph/...` as a bare specifier).
- `BabelPluginIsograph.js`: unchanged.
- `package.json`: drops `babel-plugin-macros`, `graphql`, `prettier`, and `prettier-plugin-hermes-parser`, which nothing reads. Adds `vitest`, `typescript`, and `@types/node`. Scripts are `test` (`vitest run`) and `typecheck` (`tsc --noEmit`).
- `tsconfig.json`: standalone (`checkJs`, `strict`, `noEmit`); upstream extended a `tsconfig.build.json` that i2 does not have.
- `BabelPluginIsograph.test.js`: inline snapshots become `toBe` assertions. The `Module._load` cosmiconfig mock becomes a real `isograph.config.json` the test writes into a temp directory and finds through `searchFrom`.
- `README.md`: the SWC paragraph and the stale version pin go. It states the import-or-require rule and where `output_directory` puts the artifact.

SWC (`crates/swc_isograph_plugin`):

```rust
// from crates/swc_isograph_plugin/src/lib.rs
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
    root_dir: PathBuf,
    config: ProjectConfig,
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

struct IsoLiteralCompilerVisitor<'a> {
    root_dir: &'a Path,
    config: &'a ProjectConfig,
    filepath: &'a Path,
    artifact_reference: ArtifactReference,
    unresolved_ctxt: SyntaxContext,
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
```

`IsographImport` keeps its fields and `as_module_item`. `ArtifactType`, `from_keyword`, `iso_call`, `build_require_expr_from_path`, `build_ident_expr_for_hoisted_import`, and `build_arrow_identity_expr` are copied verbatim.

Deltas from `435f7352^`:

- `isograph_config::IsographProjectConfig` becomes `ProjectConfig` (above); `ISOGRAPH_FOLDER` still comes from `isograph_config`.
- `path_for_artifact` returns `Result<PathBuf, IsographTransformError>`. `real_filepath.parent()` returning `None` and `pathdiff::diff_paths` returning `None` or an absolute path (it returns the artifact directory unchanged when the file's folder is relative) are `ArtifactPathUnavailable` instead of `unwrap`/`expect`. The `./` prefix applies to every path that does not start with `.`, as in Babel.
- `handle_valid_isograph_entrypoint_literal` returns `Result<Expr, IsographTransformError>` and matches `self.artifact_reference` instead of `config.options.module`. `Import` pushes an `IsographImport` only when no import of that path is already there, so two literals of one entrypoint produce one import (upstream produced a duplicate binding).
- `parse_iso_template_literal` becomes a free function; it never read `self` (clippy `unused_self`).
- `fold_module_items` prepends the imports of `ArtifactReference::Import`.
- The plugin entry does not panic. A missing or unparseable plugin config, or a missing file name, emits an error through `HANDLER` and returns the program unchanged.

```rust
// from crates/swc_isograph_plugin/src/lib.rs
#[plugin_transform]
fn isograph_plugin_transform(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
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

fn plugin_setup(metadata: &TransformPluginProgramMetadata) -> Result<PluginSetup, PluginSetupError> {
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
```

Dependencies drop `once_cell` (std `LazyLock`, as `isograph_extract_typescript` does for its regex), `swc_atoms`, `swc_ecma_utils`, and `swc_plugin_macro` (all through `swc_core` features `ecma_plugin_transform` and `ecma_utils`), and `swc_trace_macro` and `tracing` (the `debug!` calls and `#[swc_trace]` go; nothing reads them). Tests use `swc_core` features `ecma_parser`, `testing_transform`, and `testing`.

The tests drop the fixture directories (`output.js` and `output.stderr` are checked-in expected output). `tests/transform.rs` uses `swc_core::ecma::transforms::testing::test_transform` with input and expected code inline, and `swc_core::testing::Tester::errors` for the error cases, asserting the diagnostic message and that the program is unchanged. Cases: bare field to identity, field call to its argument, entrypoint in a module to a hoisted import, entrypoint in a script to `require(...).default`, nested entrypoint inside a field function, two literals of one entrypoint to one import, absent and present `output_directory`, a file above the output directory (`./` prefix), the two upstream error fixtures, and a template literal with a substitution.

## Build and test

```sh
# Babel
pnpm --filter @isograph/babel-plugin test
pnpm --filter @isograph/babel-plugin typecheck

# SWC, host target
cargo test --manifest-path crates/swc_isograph_plugin/Cargo.toml
cargo clippy --manifest-path crates/swc_isograph_plugin/Cargo.toml --all-targets -- -D warnings

# SWC, wasm
pnpm --filter @isograph/plugin-swc build   # cargo build --release --target wasm32-wasip1, copies the .wasm into the package
pnpm --filter @isograph/plugin-swc test    # vitest: @swc/core 1.16.22 runs the built wasm
```

The crate's `Cargo.toml` holds `[profile.release]` (`codegen-units = 1`, `opt-level = "s"`, `strip = "symbols"`), moved from the root's `[profile.release.package.swc_isograph_plugin]` at `435f7352^`, and a copy of the root `[workspace.lints.clippy]`.

## CI and pre-commit

- `.github/workflows/js.yml` gains a `babel-plugin` job: `pnpm --filter @isograph/babel-plugin typecheck` and `test`.
- `.github/workflows/rust.yml` gains a `swc-plugin` job: `cargo fmt --check`, `clippy`, and `test` with `--manifest-path crates/swc_isograph_plugin/Cargo.toml`, then installs `wasm32-wasip1`, runs `pnpm --filter @isograph/plugin-swc build` and `test`.
- `ci.yml` needs no change; `rust` and `js` are already in `all-checks-passed`.
- `.pre-commit-config.yaml` gains `swc-plugin-fmt`, `swc-plugin-clippy`, and `swc-plugin-test` (files `^crates/swc_isograph_plugin/`), and `babel-plugin-test` and `babel-plugin-typecheck` (files `^libs/isograph-babel-plugin/`).
- `crates/prelude/src/postfix_constructors.rs` adds `crates/swc_isograph_plugin` to the walked crates.
