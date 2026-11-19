# Per-client open files

Requires lsp-sessions.md and lsp-outstanding.md (landed). Diagnostics (lsp-diagnostics.md) builds on this file.

A client is a reader of file text. Each LSP connection (one VS Code window) is a client with its own open-file state: the buffers it sent with `didOpen` / `didChange` and has not closed. The compiler is a client whose open-file state is empty. Every read of file text goes through one family of memos keyed by `(ClientId, RelativePathToSourceFile)`: the client's open buffer for that path if it has one, otherwise the `DiskFile`. The compiler client cannot hold an open file because the write path takes `LspClientId`, not `ClientId`. Highlighting, Format Document, and `willSaveWaitUntil` read through that family with the asking client.

Origin of the open-file map and handlers: isograph `OpenFileMap` / `OpenFileSource` / `insert_open_file` / `remove_open_file` (`crates/isograph_schema/src/isograph_database.rs`) and `crates/isograph_lsp/src/text_document.rs`. Origin of overlay-then-disk: isograph `read_iso_literals_source` (`crates/isograph_schema/src/validated_isograph_schema/isograph_literals.rs`) and `file_text_at_location`. Delta from isograph: isograph has one client and one path-keyed `OpenFileMap`; its overlay applies to compile as well. Here, open-file state is per LSP client, each client's set of open paths is its own pico source, and the compiler reads the same memos with `ClientId::Compiler`, which never consults an open file. No `expect` on a URI. Full document sync only; any other `didChange` shape is logged with `warn!` and ignored.

## Open decisions

These are not approved. The body is written against the first option of each; each other option states its delta.

1. Name and shape of the reader id. (a) `isograph_compiler::ClientId { Compiler, Lsp(LspClientId) }`, with `LspClientId(pub u64)` for a connection. `ClientId` is taken today: `crates/isograph_cli/src/event.rs` has `pub(crate) struct ClientId(pub u64)` meaning a connection. Change 5 deletes that struct and replaces every use with `LspClientId`, so after change 5 `ClientId` means "reader of file text" and nothing else. During review the two meanings coexist in docs. (b) Keep the CLI name for connections and name the enum `FileReader { Compiler, Lsp(LspClientId) }`; no name changes meaning, but "client" in the user's sense is spelled `FileReader` in code. (c) `Client { Compiler, Lsp(LspClientId) }`; a value of it is an id, not a client, so the name reads wrong at call sites (`file_contents(db, client, path)` with `client: Client`).
2. The compiler client has no production reader. i2 has no artifact generation and no compile command; nothing in production reads file text except the LSP. (a) Ship `ClientId::Compiler` now. Its only constructors are tests (`database.rs`, `isograph_extract_typescript` memo tests, `watch.rs`). `ClientId` is `pub` and re-exported, so rustc's `dead_code` lint does not fire and no `expect` attribute can mark it; nothing enforces that a production reader arrives. This contradicts "we do not have anything we do not use"; the user's direction that the compiler is a client is the reason. (b) Ship the family keyed by `LspClientId`; the artifact-generation change introduces `ClientId` and widens every signature. All existing disk-only tests then register an LSP client to read disk, so they read as LSP tests. (c) Ship (a), and make artifact generation the next pending doc, so the variant gets its production reader in a named change.
3. Per-client storage shape in pico. (a) A tracked `lsp_client_map: HashMap<LspClientId, SourceId<LspClientOpenFiles>>` written only on connect and disconnect; one `LspClientOpenFiles` source per client holding `HashMap<RelativePathToSourceFile, SourceId<OpenFile>>`; one `OpenFile` source per `(client, path)`. A keystroke writes one `OpenFile`. Open or close writes one client's `LspClientOpenFiles`. Neither touches a tracked map, so neither re-invokes another client's memos or the compiler's. (b) One global tracked `OpenFileMap: HashMap<OpenFileKey, SourceId<OpenFile>>`. `MutView::tracked()` (`crates/pico/src/view.rs:67-74`) increments the counter on every call, so every open, close, and (if it calls `tracked()`) every keystroke of any client re-invokes every memo that missed on that map, which is every disk-fallback read of every client including the compiler. (c) Per-client `LspClientOpenFiles` without `lsp_client_map`: compute `SourceId<LspClientOpenFiles>` from a placeholder value (`SourceId::new(&LspClientOpenFiles { client, open_files: HashMap::new() })`). Removes one map, but constructs a value only to hash it, and `db.get` of that id panics for a client with no state (a `Gone` without `Started`).
4. A tracked map whose counter was never written. `Storage::get_impl` (`crates/pico/src/database.rs:134-140`) returns `None` before registering a dependency, so a miss-then-`tracked()` read before the first write to that map records nothing, and a later insert is not seen. `missing_then_present_extracts` in `isograph_extract_typescript` inserts an unrelated file first to work around this. (a) `IsographDb::default` calls `tracked()` once on each tracked field, which writes each counter; no pico change. A future tracked field that is not listed there has the bug again, silently. (b) pico prefactor: record a dependency on an absent source. `Dependency` gains the presence observed at read time, `get_impl` registers on a miss, and `source_node_changed_since` reports "changed" only when presence differs or the present node is newer. Fixes every absent read (singletons included) for every tracked field. pico is kept unchanged from upstream isograph per AGENTS.md, so this is an upstream divergence; it would be its own change before change 2, with pico tests.
5. Memo growth. pico never collects sources (`handle` removes them); derived nodes are collected only by `run_garbage_collection`, which keeps the last 10_000 top-level calls (LRU) and what they reach. Nothing in i2 calls `run_garbage_collection` today, so every re-invocation's new derived node and every `top_level_calls` entry accumulates for the daemon's lifetime. (a) `handle` of `ClientConnectionEvent::Gone` calls `state.db.run_garbage_collection()` after removing the client's sources. A disconnected client's memos are dropped once 10_000 newer top-level calls push them out of the LRU, not at once. Growth during a long session with no disconnect is unchanged. (b) No GC in this doc; a separate GC-policy doc decides when `handle` collects (after each write event, on a timer effect, on `Gone`). (c) pico prefactor: evict LRU entries whose params contain a given value (a disconnected `LspClientId`). Does not exist in pico; upstream divergence.
6. Order relative to format-file-from-segments.md. (a) This doc first. Change 5's `format_iso_literals` uses `parsed_iso_literals_in_file` and `text_through_last_iso_literal` with the client. format-file-from-segments.md then reads `file_contents(db, client, path)` instead of `state.disk_file(path)` (which change 5 deletes). (b) format-file-from-segments.md first. Change 5's `format_iso_literals` after-snippet is the second one in that section, with `file_contents` replacing `disk_file`. Either order ships; (b) makes change 5's format diff smaller, (a) makes format-file-from-segments.md read the buffer from the start.
7. `SendLspResponseEffect` as data (change 4). (a) Make it `{ sender, response }` with private fields, `pub(crate) fn response(&self) -> &lsp_server::Response`, and `pub(crate) fn send(self, token: &SideEffectToken)` called by `perform`. `LiveSessions::after_compute` returns it. Tests match the response on the effect without `perform` and a channel; `#[expect(clippy::type_complexity)]` and the hand-written `Debug` go away; `run_event_loop` is unchanged; `send` still takes the token. (b) Keep `Box<dyn FnOnce(&SideEffectToken) + Send>`; change 5's `handle` tests own the crossbeam receiver behind the `LiveSession`, call `perform`, and `recv` the response (the pattern in `lsp_formatting_of_a_live_client_without_a_file_is_null`).
8. Buffers in the event log. `run_event_loop` (`crates/isograph_cli/src/daemon.rs:170`) runs `tracing::debug!(?event, "event")` on every event. With a derived `Debug`, every `didChange` logs the whole buffer, and every `DiskChanged` already logs the whole file. (a) Hand-written `Debug` for `LspNotification` (client and method) and `Presence` (byte length of `Present`). `run_event_loop` is unchanged. (b) Delete the `?event` line; `handle` logs nothing per event. (c) Lower it to `trace!`; buffers still reach the log at that level.

## What the user does

Highlighting. The user types in an iso literal and does not save. vscode-languageclient 9.0.1 holds full-sync `didChange` for up to 250ms, and `sendRequest` flushes pending full-text changes before sending any request (`vscode-extension/node_modules/vscode-languageclient/lib/common/client.js:335-345`). The daemon handles events in order on one loop. So `semanticTokens/full` colors the text the editor has, not the last save. Typing `iso(\`field Query.x{id}\`)` over a saved `iso(\`entrypoint Query.HomeRoute\`)` colors `field` as a keyword without a save.

Formatting. Format Document and save (`willSaveWaitUntil`) compute edits from that window's buffer. Applying them to that buffer yields the formatted file even when disk differs; the extra `}` from disk ranges applied to a longer buffer no longer happens.

Two VS Code windows on one project are two `isograph lsp` processes, two connections, and two `LspClientId`s on one daemon. Each window has its own buffer for a file both have open. Window A's unsaved edits color and format only in A. Window B colors and formats B's buffer. When A saves, the watcher posts `DiskChanged`; B's buffer is still B's text. If B has no unsaved changes, VS Code reloads the file into B and sends `didChange`; B then shows the new text. A window that does not have the file open reads disk. The daemon does not push `workspace/semanticTokens/refresh`; B recolors on its next tokens request.

Closing a tab sends `didClose`; that window reads disk for that path again. Closing or crashing a window ends its connection; `handle` of `Gone` drops all of that window's buffers. The other window is unaffected.

A URI with neither a buffer for the asking window nor a `DiskFile` is JSON `null` for tokens and formatting. A non-`file:` URI is `null`. A `file:` URI whose path is not UTF-8, or a request before the config directory is interned, is an error response (`UnknownErrorCode`) naming the cause; the daemon does not panic.

The compiler client reads disk only. An unsaved buffer never reaches anything read with `ClientId::Compiler`. Today no production code reads with it (open decision 2).

`isograph send` is unchanged. A send connection never sends `didOpen`, so it has no buffers.

## Changes

Five changes, prefactors first. Change 4 ships only if open decision 7 picks (a).

1. Typed path conversion.
2. Tracked counters exist from construction; tracked-map writes only bump on a key change; `disk_file` tracks its miss.
3. `HostLanguage::extract_iso_literals_from_text`.
4. `SendLspResponseEffect` is data.
5. Clients and their open files.

## Change 1: typed path conversion

