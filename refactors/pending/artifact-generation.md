# Artifact generation: iso.ts and entrypoint artifacts

The daemon generates artifacts into the `__isograph` folder of an optional output directory. The config field is `output_directory`, relative to the config file's directory. Generated files live under `<config dir>/<output_directory>/__isograph/`: `iso.ts` at the top of that folder, and `<Entity>/<selectable>/entrypoint.ts` for each entrypoint, upstream's layout. The Babel transform (`libs/isograph-babel-plugin/compileTag.js`) and the SWC transform (`crates/swc_isograph_plugin/src/lib.rs`, `path_for_artifact`) read the same field and import `<output_directory>/__isograph/<Entity>/<selectable>/entrypoint.ts`. The transforms treat a missing `output_directory` as the config file's directory. Generation writes nothing when the field is missing.

The artifacts in this doc are the top-level `iso.ts` and one entrypoint artifact per entrypoint literal. Generation is host-language work: `HostLanguage::generate_artifacts`, which the TypeScript host language implements with the new crate `typescript_artifacts`. `NetworkProtocol` gains nothing in this doc.

Artifacts are built as values, not strings. A module artifact is an `ArtifactValue` (its default export) at a path. `iso.ts` has overloads and type aliases, which a value does not describe, so it is built from a small TypeScript declaration and type layer. One module converts both into swc nodes (`swc_ecma_ast`) and prints them with `swc_ecma_codegen`.

All artifacts are generated and printed in memory. Only when every literal produced its sources are they written, together, and only the files whose text changed.

## End-user experience

```json
{
  "source_files": ["src/**/*.ts"],
  "schema": ["schema/**/*.graphql"],
  "output_directory": "src"
}
```

The user runs the daemon (`isograph lsp` from the editor, or `isograph start`). After the boot scan, and after every later burst of file changes settles for 100 ms, the daemon generates every artifact in memory. If every iso literal in every source file parses, the daemon writes the changes into `src/__isograph/`:

```text
src/__isograph/iso.ts
src/__isograph/Query/HomeRoute/entrypoint.ts
src/__isograph/Mutation/SetTagline/entrypoint.ts
```

`src/__isograph/Query/HomeRoute/entrypoint.ts`:

```ts
export default {
  kind: "Entrypoint",
  name: "Query.HomeRoute"
};
```

The user imports `iso` from the generated file:

```ts
import { iso } from './__isograph/iso';

export const fullName = iso(`
  field Pet.fullName { firstName lastName }
`)(function fullName({ data }: ResolverParameters<{ firstName: string; lastName: string }>) {
  return `${data.firstName} ${data.lastName}`;
});

console.log(iso(`entrypoint Query.HomeRoute`).name);
```

A `field` literal returns an identity function: the exported value has exactly the type of the resolver passed to it. The resolver's parameter is typed by the user until param types are generated (Later). An `entrypoint` literal has the type of its entrypoint artifact's default export; with a transform, the call becomes an import of that artifact. `` iso(`entity Owner`) `` returns `void`. A literal whose text matches no overload is a TypeScript error at the call site.

If any iso literal in any source file has a parse error, the daemon writes nothing and logs one warning naming the literal. Files already on disk stay as they are. Fixing the literal makes the next compile write.

The `__isograph` folder belongs to the compiler. When the daemon starts, it reads every file under `<output_directory>/__isograph/`. The first compile writes each artifact whose text differs from what is there, leaves identical files untouched (their modification time stays), and deletes every other file under the folder, removing folders that become empty. Later compiles do the same against what the daemon last wrote. Nothing outside the `__isograph` folder is created, written, or deleted.

The watcher does not post files under `<output_directory>/__isograph/`, so generated files never become source files.

## Config

```rust
// from crates/isograph_config/src/compilation_options.rs
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct IsographProjectConfig {
    /// The user may hard-code the JSON Schema for their version of the config.
    #[serde(rename = "$schema")]
    pub json_schema: Option<String>,
    /// Glob patterns relative to the config file's directory.
    pub source_files: Vec<String>,
    /// Glob patterns relative to the config file's directory.
    #[serde(default)]
    pub schema: Vec<String>,
    /// The directory, relative to the config file's directory, whose `__isograph` folder holds
    /// the generated artifacts. Absent: the compiler writes no artifacts, and the transforms
    /// look in the config file's directory.
    #[serde(default)]
    pub output_directory: Option<PathBuf>,
}
```

Upstream's field is `artifact_directory` with the same `__isograph` layout; it defaults to `project_root`, which i2 does not have.

## Prerequisites from compiler-errors.md

refactors/pending/compiler-errors.md changes 1 to 3 (landed): the boot scan is posted before the LSP port accepts, `handle` mints every `ClientId` from `IsographState.client_ids` with `compiler_client` minted first in `IsographState::new`, and every applied `DiskChanged` arms `compile_debounce`, whose fire calls `crate::compile::compile`. `compiler_client` is an ordinary client: its id comes from the same counter, it reads through `file_contents` and the same memos, and the only thing particular to it is that its compile writes artifacts.

This doc changes three things in that code. `IsographState::new` takes the artifact folder. `compile` takes `&mut IsographState`, because recording what was written is a state write. The source-file filter inside `project_errors` becomes `source_paths`, which both `project_errors` and `project_artifact_sources` call.

