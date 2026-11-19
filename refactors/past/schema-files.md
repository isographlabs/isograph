# Schema files

`isograph.config.json` names two glob lists: `source_files` and `schema`. Both are relative to the config file's directory. The watcher stores the text of every source file and every schema file in `disk_file_map`, so `file_contents` reads both unchanged. `IsographDb` gains a tracked set of the disk paths that are schema files, so a later reader that iterates schema paths does not re-invoke when a TypeScript file is added or removed.

The GraphQL schema parser does not exist yet. Nothing in production reads the schema set. The memo `schema_file_paths(db, client)` (schema paths on disk plus the client's open paths under the schema globs) arrives with the parser integration, together with a place for the schema globs that a memo can read (a database singleton; `IsographState` is out of a memo's reach).

## What the user does

```json
{
  "source_files": ["src/**/*.ts", "src/**/*.tsx"],
  "schema": ["schema/**/*.graphql", "schema/**/*.graphqls"]
}
```

`schema` is optional and defaults to `[]`. Every existing config keeps working. An unknown key is still a parse error. An invalid glob in `schema` stops the daemon at startup and logs `invalid schema glob {glob}: {error}`.

`isograph start` reads `schema/a.graphql` at boot and on every change. A `.graphql` file outside the `schema` globs is not read. A `.ts` file under the `schema` globs is not a schema file (it is a source file only if `source_files` also matches it). Nothing user-visible consumes schema files in this change.

## Classification

A path is a source file when it matches `source_files` and `TProfile::HostLanguage::should_include_source_file` returns `Include`. A path is a schema file when it matches `schema` and `TProfile::NetworkProtocol::should_include_schema_file` returns `Include`. The two predicates are independent.

The watcher reads and posts a path that is a source file or a schema file. `handle` stores every `DiskChanged::File` `Present` in `disk_file_map`, as today, and also adds the path to `schema_file_set` when the path is a schema file. `handle` evaluates the schema predicate itself, from the globs on `IsographState`, so an injected `DiskChanged` (`isograph send`) is classified exactly like a watched one and the wire format does not change.

A path that satisfies both predicates is both: one `DiskFile`, listed in `schema_file_set`, and treated as a source file by every source-file reader (diagnostics). No special case and no startup error exist for it. With the shipped profile the intersection is empty: `TypeScriptHostLanguage` includes `ts`, `tsx`, `js`, `jsx`, and `GraphQLNetworkProtocol` includes `graphql`, `graphqls`. A startup error is not possible from the globs alone (whether two glob lists overlap on an existing path is known only per file), and "schema wins" would add a branch to the source predicate that no shipped profile can reach.

## Types

```rust
// from crates/isograph_config/src/compilation_options.rs
/// This struct is deserialized from an isograph.config.json file.
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
}
```

Delta: `schema`. There is no JSON Schema file for the config in this repo (no `schemars`), so nothing else changes.

```rust
// from crates/isograph_compiler/src/database.rs
#[derive(Debug, Db)]
pub struct IsographDb<TProfile: CompilationProfile> {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
    #[tracked]
    open_file_map: OpenFileMap,
    #[tracked]
    schema_file_set: SchemaFileSet,
    phantom_data: PhantomData<TProfile>,
}

/// The disk paths that are schema files. Each is a key of `disk_file_map`.
#[derive(Debug, Default)]
pub struct SchemaFileSet(pub HashSet<RelativePathToSourceFile>);
```

`Default for IsographDb` adds `schema_file_set: SchemaFileSet::default()`. The set holds paths, not `SourceId<DiskFile>`: the text is read through `file_contents` (an open buffer wins over disk), so a `SourceId` in this set would have no reader. `lib.rs` re-exports `SchemaFileSet`.

```rust
// from crates/isograph_compiler/src/host_language.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IncludeFile {
    Include,
    Skip,
}

pub trait HostLanguage: Send + Sync + Sized + 'static {
    type Error: std::fmt::Display + std::error::Error + Clone + PartialEq + Eq + 'static;
    type LiteralContext: Clone + PartialEq + Eq + Debug + 'static;

    fn extract_iso_literals_from_text(contents: &str) -> Vec<IsoLiteralExtraction<Self>>;

    fn should_include_source_file(relative_path: &Path) -> IncludeFile;
}

/// The way the compiler communicates with the upstream.
pub trait NetworkProtocol: 'static {
    fn should_include_schema_file(relative_path: &Path) -> IncludeFile;
}
```