`relative_path_from_absolute_and_working_directory` `expect`s at `crates/common_lang_types/src/location.rs:101,103`. `semanticTokens/full` and formatting of `file:///tmp/proj/%FF.ts` reach the second `expect` today (`Url::to_file_path` percent-decodes to a non-UTF-8 path) and panic the event loop. A watcher path that is not UTF-8 reaches it from `DiskChanged`. This change returns a typed error, and gives the adapter one URI-to-path function that distinguishes "not a file URI" from "no working directory" from "no relative form".

### Types

```rust
// from crates/common_lang_types/src/location.rs
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RelativePathError {
    #[error("the path has no form relative to the working directory")]
    NotDiffable,
    #[error("the relative path is not UTF-8")]
    NotUtf8,
}
```

```rust
// from crates/isograph_cli/src/adapter.rs
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum UriPathError {
    #[error("the URI is not a file URI")]
    NotAFileUri,
    #[error("CurrentWorkingDirectory is not interned")]
    MissingWorkingDirectory,
    #[error("{0}")]
    Relative(RelativePathError),
}
```

```rust
// from crates/isograph_cli/src/state.rs
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{}: {error}", path.display())]
struct DiskChangedPathError {
    path: std::path::PathBuf,
    error: RelativePathError,
}
```

### `relative_path_from_absolute_and_working_directory`

Before:

```rust
// from crates/common_lang_types/src/location.rs
pub fn relative_path_from_absolute_and_working_directory(
    current_working_directory: CurrentWorkingDirectory,
    absolute_path: &Path,
) -> RelativePathToSourceFile {
    let cwd = PathBuf::from(current_working_directory.lookup());
    pathdiff::diff_paths(
        without_windows_verbatim_prefix(absolute_path),
        without_windows_verbatim_prefix(cwd.as_path()),
    )
    .expect("Expected path to be diffable")
    .to_str()
    .expect("Expected path to be able to be stringified")
    .intern()
    .to()
}
```

After:

```rust
// from crates/common_lang_types/src/location.rs
pub fn relative_path_from_absolute_and_working_directory(
    current_working_directory: CurrentWorkingDirectory,
    absolute_path: &Path,
) -> Result<RelativePathToSourceFile, RelativePathError> {
    let cwd = PathBuf::from(current_working_directory.lookup());
    let relative = pathdiff::diff_paths(
        without_windows_verbatim_prefix(absolute_path),
        without_windows_verbatim_prefix(cwd.as_path()),
    )
    .ok_or(RelativePathError::NotDiffable)?;
    relative
        .to_str()
        .ok_or(RelativePathError::NotUtf8)?
        .intern()
        .to()
        .wrap_ok()
}
```

`pathdiff::diff_paths` returns `None` when the path is relative and the base is absolute. A URI's `to_file_path` is absolute; a `DiskChanged` path from a caller that is not the watcher may be relative.

### `DiskChanged` logs instead of panicking

Before:

```rust
// from crates/isograph_cli/src/state.rs
fn relative_path_to_source_file(
    state: &IsographDb<impl HostLanguage>,
    absolute: &Path,
) -> RelativePathToSourceFile {
    let cwd = *state
        .get_singleton::<CurrentWorkingDirectory>()
        .expect("CurrentWorkingDirectory is interned from the config path before DiskChanged");
    relative_path_from_absolute_and_working_directory(cwd, absolute)
}

fn handle_disk_changed<THostLanguage: HostLanguage>(
    state: &mut IsographDb<THostLanguage>,
    change: DiskChanged,
) -> Vec<IsographEffect> {
    match change {
        DiskChanged::File(change) => {
            let path = relative_path_to_source_file(state, &change.path);
            match change.presence {
                Presence::Present(contents) => {
                    state.insert_disk_file(path, contents);
                }
                Presence::Absent => {
                    state.remove_disk_file(path);
                }
            }
        }
        DiskChanged::FolderRemoved(folder) => {
            let path = relative_path_to_source_file(state, &folder.path);
            state.remove_disk_files_from_path(path);
        }
    }
    no_effects()
}
```

After:

```rust
// from crates/isograph_cli/src/state.rs
fn relative_path_to_source_file(
    state: &IsographDb<impl HostLanguage>,
    absolute: &Path,
) -> Result<RelativePathToSourceFile, DiskChangedPathError> {
    let cwd = *state
        .get_singleton::<CurrentWorkingDirectory>()
        .expect("CurrentWorkingDirectory is interned from the config path before DiskChanged");
    relative_path_from_absolute_and_working_directory(cwd, absolute).map_err(|error| {
        DiskChangedPathError {
            path: absolute.to_owned(),
            error,
        }
    })
}

fn handle_disk_changed<THostLanguage: HostLanguage>(
    state: &mut IsographDb<THostLanguage>,
    change: DiskChanged,
) -> Vec<IsographEffect> {
    apply_disk_changed(state, change)
        .unwrap_or_else(|error| tracing::warn!(%error, "DiskChanged path"));
    no_effects()
}

fn apply_disk_changed<THostLanguage: HostLanguage>(
    state: &mut IsographDb<THostLanguage>,
    change: DiskChanged,
) -> Result<(), DiskChangedPathError> {
    match change {
        DiskChanged::File(change) => {
            let path = relative_path_to_source_file(state, &change.path)?;
            match change.presence {
                Presence::Present(contents) => state.insert_disk_file(path, contents),
                Presence::Absent => state.remove_disk_file(path),
            }
        }
        DiskChanged::FolderRemoved(folder) => {
            let path = relative_path_to_source_file(state, &folder.path)?;
            state.remove_disk_files_from_path(path);
        }
    }
    ().wrap_ok()
}
```

The `CurrentWorkingDirectory` `expect` is unchanged: `daemon::run` interns it before the event loop starts.

### `relative_from_uri` replaces the conversion in `semantic_tokens` and `format_iso_literals`

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn relative_from_uri<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    uri: &lsp_types::Uri,
) -> Result<RelativePathToSourceFile, UriPathError> {
    let absolute = file_path(uri).ok_or(UriPathError::NotAFileUri)?;
    let cwd = db
        .get_singleton::<CurrentWorkingDirectory>()
        .ok_or(UriPathError::MissingWorkingDirectory)?
        .dereference();
    relative_path_from_absolute_and_working_directory(cwd, absolute.as_path())
        .map_err(UriPathError::Relative)
}

fn lsp_error(error: UriPathError) -> LSPRuntimeError {
    match error {
        UriPathError::NotAFileUri => LSPRuntimeError::ExpectedError,
        UriPathError::MissingWorkingDirectory | UriPathError::Relative(_) => {
            LSPRuntimeError::UnexpectedError(error.to_string())
        }
    }
}
```

`file_path` is unchanged and stays private. `relative_from_uri` is `pub(crate)` because change 5's document-sync handlers call it.

A non-file URI stays JSON `null` (`ExpectedError`). A missing working directory is a broken startup invariant; it is now an error response. A path with no relative form is a real file the daemon cannot key; it is an error response naming the cause.

Before (the conversion `semantic_tokens` and `format_iso_literals` each repeat):

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn semantic_tokens_response<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographDb<THostLanguage>,
    params: lsp_types::SemanticTokensParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::SemanticTokensFullRequest as lsp_types::request::Request>::Result,
> {
    let Some(absolute) = file_path(params.text_document.uri.reference()) else {
        return isograph_lsp::lsp_runtime_error::LSPRuntimeError::ExpectedError.wrap_err();
    };
    let tokens = semantic_tokens(state, absolute.reference());
    tokens
        .map(|data| {
            lsp_types::SemanticTokensResult::Tokens(lsp_types::SemanticTokens {
                result_id: None,
                data,
            })
        })
        .wrap_ok()
}

fn semantic_tokens<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographDb<THostLanguage>,
    absolute: &std::path::Path,
) -> Option<Vec<lsp_types::SemanticToken>> {
    let cwd = state.get_singleton::<common_lang_types::CurrentWorkingDirectory>()?;
    let path = common_lang_types::relative_path_from_absolute_and_working_directory(*cwd, absolute);
    isograph_lsp::lsp_semantic_tokens_for_file::<THostLanguage>(state, path).clone()
}

pub(crate) fn formatting_response<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographDb<THostLanguage>,
    params: lsp_types::DocumentFormattingParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::Formatting as lsp_types::request::Request>::Result,
> {
    format_iso_literals(state, params.text_document.uri.reference()).wrap_ok()
}

fn format_iso_literals<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographDb<THostLanguage>,
    uri: &lsp_types::Uri,
) -> Option<Vec<lsp_types::TextEdit>> {
    let absolute = file_path(uri)?;
    let cwd = *state.get_singleton::<common_lang_types::CurrentWorkingDirectory>()?;
    let path = common_lang_types::relative_path_from_absolute_and_working_directory(cwd, &absolute);
    let literals = isograph_compiler::parsed_iso_literals_in_file(state, path).as_ref()?;
    // ... unchanged
}
```

After (`semantic_tokens` is deleted):

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn semantic_tokens_response<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    params: lsp_types::SemanticTokensParams,
) -> LSPRuntimeResult<
    <lsp_types::request::SemanticTokensFullRequest as lsp_types::request::Request>::Result,
> {
    let path = relative_from_uri(db, params.text_document.uri.reference()).map_err(lsp_error)?;
    isograph_lsp::lsp_semantic_tokens_for_file::<THostLanguage>(db, path)
        .clone()
        .map(|data| {
            lsp_types::SemanticTokensResult::Tokens(lsp_types::SemanticTokens {
                result_id: None,
                data,
            })
        })
        .wrap_ok()
}

pub(crate) fn formatting_response<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    params: lsp_types::DocumentFormattingParams,
) -> LSPRuntimeResult<<lsp_types::request::Formatting as lsp_types::request::Request>::Result> {
    let path = relative_from_uri(db, params.text_document.uri.reference()).map_err(lsp_error)?;
    format_iso_literals(db, path).wrap_ok()
}

pub(crate) fn will_save_wait_until_response<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    params: lsp_types::WillSaveTextDocumentParams,
) -> LSPRuntimeResult<
    <lsp_types::request::WillSaveWaitUntil as lsp_types::request::Request>::Result,
> {
    let path = relative_from_uri(db, params.text_document.uri.reference()).map_err(lsp_error)?;
    format_iso_literals(db, path).wrap_ok()
}

