# Source and schema path maps

`IsographDb` lists which paths are source files and which are schema files, for disk and for each client's open buffers. `handle` already knows the config's globs; it classifies each path once, where it writes `disk_file_map` or `open_file_map`, and passes the classification to the database writer. A memo can then iterate the source files a client sees without reaching `IsographState`.

Today `disk_file_map` holds source files and schema files together, `schema_file_set` marks only the schema disk paths, `apply_disk_changed` stores an injected path that matches neither glob list, and `did_open` stores any buffer the editor opens. `client_diagnostics` filters open paths with `ProjectGlobs` outside pico. None of these lets a memo list source files.

This is a prefactor of source-backed lookups (`refactors/pending/source-backed-lookups.md`), whose `source_file_paths` memo reads `source_path_map`.

## What the user sees

Nothing changes. Diagnostics are still published for every source file a client has open, and for no other open file.

## Types

```rust
// from crates/isograph_compiler/src/database.rs
/// Every source file path on disk, and every open buffer of a source file path.
#[derive(Debug, Default)]
pub struct SourcePathMap {
    pub disk: BTreeSet<RelativePathToSourceFile>,
    pub open: BTreeSet<OpenFileKey>,
}

/// Every schema file path on disk, and every open buffer of a schema file path.
#[derive(Debug, Default)]
pub struct SchemaPathMap {
    pub disk: BTreeSet<RelativePathToSourceFile>,
    pub open: BTreeSet<OpenFileKey>,
}

/// Which of the config's file lists a path belongs to. A path can be in both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileMembership {
    pub source_file: IncludeFile,
    pub schema_file: IncludeFile,
}
```

`SchemaPathMap` is the shape `refactors/pending/graphql-schema-parser/integration.md` proposes. It replaces `SchemaFileSet`. `FileMembership` is two `IncludeFile`s, not two `bool`s, because `IncludeFile` already names the two states.

Before:

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

/// A reader of file text.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ClientId(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OpenFileKey {
    pub client: ClientId,
    pub path: RelativePathToSourceFile,
}
```

After:

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
    source_path_map: SourcePathMap,
    #[tracked]
    schema_path_map: SchemaPathMap,
    phantom_data: PhantomData<TProfile>,
}

/// A reader of file text.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ClientId(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OpenFileKey {
    pub client: ClientId,
    pub path: RelativePathToSourceFile,
}
```

`Ord` is for `BTreeSet<OpenFileKey>`. `Default for IsographDb` builds both maps with `SourcePathMap::default()` and `SchemaPathMap::default()`.

## Writers

Every writer of `disk_file_map` or `open_file_map` also writes the two path maps, so the maps cannot disagree with the files. Each path-map write reads `untracked()` first and writes `tracked()` only when membership changes: `MutView::tracked` bumps the counter on every call, and a content change of an existing file must not re-invoke a reader of the path maps.

Before:

```rust
// from crates/isograph_compiler/src/database.rs
pub fn set_open_file(&mut self, client: ClientId, path: RelativePathToSourceFile, contents: String)
pub fn remove_open_file(&mut self, client: ClientId, path: RelativePathToSourceFile)
pub fn remove_open_files_of_client(&mut self, client: ClientId)
pub fn open_paths_of_client(&self, client: ClientId) -> Vec<RelativePathToSourceFile>
pub fn insert_disk_file(&mut self, path: RelativePathToSourceFile, contents: String)
pub fn insert_schema_file(&mut self, path: RelativePathToSourceFile, contents: String)
pub fn remove_disk_file(&mut self, path: RelativePathToSourceFile)
pub fn remove_disk_files_from_path(&mut self, path: RelativePathToSourceFile)
```

After:

```rust
// from crates/isograph_compiler/src/database.rs
pub fn set_open_file(
    &mut self,
    client: ClientId,
    path: RelativePathToSourceFile,
    membership: FileMembership,
    contents: String,
)
pub fn remove_open_file(&mut self, client: ClientId, path: RelativePathToSourceFile)
pub fn remove_open_files_of_client(&mut self, client: ClientId)
pub fn open_source_paths_of_client(&self, client: ClientId) -> Vec<RelativePathToSourceFile>
pub fn insert_disk_file(
    &mut self,
    path: RelativePathToSourceFile,
    membership: FileMembership,
    contents: String,
)
pub fn remove_disk_file(&mut self, path: RelativePathToSourceFile)
pub fn remove_disk_files_from_path(&mut self, path: RelativePathToSourceFile)
```

`insert_schema_file` is gone: `insert_disk_file` with `schema_file: Include` is that call. `open_paths_of_client` is gone: its only reader was `client_diagnostics`, which now reads `open_source_paths_of_client`.

```rust
// from crates/isograph_compiler/src/database.rs
pub fn insert_disk_file(
    &mut self,
    path: RelativePathToSourceFile,
    membership: FileMembership,
    contents: String,
) {
    let source_id = self.set(DiskFile { path, contents });
    self.get_disk_file_map_mut()
        .tracked()
        .0
        .insert(path, source_id);
    if matches!(membership.source_file, IncludeFile::Include)
        && !self.get_source_path_map().untracked().disk.contains(&path)
    {
        self.get_source_path_map_mut().tracked().disk.insert(path);
    }
    if matches!(membership.schema_file, IncludeFile::Include)
        && !self.get_schema_path_map().untracked().disk.contains(&path)
    {
        self.get_schema_path_map_mut().tracked().disk.insert(path);
    }
}

pub fn remove_disk_file(&mut self, path: RelativePathToSourceFile) {
    if let Some(source_id) = self.get_disk_file_map_mut().tracked().0.remove(&path) {
        self.remove(source_id);
    }
    if self.get_source_path_map().untracked().disk.contains(&path) {
        self.get_source_path_map_mut().tracked().disk.remove(&path);
    }
    if self.get_schema_path_map().untracked().disk.contains(&path) {
        self.get_schema_path_map_mut().tracked().disk.remove(&path);
    }
}
```

`remove_disk_files_from_path` retains, in each map's `disk`, the paths not under `path`, writing a map only when one of its paths is under `path` (the existing `schema_file_set` pattern). `set_open_file` inserts `OpenFileKey { client, path }` into each map's `open` whose membership is `Include` and which lacks the key. `remove_open_file` removes the key from each map's `open` that has it. `remove_open_files_of_client` removes every key of `client` from each map's `open` that has one.

```rust
// from crates/isograph_compiler/src/database.rs
/// Every source file path `client` has open.
pub fn open_source_paths_of_client(&self, client: ClientId) -> Vec<RelativePathToSourceFile> {
    self.get_source_path_map()
        .tracked()
        .open
        .iter()
        .filter(|key| key.client == client)
        .map(|key| key.path)
        .collect()
}
```

The membership of a path is a function of the path and the config's globs, which are fixed for the life of the process, so a second write of one path never carries a different membership and no writer moves a path between maps.

## Classification in `handle`

```rust
// from crates/isograph_cli/src/watch.rs
impl ProjectGlobs {
    /// Which of the config's file lists `relative` belongs to.
    pub(crate) fn membership<TProfile: CompilationProfile>(&self, relative: &Path) -> FileMembership {
        FileMembership {
            source_file: self.should_include_source_file::<TProfile>(relative),
            schema_file: self.should_include_schema_file::<TProfile>(relative),
        }
    }
}
```

Before:

```rust
// from crates/isograph_cli/src/state.rs
Presence::Present(contents) => {
    match globs.should_include_schema_file::<TProfile>(Path::new(path.lookup())) {
        IncludeFile::Include => db.insert_schema_file(path, contents),
        IncludeFile::Skip => db.insert_disk_file(path, contents),
    }
}
```

After:

```rust
// from crates/isograph_cli/src/state.rs
Presence::Present(contents) => db.insert_disk_file(
    path,
    globs.membership::<TProfile>(Path::new(path.lookup())),
    contents,
),
```