Before:

```rust
// from crates/isograph_compiler/src/host_language.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipSourceFile {
    Skip,
    Keep,
}

    fn should_skip_source_file(relative_path: &Path) -> SkipSourceFile;

pub trait NetworkProtocol: 'static {}
```

`SkipSourceFile` becomes `IncludeFile` because a schema predicate returns it too, and `Keep` becomes `Include` to match `should_include_*`. The two states are unchanged.

```rust
// from crates/graphql_network_protocol/src/lib.rs
impl NetworkProtocol for GraphQLNetworkProtocol {
    fn should_include_schema_file(relative_path: &Path) -> IncludeFile {
        match relative_path.extension().and_then(|e| e.to_str()) {
            Some("graphql" | "graphqls") => IncludeFile::Include,
            _ => IncludeFile::Skip,
        }
    }
}
```

The body is the shape of `TypeScriptHostLanguage::should_include_source_file`.

```rust
// from crates/isograph_cli/src/watch.rs
/// The config's `source_files` and `schema` globs.
#[derive(Clone)]
pub(crate) struct ProjectGlobs {
    source_files: Globs,
    schema: Globs,
}

/// Include and `!`-exclude globs; the last match wins.
#[derive(Clone)]
pub(crate) struct Globs {
    patterns: Vec<GlobPattern>,
}

#[derive(Debug)]
pub struct InvalidGlob {
    pub glob: String,
    pub source: globset::Error,
}

#[derive(Debug, thiserror::Error)]
pub enum WatchError {
    #[error("could not watch {}: {}", .0.path.display(), .0.source)]
    Watch(WatchRoot),
    #[error("could not start the notify watcher: {0}")]
    Notify(notify::Error),
    #[error("invalid source_files glob {}: {}", .0.glob, .0.source)]
    SourceFilesGlob(InvalidGlob),
    #[error("invalid schema glob {}: {}", .0.glob, .0.source)]
    SchemaGlob(InvalidGlob),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangedFileKind {
    Config,
    File,
    Folder,
}
```

Before: `SourceGlobs { patterns }` (renamed `Globs`, body unchanged), `WatchError::Glob { glob: String, source: globset::Error }`, and `ChangedFileKind::{SourceFile, SourceFolder}`. `File` and `Folder` carry schema files as well as source files, so `Source` no longer names them. `GlobPattern` and `ExcludeGlob` are unchanged.

```rust
// from crates/isograph_cli/src/state.rs
pub struct IsographState<TProfile: CompilationProfile> {
    pub db: IsographDb<TProfile>,
    pub live_sessions: LiveSessions,
    pub(crate) globs: crate::watch::ProjectGlobs,
    pub(crate) diagnostics_debounce: Option<freddie_timer::TimerGuard>,
}
```

was `source_globs: crate::watch::SourceGlobs`.

## Functions

```rust
// from crates/isograph_cli/src/watch.rs
impl ProjectGlobs {
    pub(crate) fn parse(source_files: &[String], schema: &[String]) -> Result<Self, WatchError> {
        ProjectGlobs {
            source_files: Globs::parse(source_files).map_err(WatchError::SourceFilesGlob)?,
            schema: Globs::parse(schema).map_err(WatchError::SchemaGlob)?,
        }
        .wrap_ok()
    }

    /// Matches `source_files` and the host language includes it.
    pub(crate) fn should_include_source_file<TProfile: CompilationProfile>(
        &self,
        relative: &Path,
    ) -> IncludeFile {
        if !self.source_files.contains(relative) {
            return IncludeFile::Skip;
        }
        TProfile::HostLanguage::should_include_source_file(relative)
    }

    /// Matches `schema` and the network protocol includes it.
    pub(crate) fn should_include_schema_file<TProfile: CompilationProfile>(
        &self,
        relative: &Path,
    ) -> IncludeFile {
        if !self.schema.contains(relative) {
            return IncludeFile::Skip;
        }
        TProfile::NetworkProtocol::should_include_schema_file(relative)
    }

    /// A source file or a schema file.
    fn should_include_file<TProfile: CompilationProfile>(&self, relative: &Path) -> IncludeFile {
        match self.should_include_source_file::<TProfile>(relative) {
            IncludeFile::Include => IncludeFile::Include,
            IncludeFile::Skip => self.should_include_schema_file::<TProfile>(relative),
        }
    }
}

impl Globs {
    pub(crate) fn parse(globs: &[String]) -> Result<Self, InvalidGlob> {
        // body of SourceGlobs::parse; the error is InvalidGlob { glob: raw.clone(), source }
    }

    pub(crate) fn contains(&self, relative: &Path) -> bool {
        // body of SourceGlobs::contains, unchanged
    }
}
```