fn format_iso_literals<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::TextEdit>> {
    let literals = isograph_compiler::parsed_iso_literals_in_file(db, path).as_ref()?;
    let page_content = isograph_compiler::text_through_last_iso_literal(db, path).as_ref()?;
    // ... the filter_map over literals is unchanged
}
```

`adapter.rs` imports `IsographDb`, `CurrentWorkingDirectory`, `RelativePathToSourceFile`, `RelativePathError`, `relative_path_from_absolute_and_working_directory`, `LSPRuntimeError`, `LSPRuntimeResult`.

### Tests

`crates/common_lang_types/src/location.rs`:

- a relative `absolute_path` (`src/a.ts`) against `/tmp/proj` is `Err(RelativePathError::NotDiffable)`
- `#[cfg(unix)]` an absolute path with a non-UTF-8 component (`OsStr::from_bytes(b"/tmp/proj/\xFF.ts")`) is `Err(RelativePathError::NotUtf8)`
- the existing tests assert `Ok(...)` of the same values

`crates/isograph_cli/src/state.rs` (`handle` seam):

- `DiskChanged::File` with path `src/a.ts` (relative) and `Present`: no effects, no `DiskFile` at any path, no panic
- `#[cfg(unix)]` `DiskChanged::File` with a non-UTF-8 absolute path: no effects, no panic
- `semanticTokens/full` of `file:///tmp/proj/%FF.ts` from a started client on `with_config()`: one `SendLspResponse`; the response has `error.code == UnknownErrorCode` and `error.message == "the relative path is not UTF-8"`
- Formatting from a started client on `empty()` (no working directory): the response has `error.message == "CurrentWorkingDirectory is not interned"`
- `lsp_formatting_of_a_live_client_without_a_file_is_null` uses `with_config()` instead of `empty()`; its fact (JSON `null` for a path with no `DiskFile`) is unchanged

`crates/isograph_cli/src/adapter.rs`: the tests call `format_iso_literals(&db, path())` with `fn path() -> RelativePathToSourceFile { "src/Pet.ts".intern().to() }`. `uri()` is deleted. Facts unchanged.

`lsp_socket.rs` `semantic_tokens_full_of_a_non_file_uri_is_null` stays.

### Call sites

- `handle_disk_changed` -> `apply_disk_changed` -> `relative_path_to_source_file` -> `relative_path_from_absolute_and_working_directory`
- `semantic_tokens_response` / `formatting_response` / `will_save_wait_until_response` -> `relative_from_uri` -> `file_path`, `relative_path_from_absolute_and_working_directory`

## Change 2: tracked counters exist from construction; map writes bump only on a key change; `disk_file` tracks its miss

Three facts about pico tracked maps, each verified in pico source:

- `View::tracked()` reads the counter with `get_singleton`. `Storage::get_impl` (`crates/pico/src/database.rs:134-140`) returns `None` before registering a dependency when the counter was never written. A memo that misses and reads the map `tracked()` before the first write records no dependency. `set_source` of a vacant key does not advance the epoch, so the next call reuses the stale `None`.
- `MutView::tracked()` (`crates/pico/src/view.rs:67-74`) writes `counter + 1` on every call, whether or not the caller then changes the map. `insert_disk_file` of an already-present path, and `remove_disk_file` of an absent path, re-invoke every memo that missed on the map.
- `IsographDb::disk_file` reads `untracked()` then `?`. A miss records nothing.

Open decision 4 picks how the first is fixed. This change is option (a).

### `IsographDb::default`

Before:

```rust
// from crates/isograph_compiler/src/database.rs
impl<THostLanguage: HostLanguage> Default for IsographDb<THostLanguage> {
    fn default() -> Self {
        Self {
            storage: Storage::default(),
            disk_file_map: DiskFileMap::default(),
            phantom_data: PhantomData,
        }
    }
}
```

After:

```rust
// from crates/isograph_compiler/src/database.rs
impl<THostLanguage: HostLanguage> Default for IsographDb<THostLanguage> {
    fn default() -> Self {
        let mut db = Self {
            storage: Storage::default(),
            disk_file_map: DiskFileMap::default(),
            phantom_data: PhantomData,
        };
        // pico registers no dependency on a counter that was never written.
        db.get_disk_file_map_mut().tracked();
        db
    }
}
```

### Map writes

Before:

```rust
// from crates/isograph_compiler/src/database.rs
    pub fn insert_disk_file(&mut self, path: RelativePathToSourceFile, contents: String) {
        let source_id = self.set(DiskFile { path, contents });
        self.get_disk_file_map_mut()
            .tracked()
            .0
            .insert(path, source_id);
    }

    pub fn remove_disk_file(&mut self, path: RelativePathToSourceFile) {
        if let Some(source_id) = self.get_disk_file_map_mut().tracked().0.remove(&path) {
            self.remove(source_id);
        }
    }

    pub fn remove_disk_files_from_path(&mut self, path: RelativePathToSourceFile) {
        let ids: Vec<_> = self
            .get_disk_file_map_mut()
            .tracked()
            .0
            .extract_if(|key, _| key.as_ref().starts_with(path.as_ref()))
            .map(|(_, source_id)| source_id)
            .collect();
        for source_id in ids {
            self.remove(source_id);
        }
    }
```

After:

```rust
// from crates/isograph_compiler/src/database.rs
    pub fn insert_disk_file(&mut self, path: RelativePathToSourceFile, contents: String) {
        let source_id = self.set(DiskFile { path, contents });
        if self.get_disk_file_map().untracked().0.contains_key(&path) {
            return;
        }
        self.get_disk_file_map_mut()
            .tracked()
            .0
            .insert(path, source_id);
    }

    pub fn remove_disk_file(&mut self, path: RelativePathToSourceFile) {
        let Some(source_id) = self.get_disk_file_map().untracked().0.get(&path).copied() else {
            return;
        };
        self.get_disk_file_map_mut().tracked().0.remove(&path);
        self.remove(source_id);
    }

    pub fn remove_disk_files_from_path(&mut self, path: RelativePathToSourceFile) {
        let is_under = |key: &RelativePathToSourceFile| key.as_ref().starts_with(path.as_ref());
        if !self.get_disk_file_map().untracked().0.keys().any(is_under) {
            return;
        }
        let ids: Vec<_> = self
            .get_disk_file_map_mut()
            .tracked()
            .0
            .extract_if(|key, _| is_under(key))
            .map(|(_, source_id)| source_id)
            .collect();
        ids.into_iter().for_each(|source_id| self.remove(source_id));
    }
```

Reading the key untracked before writing it is the shape AGENTS.md steers away from in favor of `entry`. It is required here: `MutView` has only `tracked()`, and calling it is the bump. The `SourceId` of a path is a function of the path, so re-inserting a present path would write the same id. `ids` is collected because `self.remove` needs `&mut self` after the map borrow ends.

### `disk_file` tracks its miss

Before:

```rust
// from crates/isograph_compiler/src/database.rs
    pub fn disk_file(&self, path: RelativePathToSourceFile) -> Option<&DiskFile> {
        let source_id = self.get_disk_file_map().untracked().0.get(&path).copied()?;
        self.get(source_id).wrap_some()
    }
```

After:

```rust
// from crates/isograph_compiler/src/database.rs
    pub fn disk_file(&self, path: RelativePathToSourceFile) -> Option<&DiskFile> {
        self.get(self.disk_file_source_id(path)?).wrap_some()
    }

    fn disk_file_source_id(&self, path: RelativePathToSourceFile) -> Option<SourceId<DiskFile>> {
        match self.get_disk_file_map().untracked().0.get(&path).copied() {
            Some(source_id) => source_id.wrap_some(),
            None => self.get_disk_file_map().tracked().0.get(&path).copied(),
        }
    }
```

`TypeScriptHostLanguage::extract_iso_literals` stops reading the map itself:

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    #[memo]
    fn extract_iso_literals(
        db: &IsographDb<Self>,
        path: RelativePathToSourceFile,
    ) -> Option<Vec<IsoLiteralExtraction<Self>>> {
        let contents = db.disk_file(path)?.contents.as_str();
        EXTRACT_ISO_LITERAL
            .captures_iter(contents)
            // ... unchanged
    }
```

### Tests

`crates/isograph_compiler/src/database.rs` `#[cfg(test)]` gains a memo that reads one path:

```rust
// from crates/isograph_compiler/src/database.rs
    #[memo]
    fn disk_contents(db: &IsographDb<TestHostLanguage>, path: RelativePathToSourceFile) -> Option<String> {
        db.disk_file(path).map(|file| file.contents.clone())
    }
```

- on a fresh `IsographDb` (no write ever), `disk_contents(path)` is `None`; `insert_disk_file(path, "a")`; `disk_contents(path)` is `Some("a")`. This fails without the `Default` change and without the miss-tracked lookup.
- `disk_contents(path)` is `Some("a")`; `remove_disk_file(path)`; `disk_contents(path)` is `None`
- `remove_disk_file` of an absent path, then `insert_disk_file` of that path, then `disk_contents` is `Some`
- `remove_disk_files_from_path` of a prefix that matches nothing leaves every file; the existing prefix tests keep their facts

`isograph_extract_typescript` `missing_then_present_extracts` drops the `other` insert. It is a fresh-database miss then insert, and it asserts the same extraction.

### Call sites

- `IsographDb::default` -> `get_disk_file_map_mut().tracked()`
- `TypeScriptHostLanguage::extract_iso_literals`, `text_through_last_iso_literal`, `literal_id_at_location` -> `disk_file` -> `disk_file_source_id`

### Docs this change amends

`docs-website/docs/design-docs/pico.md` Tracked maps: replace "`IsographDb::disk_file` is that untracked lookup. Callers that already resolved the path (extract or `parsed_iso_literals_in_file` returned `Some`) write `db.disk_file(path)`. Extract is the first lookup of a path and does not use it." with: "`IsographDb::disk_file` looks up untracked and, on a miss, reads the map tracked. Extract calls it." Add: "A memo that reads a tracked map before any write to it records nothing: `get_impl` returns `None` before registering a dependency on the counter. `IsographDb::default` writes each counter once. `MutView::tracked()` increments the counter on every call, so a write that does not change the key set does not call it."

## Change 3: `extract_iso_literals_from_text`

The TypeScript extract memo looks up a `DiskFile` and runs the regex. Change 5 runs that regex on whatever text the client reads. This change splits lookup from extract-from-text. Disk extract still looks up `DiskFile`. Highlighting is unchanged.