## Artifact sources

The per-file memo lists the work. Each literal is its own `Result`, so the log names the broken literal, and a broken literal in one file does not recompute another file's entries.

```rust
// from crates/isograph_compiler/src/artifacts.rs
/// What one iso literal contributes to generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ArtifactSource {
    Entity(EntityName),
    Field(EntityNameAndSelectableName),
    Entrypoint(EntityNameAndSelectableName),
}

/// An iso literal with at least one parse error: its file and its index among that file's
/// literals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("The iso literal at index {index} in {path} has a parse error, so no artifacts are written.")]
pub struct LiteralParseFailed {
    pub path: RelativePathToSourceFile,
    pub index: usize,
}

/// One entry per iso literal of `path` as `client` reads it, in source order. `None` when the
/// file has no text.
#[memo]
pub fn artifact_sources_in_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<Result<ArtifactSource, LiteralParseFailed>>> {
    let literals = parsed_iso_literals_in_file(db, client, path).as_ref()?;
    literals
        .iter()
        .enumerate()
        .map(|(index, (_, parsed))| {
            artifact_source(parsed).ok_or(LiteralParseFailed { path, index })
        })
        .collect::<Vec<_>>()
        .wrap_some()
}

/// The source of a literal that parsed without errors.
fn artifact_source(parsed: &ParsedIsoLiteral) -> Option<ArtifactSource> {
    if !parsed.errors.is_empty() {
        return None;
    }
    match parsed.item.as_ref()?.item.item.as_ref()?.item.reference() {
        IsoLiteralItem::Entity(entity) => ArtifactSource::Entity(entity.name.item.0),
        IsoLiteralItem::Selectable(declaration) => {
            ArtifactSource::Field(EntityNameAndSelectableName {
                parent_entity_name: declaration.parent_type.item.0,
                selectable_name: declaration.name.item.0,
            })
        }
        IsoLiteralItem::Entrypoint(declaration) => {
            ArtifactSource::Entrypoint(EntityNameAndSelectableName {
                parent_entity_name: declaration.parent_type.item.0,
                selectable_name: declaration.name.item.0,
            })
        }
    }
    .wrap_some()
}
```

`EntityNameAndSelectableName` is the existing `common_lang_types` type (upstream's name). The memo reads only the literal: no schema lookup. A literal that names an entity the schema lacks still generates; validating selections is not a precondition of generation.

The project-wide list is a plain function over `CompileInput`, beside `project_errors`, because the source-file filter reads `ProjectGlobs`, which is not in the database.

```rust
// from crates/isograph_cli/src/compile.rs
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum ArtifactSourcesError {
    #[error("{0}")]
    LiteralParseFailed(LiteralParseFailed),
    #[error("{} is in disk_file_map but has no text", .0.lookup())]
    ListedPathWithoutText(RelativePathToSourceFile),
}

/// Every source file on disk, by the config's `source_files` globs and the host language.
fn source_paths<'a, TProfile: CompilationProfile>(
    input: &'a CompileInput<'_, TProfile>,
) -> impl Iterator<Item = RelativePathToSourceFile> + 'a {
    input.db.disk_paths().into_iter().filter(|path| {
        matches!(
            input
                .globs
                .should_include_source_file::<TProfile>(Path::new(path.lookup())),
            IncludeFile::Include
        )
    })
}

/// The artifact sources of every source file on disk as `input.client` reads them. `Err` names
/// the first literal, in path order, that failed.
pub(crate) fn project_artifact_sources<TProfile: CompilationProfile>(
    input: &CompileInput<'_, TProfile>,
) -> Result<BTreeSet<ArtifactSource>, ArtifactSourcesError> {
    let mut paths: Vec<RelativePathToSourceFile> = source_paths(input).collect();
    paths.sort_by(|a, b| a.lookup().cmp(b.lookup()));
    paths
        .into_iter()
        .map(|path| {
            artifact_sources_in_file(input.db, input.client, path)
                .as_ref()
                .ok_or(ArtifactSourcesError::ListedPathWithoutText(path))
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .map(|source| {
            source
                .dereference()
                .map_err(ArtifactSourcesError::LiteralParseFailed)
        })
        .collect()
}
```

`project_errors` before (compiler-errors.md change 3):

```rust
// from crates/isograph_cli/src/compile.rs
    let per_file = input
        .db
        .disk_paths()
        .into_iter()
        .filter(|path| {
            matches!(
                input
                    .globs
                    .should_include_source_file::<TProfile>(Path::new(path.lookup())),
                IncludeFile::Include
            )
        })
        .map(|path| file_reports(input.db, input.client, path))
        .collect::<Result<Vec<_>, _>>()?;
```

After:

```rust
// from crates/isograph_cli/src/compile.rs
    let per_file = source_paths(input)
        .map(|path| file_reports(input.db, input.client, path))
        .collect::<Result<Vec<_>, _>>()?;
```

`ListedPathWithoutText` is the broken invariant compiler-errors.md names for `project_errors`: a path `disk_paths` listed has a `DiskFile`, and `compiler_client` has no `OpenFile`. `compile` logs it and writes nothing.

## Host language

```rust
// from crates/isograph_compiler/src/artifacts.rs
/// A path inside the `__isograph` folder, `/`-separated, with no `.` or `..` component.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtifactPath(pub String);
```

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait HostLanguage: Send + Sync + Sized + 'static {
    type Error: std::fmt::Display + std::error::Error + Clone + PartialEq + Eq + 'static;
    type LiteralContext: Clone + PartialEq + Eq + Debug + 'static;

    fn extract_iso_literals_from_text(contents: &str) -> Vec<IsoLiteralExtraction<Self>>;

    fn should_include_source_file(relative_path: &Path) -> IncludeFile;

    /// The path and text of every artifact of a project whose iso literals are `sources`.
    fn generate_artifacts(sources: &BTreeSet<ArtifactSource>) -> Vec<(ArtifactPath, String)>;
}
```

`ArtifactPath` is a `String` with `/` separators, not a `PathBuf`, because it is also the source of import specifiers, which are `/`-separated on every platform. `write_artifacts` joins its segments onto the folder.

The `BTreeSet` drops duplicate literals: two `field Pet.fullName` literals produce one overload, and two `entrypoint Query.HomeRoute` literals one artifact. Rejecting duplicates is validation, which this doc does not do.

The test hosts (`TestHostLanguage` in isograph_compiler, `InHost` in isograph_cli's watch tests) return `Vec::new()`. `TypeScriptHostLanguage` (`crates/isograph_extract_typescript`) delegates:

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    fn generate_artifacts(
        sources: &std::collections::BTreeSet<isograph_compiler::ArtifactSource>,
    ) -> Vec<(isograph_compiler::ArtifactPath, String)> {
        typescript_artifacts::generate_artifacts(sources)
    }
```