`Globs::contains` keeps its `bool`: it is the yes/no of one glob list, and it is private to the predicates above and `categorize_file`.

The watcher receives the parsed globs instead of re-parsing a `&[String]` on every batch:

```rust
// from crates/isograph_cli/src/watch.rs
pub fn start_if_watching<TProfile: CompilationProfile>(
    filesystem: Filesystem,
    watch_tx: UnboundedSender<Vec<SourceFileEvent>>,
    config_path: &Path,
    globs: ProjectGlobs,
) -> Result<Option<Watcher>, WatchError>

#[expect(clippy::extra_unused_type_parameters)]
pub fn start<TProfile: CompilationProfile>(
    watch_tx: UnboundedSender<Vec<SourceFileEvent>>,
    config_path: &Path,
    globs: ProjectGlobs,
) -> Result<Watcher, WatchError>

pub fn apply<TProfile: CompilationProfile>(
    event_tx: &UnboundedSender<IsographEvent>,
    config_directory: &Path,
    events: Vec<SourceFileEvent>,
    globs: &ProjectGlobs,
)
```

Before: `source_files: &[String]` in all three; `start` and `apply` each called `SourceGlobs::parse`, and `apply` logged `invalid source_files` and returned on error. `categorize_and_filter_events`, `process_*`, `categorize_*`, `scan_folder`, and `post_file` take `&ProjectGlobs` in place of `&SourceGlobs`.

`categorize_file` matches either glob list:

```rust
// from crates/isograph_cli/src/watch.rs
    let relative = pathdiff::diff_paths(path, config_directory)?;
    if globs.source_files.contains(relative.reference()) || globs.schema.contains(relative.reference()) {
        ChangedFileKind::File.wrap_some()
    } else {
        None
    }
```

was `if globs.contains(relative.reference())`.

`post_file` reads a source file or a schema file:

```rust
// from crates/isograph_cli/src/watch.rs
    match globs.should_include_file::<TProfile>(relative.reference()) {
        IncludeFile::Skip => return,
        IncludeFile::Include => {}
    }
```

Before:

```rust
// from crates/isograph_cli/src/watch.rs
    if !globs.contains(relative.reference()) {
        return;
    }
    match TProfile::HostLanguage::should_skip_source_file(relative.reference()) {
        SkipSourceFile::Skip => return,
        SkipSourceFile::Keep => {}
    }
```

The boot scan (`visit_dirs_skipping_isograph`, then `categorize_file`, then `apply`, then `post_file`) and folder scans pick up schema files through these two changes and need no other edit.

Database writers:

```rust
// from crates/isograph_compiler/src/database.rs
    /// Inserts a disk file that is a schema file.
    pub fn insert_schema_file(&mut self, path: RelativePathToSourceFile, contents: String) {
        self.insert_disk_file(path, contents);
        if self.get_schema_file_set().untracked().0.contains(&path) {
            return;
        }
        self.get_schema_file_set_mut().tracked().0.insert(path);
    }

    pub fn remove_disk_file(&mut self, path: RelativePathToSourceFile) {
        if let Some(source_id) = self.get_disk_file_map_mut().tracked().0.remove(&path) {
            self.remove(source_id);
        }
        if self.get_schema_file_set().untracked().0.contains(&path) {
            self.get_schema_file_set_mut().tracked().0.remove(&path);
        }
    }

    pub fn remove_disk_files_from_path(&mut self, path: RelativePathToSourceFile) {
        let in_folder = |key: &RelativePathToSourceFile| key.as_ref().starts_with(path.as_ref());
        let ids: Vec<_> = self
            .get_disk_file_map_mut()
            .tracked()
            .0
            .extract_if(|key, _| in_folder(key))
            .map(|(_, source_id)| source_id)
            .collect();
        for source_id in ids {
            self.remove(source_id);
        }
        if self.get_schema_file_set().untracked().0.iter().any(in_folder) {
            self.get_schema_file_set_mut()
                .tracked()
                .0
                .retain(|key| !in_folder(key));
        }
    }
```