Origin: the regex body of `TypeScriptHostLanguage::extract_iso_literals`. Delta: the body after the lookup is an associated function on `HostLanguage` that takes `&str`.

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait HostLanguage: Send + Sync + Sized + 'static {
    type Error: std::fmt::Display + std::error::Error + Clone + PartialEq + Eq + 'static;
    type LiteralContext: Clone + PartialEq + Eq + Debug + 'static;

    fn extract_iso_literals_from_text(contents: &str) -> Vec<IsoLiteralExtraction<Self>>;

    fn extract_iso_literals(
        db: &IsographDb<Self>,
        path: RelativePathToSourceFile,
    ) -> &Option<Vec<IsoLiteralExtraction<Self>>>;

    fn should_skip_source_file(relative_path: &Path) -> SkipSourceFile;
}
```

`extract_iso_literals_from_text` is not a memo. A missing file is the lookup's `None`. An empty string is `vec![]`.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    #[memo]
    fn extract_iso_literals(
        db: &IsographDb<Self>,
        path: RelativePathToSourceFile,
    ) -> Option<Vec<IsoLiteralExtraction<Self>>> {
        let contents = db.disk_file(path)?.contents.as_str();
        Self::extract_iso_literals_from_text(contents).wrap_some()
    }

    fn extract_iso_literals_from_text(contents: &str) -> Vec<IsoLiteralExtraction<Self>> {
        EXTRACT_ISO_LITERAL
            .captures_iter(contents)
            .filter_map(|captures| { /* the body from today's memo, unchanged */ })
            .collect()
    }
```

`TestHostLanguage` (`database.rs`) and `InHost` (`watch.rs`) add:

```rust
// from crates/isograph_compiler/src/database.rs
        fn extract_iso_literals_from_text(_contents: &str) -> Vec<IsoLiteralExtraction<Self>> {
            Vec::new()
        }
```

### Tests

Existing extract, `parsed_iso_literals_in_file`, and tokens tests keep their facts. Added in `isograph_extract_typescript`:

- `extract_iso_literals_from_text` of `export const Home = iso(\`entrypoint Query.HomeRoute\`)` is one extraction with text `entrypoint Query.HomeRoute`
- of `""` is empty
- of `const x = 1;` is empty
- of two `iso(\`...\`)` calls is two extractions in source order
- `extract_iso_literals(db, path)` of an interned `DiskFile` equals `Some(extract_iso_literals_from_text(text))`

### Call sites

- `TypeScriptHostLanguage::extract_iso_literals` -> `disk_file`, `extract_iso_literals_from_text`

### Docs this change amends

`docs-website/docs/design-docs/pico.md` Parser and extract: "Extract is a memo over a `DiskFile`" becomes "Extract is a memo that looks up file text and calls `HostLanguage::extract_iso_literals_from_text`."

## Change 4: `SendLspResponseEffect` is data

Ships only if open decision 7 picks (a).

Before:

```rust
// from crates/isograph_cli/src/effect.rs
pub struct SendLspResponseEffect {
    pub send: Box<dyn FnOnce(&SideEffectToken) + Send>,
}

impl std::fmt::Debug for SendLspResponseEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SendLspResponseEffect")
    }
}
```

After:

```rust
// from crates/isograph_cli/src/effect.rs
#[derive(Debug)]
pub struct SendLspResponseEffect {
    sender: crossbeam::channel::Sender<lsp_server::Message>,
    response: lsp_server::Response,
}

impl SendLspResponseEffect {
    pub(crate) fn new(
        sender: crossbeam::channel::Sender<lsp_server::Message>,
        response: lsp_server::Response,
    ) -> Self {
        Self { sender, response }
    }

    pub(crate) fn response(&self) -> &lsp_server::Response {
        self.response.reference()
    }

    pub(crate) fn send(self, _token: &SideEffectToken) {
        let _ = self.sender.send(lsp_server::Message::Response(self.response));
    }
}
```

Before:

```rust
// from crates/isograph_cli/src/event.rs
    #[expect(clippy::type_complexity)]
    pub fn after_compute(
        &mut self,
        after: AfterCompute,
        body: Result<serde_json::Value, lsp_server::ResponseError>,
    ) -> Option<Box<dyn FnOnce(&crate::effect::SideEffectToken) + Send>> {
        let session = self.0.get_mut(&after.client)?;
        if !session.outstanding.remove(&after.id) {
            return None;
        }
        let sender = session.sender.clone();
        let (result, error) = match body {
            Ok(value) => (value.wrap_some(), None),
            Err(error) => (None, error.wrap_some()),
        };
        let message = lsp_server::Message::Response(lsp_server::Response {
            id: after.id,
            result,
            error,
        });
        let send: Box<dyn FnOnce(&crate::effect::SideEffectToken) + Send> =
            (move |_token: &crate::effect::SideEffectToken| {
                let _ = sender.send(message);
            })
            .boxed();
        send.wrap_some()
    }
```

After:

```rust
// from crates/isograph_cli/src/event.rs
    pub fn after_compute(
        &mut self,
        after: AfterCompute,
        body: Result<serde_json::Value, lsp_server::ResponseError>,
    ) -> Option<crate::effect::SendLspResponseEffect> {
        let session = self.0.get_mut(&after.client)?;
        if !session.outstanding.remove(&after.id) {
            return None;
        }
        let (result, error) = match body {
            Ok(value) => (value.wrap_some(), None),
            Err(error) => (None, error.wrap_some()),
        };
        crate::effect::SendLspResponseEffect::new(
            session.sender.clone(),
            lsp_server::Response {
                id: after.id,
                result,
                error,
            },
        )
        .wrap_some()
    }
```

```rust
// from crates/isograph_cli/src/state.rs
fn send_lsp_response(
    sessions: &mut crate::event::LiveSessions,
    after: crate::event::AfterCompute,
    body: Result<serde_json::Value, lsp_server::ResponseError>,
) -> Vec<IsographEffect> {
    let Some(effect) = sessions.after_compute(after, body) else {
        return no_effects();
    };
    IsographEffect::SendLspResponse(effect.boxed()).wrap_vec()
}
```

```rust
// from crates/isograph_cli/src/daemon.rs
        IsographEffect::SendLspResponse(respond) => {
            respond.send(token);
            ControlFlow::Continue(())
        }
```

### Tests

`state.rs` tests that `perform` then `recv` keep one test that does so (`lsp_request_returns_method_not_found`), which pins that `perform` sends the response on the session's sender. The others assert on `effect.response()` directly: `let [IsographEffect::SendLspResponse(effect)] = effects.as_slice() else { panic!(...) };` then facts on `effect.response().id`, `.result`, `.error`. `event.rs` / `state.rs` tests of `after_compute` assert `response().id` is the closed-over id.

### Call sites

- `send_lsp_response` -> `LiveSessions::after_compute` -> `SendLspResponseEffect::new`
- `perform` -> `SendLspResponseEffect::send`
- tests -> `SendLspResponseEffect::response`

## Change 5: clients and their open files

After this change:

- `IsographDb` holds each LSP client's open files. `handle` of `ClientConnectionEvent::Started` creates that client's empty open-file state; `Gone` removes it and every `OpenFile` it held.
- `didOpen` / `didChange` / `didClose` write that client's open files.
- Every read of file text is `file_contents(db, client, path)` or a memo built on it, keyed by `ClientId`.
- `semanticTokens/full`, `textDocument/formatting`, and `textDocument/willSaveWaitUntil` read with `ClientId::Lsp(asking client)`.
- `initialize` advertises full document sync.
- `HostLanguage::extract_iso_literals` and `IsographDb::disk_file` are deleted; their readers use the client-keyed family.

### Where the client lives

Connectedness and the reply channel stay in `LiveSessions` on `IsographState`. Open-file state is pico state on `IsographDb`, because memos read it. `handle` of `Started` and `Gone` writes both in one arm. The compiler client has no state anywhere: `ClientId::Compiler` is a constant, always present, and `file_source` of it never reads an open file. `lsp_client_map` is keyed by `LspClientId`, so it cannot hold an entry for the compiler.

### Types

Most important first.

```rust
// from crates/isograph_compiler/src/database.rs
/// A reader of file text.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ClientId {
    Compiler,
    Lsp(LspClientId),
}

/// One LSP connection. `accept_loop` mints each value once.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LspClientId(pub u64);
```

`Compiler` has no production constructor; open decision 2.

```rust
// from crates/isograph_compiler/src/database.rs
#[derive(Debug, Db)]
pub struct IsographDb<THostLanguage: HostLanguage> {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
    #[tracked]
    lsp_client_map: LspClientMap,
    phantom_data: PhantomData<THostLanguage>,
}

#[derive(Debug, Default)]
pub struct LspClientMap(pub HashMap<LspClientId, SourceId<LspClientOpenFiles>>);

/// The buffers one LSP client has open.
#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct LspClientOpenFiles {
    #[key]
    pub client: LspClientId,
    pub open_files: HashMap<RelativePathToSourceFile, SourceId<OpenFile>>,
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct OpenFileKey {
    pub client: LspClientId,
    pub path: RelativePathToSourceFile,
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct OpenFile {
    #[key]
    pub key: OpenFileKey,
    pub contents: String,
}

/// The source `file_contents` reads for one client and path.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FileSourceId {
    Disk(SourceId<DiskFile>),
    Open(SourceId<OpenFile>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("LSP client {0:?} has no open-file state")]
pub struct UnknownLspClientError(pub LspClientId);
```

pico `#[key]` is one field. `OpenFile`'s key is `(client, path)` because two clients can hold different text for one path; a path-only key would make the second client's `set` replace the first's.

`SourceId<T>` derives `Debug`, `PartialEq`, `Eq` with `T` bounds; `DiskFile` and `OpenFile` derive all three, so `FileSourceId` and `LspClientOpenFiles` derive them. `SourceId`'s `Hash`, `Clone`, and `Copy` are hand-written without bounds.

Which writes touch which pico node:

- `didChange` of an open path: `set` one `OpenFile`. No tracked map, no `LspClientOpenFiles`.
- `didOpen`, or `didChange` of a path not yet open: `set` one `OpenFile` and that client's `LspClientOpenFiles`.
- `didClose`: that client's `LspClientOpenFiles`, and `remove` one `OpenFile`.
- `Started` / `Gone`: `lsp_client_map` (tracked), that client's `LspClientOpenFiles`, and on `Gone` its `OpenFile`s.
- `DiskChanged`: one `DiskFile`, and `disk_file_map` only when the key set changes (change 2).

A memo for client B or the compiler never reads client A's `LspClientOpenFiles` or `OpenFile`s. The only reads of `lsp_client_map` tracked are misses (a client with no state), so `Started` / `Gone` re-invoke only those.

`lib.rs`:

```rust
// from crates/isograph_compiler/src/lib.rs
pub use database::{
    ClientId, DiskFile, DiskFileMap, IsographDb, LspClientId, UnknownLspClientError, file_contents,
};
pub use iso_literals::{
    LineChar, LiteralId, extract_iso_literals, iso_literal_extraction, literal_id_at_location,
    parsed_iso_literal, parsed_iso_literals_in_file, text_through_last_iso_literal,
};
```

