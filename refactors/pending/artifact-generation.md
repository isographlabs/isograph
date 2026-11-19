# Artifact generation: iso.ts and entrypoint artifacts

The daemon generates artifacts into an optional output directory. The config field is `artifact_directory`, upstream's name and semantics: the compiler writes into `<config dir>/<artifact_directory>/__isograph/`. The Babel and SWC transforms read the same field (upstream `swc_isograph_plugin` `path_for_artifact` joins `artifact_directory` and `__isograph`). When the field is absent, the daemon writes nothing.

The artifacts in this doc are the top-level `iso.ts` in upstream's format and one entrypoint artifact per entrypoint literal. Per-field param types, output types, readers, query text, and normalization ASTs are later docs. Generation is host-language work: `HostLanguage::generate_artifacts`. `NetworkProtocol` gains nothing.

All artifacts are generated in memory. Only when every one was generated (no literal failed) are they written, together, and only the files that changed.

## End-user experience

```json
{
  "source_files": ["src/**/*.ts"],
  "schema": ["schema/**/*.graphql"],
  "artifact_directory": "src"
}
```

The user runs the daemon (`isograph lsp` from the editor, or `isograph start`). After the boot scan, and after every later burst of file changes settles for 100 ms, the daemon generates every artifact in memory. If every iso literal in every source file parses, the daemon writes the changes into `src/__isograph/`:

```text
src/__isograph/iso.ts
src/__isograph/Query/HomeRoute/entrypoint.ts
src/__isograph/Mutation/SetTagline/entrypoint.ts
```

The user imports `iso` from the generated file:

```ts
import { iso } from './__isograph/iso';

export const fullName = iso(`
  field Pet.fullName { firstName lastName }
`)(function fullName({ data }: ResolverParameters<{ firstName: string; lastName: string }>) {
  return `${data.firstName} ${data.lastName}`;
});

const homeRoute = iso(`entrypoint Query.HomeRoute`);
// typeof homeRoute is "Query.HomeRoute"
```

A `field` literal returns an identity function: the exported value has exactly the type of the resolver passed to it. An `entrypoint` literal has the type of its entrypoint artifact's default export. `` iso(`entity Owner`) `` returns `void`. A literal whose text matches no overload is a TypeScript error at the call site.

If any iso literal in any source file has a parse error, the daemon writes nothing and logs one warning naming the literal. Files already on disk stay as they are. Fixing the literal makes the next compile write.

Only changed files are written. A file whose contents did not change is not touched (its modification time stays). A file that is no longer generated is deleted, and a folder that becomes empty is removed. The first write after the daemon starts deletes `src/__isograph/` and writes every artifact, so stale files from an earlier run are gone. Nothing outside `src/__isograph/` is created, written, or deleted.

The watcher does not post files under `<config dir>/<artifact_directory>/__isograph/`, so generated files never become source files.

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
    /// The directory, relative to the config file's directory, that holds the generated
    /// `__isograph` folder. Absent: no artifacts are written.
    #[serde(default)]
    pub artifact_directory: Option<PathBuf>,
}
```

Upstream's `artifact_directory` defaults to `project_root`. i2 has no `project_root`; absent means no output.

## Prerequisites from compiler-errors.md

refactors/pending/compiler-errors.md changes 1 to 3 land first: the boot scan is posted before the LSP port accepts, `handle` mints every `ClientId` from `IsographState.client_ids` with `compiler_client` minted first in `IsographState::new`, and every applied `DiskChanged` arms `compile_debounce`, whose fire calls `crate::compile::compile`. Generation reads as `compiler_client` through `CompileInput` and runs inside `compile`.

This doc changes three things in that code. `IsographState::new` takes the artifact directory. `compile` takes `&mut IsographState`, because recording what was written is a state write. The source-file filter of `project_errors` becomes `source_paths`, which both `project_errors` and `project_artifact_sources` call.

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
#[error(
    "The iso literal at index {index} in {} has a parse error, so no artifacts are written.",
    .path.lookup()
)]
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
        .map(|source| source.clone().map_err(ArtifactSourcesError::LiteralParseFailed))
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

## Generation

```rust
// from crates/isograph_compiler/src/artifacts.rs
/// A path relative to the artifact directory.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtifactPath(pub PathBuf);