## typescript_artifacts

`crates/typescript_artifacts` is TypeScript generation: the value layer, a small TypeScript declaration and type layer, the generators, and the one module that converts to swc nodes and prints. It depends on `common_lang_types`, `isograph_compiler`, `prelude`, and swc:

```toml
# from crates/typescript_artifacts/Cargo.toml
swc_common = { version = "=27.0.0", default-features = false }
swc_ecma_ast = { version = "=30.0.2", default-features = false }
swc_ecma_codegen = { version = "=33.0.2", default-features = false }
```

These are the AST crates `swc_core` 84.2.0 uses, the version `crates/swc_isograph_plugin` pins. They are named directly, not through `swc_core`. No serde, encoding, or concurrent feature is enabled. `isograph_extract_typescript` is the only crate that depends on `typescript_artifacts`, because it implements `HostLanguage` for TypeScript; the `isograph` binary depends on it through that crate, and `isograph_cli`'s tests through their TypeScript test profile. `isograph_compiler`, `isograph_lsp`, the parsers, and the `isograph_cli` library do not depend on swc. `src/swc.rs` is the only module that names swc, so an swc bump touches one module.

```rust
// from crates/typescript_artifacts/src/lib.rs
/// The path and text of every artifact of a project whose iso literals are `sources`: one
/// entrypoint artifact per entrypoint, then `iso.ts`.
pub fn generate_artifacts(sources: &BTreeSet<ArtifactSource>) -> Vec<(ArtifactPath, String)>;
```

### Values

```rust
// from crates/typescript_artifacts/src/module.rs
/// A module whose only export is its default export.
#[derive(Clone, Debug, PartialEq)]
pub struct ArtifactModule {
    pub path: ArtifactPath,
    pub default_export: ArtifactValue,
}

/// A value a module exports, in no particular host language.
#[derive(Clone, Debug, PartialEq)]
pub enum ArtifactValue {
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<ArtifactValue>),
    Object(Vec<ArtifactObjectEntry>),
    Import(ArtifactImport),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArtifactObjectEntry {
    pub key: String,
    pub value: ArtifactValue,
}

/// The default export of the module at this path.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtifactImport(pub ArtifactPath);
```

`ArtifactValue` is the value layer reader ASTs and query text will also use; its variants are the user's specification, and the printer tests exercise each. Named exports are not in it: nothing imports one. Every artifact in this doc is a module. When the network protocol produces a raw query file (`query_text.graphql` for GraphQL), a raw-file kind (`path`, exact `contents`) joins it, written and diffed like every other file.

### TypeScript declarations and types

```rust
// from crates/typescript_artifacts/src/module.rs
/// A TypeScript module: its path and its items, in order.
pub(crate) struct TypeScriptModule {
    pub path: ArtifactPath,
    pub items: Vec<TypeScriptItem>,
}

pub(crate) enum TypeScriptItem {
    /// `import <identifier> from "<specifier>";`
    DefaultImport(ArtifactImport),
    /// `import type <identifier> from "<specifier>";`
    TypeImport(ArtifactImport),
    /// `type <name><params> = <type>;`
    TypeAlias(TypeAlias),
    /// `export function <name><params>(<params>): <type>;`
    ExportedOverload(FunctionSignature),
    /// `export function <name><params>(<params>): <type> { throw new Error("<message>"); }`
    ExportedThrowingFunction(ThrowingFunction),
    /// `export default <value>;`
    ExportDefault(ArtifactValue),
}

pub(crate) struct TypeAlias {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub aliased: TypeScriptType,
}

pub(crate) struct TypeParam {
    pub name: String,
    pub constraint: Option<TypeScriptType>,
}

pub(crate) struct FunctionSignature {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<FunctionParam>,
    pub return_type: TypeScriptType,
}

pub(crate) struct FunctionParam {
    pub name: String,
    pub type_: TypeScriptType,
}

pub(crate) struct ThrowingFunction {
    pub signature: FunctionSignature,
    pub message: String,
}

pub(crate) enum TypeScriptType {
    Keyword(TypeScriptKeyword),
    /// `Name<Args>`, or `Name` with no arguments.
    Reference(TypeReference),
    StringLiteral(String),
    TemplateLiteral(TemplateLiteralType),
    Union(Vec<TypeScriptType>),
    Intersection(Vec<TypeScriptType>),
    Array(Box<TypeScriptType>),
    Conditional(Box<ConditionalType>),
    /// `infer Name`
    Infer(String),
    /// `typeof <the import's identifier>`
    TypeOf(ArtifactImport),
    Function(Box<FunctionType>),
}