`OpenFile`, `OpenFileKey`, `LspClientOpenFiles`, `LspClientMap`, `FileSourceId`, and `file_source` are not exported; nothing outside `isograph_compiler` reads them.

### `IsographDb::default`

```rust
// from crates/isograph_compiler/src/database.rs
impl<THostLanguage: HostLanguage> Default for IsographDb<THostLanguage> {
    fn default() -> Self {
        let mut db = Self {
            storage: Storage::default(),
            disk_file_map: DiskFileMap::default(),
            lsp_client_map: LspClientMap::default(),
            phantom_data: PhantomData,
        };
        // pico registers no dependency on a counter that was never written.
        db.get_disk_file_map_mut().tracked();
        db.get_lsp_client_map_mut().tracked();
        db
    }
}
```

### Writers

Called only by `handle` (through the adapter handlers and `handle_client_connection`), never inside a memo.

```rust
// from crates/isograph_compiler/src/database.rs
impl<THostLanguage: HostLanguage> IsographDb<THostLanguage> {
    pub fn insert_lsp_client(&mut self, client: LspClientId) {
        let source_id = self.set(LspClientOpenFiles {
            client,
            open_files: HashMap::new(),
        });
        self.get_lsp_client_map_mut()
            .tracked()
            .0
            .insert(client, source_id);
    }

    pub fn remove_lsp_client(&mut self, client: LspClientId) {
        let Some(open_files_id) = self.get_lsp_client_map().untracked().0.get(&client).copied()
        else {
            return;
        };
        self.get_lsp_client_map_mut().tracked().0.remove(&client);
        let open_file_ids: Vec<SourceId<OpenFile>> =
            self.get(open_files_id).open_files.values().copied().collect();
        self.remove(open_files_id);
        open_file_ids
            .into_iter()
            .for_each(|open_file_id| self.remove(open_file_id));
    }

    pub fn set_open_file(
        &mut self,
        client: LspClientId,
        path: RelativePathToSourceFile,
        contents: String,
    ) -> Result<(), UnknownLspClientError> {
        let open_files_id = self
            .get_lsp_client_map()
            .untracked()
            .0
            .get(&client)
            .copied()
            .ok_or(UnknownLspClientError(client))?;
        let open_file_id = self.set(OpenFile {
            key: OpenFileKey { client, path },
            contents,
        });
        let mut open_files = self.get(open_files_id).clone();
        if let Entry::Vacant(vacant) = open_files.open_files.entry(path) {
            vacant.insert(open_file_id);
            self.set(open_files);
        }
        ().wrap_ok()
    }

    pub fn remove_open_file(
        &mut self,
        client: LspClientId,
        path: RelativePathToSourceFile,
    ) -> Result<(), UnknownLspClientError> {
        let open_files_id = self
            .get_lsp_client_map()
            .untracked()
            .0
            .get(&client)
            .copied()
            .ok_or(UnknownLspClientError(client))?;
        let mut open_files = self.get(open_files_id).clone();
        let Some(open_file_id) = open_files.open_files.remove(&path) else {
            return ().wrap_ok();
        };
        self.set(open_files);
        self.remove(open_file_id);
        ().wrap_ok()
    }
}
```

`insert_lsp_client` of an id already present replaces its state with an empty one. `accept_loop` mints each id once and `run_session` posts `Started` once, so that does not occur.

`remove_lsp_client` of an id with no state is a no-op. That case is real: a failed handshake posts `Gone` with no `Started` (`truncated_body_ends_that_connection`). The untracked lookup first keeps that no-op from bumping the counter.

`set_open_file` `set`s `LspClientOpenFiles` only when the path is new. pico `set_source` of an equal value moves `time_updated` to the current epoch (`crates/pico/src/database.rs` `set_source`, `Entry::Occupied` equal branch), which can report a change to a memo verified at an earlier epoch; writing only on a new path avoids that.

`set_open_file` matches on `std::collections::hash_map::Entry`. `set_open_file` and `remove_open_file` read the map and the client's source outside a memo. No dependency is recorded there.

`open_file_ids` in `remove_lsp_client` is collected because `self.remove` needs `&mut self` after the `self.get` borrow ends.

### Reads

```rust
// from crates/isograph_compiler/src/database.rs
impl<THostLanguage: HostLanguage> IsographDb<THostLanguage> {
    /// The `DiskFile` of `path`. Reads that source, so the calling memo depends on it.
    fn disk_file_source_id(&self, path: RelativePathToSourceFile) -> Option<SourceId<DiskFile>> {
        let source_id = match self.get_disk_file_map().untracked().0.get(&path).copied() {
            Some(source_id) => source_id,
            None => self.get_disk_file_map().tracked().0.get(&path).copied()?,
        };
        self.get(source_id);
        source_id.wrap_some()
    }

    /// `client`'s `OpenFile` of `path`. Reads that source, so the calling memo depends on it.
    fn open_file_source_id(
        &self,
        client: LspClientId,
        path: RelativePathToSourceFile,
    ) -> Option<SourceId<OpenFile>> {
        let open_files_id = match self.get_lsp_client_map().untracked().0.get(&client).copied() {
            Some(source_id) => source_id,
            None => self.get_lsp_client_map().tracked().0.get(&client).copied()?,
        };
        let open_file_id = self.get(open_files_id).open_files.get(&path).copied()?;
        self.get(open_file_id);
        open_file_id.wrap_some()
    }
}

#[memo]
pub(crate) fn file_source<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<FileSourceId> {
    match client {
        ClientId::Compiler => db.disk_file_source_id(path).map(FileSourceId::Disk),
        ClientId::Lsp(lsp_client) => match db.open_file_source_id(lsp_client, path) {
            Some(open_file_id) => FileSourceId::Open(open_file_id).wrap_some(),
            None => db.disk_file_source_id(path).map(FileSourceId::Disk),
        },
    }
}

/// The text of `path` as `client` reads it: that client's open buffer, otherwise disk.
pub fn file_contents<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<&str> {
    match file_source(db, client, path).as_ref()? {
        FileSourceId::Disk(source_id) => db.get(source_id.dereference()).contents.as_str(),
        FileSourceId::Open(source_id) => db.get(source_id.dereference()).contents.as_str(),
    }
    .wrap_some()
}
```

`file_source` is a memo so that client A opening or closing path `q` re-invokes `file_source(A, p)` (it read A's `LspClientOpenFiles`), which returns an equal value and backdates; `extract_iso_literals(A, p)` does not re-run its regex.

`file_source` reads the source it returns (`self.get(source_id)` in both helpers). Without that read, a hit records no dependency at all: the untracked map lookup records nothing. pico treats a derived node with no dependencies as never changed (`derived_node_changed_since` returns `false` when the dependency list is empty), so after `remove_disk_file` the memo would keep returning the removed id and `file_contents`'s `db.get` would panic. With the read, removal makes `source_node_changed_since` true and `file_source` re-runs.

A miss on `lsp_client_map` (no state for that client) reads the map tracked and falls back to disk. A miss on `open_files` is already a dependency on that client's `LspClientOpenFiles`; a later `didOpen` of that path changes it. A miss on `disk_file_map` reads the map tracked; change 2 made its counter exist from construction.

Hit on an open buffer: the memo does not read `DiskFile`. A disk change does not re-invoke it. The buffer wins until `didClose` or `Gone`.

### The client-keyed family

`HostLanguage::extract_iso_literals` is deleted from the trait and from `TypeScriptHostLanguage`, `TestHostLanguage`, and `InHost`. `IsographDb::disk_file` is deleted; `disk_file_source_id` is its replacement inside `file_source`.

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait HostLanguage: Send + Sync + Sized + 'static {
    type Error: std::fmt::Display + std::error::Error + Clone + PartialEq + Eq + 'static;
    type LiteralContext: Clone + PartialEq + Eq + Debug + 'static;

    fn extract_iso_literals_from_text(contents: &str) -> Vec<IsoLiteralExtraction<Self>>;

    fn should_skip_source_file(relative_path: &Path) -> SkipSourceFile;
}
```

Before:

```rust
// from crates/isograph_compiler/src/iso_literals.rs
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LiteralId {
    pub path: RelativePathToSourceFile,
    pub index: usize,
}

#[memo]
pub fn parsed_iso_literals_in_file<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    path: RelativePathToSourceFile,
) -> Option<Vec<(IsoLiteralExtraction<THostLanguage>, ParsedIsoLiteral)>> {
    let extractions = THostLanguage::extract_iso_literals(db, path).as_ref()?;
    // ...
}

#[memo]
pub fn text_through_last_iso_literal<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    path: RelativePathToSourceFile,
) -> Option<String> {
    let extractions = THostLanguage::extract_iso_literals(db, path).as_ref()?;
    let contents = db.disk_file(path)?.contents.reference();
    // ...
}

#[memo]
pub fn literal_id_at_location<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    path: RelativePathToSourceFile,
    line_char: LineChar,
) -> Option<LiteralId> {
    let extractions = THostLanguage::extract_iso_literals(db, path).as_ref()?;
    let file_content = db.disk_file(path)?.contents.reference();
    let index = find_iso_literal_index(line_char, file_content, extractions)?;
    LiteralId { path, index }.wrap_some()
}

#[memo]
pub fn iso_literal_extraction<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    literal_id: LiteralId,
) -> Option<IsoLiteralExtraction<THostLanguage>> {
    let extractions = THostLanguage::extract_iso_literals(db, literal_id.path).as_ref()?;
    extractions.get(literal_id.index).cloned()
}
```

After:

```rust
// from crates/isograph_compiler/src/iso_literals.rs
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LiteralId {
    pub client: ClientId,
    pub path: RelativePathToSourceFile,
    pub index: usize,
}

#[memo]
pub fn extract_iso_literals<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<IsoLiteralExtraction<THostLanguage>>> {
    let contents = file_contents(db, client, path)?;
    THostLanguage::extract_iso_literals_from_text(contents).wrap_some()
}

#[memo]
pub fn parsed_iso_literals_in_file<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<(IsoLiteralExtraction<THostLanguage>, ParsedIsoLiteral)>> {
    let extractions = extract_iso_literals(db, client, path).as_ref()?;
    extractions
        .iter()
        .map(|extraction| {
            (
                extraction.clone(),
                parsed_iso_literal(db, extraction.iso_literal_text.clone()).clone(),
            )
        })
        .collect::<Vec<_>>()
        .wrap_some()
}

#[memo]
pub fn text_through_last_iso_literal<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<String> {
    let extractions = extract_iso_literals(db, client, path).as_ref()?;
    let contents = file_contents(db, client, path)?;
    let end = extractions
        .last()
        .map(|last| last.span().as_usize_range().end)
        .unwrap_or(0);
    contents[..end].to_owned().wrap_some()
}