/// A generated file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Artifact {
    pub path: ArtifactPath,
    pub contents: String,
}
```

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait HostLanguage: Send + Sync + Sized + 'static {
    type Error: std::fmt::Display + std::error::Error + Clone + PartialEq + Eq + 'static;
    type LiteralContext: Clone + PartialEq + Eq + Debug + 'static;

    fn extract_iso_literals_from_text(contents: &str) -> Vec<IsoLiteralExtraction<Self>>;

    fn should_include_source_file(relative_path: &Path) -> IncludeFile;

    /// Every artifact of a project whose iso literals are `sources`.
    fn generate_artifacts(sources: &BTreeSet<ArtifactSource>) -> Vec<Artifact>;
}
```

The `BTreeSet` drops duplicate literals: two `field Pet.fullName` literals produce one overload, and two `entrypoint Query.HomeRoute` literals one artifact and one import. Rejecting duplicates is validation, which this doc does not do.

The test hosts (`TestHostLanguage` in isograph_compiler, `InHost` in isograph_cli's watch tests) return `Vec::new()`.

`TypeScriptHostLanguage::generate_artifacts` lives in `crates/isograph_extract_typescript/src/generate_artifacts.rs` and returns the entrypoint artifacts, then `iso.ts`.

### Entrypoint artifact

Path `<Parent>/<name>/entrypoint.ts`, upstream's entrypoint file in the per-field folder. For `entrypoint Query.HomeRoute` the contents are this line and a newline:

```ts
export default "Query.HomeRoute";
```

The default export's type is the string literal type `"Query.HomeRoute"`.

### iso.ts

Path `iso.ts`. Upstream's `build_iso_overload_artifact` (`isograph/crates/artifact_content/src/iso_overload_file.rs`) with these deltas:

- No `import type { IsographEntrypoint } from '@isograph/react';`. Nothing refers to it: entrypoint overloads return the artifact's type, and the implementation signature does not name it. A project without `@isograph/react` installed type-checks.
- No param-type imports and no `__link` output-type imports: those files are not generated yet.
- `IdentityWithParam<TParam>` and `IdentityWithParamComponent<TParam>` are replaced by one `Identity` type with no param type. `@component` fields, fields with a `to` clause (upstream's `pointer`, whose overload names the target's `__link` output type), and other fields get the same overload.
- `entity` literals get an overload returning `void`. Upstream has no `entity` keyword.
- There is no `no_babel_transform` switch form, because i2 has no such option. The implementation throws, as upstream's default does, without the sentence naming `options.no_babel_transform`.
- Overloads are ordered by comparing the matched strings byte by byte with end-of-string after every byte. That is upstream's `sort_field_name` rule (a string sorts before every string it is a prefix of) applied to the whole `field Parent.name` text, so `field Pet.Avatar` cannot shadow `field Pet.AvatarCard`.

For sources `Field(Pet.fullName)`, `Entity(Owner)`, `Entrypoint(Query.HomeRoute)`, the file is exactly:

````ts
import entrypoint_Query__HomeRoute from '../__isograph/Query/HomeRoute/entrypoint';

// This is the type given to client fields. The exported iso literal
// has exactly the type of the passed-in function.
type Identity = <TClientField extends (...args: never[]) => unknown>(
  clientField: TClientField
) => TClientField;

type WhitespaceCharacter = ' ' | '\t' | '\n';
type Whitespace<In> = In extends `${WhitespaceCharacter}${infer In}`
  ? Whitespace<In>
  : In;

// This is a recursive TypeScript type that matches strings that
// start with whitespace, followed by TString. So e.g. if we have
// ```
// export function iso<T>(
//   isographLiteralText: T & MatchesWhitespaceAndString<'field Query.foo', T>
// ): Bar;
// ```
// then, when you call
// ```
// const x = iso(`
//   field Query.foo ...
// `);
// ```
// then the type of `x` will be `Bar`, both in VSCode and when running
// tsc. This is how we achieve type safety — you can only use fields
// that you have explicitly selected.
type MatchesWhitespaceAndString<
  TString extends string,
  T
> = Whitespace<T> extends `${TString}${string}` ? T : never;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.fullName', T>
): Identity;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entity Owner', T>
): void;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.HomeRoute', T>
): typeof entrypoint_Query__HomeRoute;