A write to `schema_file_set` happens only when membership changes, the guard `set_open_file` uses: `MutView::tracked` bumps the counter on every call, so saving a schema file again must not re-invoke a reader that iterates the set. Iterating `untracked()` here is a writer's membership check, not a memo read, the same as `remove_open_files_of_client`.

`handle`:

```rust
// from crates/isograph_cli/src/state.rs
        InternalEvent::DiskChanged(change) => {
            handle_disk_changed(&mut state.db, state.globs.reference(), change)
        }

fn apply_disk_changed<TProfile: CompilationProfile>(
    db: &mut IsographDb<TProfile>,
    globs: &ProjectGlobs,
    change: DiskChanged,
) -> Result<(), DiskChangedPathError> {
    match change {
        DiskChanged::File(change) => {
            let path = relative_path_to_source_file(db, &change.path)?;
            match change.presence {
                Presence::Present(contents) => {
                    match globs.should_include_schema_file::<TProfile>(Path::new(path.lookup())) {
                        IncludeFile::Include => db.insert_schema_file(path, contents),
                        IncludeFile::Skip => db.insert_disk_file(path, contents),
                    }
                }
                Presence::Absent => db.remove_disk_file(path),
            }
        }
        DiskChanged::FolderRemoved(folder) => {
            let path = relative_path_to_source_file(db, &folder.path)?;
            db.remove_disk_files_from_path(path);
        }
    }
    ().wrap_ok()
}
```

`handle_disk_changed` gains the same `globs: &ProjectGlobs` parameter and passes it through. A path's schema classification is a function of the path and the globs, and the globs are fixed for the daemon's life, so a `Present` never moves a path out of the set.

Diagnostics:

```rust
// from crates/isograph_cli/src/diagnostics.rs
    db.open_paths_of_client(client)
        .into_iter()
        .filter(|path| {
            matches!(
                globs.should_include_source_file::<TProfile>(Path::new(path.lookup())),
                IncludeFile::Include
            )
        })
```

was `source_globs.contains(relative) && matches!(TProfile::HostLanguage::should_skip_source_file(relative), SkipSourceFile::Keep)`. `publish_diagnostics`, `publish_to_client`, and `client_diagnostics` take `globs: &ProjectGlobs`.

Daemon: one parse, cloned into the state, the watcher, and the watcher task.

```rust
// from crates/isograph_cli/src/daemon.rs
    let globs = match ProjectGlobs::parse(config.source_files.as_slice(), config.schema.as_slice()) {
        Ok(globs) => globs,
        Err(e) => {
            tracing::error!(error = %e, "invalid config globs");
            return;
        }
    };
    let mut state = IsographState {
        db: IsographDb::<TProfile>::default(),
        live_sessions: LiveSessions::default(),
        globs: globs.clone(),
        diagnostics_debounce: None,
    };
    // ...
    let _watcher = match crate::watch::start_if_watching::<TProfile>(
        filesystem,
        watch_tx,
        config_path.reference(),
        globs.clone(),
    ) {
    // ...
        () = run_filesystem_watcher::<TProfile>(watch_rx, event_tx, config_directory, globs) => {}
```

`run_filesystem_watcher` takes `globs: ProjectGlobs` in place of `source_files: Vec<String>` and calls `apply::<TProfile>(..., globs.reference())`. `let source_files = config.source_files.clone();` is deleted.

## Test profiles

`isograph_compiler` (`database.rs`), `isograph_extract_typescript`, and `isograph_lsp` test protocols include nothing:

```rust
// from crates/isograph_lsp/src/test_profile.rs
impl NetworkProtocol for TestNetworkProtocol {
    fn should_include_schema_file(_relative_path: &Path) -> IncludeFile {
        IncludeFile::Skip
    }
}
```

`isograph_cli` `test_profile.rs` includes `graphql`, the extension the watch tests write:

```rust
// from crates/isograph_cli/src/test_profile.rs
impl NetworkProtocol for TestNetworkProtocol {
    fn should_include_schema_file(relative_path: &Path) -> IncludeFile {
        match relative_path.extension().and_then(|e| e.to_str()) {
            Some("graphql") => IncludeFile::Include,
            _ => IncludeFile::Skip,
        }
    }
}
```

The test `HostLanguage` impls (`TestHostLanguage` in `database.rs`, `InHost` in `watch.rs`) rename the method and return `IncludeFile`. Test fixtures that built `source_globs: SourceGlobs::parse(&["src/**".to_owned()])` (`state.rs`, `daemon.rs`, `lsp_socket.rs`) build `globs: ProjectGlobs::parse(&["src/**".to_owned()], &[])`. The watch `Harness` holds `globs: ProjectGlobs` in place of `source_files: Vec<String>`. `Harness::with_schema(source_files: &[&str], schema: &[&str])` builds it; `Harness::new(source_files)` is `with_schema(source_files, &[])`.

## Tests

`isograph_config` (dev-dependency `serde_json`):

- `{"source_files":["src/**"]}` parses with `schema == []`.
- `{"source_files":[],"schema":["schema/**"]}` parses with that list.
- `{"source_files":[],"schemas":[]}` is an error.

`graphql_network_protocol`: `a.graphql`, `schema/a.graphqls` are `Include`; `a.gql`, `a.ts`, `a.json`, `""` are `Skip`.

`isograph_compiler` `database.rs`:

- `insert_schema_file` stores text and lists the path; `insert_disk_file` does not list it.
- `insert_schema_file` twice lists the path once and reads the second text.
- `remove_disk_file` of a schema path removes the text and the listing.
- `remove_disk_files_from_path` removes listed descendants and keeps a schema path outside the folder.

`isograph_cli` `watch.rs`, through `apply` and `handle` (source glob `src/**/*.in`, schema glob `schema/**`, unless stated):

- `schema/a.graphql` is stored and listed.
- `other/a.graphql` is not stored and not listed.
- `schema/a.ts` is not stored and not listed.
- With source glob `**/*.in`, `schema/a.in` is stored and not listed.
- `src/a.in` is stored and not listed.
- Remove of `schema/a.graphql` after create removes text and listing; remove of folder `schema` does the same.
- Boot (`start`) reads `schema/a.graphql` written before start.
- Empty `source_files` and empty `schema` boot posts nothing (existing test, both lists empty).
- `ProjectGlobs::parse` of an invalid `schema` glob is `WatchError::SchemaGlob`.

`isograph_extract_typescript`: the existing extension test is renamed `should_include_source_file_includes_ts_tsx_js_jsx` and asserts `Include` / `Skip`.

## Docs

- `event-model.md`: `IsographState` lists `globs` (the `source_files` and `schema` globs) in place of `source_globs`. The "Scope of disk files" paragraph states that the watcher posts a source file or a schema file, and that `handle` reads the schema globs to add a `Present` path to `schema_file_set` and the source globs to choose which open files get diagnostics. The State snippet lists `schema_file_set`.
- `pico.md`: the Database snippet lists `schema_file_set`. The list of readers and writers of the tracked maps names `insert_schema_file` and `schema_file_set`.

## Commits

1. `IncludeFile` rename: `SkipSourceFile` to `IncludeFile`, `should_skip_source_file` to `should_include_source_file`, `Keep` to `Include`. No behavior change.
2. `NetworkProtocol::should_include_schema_file`, the GraphQL impl and its test, and the test protocols.
3. `schema` in `IsographProjectConfig` and its tests.
4. `schema_file_set`, `insert_schema_file`, the remove paths, and their tests.
5. `ProjectGlobs` / `Globs` / `InvalidGlob`, `ChangedFileKind::{File, Folder}`, the watcher, `handle`, diagnostics, daemon, and the watch tests.
6. Docs; move this file to `refactors/past/`.