#[memo]
pub fn literal_id_at_location<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
    line_char: LineChar,
) -> Option<LiteralId> {
    let extractions = extract_iso_literals(db, client, path).as_ref()?;
    let file_content = file_contents(db, client, path)?;
    let index = find_iso_literal_index(line_char, file_content, extractions)?;
    LiteralId {
        client,
        path,
        index,
    }
    .wrap_some()
}

#[memo]
pub fn iso_literal_extraction<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    literal_id: LiteralId,
) -> Option<IsoLiteralExtraction<THostLanguage>> {
    let extractions =
        extract_iso_literals(db, literal_id.client, literal_id.path).as_ref()?;
    extractions.get(literal_id.index).cloned()
}
```

`LiteralId` carries the client because its `index` is a position in one client's text; a `LiteralId` from client A passed with client B's text would name a different literal. With the client inside the id, that pairing cannot be written.

`isograph_extract_typescript`:

```rust
// from crates/isograph_extract_typescript/src/lib.rs
pub fn file_literals<'a>(
    db: &'a IsographDb<TypeScriptHostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<FileLiteral<'a>>> {
    let extractions = extract_iso_literals(db, client, path).as_ref()?;
    // ... unchanged
}
```

The `impl HostLanguage for TypeScriptHostLanguage` loses its `#[memo] fn extract_iso_literals` and keeps `extract_iso_literals_from_text` (change 3) and `should_skip_source_file`.

### Semantic tokens

```rust
// from crates/isograph_lsp/src/file_semantic_tokens.rs
#[memo]
pub fn lsp_semantic_tokens_for_file<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: LspClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::SemanticToken>> {
    let client = ClientId::Lsp(client);
    let literals = parsed_iso_literals_in_file(db, client, path).as_ref()?;
    let page_content = text_through_last_iso_literal(db, client, path).as_ref()?;
    lsp_semantic_tokens(
        page_content,
        literals.iter().map(|(extraction, parsed)| {
            (extraction.iso_literal_start_index, parsed.tokens.as_slice())
        }),
    )
    .wrap_some()
}
```

It takes `LspClientId`, not `ClientId`: only an LSP client requests tokens.

### Formatting and `willSaveWaitUntil`

With open decision 6 (a), format-file-from-segments.md not landed:

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn formatting_response<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: LspClientId,
    params: lsp_types::DocumentFormattingParams,
) -> LSPRuntimeResult<<lsp_types::request::Formatting as lsp_types::request::Request>::Result> {
    let path = relative_from_uri(db, params.text_document.uri.reference()).map_err(lsp_error)?;
    format_iso_literals(db, client, path).wrap_ok()
}

pub(crate) fn will_save_wait_until_response<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: LspClientId,
    params: lsp_types::WillSaveTextDocumentParams,
) -> LSPRuntimeResult<
    <lsp_types::request::WillSaveWaitUntil as lsp_types::request::Request>::Result,
> {
    let path = relative_from_uri(db, params.text_document.uri.reference()).map_err(lsp_error)?;
    format_iso_literals(db, client, path).wrap_ok()
}

fn format_iso_literals<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: LspClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::TextEdit>> {
    let client = ClientId::Lsp(client);
    let literals = isograph_compiler::parsed_iso_literals_in_file(db, client, path).as_ref()?;
    let page_content =
        isograph_compiler::text_through_last_iso_literal(db, client, path).as_ref()?;
    literals
        .iter()
        .filter_map(|(extraction, parsed)| {
            if !parsed.errors.is_empty() {
                return None;
            }
            let start = extraction.iso_literal_start_index.0 as u32;
            let end = start + extraction.iso_literal_text.len() as u32;
            lsp_types::TextEdit {
                range: isograph_lsp::lsp_range_for_byte_range(page_content, start, end),
                new_text: isograph_parser::format_recorded(
                    extraction.iso_literal_text.as_str(),
                    parsed.tokens.as_slice(),
                ),
            }
            .wrap_some()
        })
        .collect::<Vec<_>>()
        .wrap_some()
}
```

With open decision 6 (b), format-file-from-segments.md landed first, `format_iso_literals` is that doc's version with the client and `file_contents`:

```rust
// from crates/isograph_cli/src/adapter.rs
fn format_iso_literals<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: LspClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::TextEdit>> {
    let client = ClientId::Lsp(client);
    let literals = isograph_compiler::parsed_iso_literals_in_file(db, client, path).as_ref()?;
    let contents = isograph_compiler::file_contents(db, client, path)?;
    let new_text = rewrite_iso_interiors(contents, literals);
    lsp_types::TextEdit {
        range: isograph_lsp::lsp_range_for_byte_range(contents, 0, contents.len() as u32),
        new_text,
    }
    .wrap_vec()
    .wrap_some()
}
```

`semantic_tokens_response` gains `client: LspClientId` and passes it to `lsp_semantic_tokens_for_file`. Its body is otherwise change 1's.

### Request dispatch takes `FnOnce`

Before:

```rust
// from crates/isograph_lsp/src/lsp_request_dispatch.rs
    pub fn on_request_sync<TRequest: Request>(
        self,
        handler: fn(&TState, TRequest::Params) -> LSPRuntimeResult<TRequest::Result>,
    ) -> ControlFlow<Response, Self> {
```

After:

```rust
// from crates/isograph_lsp/src/lsp_request_dispatch.rs
    pub fn on_request_sync<TRequest: Request>(
        self,
        handler: impl FnOnce(&TState, TRequest::Params) -> LSPRuntimeResult<TRequest::Result>,
    ) -> ControlFlow<Response, Self> {
```

The body is unchanged. Explicit generic arguments with `impl Trait` in argument position are stable (Rust 1.63), so `on_request_sync::<SemanticTokensFullRequest>(...)` still compiles. `LSPRequestDispatch` does not gain a `client` field. Dispatch tests that pass `fn` items keep working.

Before:

```rust
// from crates/isograph_cli/src/state.rs
fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    incoming: crate::event::LspRequest,
    before: crate::event::BeforeCompute,
) -> Vec<IsographEffect> {
    let request = incoming.request;
    let Some(after) = state.live_sessions.before_compute(before) else {
        return no_effects();
    };
    let get_response = || {
        let request =
            isograph_lsp::lsp_request_dispatch::LSPRequestDispatch::new(request, &state.db)
                .on_request_sync::<lsp_types::request::SemanticTokensFullRequest>(
                    crate::adapter::semantic_tokens_response::<THostLanguage>,
                )?
                .on_request_sync::<lsp_types::request::Formatting>(
                    crate::adapter::formatting_response::<THostLanguage>,
                )?
                .on_request_sync::<lsp_types::request::WillSaveWaitUntil>(
                    crate::adapter::will_save_wait_until_response::<THostLanguage>,
                )?
                .request();
        ControlFlow::Continue(request)
    };
```

After:

```rust
// from crates/isograph_cli/src/state.rs
fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    incoming: crate::event::LspRequest,
    before: crate::event::BeforeCompute,
) -> Vec<IsographEffect> {
    let crate::event::LspRequest { client, request } = incoming;
    let Some(after) = state.live_sessions.before_compute(before) else {
        return no_effects();
    };
    let get_response = || {
        let request =
            isograph_lsp::lsp_request_dispatch::LSPRequestDispatch::new(request, &state.db)
                .on_request_sync::<lsp_types::request::SemanticTokensFullRequest>(
                    |db, params| crate::adapter::semantic_tokens_response(db, client, params),
                )?
                .on_request_sync::<lsp_types::request::Formatting>(|db, params| {
                    crate::adapter::formatting_response(db, client, params)
                })?
                .on_request_sync::<lsp_types::request::WillSaveWaitUntil>(|db, params| {
                    crate::adapter::will_save_wait_until_response(db, client, params)
                })?
                .request();
        ControlFlow::Continue(request)
    };
```

The rest of `dispatch_lsp_request` is unchanged.

### Notifications carry the client

Before:

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug)]
pub struct LspRequest {
    pub client: ClientId,
    pub request: lsp_server::Request,
}

#[derive(Clone, Debug)]
pub enum LspEvent {
    Request(LspRequest),
    Notification(lsp_server::Notification),
    Response(lsp_server::Response),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ClientId(pub u64);
```

After:

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug)]
pub struct LspRequest {
    pub client: LspClientId,
    pub request: lsp_server::Request,
}

#[derive(Clone)]
pub struct LspNotification {
    pub client: LspClientId,
    pub notification: lsp_server::Notification,
}

impl std::fmt::Debug for LspNotification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LspNotification")
            .field("client", self.client.reference())
            .field("method", self.notification.method.reference())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub enum LspEvent {
    Request(LspRequest),
    Notification(LspNotification),
    Response(lsp_server::Response),
}
```

`LspEvent` derives `Clone` and `Debug`, so `LspNotification` provides both. `Debug` is hand-written so `run_event_loop`'s `debug!(?event)` logs the method, not the buffer (open decision 8 (a)). `ClientId` is deleted from `event.rs`; `LiveSessions`, `BeforeCompute`, `AfterCompute`, `ClientStartedEvent`, `ClientConnectionEvent::Gone`, and `LiveSessions::reply` take `isograph_compiler::LspClientId`.

`Presence` keeps its other derives and replaces `Debug`:

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum Presence {
    Present(String),
    Absent,
}

impl std::fmt::Debug for Presence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Presence::Present(contents) => f.debug_tuple("Present").field(&contents.len()).finish(),
            Presence::Absent => f.write_str("Absent"),
        }
    }
}
```

Before:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
                let client = crate::event::ClientId(next);
// ...
            lsp_server::Message::Notification(notification) => {
                crate::event::LspEvent::Notification(notification)
            }
```

After:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
                let client = isograph_compiler::LspClientId(next);
// ...
            lsp_server::Message::Notification(notification) => {
                crate::event::LspEvent::Notification(crate::event::LspNotification {
                    client,
                    notification,
                })
            }
```

`session` and `run_session` take `client: isograph_compiler::LspClientId`.

### Notification dispatch takes `FnOnce`

Before:

```rust
// from crates/isograph_cli/src/lsp_notification_dispatch.rs
    pub fn on_notification_sync<TNotification: Notification>(
        self,
        handler: fn(&mut TState, TNotification::Params) -> Vec<IsographEffect>,
    ) -> ControlFlow<Vec<IsographEffect>, Self> {
```

After:

```rust
// from crates/isograph_cli/src/lsp_notification_dispatch.rs
    pub fn on_notification_sync<TNotification: Notification>(
        self,
        handler: impl FnOnce(&mut TState, TNotification::Params) -> Vec<IsographEffect>,
    ) -> ControlFlow<Vec<IsographEffect>, Self> {
```

The body is unchanged. `LSPNotificationDispatch` does not gain a `client` field.

Before:

```rust
// from crates/isograph_cli/src/state.rs
fn dispatch_lsp_notification<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    notification: lsp_server::Notification,
) -> Vec<IsographEffect> {
    let dispatch = || {
        crate::lsp_notification_dispatch::LSPNotificationDispatch::new(notification, state)
            .on_notification_sync::<crate::lsp_socket::IsographEventNotification>(
                handle_internal_event::<THostLanguage>,
            )?
            .notification();
        ControlFlow::Continue(())
    };
    match dispatch() {
        ControlFlow::Break(effects) => effects,
        ControlFlow::Continue(()) => no_effects(),
    }
}
```

After:

```rust
// from crates/isograph_cli/src/state.rs
fn dispatch_lsp_notification<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    incoming: crate::event::LspNotification,
) -> Vec<IsographEffect> {
    let crate::event::LspNotification {
        client,
        notification,
    } = incoming;
    let dispatch = || {
        crate::lsp_notification_dispatch::LSPNotificationDispatch::new(notification, state)
            .on_notification_sync::<crate::lsp_socket::IsographEventNotification>(
                handle_internal_event::<THostLanguage>,
            )?
            .on_notification_sync::<lsp_types::notification::DidOpenTextDocument>(
                |state, params| crate::adapter::on_did_open(&mut state.db, client, params),
            )?
            .on_notification_sync::<lsp_types::notification::DidChangeTextDocument>(
                |state, params| crate::adapter::on_did_change(&mut state.db, client, params),
            )?
            .on_notification_sync::<lsp_types::notification::DidCloseTextDocument>(
                |state, params| crate::adapter::on_did_close(&mut state.db, client, params),
            )?
            .notification();
        ControlFlow::Continue(())
    };
    match dispatch() {
        ControlFlow::Break(effects) => effects,
        ControlFlow::Continue(()) => no_effects(),
    }
}
```

`dispatch_lsp` passes `LspEvent::Notification(notification)` through unchanged.

### Document sync handlers

Origin: isograph `crates/isograph_lsp/src/text_document.rs`. Delta: the client on every write; the URI through `relative_from_uri` with a typed error, not `expect`; full sync only, other `didChange` shapes are a typed error logged with `warn!`; no diagnostics publish (lsp-diagnostics.md).

```rust
// from crates/isograph_cli/src/adapter.rs
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error, derive_more::From)]
pub(crate) enum DocumentSyncError {
    #[error("{0}")]
    #[from]
    Uri(UriPathError),
    #[error("{0}")]
    #[from]
    UnknownClient(UnknownLspClientError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error, derive_more::From)]
pub(crate) enum DidChangeError {
    #[error("didChange carried {0} content changes; full document sync sends exactly one")]
    NotOneChange(usize),
    #[error("didChange carried a ranged change; full document sync sends the whole text")]
    RangedChange,
    #[error("{0}")]
    #[from]
    Sync(DocumentSyncError),
}

pub(crate) fn on_did_open<THostLanguage: HostLanguage>(
    db: &mut IsographDb<THostLanguage>,
    client: LspClientId,
    params: lsp_types::DidOpenTextDocumentParams,
) -> Vec<IsographEffect> {
    did_open(db, client, params).unwrap_or_else(|error| warn!(%error, "didOpen"));
    no_effects()
}

pub(crate) fn on_did_change<THostLanguage: HostLanguage>(
    db: &mut IsographDb<THostLanguage>,
    client: LspClientId,
    params: lsp_types::DidChangeTextDocumentParams,
) -> Vec<IsographEffect> {
    did_change(db, client, params).unwrap_or_else(|error| warn!(%error, "didChange"));
    no_effects()
}

pub(crate) fn on_did_close<THostLanguage: HostLanguage>(
    db: &mut IsographDb<THostLanguage>,
    client: LspClientId,
    params: lsp_types::DidCloseTextDocumentParams,
) -> Vec<IsographEffect> {
    did_close(db, client, params).unwrap_or_else(|error| warn!(%error, "didClose"));
    no_effects()
}

fn did_open<THostLanguage: HostLanguage>(
    db: &mut IsographDb<THostLanguage>,
    client: LspClientId,
    params: lsp_types::DidOpenTextDocumentParams,
) -> Result<(), DocumentSyncError> {
    let path = relative_from_uri(db, params.text_document.uri.reference())?;
    db.set_open_file(client, path, params.text_document.text)?;
    ().wrap_ok()
}

fn did_change<THostLanguage: HostLanguage>(
    db: &mut IsographDb<THostLanguage>,
    client: LspClientId,
    params: lsp_types::DidChangeTextDocumentParams,
) -> Result<(), DidChangeError> {
    let [change] = <[lsp_types::TextDocumentContentChangeEvent; 1]>::try_from(
        params.content_changes,
    )
    .map_err(|changes| DidChangeError::NotOneChange(changes.len()))?;
    if change.range.is_some() {
        return DidChangeError::RangedChange.wrap_err();
    }
    let path = relative_from_uri(db, params.text_document.uri.reference())
        .map_err(DocumentSyncError::Uri)?;
    db.set_open_file(client, path, change.text)
        .map_err(DocumentSyncError::UnknownClient)?;
    ().wrap_ok()
}

fn did_close<THostLanguage: HostLanguage>(
    db: &mut IsographDb<THostLanguage>,
    client: LspClientId,
    params: lsp_types::DidCloseTextDocumentParams,
) -> Result<(), DocumentSyncError> {
    let path = relative_from_uri(db, params.text_document.uri.reference())?;
    db.remove_open_file(client, path)?;
    ().wrap_ok()
}
```

`warn!` is `tracing::warn!`, the same as `on_notification_sync`'s params error. `change.text` is moved out of the one-element array, not cloned. `language_id` and `version` are not stored; nothing reads them.

`didChange` of a path the client never opened stores it as open. An unknown client is a broken invariant (`run_session` posts `Started` before any notification, and `Gone` after the last); it is logged, and reads for that client fall back to disk.

### `initialize` advertises full sync

Before:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
        text_document_sync: lsp_types::TextDocumentSyncCapability::Options(
            lsp_types::TextDocumentSyncOptions {
                will_save_wait_until: true.wrap_some(),
                ..Default::default()
            },
        )
        .wrap_some(),
```

After:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
        text_document_sync: lsp_types::TextDocumentSyncCapability::Options(
            lsp_types::TextDocumentSyncOptions {
                open_close: true.wrap_some(),
                change: lsp_types::TextDocumentSyncKind::FULL.wrap_some(),
                will_save_wait_until: true.wrap_some(),
                ..Default::default()
            },
        )
        .wrap_some(),
```

### `Started` and `Gone`

Before:

```rust
// from crates/isograph_cli/src/state.rs
    match event {
        ClientConnectionEvent::Started(started) => {
            state
                .live_sessions
                .0
                .insert(started.client, started.session);
            no_effects()
        }
        ClientConnectionEvent::Gone(client) => {
            state.live_sessions.0.remove(&client);
            no_effects()
        }
    }
```

After:

```rust
// from crates/isograph_cli/src/state.rs
    match event {
        ClientConnectionEvent::Started(started) => {
            state.db.insert_lsp_client(started.client);
            state
                .live_sessions
                .0
                .insert(started.client, started.session);
            no_effects()
        }
        ClientConnectionEvent::Gone(client) => {
            state.live_sessions.0.remove(&client);
            state.db.remove_lsp_client(client);
            state.db.run_garbage_collection();
            no_effects()
        }
    }
```

`run_garbage_collection` is open decision 5 (a). It asserts an empty dependency stack; `handle` runs outside memos.

### Tests

All feature behavior is asserted at the `handle` seam in `crates/isograph_cli/src/state.rs`: one process, no socket, no filesystem, no clock. Disk arrives as `InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged { path, presence: Presence::Present(..) }))`. Existing helpers: `with_config` (interns `/tmp/proj`), `start_client`, `lsp_request`, `formatting_params`, `perform_send`. Added helpers build `LspEvent::Notification(LspNotification { client, notification })` for `didOpen` / `didChange` / `didClose`, and `response_to(state, rx, client, id, method, params) -> lsp_server::Response` (with change 4: read `effect.response()`; without: `perform` then `rx.recv()`). `KEYWORD` is token type 15. `LspClientId(1)` and `LspClientId(2)` are test ids.

`state.rs`:

- disk `src/x.ts` is `const x = 1;\n`; client 1 `didOpen` `file:///tmp/proj/src/x.ts` with `iso(\`field Query.x{id}\`)`; Formatting from client 1: exactly one `TextEdit`, `new_text == "\n  field Query.x {\n    id,\n  }\n"`
- the same setup; `willSaveWaitUntil` from client 1 returns the same edits as Formatting
- disk `src/Home.ts` is `export const Home = iso(\`entrypoint Query.HomeRoute\`)`; client 1 `didOpen` with `iso(\`field Query.x{id}\`)`; `semanticTokens/full` from client 1: first token type `KEYWORD`, length 5
- then client 1 `didChange` (one change, no range) back to `iso(\`entrypoint Query.A\`)`: first token length 10
- then client 1 `didClose`: tokens are disk's (first token length 10, from `entrypoint Query.HomeRoute`; assert the encoded data equals a client with no buffer)
- clients 1 and 2 `didOpen` the same URI with `iso(\`field Query.x{id}\`)` and `iso(\`entrypoint Query.A\`)`: client 1's first token length 5, client 2's length 10
- client 1 has a buffer; client 2 never opened the URI: client 2's tokens equal disk's
- client 1 `didOpen` then `Gone(1)`: `live_sessions` lacks 1; `file_contents(&state.db, ClientId::Lsp(LspClientId(1)), path)` is disk's text; client 2 started afterwards reads disk
- the compiler reads disk: client 1 has a buffer; `file_contents(&state.db, ClientId::Compiler, path)` is disk's text
- open-only file: no `DiskFile` at `src/new.ts`; client 1 `didOpen` with an iso literal: tokens non-null; `didClose`: JSON `null`
- `didChange` with `contentChanges: []`, with two full changes, and with one ranged change: `file_contents` for client 1 is the `didOpen` text after each
- `didOpen` of `https://example.net/a.ts`: no effects, no open file
- `#[cfg(unix)]` `didOpen` of `file:///tmp/proj/%FF.ts`: no effects, no panic
- `didOpen` from a client that never `Started`: no effects; `file_contents` for that client is disk's text
- `Gone` of a client that never `Started`: no effects, `live_sessions` empty, no panic
- `isograph/event` HelloWorld as an `LspNotification` still returns `Timer`; an unknown notification still returns no effects
- an empty buffer: disk has an iso literal; client 1 `didOpen` with `""`: `file_contents` is `Some("")`; tokens are `Some(vec![])`, not disk's