pub(crate) enum TypeScriptKeyword {
    String,
    Unknown,
    Never,
    Void,
    Object,
}

pub(crate) struct TypeReference {
    pub name: String,
    pub args: Vec<TypeScriptType>,
}

/// `` `<head>${<type>}<text>${<type>}<text>` ``
pub(crate) struct TemplateLiteralType {
    pub head: String,
    pub spans: Vec<TemplateLiteralSpan>,
}

/// A substituted type and the text after it.
pub(crate) struct TemplateLiteralSpan {
    pub type_: TypeScriptType,
    pub text: String,
}

/// `<check> extends <extends> ? <true> : <false>`
pub(crate) struct ConditionalType {
    pub check: TypeScriptType,
    pub extends: TypeScriptType,
    pub true_type: TypeScriptType,
    pub false_type: TypeScriptType,
}

/// `<params>(<params>) => <return>`
pub(crate) struct FunctionType {
    pub type_params: Vec<TypeParam>,
    pub params: Vec<FunctionTypeParam>,
    pub return_type: TypeScriptType,
}

pub(crate) enum FunctionTypeParam {
    /// `name: type`
    Named(FunctionParam),
    /// `...name: type`
    Rest(FunctionParam),
}

/// `artifact` as a TypeScript module: a default import for each distinct import in its value,
/// in path order, then the default export.
pub(crate) fn module_of_artifact(artifact: &ArtifactModule) -> TypeScriptModule;
```

All derive `Clone, Debug, PartialEq`. Each item and type kind is one `iso.ts` or entrypoint artifact needs: object-literal types, `keyof`, indexed access, arrow thunks, and annotated consts join when an artifact needs them.

### Imports

```rust
// from crates/typescript_artifacts/src/imports.rs
/// Every `Import` inside `value`, at any depth.
pub(crate) fn imports_of(value: &ArtifactValue) -> BTreeSet<&ArtifactImport>;

/// The local name of `import`: its path's segments joined by `__`, `.ts` dropped
/// (`Query/HomeRoute/entrypoint.ts` is `Query__HomeRoute__entrypoint`).
pub(crate) fn artifact_import_identifier(import: &ArtifactImport) -> String;

/// The specifier that imports `import` from the module at `from`: the import's path relative
/// to `from`'s folder, `./`-prefixed, `.ts` dropped.
pub(crate) fn artifact_import_specifier(from: &ArtifactPath, import: &ArtifactImport) -> String;
```

Identifiers are a function of the import's path alone, and imports print in path order, so a module whose value did not change prints byte-identically.

### Printing

```rust
// from crates/typescript_artifacts/src/swc.rs
/// The text of `module`, two-space indented.
pub(crate) fn print(module: &TypeScriptModule) -> String;
```

`print` converts each item to an swc `ModuleItem` and emits the module with `swc_ecma_codegen::Emitter` over a `JsWriter` into a `Vec<u8>`, `Config::default().with_target(EsVersion::latest())`, `set_indent_str("  ")`, and a local `Lrc<SourceMap>`. `emit_module(..).expect("writing to a Vec does not fail")`: the emitter's only error is the writer's `io::Error`, and writing into a `Vec` does not fail. The `Lrc` (an `Rc`) is the `SourceMap` handle the emitter's API takes; it is local to `print` and shares nothing. Both were raised with the user.

The conversion:

- Strings and numbers have `raw: None`, so the emitter writes their escapes.
- An object key that is an identifier prints bare; any other key is a string literal.
- A union, intersection, function, or conditional type in an operand position (array element, union or intersection member, conditional check or extends type) is wrapped in `TsParenthesizedType`. swc's emitter adds no parentheses in types: without this, `("a" | "b")[]` prints as `"a" | "b"[]`.
- Template literal type text escapes `\`, `` ` ``, and `${` in `raw`; `cooked` is the text.

### Entrypoint artifact

Path `<Entity>/<selectable>/entrypoint.ts`, the path the transforms import:

```rust
// from crates/typescript_artifacts/src/entrypoint.rs
/// The entrypoint artifact of `entrypoint`, at the path the iso transforms import.
pub(crate) fn entrypoint_artifact(entrypoint: EntityNameAndSelectableName) -> ArtifactModule {
    ArtifactModule {
        path: entrypoint_path(entrypoint),
        default_export: ArtifactValue::Object(vec![
            ArtifactObjectEntry {
                key: "kind".to_owned(),
                value: ArtifactValue::String("Entrypoint".to_owned()),
            },
            ArtifactObjectEntry {
                key: "name".to_owned(),
                value: ArtifactValue::String(format!(
                    "{}.{}",
                    entrypoint.parent_entity_name, entrypoint.selectable_name
                )),
            },
        ]),
    }
}

pub(crate) fn entrypoint_path(entrypoint: EntityNameAndSelectableName) -> ArtifactPath {
    ArtifactPath(format!(
        "{}/{}/entrypoint.ts",
        entrypoint.parent_entity_name, entrypoint.selectable_name
    ))
}
```

Its text:

```ts
export default {
  kind: "Entrypoint",
  name: "Query.HomeRoute"
};
```

### iso.ts

```rust
// from crates/typescript_artifacts/src/iso_overloads.rs
/// `iso.ts`: an `iso` overload per field, entity, and entrypoint literal, and the
/// implementation.
pub(crate) fn iso_overload_module(sources: &BTreeSet<ArtifactSource>) -> TypeScriptModule;

/// Upstream's `sort_field_name` rule over the whole match text: a string sorts before every
/// string it is a prefix of, so a shorter match text cannot shadow a longer one.
pub(crate) fn overload_order(a: &str, b: &str) -> Ordering;
```

Upstream's `build_iso_overload_artifact` (`isograph/crates/artifact_content/src/iso_overload_file.rs`) prints the same declarations from strings. The deltas:

- No `import type { IsographEntrypoint } from '@isograph/react';`. Nothing refers to it: entrypoint overloads return the artifact's type, and the implementation signature does not name it. A project without `@isograph/react` installed type-checks.
- No param-type imports and no `__link` output-type imports: those files are not generated yet.
- `IdentityWithParam<TParam>` and `IdentityWithParamComponent<TParam>` are replaced by one `Identity` type with no param type. `@component` fields, fields with a `to` clause (upstream's `pointer`, whose overload names the target's `__link` output type), and other fields get the same overload.
- `entity` literals get an overload returning `void`. Upstream has no `entity` keyword.
- There is no `no_babel_transform` switch form, because i2 has no such option. The implementation throws, as upstream's default does, without the sentence naming `options.no_babel_transform`.
- Entrypoint imports are `import type`, with `artifact_import_identifier`'s name and the specifier from `iso.ts` (`./Query/HomeRoute/entrypoint`). Upstream writes `'../__isograph/...'`, which names the same file.
- Upstream's comments are not printed: the emitter prints the AST, and the AST carries no comments.
- The type aliases are not exported, as upstream's are not.
- Overloads are ordered by `overload_order`, upstream's `sort_field_name` rule applied to the whole match text, so `field Pet.Avatar` cannot shadow `field Pet.AvatarCard`. Sections are field overloads, then entity overloads, then entrypoint overloads; imports are the entrypoint imports in entrypoint-overload order.

For sources `Field(Pet.fullName)`, `Entity(Owner)`, `Entrypoint(Query.HomeRoute)`, the text is:

```ts
import type Query__HomeRoute__entrypoint from "./Query/HomeRoute/entrypoint";
type Identity = <TClientField extends (...args: never[]) => unknown>(clientField: TClientField) => TClientField;
type WhitespaceCharacter = " " | "	" | "\n";
type Whitespace<In> = In extends `${WhitespaceCharacter}${infer In}` ? Whitespace<In> : In;
type MatchesWhitespaceAndString<TString extends string, T> = Whitespace<T> extends `${TString}${string}` ? T : never;
export function iso<T>(param: T & MatchesWhitespaceAndString<"field Pet.fullName", T>): Identity;
export function iso<T>(param: T & MatchesWhitespaceAndString<"entity Owner", T>): void;
export function iso<T>(param: T & MatchesWhitespaceAndString<"entrypoint Query.HomeRoute", T>): typeof Query__HomeRoute__entrypoint;
export function iso(_isographLiteralText: string): Identity | object | void {
  throw new Error("iso: Unexpected invocation at runtime. Either the Babel transform was not set up, or it failed to identify this call site. Make sure it is being used verbatim as `iso`.");
}
```

The `"	"` literal holds a tab character: the emitter writes tabs in strings as-is. `import type X from` a module whose default export is a value lets `typeof X` name that value's type with no runtime import. The implementation's return union has `object` because every entrypoint artifact's default export is an object.

## Compiling and writing

```rust
// from crates/isograph_cli/src/state.rs
/// The absolute path of `<output_directory>/__isograph`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactFolder(pub PathBuf);

/// Every file under the artifact folder and its text: what the daemon read there at boot,
/// then what it last wrote.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArtifactFiles(pub BTreeMap<ArtifactPath, String>);

pub struct IsographState<TProfile: CompilationProfile> {
    pub db: IsographDb<TProfile>,
    pub live_sessions: LiveSessions,
    pub(crate) globs: ProjectGlobs,
    pub(crate) diagnostics_debounce: Option<freddie_timer::TimerGuard>,
    pub(crate) compile_debounce: Option<freddie_timer::TimerGuard>,
    client_ids: ClientIdSource,
    /// The compiler's reader of file text. No `LiveSession` and no `OpenFile` has it.
    compiler_client: ClientId,
    /// `None` when the config has no `output_directory`.
    pub(crate) artifact_folder: Option<ArtifactFolder>,
    pub(crate) artifact_files: ArtifactFiles,
}

impl<TProfile: CompilationProfile> IsographState<TProfile> {
    pub(crate) fn new(globs: ProjectGlobs, artifact_folder: Option<ArtifactFolder>) -> Self {
        let mut client_ids = ClientIdSource::default();
        let compiler_client = client_ids.mint();
        Self {
            db: IsographDb::default(),
            live_sessions: LiveSessions::default(),
            globs,
            diagnostics_debounce: None,
            compile_debounce: None,
            client_ids,
            compiler_client,
            artifact_folder,
            artifact_files: ArtifactFiles::default(),
        }
    }
}
```