export function iso(_isographLiteralText: string):
  | Identity
  | string
  | void
{
  throw new Error('iso: Unexpected invocation at runtime. Either the Babel transform ' +
      'was not set up, or it failed to identify this call site. Make sure it ' +
      'is being used verbatim as `iso`.');
}
````

followed by one newline. Sections are field overloads, then entity overloads, then entrypoint overloads, each sorted by the rule above. The import list is the entrypoint imports in entrypoint-overload order, one per line, then one blank line. With no entrypoints the file starts with the blank line, as upstream's does. The import path `'../__isograph/<Parent>/<name>/entrypoint'` is upstream's. The identifier is `entrypoint_` followed by `EntityNameAndSelectableName::underscore_separated()`.

The preamble keeps the comment block above `MatchesWhitespaceAndString` verbatim. It contains the text `` iso(` ``, which the TypeScript extractor would read as a literal; the watcher skip below keeps generated files out of the source files.

## Compiling and writing

```rust
// from crates/isograph_cli/src/state.rs
/// The absolute path of the generated `__isograph` folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactDirectory(pub PathBuf);

/// What the daemon last wrote into the artifact directory.
#[derive(Debug)]
pub enum WrittenArtifacts {
    /// Nothing written since the daemon started, or the last write failed.
    Unknown,
    Written(BTreeMap<ArtifactPath, String>),
}

pub struct IsographState<TProfile: CompilationProfile> {
    pub db: IsographDb<TProfile>,
    pub live_sessions: LiveSessions,
    pub(crate) globs: ProjectGlobs,
    pub(crate) diagnostics_debounce: Option<freddie_timer::TimerGuard>,
    pub(crate) compile_debounce: Option<freddie_timer::TimerGuard>,
    client_ids: ClientIdSource,
    /// The compiler's reader of file text. No `LiveSession` and no `OpenFile` has it.
    compiler_client: ClientId,
    /// `None` when the config has no `artifact_directory`.
    pub(crate) artifact_directory: Option<ArtifactDirectory>,
    pub(crate) written_artifacts: WrittenArtifacts,
}

impl<TProfile: CompilationProfile> IsographState<TProfile> {
    pub(crate) fn new(globs: ProjectGlobs, artifact_directory: Option<ArtifactDirectory>) -> Self {
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
            artifact_directory,
            written_artifacts: WrittenArtifacts::Unknown,
        }
    }
}
```

`serve` passes `config.artifact_directory.map(|directory| ArtifactDirectory(config_directory.join(directory).join(ISOGRAPH_FOLDER)))`. Tests pass `None` unless they test generation.

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

/// The write of every artifact that changed since the last write. No effects when the config
/// has no `artifact_directory`, when any literal failed, or when nothing changed.
fn generate_artifacts<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
) -> Vec<IsographEffect> {
    let Some(directory) = state.artifact_directory.clone() else {
        return no_effects();
    };
    let Some(sources) = project_artifact_sources(state.compile_input().reference())
        .map_err(|error| tracing::warn!(%error, "artifacts not written"))
        .ok()
    else {
        return no_effects();
    };
    let artifacts = TProfile::HostLanguage::generate_artifacts(sources.reference());
    state
        .written_artifacts
        .replace(artifacts)
        .map(|changes| {
            IsographEffect::WriteArtifacts(WriteArtifactsEffect { directory, changes }.boxed())
        })
        .into_iter()
        .collect()
}
```

`handle_timer_fired` calls `crate::compile::compile(state)` with `state: &mut`, which it already holds.

```rust
// from crates/isograph_cli/src/compile.rs
impl WrittenArtifacts {
    /// Records `artifacts` as written and returns how the directory must change. `None` when
    /// nothing changes.
    pub(crate) fn replace(&mut self, artifacts: Vec<Artifact>) -> Option<ArtifactChanges>;
}
```

`Unknown` becomes `Written` and the result is `Replace(artifacts)`. From `Written(old)`: `writes` is every artifact whose path is not in `old` or whose contents differ; `deletes` is every path of `old` not among `artifacts`; both empty is `None`. The new map replaces `old` in every case.