`did_open` and `did_change` take `globs: &ProjectGlobs` beside `db` and pass `globs.membership::<TProfile>(Path::new(path.lookup()))` to `set_open_file`; `on_did_open` and `on_did_change` pass `state.globs.reference()`. `did_close`, `Absent`, `FolderRemoved`, and `Gone` call the unchanged removers, which clear the path maps.

## Diagnostics

Before:

```rust
// from crates/isograph_cli/src/diagnostics.rs
fn client_diagnostics<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    globs: &ProjectGlobs,
    client: ClientId,
) -> HashMap<RelativePathToSourceFile, Vec<lsp_types::Diagnostic>> {
    db.open_paths_of_client(client)
        .into_iter()
        .filter(|path| {
            matches!(
                globs.should_include_source_file::<TProfile>(Path::new(path.lookup())),
                IncludeFile::Include
            )
        })
        .filter_map(|path| { /* unchanged */ })
        .collect()
}
```

After:

```rust
// from crates/isograph_cli/src/diagnostics.rs
fn client_diagnostics<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
) -> HashMap<RelativePathToSourceFile, Vec<lsp_types::Diagnostic>> {
    db.open_source_paths_of_client(client)
        .into_iter()
        .filter_map(|path| { /* unchanged */ })
        .collect()
}
```

`publish_diagnostics` and `publish_to_client` drop their `globs` parameter.

## Readers

`source_path_map` has a production reader in this change: `open_source_paths_of_client`. Source-backed lookups add a second, the memo `source_file_paths(db, client)`.

`schema_path_map` has no production reader. It keeps the writers that `schema_file_set` had, extended to open buffers, so the GraphQL integration finds it populated. The memo `schema_file_paths(db, client)` arrives with that integration.

## Known gap

pico records no dependency when a memo reads a tracked map whose counter was never written (`refactors/pending/pico-missing-key-dependency.md`). A memo that reads `source_path_map` before its first write does not see that write. `open_source_paths_of_client` is called outside any memo, so diagnostics are unaffected.

## Docs

`docs-website/docs/design-docs/pico.md` and `event-model.md` replace `schema_file_set: SchemaFileSet` with the two maps and say that `handle` classifies a path with `ProjectGlobs::membership`. `refactors/pending/graphql-schema-parser/integration.md` replaces its four `insert_schema_disk_path` / `remove_schema_disk_path` / `insert_schema_open_path` / `remove_schema_open_path` methods with the `FileMembership` parameter of `insert_disk_file` and `set_open_file`.

## Tests

`crates/isograph_compiler/src/database.rs` (the `schema_file_set` tests migrate):

- `insert_disk_file` with source membership lists the path in `source_path_map.disk` and not in `schema_path_map.disk`; with schema membership the reverse; with both, both; with neither, neither, and `file_contents` still reads the text.
- a second `insert_disk_file` of a listed path lists it once and reads the second text.
- `remove_disk_file` and `remove_disk_files_from_path` drop listed descendants only (`schema/a.graphql` goes, `schema2/b.graphql` stays).
- `set_open_file` with source membership lists `OpenFileKey` in `source_path_map.open`; `remove_open_file` drops it; `remove_open_files_of_client` drops that client's keys from both maps and keeps another client's.
- `open_source_paths_of_client`: empty database; disk-only path excluded; a non-source open path excluded; another client's open path excluded; a path on disk and open listed once.

`crates/isograph_cli/src/state.rs` (`handle`):

- a `DiskChanged` `Present` of `src/a.ts` lists it as a source disk path; of `schema/a.graphql` (schema glob `schema/**`) as a schema disk path; `Absent` drops it.
- `didOpen` of `src/a.ts` lists a source open path for that client; `didOpen` of `other/a.ts` (outside `src/**`) does not; `didClose` and `Gone` drop it.
- a diagnostics debounce after `didOpen` of a broken `other/a.ts` publishes nothing for it (the existing test of that scope keeps passing).