`serve` builds the folder as `config_directory.join(output_directory).join(ISOGRAPH_FOLDER)`. isograph_compiler gains a `serde` dependency for `ArtifactPath`, because `ArtifactFiles` rides in an `InternalEvent`.

The folder's files are external truth, so they arrive as an event, like the boot scan:

```rust
// from crates/isograph_cli/src/event.rs
pub enum InternalEvent {
    HelloWorld,
    Quit,
    DiskChanged(DiskChanged),
    ArtifactFilesRead(ArtifactFiles),
}
```

```rust
// from crates/isograph_cli/src/write_artifacts.rs
/// Every file under `folder` and its text. A missing folder has no files. A file that is not
/// UTF-8 is listed with its lossy text, which matches no artifact, so the next write replaces
/// it.
pub(crate) fn read_artifact_files(folder: &ArtifactFolder) -> ArtifactFiles;
```

`serve` posts `ArtifactFilesRead(read_artifact_files(&folder))` right after the boot scan's `DiskChanged`s, before `select!`, when the config has an output directory. `handle` of `ArtifactFilesRead` replaces `artifact_files` and returns no effects. Every boot `DiskChanged` arms the compile debounce before this event, and the fire comes 100 ms later, so the first compile diffs against the files on disk.

`compile` before (compiler-errors.md change 3):

```rust
// from crates/isograph_cli/src/compile.rs
pub(crate) fn compile<TProfile: CompilationProfile>(
    state: &IsographState<TProfile>,
) -> Vec<IsographEffect> {
    project_errors(state.compile_input().reference()).map_or_else(
        |error| tracing::error!(%error, "compile"),
        |reports| tracing::info!(errors = reports.len(), "compiled"),
    );
    no_effects()
}
```

After:

```rust
// from crates/isograph_cli/src/compile.rs
/// Computes every source file's errors and logs the count, then generates the artifacts.
pub(crate) fn compile<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
) -> Vec<IsographEffect> {
    project_errors(state.compile_input().reference()).map_or_else(
        |error| tracing::error!(%error, "compile"),
        |reports| tracing::info!(errors = reports.len(), "compiled"),
    );
    generate_artifacts(state)
}

/// The write of every artifact whose text differs from the artifact folder's. No effects when
/// the config has no `output_directory`, when any literal failed, or when nothing changed.
fn generate_artifacts<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
) -> Vec<IsographEffect> {
    let Some(folder) = state.artifact_folder.clone() else {
        return no_effects();
    };
    let Some(sources) = project_artifact_sources(state.compile_input().reference())
        .map_err(|error| tracing::warn!(%error, "artifacts not written"))
        .ok()
    else {
        return no_effects();
    };
    let next = ArtifactFiles(
        TProfile::HostLanguage::generate_artifacts(sources.reference())
            .into_iter()
            .collect(),
    );
    state
        .artifact_files
        .replace(next)
        .map(|changes| IsographEffect::WriteArtifacts(WriteArtifactsEffect { folder, changes }.boxed()))
        .into_iter()
        .collect()
}
```

`handle_timer_fired` already holds `state: &mut` and passes it.

```rust
// from crates/isograph_cli/src/compile.rs
impl ArtifactFiles {
    /// Records `next` as the artifact folder's files and returns how the folder must change.
    /// `None` when nothing changes.
    pub(crate) fn replace(&mut self, next: ArtifactFiles) -> Option<ArtifactChanges>;
}
```

`writes` is every file of `next` whose path is not in `self` or whose text differs; `deletes` is every path of `self` not in `next`; both empty is `None`. `next` replaces `self` in every case.

```rust
// from crates/isograph_cli/src/effect.rs
#[derive(Debug)]
pub struct WriteArtifactsEffect {
    pub folder: ArtifactFolder,
    pub changes: ArtifactChanges,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ArtifactChanges {
    pub writes: Vec<(ArtifactPath, String)>,
    pub deletes: Vec<ArtifactPath>,
}

#[derive(Debug)]
pub enum IsographEffect {
    LogHelloWorld,
    Kill,
    SendLspResponse(Box<SendLspResponseEffect>),
    PublishDiagnostics(Box<PublishDiagnosticsEffect>),
    AssignClientId(AssignClientIdEffect),
    Timer(TimerEffect<TimerPayload>),
    WriteArtifacts(Box<WriteArtifactsEffect>),
}
```

`perform` (`crates/isograph_cli/src/daemon.rs`):