```rust
// from crates/isograph_cli/src/effect.rs
#[derive(Debug)]
pub struct WriteArtifactsEffect {
    pub directory: ArtifactDirectory,
    pub changes: ArtifactChanges,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ArtifactChanges {
    /// Delete the directory, then write every artifact.
    Replace(Vec<Artifact>),
    Update(ArtifactUpdate),
}

#[derive(Debug, PartialEq, Eq)]
pub struct ArtifactUpdate {
    pub writes: Vec<Artifact>,
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
            crate::write_artifacts::write_artifacts(*write).unwrap_or_else(|error| {
                tracing::error!(%error, "could not write the artifacts");
                let _ = event_tx.send(crate::event::InternalEvent::ArtifactWriteFailed.to());
            });
            ControlFlow::Continue(())
        }
```

```rust
// from crates/isograph_cli/src/write_artifacts.rs
/// Applies `effect.changes` under `effect.directory`. Every path it touches is the directory
/// joined with an `ArtifactPath`.
pub(crate) fn write_artifacts(effect: WriteArtifactsEffect) -> std::io::Result<()>;
```

`Replace`: `remove_dir_all(directory)` (`NotFound` is fine), then for each artifact `create_dir_all` of its parent and `fs::write`. `Update`: each write the same way; each delete `remove_file`, then `remove_dir` of each ancestor below the directory, innermost first, stopping at the first that is not empty.

```rust
// from crates/isograph_cli/src/event.rs
pub enum InternalEvent {
    HelloWorld,
    Quit,
    DiskChanged(DiskChanged),
    ArtifactWriteFailed,
}
```

`handle` of `ArtifactWriteFailed` sets `written_artifacts` to `Unknown`, so the next compile replaces the directory. It arms nothing; the next `DiskChanged` compiles.

`ArtifactPath`s are built from entity and selectable names (`Query/HomeRoute/entrypoint.ts`) and the fixed `iso.ts`. The tokenizer admits only `[_A-Za-z][_0-9A-Za-z]*` as a name, so no `ArtifactPath` has `/` in a name segment or a `..` component, and no write leaves the directory.

### Watcher skip

Before: `is_skipped_isograph(path, config_directory)` skips `<config dir>/__isograph`. After, `ProjectGlobs` carries the generated folder relative to the config directory, and every caller of `is_skipped_isograph` asks it:

```rust
// from crates/isograph_cli/src/watch.rs
/// The config's `source_files` and `schema` globs, and the generated folder the watcher skips.
#[derive(Clone)]
pub(crate) struct ProjectGlobs {
    source_files: Globs,
    schema: Globs,
    /// `<artifact_directory>/__isograph`, or `__isograph` when the config has none.
    artifact_folder: PathBuf,
}

impl ProjectGlobs {
    pub(crate) fn parse(
        source_files: &[String],
        schema: &[String],
        artifact_directory: Option<&Path>,
    ) -> Result<Self, WatchError>;

    /// `path` is inside the generated folder.
    fn is_in_artifact_folder(&self, path: &Path, config_directory: &Path) -> bool {
        path.starts_with(config_directory.join(self.artifact_folder.reference()))
    }
}
```

`visit_dirs_skipping_isograph` and `visit_dirs_skipping_isograph_from` take `&ProjectGlobs`. With no `artifact_directory`, the skipped folder is `<config dir>/__isograph`, today's behavior.

## Pet demo

`demos/pet-demo/isograph.config.json` gains `"artifact_directory": "src"`. `src/iso.ts` keeps only `ResolverParameters`; every source imports `iso` from `./__isograph/iso`. Upstream commits its demos' `__isograph` folders, so `demos/pet-demo/src/__isograph/` is committed, generated by running the daemon on a copy of the demo. `tsc --noEmit --strict` over the demo's `src` passes.

## Changes

Each change is one commit. compiler-errors.md changes 1 to 3 come first, as written there.

### 1. `artifact_directory` and the watcher skip

The config field, `ProjectGlobs::artifact_folder`, `is_in_artifact_folder`, `visit_dirs_skipping_isograph*` taking `&ProjectGlobs`.