`crates/isograph_compiler/src/database.rs` (the `DiskFile` test shape; intern paths, call writers, read `file_contents`; `TestHostLanguage`):

- `insert_lsp_client(1)`; `set_open_file(1, path, "open")`: `file_contents(Lsp(1), path)` is `"open"`; `file_contents(Compiler, path)` is disk's text or `None`
- `set_open_file` for a client with no state is `Err(UnknownLspClientError(LspClientId(9)))`; `file_contents(Lsp(9), path)` is disk's text
- a second `set_open_file` of the same path replaces the text
- two clients, one path, different text: each reads its own
- `remove_open_file` then `file_contents(Lsp(1), path)` is disk's text; `remove_open_file` of a path never opened is `Ok(())`
- `remove_lsp_client(1)` with two open paths: both read disk for client 1; client 2's buffer at one of those paths is unchanged
- `remove_lsp_client` of a never-inserted id: no-op; `lsp_client_map` length unchanged
- `remove_disk_file` of a path client 1 has open: `Lsp(1)` still reads the buffer; `Compiler` reads `None`
- inside a memo (`#[cfg(test)] #[memo] fn contents_of(db, client: ClientId, path) -> Option<String>` calling `file_contents`): `Lsp(1)` reads disk; `set_open_file`; reads the buffer; `remove_open_file`; reads disk; `remove_disk_file`; `None`, no panic. This pins the miss-then-hit transitions and `file_source`'s read of the source it returns.
- inside the same memo on a fresh database: `Compiler` reads `None`; `insert_disk_file`; reads the text

`isograph_extract_typescript` memo tests: `TypeScriptHostLanguage::extract_iso_literals(&db, path)` becomes `extract_iso_literals(&db, ClientId::Compiler, path)`; `parsed_iso_literals_in_file`, `text_through_last_iso_literal`, `literal_id_at_location` gain `ClientId::Compiler`; `LiteralId { path, index }` becomes `LiteralId { client: ClientId::Compiler, path, index }`; `file_literals` gains `ClientId::Compiler`. Facts unchanged. Added: a `LiteralId` from `literal_id_at_location` with `Lsp(1)` over a buffer names that buffer's literal in `iso_literal_extraction`, while `Compiler` over disk names disk's.

`crates/isograph_lsp/src/file_semantic_tokens.rs`: tests call `db.insert_lsp_client(LspClientId(1))` and pass `LspClientId(1)`; facts unchanged. Added: a buffer whose iso text differs from disk (disk has no iso, buffer has `entrypoint`): first token type 15, length 10; `remove_open_file` then tokens equal disk's.

`crates/isograph_cli/src/adapter.rs`: `db_with` calls `db.insert_lsp_client(client())` with `fn client() -> LspClientId { LspClientId(1) }`; `format_iso_literals(&db, client(), path())`.

- `pretty_disk_edits_applied_to_a_buffer_with_an_extra_blank_line_leave_an_extra_brace` is renamed `edits_for_a_client_without_a_buffer_applied_to_a_longer_buffer_leave_an_extra_brace`. The client has no open file, so the edits are disk ranges; the fact is unchanged.
- added `edits_for_a_client_with_that_buffer_open_are_pretty_when_applied_to_it`: disk is `PRETTY`; for each `spaces` in `0..=4`, `set_open_file(client(), path(), source_with_blank_spaces(spaces))`; the edits applied to `source_with_blank_spaces(spaces)` equal `PRETTY`
- `formatting_disk_applied_to_a_buffer_with_a_different_blank_width_is_pretty` and `applying_the_same_edits_twice_with_stale_ranges_drops_the_closing_template` keep their facts with a client that has no buffer

`crates/isograph_cli/src/lsp_socket.rs`:

- `listen_for_events` and `listen_and_reply` drop the `tokio::time::sleep(SETTLE)` before returning: the listener is bound before the function returns, so a connect queues in the backlog. Other existing sleeps stay.
- `initialize_advertises_formatting_and_will_save_wait_until` also asserts `textDocumentSync.openClose == true` and `textDocumentSync.change == 1`
- added, on one connection, no sleep: `initialize`; an `isograph/event` `DiskChanged` notification for `/tmp/proj/src/Home.ts` with `export const Home = iso(\`entrypoint Query.HomeRoute\`)`; `didOpen` of that URI with `iso(\`field Query.x{id}\`)`; `semanticTokens/full`: first token length 5. One reader thread per connection and one channel to `run_event_loop` deliver the three in order.
- tests that match `LspEvent::Notification(notification)` match `LspEvent::Notification(LspNotification { notification, .. })`; `client_gone` returns `LspClientId`

`watch.rs`: `contents` reads `file_contents(&state.db, ClientId::Compiler, interned(path)).map(str::to_owned)`; `InHost` drops `extract_iso_literals`.

`ts_graphql_react_isograph_cli/tests/cli.rs` e2e send / semantic-tokens tests keep their facts: send never opens a file.

No test asserts re-invoke counts; which memo re-runs is pico's behavior.

### Call sites

- `accept_loop` -> `LspClientId` -> `session` / `run_session` -> `LspRequest.client`, `LspNotification.client`, `ClientStartedEvent.client`, `ClientConnectionEvent::Gone`
- `handle_client_connection` `Started` -> `insert_lsp_client`, `live_sessions.0.insert`
- `handle_client_connection` `Gone` -> `live_sessions.0.remove`, `remove_lsp_client`, `run_garbage_collection`
- `dispatch_lsp_notification` -> `handle_internal_event` (function item), `on_did_open` / `on_did_change` / `on_did_close` (closures over `client`)
- `on_did_open` / `on_did_change` -> `relative_from_uri`, `set_open_file`
- `on_did_close` -> `relative_from_uri`, `remove_open_file`
- `dispatch_lsp_request` -> `semantic_tokens_response(db, client, params)` -> `lsp_semantic_tokens_for_file(db, client, path)` -> `parsed_iso_literals_in_file` / `text_through_last_iso_literal` (with `ClientId::Lsp(client)`) -> `extract_iso_literals` -> `file_contents` -> `file_source` -> `open_file_source_id` / `disk_file_source_id`
- `dispatch_lsp_request` -> `formatting_response` / `will_save_wait_until_response` (with `client`) -> `format_iso_literals(db, client, path)` -> the same family
- `file_literals`, `literal_id_at_location`, `iso_literal_extraction` -> `extract_iso_literals` / `file_contents` with the caller's `ClientId`
- tests -> `ClientId::Compiler`

### Docs this change amends

`docs-website/docs/design-docs/pico.md` Source (the `OpenFile` snippet): replace the path-keyed `OpenFile` with `OpenFileKey`, `OpenFile`, `LspClientOpenFiles`, `LspClientMap` as in this change. Replace "A path may have a `DiskFile`, an `OpenFile`, both, or neither. Artifact generation reads `DiskFile`. The LSP reads `OpenFile` when that path has one, otherwise `DiskFile`. Those are different memos. One overlay used by both, as isograph's `read_iso_literals_source` does, makes artifact generation depend on editor buffers." with: "A path may have a `DiskFile`, one `OpenFile` per LSP client that has it open, both, or neither. Every read of file text is `file_contents(db, client, path)`. `ClientId::Lsp(c)` reads `c`'s `OpenFile` if present, otherwise `DiskFile`. `ClientId::Compiler` reads `DiskFile` only. The memos are keyed by `ClientId`, so a compile reads memo slots no editor buffer reaches; isograph's `read_iso_literals_source` overlays buffers for every reader."

`pico.md` Tracked maps: the `disk_file` sentences from change 2 name `file_source` instead: "`file_source` looks up one key untracked, reads the source it found, and on a miss reads the map tracked." Add: "Open-file state is one `LspClientOpenFiles` source per client. A keystroke writes one `OpenFile`; open and close write that client's `LspClientOpenFiles`; neither touches a tracked map."

`pico.md` Garbage collection: "`handle` of `ClientConnectionEvent::Gone` runs `run_garbage_collection`." (open decision 5 (a)).

`docs-website/docs/design-docs/event-model.md`:

- Building blocks: "`DiskFile`, and `OpenFile`" stays; add `ClientId`.
- State: drop the path-keyed `struct OpenFile { path, contents }` and "`OpenFile` and its map land with the adapter." Use this change's types. "Artifact generation and watch mode read `DiskFile`. They do not read `OpenFile`." becomes "Artifact generation and watch mode read with `ClientId::Compiler`, which never reads an `OpenFile`." "The LSP adapter reads `OpenFile` when it exists for that path, otherwise `DiskFile`." becomes "The LSP adapter reads with `ClientId::Lsp(asking client)`."
- Event: `EditorChanged` and its `Buffer` are not added. "`InternalEvent` values come from outside `handle`: `DiskChanged`, `EditorChanged`." drops `EditorChanged`. "`EditorChanged` is an editor-buffer fact..." is deleted.
- Dispatch: "`EditorChanged` with `Open` sets `OpenFile`. `EditorChanged` with `Closed` removes it." becomes "`DidOpenTextDocument` and `DidChangeTextDocument` (one change, no range) call `set_open_file` for the notification's client. `DidCloseTextDocument` calls `remove_open_file`. `ClientConnectionEvent::Started` calls `insert_lsp_client`; `Gone` calls `remove_lsp_client`."
- The test-harness sentence "A test constructs a `DiskChanged` or `EditorChanged`" becomes "A test constructs a `DiskChanged` or an `LspNotification`".

`refactors/pending/format-file-from-segments.md`: "Independent of lsp-open-files.md. Does not add `OpenFile`." becomes the ordering from open decision 6. Its `format_iso_literals` takes `client: LspClientId` and reads `isograph_compiler::file_contents(db, ClientId::Lsp(client), path)`; `state.disk_file(path)` no longer exists after change 5. Its adapter-test section names the renamed extra-`}` test.

`refactors/pending/lsp-diagnostics.md`: `report_diagnostics`, for each `LspClientId` in `live_sessions` whose `PublishDiagnosticsCap` is `Yes`, reads `file_literals(db, ClientId::Lsp(id), path)` over the disk paths and that client's open paths, and publishes that client's set to that client only. `didOpen` / `didChange` arm the debounce like `DiskChanged`.