```rust
// from crates/isograph_cli/src/daemon.rs
        IsographEffect::WriteArtifacts(write) => {
            let folder = write.folder.clone();
            crate::write_artifacts::write_artifacts(*write).unwrap_or_else(|error| {
                tracing::error!(%error, "could not write the artifacts");
                let _ = event_tx.send(
                    crate::event::InternalEvent::ArtifactFilesRead(
                        crate::write_artifacts::read_artifact_files(folder.reference()),
                    )
                    .to(),
                );
            });
            ControlFlow::Continue(())
        }
```

```rust
// from crates/isograph_cli/src/write_artifacts.rs
/// Applies `effect.changes` under `effect.folder`. Every path it touches is the folder joined
/// with an `ArtifactPath`'s segments.
pub(crate) fn write_artifacts(effect: WriteArtifactsEffect) -> std::io::Result<()>;
```

Each write is `create_dir_all` of the parent and `fs::write`. Each delete is `remove_file`, then `remove_dir` of each ancestor below the folder, innermost first, stopping at the first that is not empty. After a failed write, `perform` reads the folder again and posts what is there, so the next compile diffs against the truth. Reading the folder in `perform` is outer I/O whose result arrives as an event, the shape the watcher has.

`ArtifactPath`s are built from entity and selectable names and fixed file names. The tokenizer admits only `[a-zA-Z_][a-zA-Z0-9_]*` as a name, so no generated `ArtifactPath` has a `..` component, and no write leaves the folder. `read_artifact_files` builds paths from entries under the folder, so no delete leaves it either.

### Watcher skip

Before: `is_skipped_isograph(path, config_directory)` skips `<config dir>/__isograph`. After, `ProjectGlobs` carries the artifact folder relative to the config directory, and every caller of `is_skipped_isograph` asks it:

```rust
// from crates/isograph_cli/src/watch.rs
/// The config's `source_files` and `schema` globs, and the artifact folder the watcher skips.
#[derive(Clone)]
pub(crate) struct ProjectGlobs {
    source_files: Globs,
    schema: Globs,
    /// `<output_directory>/__isograph`, or `__isograph` when the config has none.
    artifact_folder: PathBuf,
}

impl ProjectGlobs {
    pub(crate) fn parse(
        source_files: &[String],
        schema: &[String],
        output_directory: Option<&Path>,
    ) -> Result<Self, WatchError>;

    /// `path` is inside the artifact folder.
    fn is_in_artifact_folder(&self, path: &Path, config_directory: &Path) -> bool {
        path.starts_with(config_directory.join(self.artifact_folder.reference()))
    }
}
```

`visit_dirs_skipping_isograph` and `visit_dirs_skipping_isograph_from` take `&ProjectGlobs`. With no `output_directory`, the skipped folder is `<config dir>/__isograph`, today's behavior and where the transforms look.

## Pet demo

`demos/pet-demo/isograph.config.json` gains `"output_directory": "src"`, so artifacts are in `src/__isograph/`, upstream's demo layout. The hand-written `iso` in `src/iso.ts` is deleted; every source imports `iso` from `./__isograph/iso`. `src/iso.ts` keeps only `ResolverParameters`, and the resolvers keep their `ResolverParameters<...>` annotations while field overloads return `Identity`. Each entrypoint literal's value is logged (`console.log(iso(\`entrypoint Query.HomeRoute\`).name)`), so the demo reads the artifact. Upstream commits its demos' `__isograph` folders, so `demos/pet-demo/src/__isograph/` is committed, generated by running the daemon on a copy of the demo. `tsc --noEmit --strict` over the demo's `src` passes.

## Changes

Each change is one commit. compiler-errors.md changes 1 to 3 come first (landed).

### 1. `output_directory` and the watcher skip

The config field, `ProjectGlobs::artifact_folder`, `is_in_artifact_folder`, `visit_dirs_skipping_isograph*` taking `&ProjectGlobs`, `serve` passing the field to `ProjectGlobs::parse`.