Tests: a config with `"artifact_directory": "src"` parses to `Some("src")`; without it, `None`. Watch harness with `artifact_directory` `src`: a file under `src/__isograph/` is not posted and the boot scan skips it; a file under `__isograph/` at the config root is posted. Without `artifact_directory`, `__isograph/` at the root is skipped and `src/__isograph/` is posted (today's test).

### 2. `artifact_sources_in_file`

`ArtifactSource`, `LiteralParseFailed`, `artifact_sources_in_file` in `crates/isograph_compiler/src/artifacts.rs`.

Tests (`crates/isograph_extract_typescript`, whose host extracts literals): a file with `field Pet.fullName`, `entity Owner`, and `entrypoint Query.HomeRoute` lists the three sources in order; a literal with a parse error is `Err(LiteralParseFailed { path, index })` and its siblings are `Ok`; `field Pet.bestFriend to Pet { id }` is `Field(Pet.bestFriend)`; a file with no literals is `Some(vec![])`; a path with no text is `None`.

### 3. `generate_artifacts` for TypeScript

`ArtifactPath`, `Artifact`, the trait method, the test hosts' empty implementations, `TypeScriptHostLanguage::generate_artifacts` in `crates/isograph_extract_typescript/src/generate_artifacts.rs`.

Tests, facts about the text:

- `Entrypoint(Query.HomeRoute)` yields an artifact at `Query/HomeRoute/entrypoint.ts` whose contents are `export default "Query.HomeRoute";\n`.
- `iso.ts` contains `import entrypoint_Query__HomeRoute from '../__isograph/Query/HomeRoute/entrypoint';` and the overload whose matched string is `'entrypoint Query.HomeRoute'` returns `typeof entrypoint_Query__HomeRoute;`.
- `Field(Pet.fullName)` yields the overload matching `'field Pet.fullName'` returning `Identity;`, and no import.
- `Entity(Owner)` yields the overload matching `'entity Owner'` returning `void;`.
- `Field(Pet.Avatar)` and `Field(Pet.AvatarCard)`: the `AvatarCard` overload comes first.
- Field overloads precede entity overloads, which precede entrypoint overloads.
- No sources: `iso.ts` is the only artifact, it has no `import`, and it ends with `}\n`.
- Every entrypoint import path names an artifact the same call returned.
- No line of any artifact ends in whitespace.

### 4. Generate on the compile fire and write

`ArtifactDirectory`, `WrittenArtifacts`, `IsographState::new` taking the directory, `serve` building it, `source_paths`, `ArtifactSourcesError`, `project_artifact_sources`, `compile` taking `&mut`, `generate_artifacts`, `ArtifactChanges`, `ArtifactUpdate`, `WriteArtifactsEffect`, `write_artifacts`, `InternalEvent::ArtifactWriteFailed`.

Handle tests (`crates/isograph_cli/src/compile.rs`, TypeScript test profile, artifact directory `/tmp/proj/src/__isograph`):

- The compile fire after the first source file returns one `WriteArtifacts` with `Replace`, containing `iso.ts` and `Query/HomeRoute/entrypoint.ts`.
- Without an artifact directory the fire returns no effects.
- A second fire with no change returns no effects.
- Editing one field literal's selection set (the iso.ts text does not change) returns no effects.
- Adding a field literal returns `Update` whose `writes` is only `iso.ts` and whose `deletes` is empty.
- Removing the file with `entrypoint Query.HomeRoute` returns `Update` with `iso.ts` in `writes` and `Query/HomeRoute/entrypoint.ts` in `deletes`.
- A source with a parse error returns no effects; fixing it returns the write.
- A file outside the `source_files` globs with a broken literal does not block the write.
- `ArtifactWriteFailed` followed by a fire returns `Replace`.

`write_artifacts` tests (a `tempfile` directory): `Replace` removes a pre-existing file in the directory and writes every artifact; `Update` leaves an unlisted file's modification time unchanged, overwrites the listed one, deletes the stale file, and removes its emptied folders; a file beside the directory is untouched by both.

### 5. Pet demo output

The demo config, `src/iso.ts`, the imports, and the committed generated folder.

## Docs this doc amends

docs-website/docs/design-docs/event-model.md, in change 4: the `IsographEffect` listing gains `WriteArtifacts(Box<WriteArtifactsEffect>)`; `InternalEvent` gains `ArtifactWriteFailed`; the Inner paragraph gains "After computing errors, the compile fire generates every artifact when the config names an `artifact_directory`, and returns `WriteArtifacts` with what changed since the last write." The Outer effect-loop bullet gains `WriteArtifacts`.