Tests: a config with `"output_directory": "src"` parses to `Some("src")`; without it, `None`. Watch harness with `output_directory` `src`: a file under `src/__isograph/` is not posted and the boot scan skips it; a file under `__isograph/` at the config root is posted. Without `output_directory`, `__isograph/` at the root is skipped and `src/__isograph/` is posted (today's test).

### 2. `artifact_sources_in_file`

`ArtifactSource`, `LiteralParseFailed`, `artifact_sources_in_file` in `crates/isograph_compiler/src/artifacts.rs`.

Tests (`crates/isograph_compiler`, whose test host extracts `` iso(`...`) ``): a file with `field Pet.fullName`, `entity Owner`, and `entrypoint Query.HomeRoute` lists the three sources in order; a literal with a parse error is `Err(LiteralParseFailed { path, index })` and its siblings are `Ok`; `field Pet.bestFriend to Pet { id }` is `Field(Pet.bestFriend)`; a file with no literals is `Some(vec![])`; a path with no text is `None`.

### 3. TypeScript generation

`ArtifactPath`, the trait method and the test hosts' implementations, `crates/typescript_artifacts` (values, declarations and types, imports, the entrypoint artifact, `iso.ts`, `swc.rs`), and `TypeScriptHostLanguage::generate_artifacts`.

Tests (`crates/typescript_artifacts`), facts about values and text:

- `Entrypoint(Query.HomeRoute)` yields `Query/HomeRoute/entrypoint.ts` whose text starts with `export default {` and has `kind: "Entrypoint"` and `name: "Query.HomeRoute"`.
- A module whose value holds one `Import` twice and another once has exactly two import declarations, in path order, and the value names each by its identifier.
- Printing the same module twice is the same text.
- `artifact_import_specifier` from `Query/A/reader.ts` to `Query/A/entrypoint.ts` is `./entrypoint`; from `iso.ts` to `Query/A/entrypoint.ts` is `./Query/A/entrypoint`; from `Query/A/x.ts` to `Pet/B/y.ts` is `./../../Pet/B/y`; from `Query/A/x.ts` to `Query/B/y.ts` is `./../B/y`. The identifier of `Query/HomeRoute/entrypoint.ts` is `Query__HomeRoute__entrypoint`.
- A string with `"`, `'`, `\`, a newline, and `é` prints as one double-quoted literal on one line with `\"`, `\\`, and `\n` escapes and `é` as-is.
- `[null, true, 2, [false], { "not-an-identifier": null }]` prints with that nesting and the key quoted.
- `("a" | "b")[]` and `A & (B | C)` print with their parentheses.
- Template literal text `` `\${ `` prints escaped.
- `iso.ts` imports `Query__HomeRoute__entrypoint` from `./Query/HomeRoute/entrypoint` with `import type`, and the overload matching `"entrypoint Query.HomeRoute"` returns `typeof Query__HomeRoute__entrypoint`.
- `Field(Pet.fullName)` yields the overload matching `"field Pet.fullName"` returning `Identity`, and no import.
- `Entity(Owner)` yields the overload matching `"entity Owner"` returning `void`.
- `Field(Pet.Avatar)` and `Field(Pet.AvatarCard)`: the `AvatarCard` overload comes first.
- Field overloads precede entity overloads, which precede entrypoint overloads.
- No sources: `iso.ts` is the only artifact, it has no import, and it ends with `}\n`.
- Every import in `iso.ts` names an artifact the same call generated.
- No line of `iso.ts` ends in whitespace.

### 4. Generate on the compile fire and write

`ArtifactFolder`, `ArtifactFiles`, `IsographState::new` taking the folder, `serve` building it and posting `ArtifactFilesRead`, `InternalEvent::ArtifactFilesRead`, `source_paths`, `ArtifactSourcesError`, `project_artifact_sources`, `compile` taking `&mut`, `generate_artifacts`, `ArtifactChanges`, `WriteArtifactsEffect`, `read_artifact_files`, `write_artifacts`.

Handle tests (`crates/isograph_cli/src/compile.rs`, TypeScript test profile, artifact folder `/tmp/proj/src/__isograph`):

- The compile fire after the first source file returns one `WriteArtifacts` whose `writes` are `iso.ts` and `Query/HomeRoute/entrypoint.ts` and whose `deletes` is empty.
- Without an artifact folder the fire returns no effects.
- A second fire with no change returns no effects.
- Editing one field literal's selection set (the iso.ts text does not change) returns no effects.
- Adding a field literal returns a write of only `iso.ts`.
- Removing the file with `entrypoint Query.HomeRoute` writes `iso.ts` and deletes `Query/HomeRoute/entrypoint.ts`.
- A source with a parse error returns no effects; fixing it returns the write.
- A file outside the `source_files` globs with a broken literal does not block the write.
- `ArtifactFilesRead` with `iso.ts` already holding the generated text and a stale `Old/x.ts`, then a fire: the write omits `iso.ts` and deletes `Old/x.ts`.

`write_artifacts` and `read_artifact_files` tests (a `tempfile` directory): `read_artifact_files` of a missing folder is empty and of a nested tree lists every file with its text; `write_artifacts` overwrites a listed file, leaves an unlisted file's modification time unchanged, creates a nested file's folders, deletes a stale file and removes its emptied folders, and leaves a file beside the folder untouched.

### 5. Pet demo output

The demo config, `src/iso.ts`, the imports, the logged entrypoints, and the committed generated folder.

## Later

- Query generation adds `queryText: Import(query_text.ts)` to the entrypoint artifact, where `query_text.ts` is an `ArtifactModule` whose default export is the protocol-produced query string. When the network protocol also produces a raw query file (`query_text.graphql` for GraphQL), a raw-file artifact kind (path plus exact contents) joins modules, written and diffed like every other file.
- Param types per field (upstream's `param_type.ts`): field overloads return `IdentityWithParam<Parent__field__param>` and type the resolver's parameter. Then `ResolverParameters` and every use of it are deleted from the pet demo.
- The Babel and SWC transforms accept `entrypoint`, `field`, and `pointer` literals; an `entity` literal (the pet demo's `Owner.ts` has one) is a transform error in both. Teaching the transforms `entity` is separate work.

## Docs this doc amends

docs-website/docs/design-docs/event-model.md, in change 4: the `IsographEffect` listing gains `WriteArtifacts(Box<WriteArtifactsEffect>)`; `InternalEvent` gains `ArtifactFilesRead(ArtifactFiles)`; the Inner paragraph gains "After computing errors, the compile fire generates every artifact when the config names an `output_directory`, and returns `WriteArtifacts` with the files whose text differs from `artifact_files`." The State paragraph's "Artifact generation will be an effect `handle` returns after compile." becomes "Artifact generation is the `WriteArtifacts` effect `handle` returns after compile." The Outer effect-loop bullet gains `WriteArtifacts`.
