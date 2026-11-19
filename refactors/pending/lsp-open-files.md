# Per-client open files

Requires lsp-sessions.md and lsp-outstanding.md (landed). Diagnostics (lsp-diagnostics.md) builds on this file.

A client is a reader of file text. Each LSP connection (one VS Code window) is a client with its own open-file state: the buffers it sent with `didOpen` / `didChange` and has not closed. Every read of file text goes through one family of memos keyed by `(ClientId, RelativePathToSourceFile)`: the client's open buffer for that path if it has one, otherwise the `DiskFile`. Reads and writes both take `ClientId`. The file-based compiler will be another client with its own `ClientId`, reading through the same client-keyed family; it holds open files like any client (empty unless something sets them). How the compiler gets its `ClientId` is out of scope for this doc. Highlighting, Format Document, and `willSaveWaitUntil` read through that family with the asking client.

Origin of the open-file map and handlers: isograph `OpenFileMap` / `OpenFileSource` / `insert_open_file` / `remove_open_file` (`crates/isograph_schema/src/isograph_database.rs`) and `crates/isograph_lsp/src/text_document.rs`. Origin of overlay-then-disk: isograph `read_iso_literals_source` (`crates/isograph_schema/src/validated_isograph_schema/isograph_literals.rs`) and `file_text_at_location`. Delta from isograph: isograph has one client and one path-keyed `OpenFileMap`; its overlay applies to compile as well. Here, open-file state is per client, each client's set of open paths is its own pico source, and the memos are keyed by `ClientId`, so a client with no open files reads memo slots no other client's buffer reaches. No `expect` on a URI. Full document sync only; any other `didChange` shape is logged with `warn!` and ignored.

## Open decisions

These are not approved. The body is written against the first option of each; each other option states its delta.

1. What `ClientOpenFiles.open_files` holds. The user's motivation for considering (b) is simpler cleanup on `Gone`.

   (a) `HashMap<RelativePathToSourceFile, SourceId<OpenFile>>`, one `OpenFile` source per `(client, path)`. A keystroke `set`s one `OpenFile` and re-invokes only memos that read that `OpenFile`. `Gone` must `db.remove` the client's `ClientOpenFiles` and each `OpenFile` it lists. A `Drop` impl cannot do that: `Database::remove` takes `&mut self` (`crates/pico/src/database.rs:30`, `database.rs:164-167`), and pico never collects sources ("We do not garbage collect source nodes. Those are managed by the end user.", `crates/pico/src/garbage_collection.rs:20`), so an `OpenFile` that `handle` does not remove stays in the database.

   (b) `HashMap<RelativePathToSourceFile, String>` inline; no `OpenFile` source and no `OpenFileKey`. `Gone` removes the `clients` entry and one `ClientOpenFiles` source; the buffers go with that value. A keystroke rewrites that client's whole `ClientOpenFiles`. Consequences, verified against pico:

   - `db.get` returns `&ClientOpenFiles`; pico has no in-place write. `set_open_file` clones the client's `ClientOpenFiles`, including every buffer it holds, replaces one entry, and `set`s it. `set_source` then compares the new value to the stored one with `dyn_eq` (`database.rs:340`). Each keystroke costs one clone and one comparison of all of that client's buffers.
   - Every memo that read that client's `ClientOpenFiles` re-invokes on every keystroke by that client, for every path, open or not. With the body's `file_contents`, which is a plain function, those memos are `extract_iso_literals(client, _)`, `text_through_last_iso_literal(client, _)`, and `literal_id_at_location(client, _, _)`, and each re-runs its whole body (the extract regex over the file). An equal result keeps its old `time_updated` (`crates/pico/src/execute_memoized_function.rs:162-174`), and `derived_node_changed_since` reports a change only for `Recalculated` or `Error` (`execute_memoized_function.rs:243-247`), so `parsed_iso_literals_in_file` and `lsp_semantic_tokens_for_file` of an unchanged path do not re-run.
   - To bound that re-run to one string comparison per path, (b) adds a memo between the sources and the family: `#[memo] fn file_text(db, client: ClientId, path) -> Option<String>`, owned because a memo cannot return a `&str` into a source (`pico.md` Memo). Each keystroke then re-runs `file_text(client, p)` for every `p` that client has a slot for (open paths and disk-fallback paths it has requested). Each re-run clones that path's text and compares it with `==`; an unchanged path backdates and nothing downstream re-runs. The family memos read `file_text` instead of `file_contents`, and every family memo holds its own copy of the file text in that slot.
   - No other client's memos re-invoke.

   Delta from the body for (b): `OpenFile` and `OpenFileKey` are deleted; `ClientOpenFiles.open_files` is `HashMap<RelativePathToSourceFile, String>`; `set_open_file` always clones and `set`s `ClientOpenFiles` (no `Entry::Vacant` check, since the text changes); `remove_open_file` removes the entry and `set`s; `remove_client` removes the `clients` entry and one source; `file_contents` is replaced by the `file_text` memo above; tests are unchanged in their facts.

2. A tracked map whose counter was never written. `Storage::get_impl` (`crates/pico/src/database.rs:134-140`) returns `None` before registering a dependency, so a memo that misses a key and then reads the map `tracked()` before the first write to that map records nothing, and a later insert is not seen. This applies to a `#[tracked]` field that a memo reads tracked on a miss. It does not apply to `ClientOpenFiles` or `OpenFile`: those are sources, a memo `db.get`s them by id, and no counter is involved.

   - `disk_file_map`. A family memo for a client with no buffer at a path that has no `DiskFile` reads `disk_file_map` tracked (change 2's `disk_file`). Before the first `insert_disk_file` it records nothing, and a later `DiskChanged` that creates the file is not seen by that memo. That path is reachable in production. `missing_then_present_extracts` in `isograph_extract_typescript` inserts an unrelated file first to work around this.
   - `IsographDb.clients`. A memo reads it tracked only on a miss, which is a client with no `ClientOpenFiles`, and handlers never call a memo for such a client (change 6, "Every client a memo reads has a `ClientOpenFiles`"). The bug reaches only that arm and the tests that exercise it.

   (a) `IsographDb::default` calls `tracked()` once on each tracked field (`disk_file_map` and `clients`), which writes each counter; no pico change. A future tracked field that is not listed there has the bug again, silently. (b) pico prefactor: record a dependency on an absent source. `Dependency` gains the presence observed at read time, `get_impl` registers on a miss, and `source_node_changed_since` reports "changed" only when presence differs or the present node is newer. Fixes every absent read (singletons included) for every tracked field. pico is kept unchanged from upstream isograph per AGENTS.md, so this is an upstream divergence; it would be its own change before change 2, with pico tests.
3. Order relative to format-file-from-segments.md. (a) This doc first. Change 6's `format_iso_literals` uses `parsed_iso_literals_in_file` and `text_through_last_iso_literal` with the client. format-file-from-segments.md then reads `file_contents(db, client, path)` instead of `state.disk_file(path)`. (b) format-file-from-segments.md first. Change 6's `format_iso_literals` after-snippet is the second one in that section, with `file_contents` replacing `disk_file`. Either order ships; (b) makes change 6's format diff smaller, (a) makes format-file-from-segments.md read the buffer from the start.
4. `SendLspResponseEffect` as data (change 5). The feature does not need it. `SendLspResponseEffect` is how `handle` returns a reply for `perform` to send. Tests already read the response by calling `perform` and `recv`ing on the crossbeam receiver behind the `LiveSession` (`lsp_formatting_of_a_live_client_without_a_file_is_null`, `crates/isograph_cli/src/state.rs:574`), and change 6's tests use that pattern. (a) Ship change 5: `{ sender, response }` with private fields, `pub(crate) fn response(&self) -> &lsp_server::Response`, and `pub(crate) fn send(self, token: &SideEffectToken)` called by `perform`. `Clients::after_compute` returns it. Tests can match the response on the effect without `perform` and a channel; `#[expect(clippy::type_complexity)]` and the hand-written `Debug` go away; `run_event_loop` is unchanged; `send` still takes the token. (b) Do not ship change 5; `Box<dyn FnOnce(&SideEffectToken) + Send>` stays.

## Panics

This doc adds no panic and no `UnexpectedError` for an unknown client. A memo reached for a client with no `ClientOpenFiles` misses `IsographDb.clients`, reads it tracked, and reads disk. A writer reached for such a client returns `UnknownClientError`, which the notification handler logs. Neither occurs in production (change 6, "Every client a memo reads has a `ClientOpenFiles`").

The `db.get` calls on ids found in `IsographDb.clients` or in `ClientOpenFiles.open_files` are the existing pico pattern (`disk_file`'s `db.get` of a `disk_file_map` hit): the id was found in a map that `handle` writes in the same step that it `set`s or `remove`s the source, and a stale id panics in pico (`crates/pico/src/database.rs:123-128`), which `pico.md` Source allows.

## What the user does

Highlighting. The user types in an iso literal and does not save. vscode-languageclient 9.0.1 holds full-sync `didChange` for up to 250ms, and `sendRequest` flushes pending full-text changes before sending any request (`vscode-extension/node_modules/vscode-languageclient/lib/common/client.js:335-345`). The daemon handles events in order on one loop. So `semanticTokens/full` colors the text the editor has, not the last save. Typing `iso(\`field Query.x{id}\`)` over a saved `iso(\`entrypoint Query.HomeRoute\`)` colors `field` as a keyword without a save.

Formatting. Format Document and save (`willSaveWaitUntil`) compute edits from that window's buffer. Applying them to that buffer yields the formatted file even when disk differs; the extra `}` from disk ranges applied to a longer buffer no longer happens.

Two VS Code windows on one project are two `isograph lsp` processes, two connections, and two `ClientId`s on one daemon. Each window has its own buffer for a file both have open. Window A's unsaved edits color and format only in A. Window B colors and formats B's buffer. When A saves, the watcher posts `DiskChanged`; B's buffer is still B's text. If B has no unsaved changes, VS Code reloads the file into B and sends `didChange`; B then shows the new text. A window that does not have the file open reads disk. The daemon does not push `workspace/semanticTokens/refresh`; B recolors on its next tokens request.

Closing a tab sends `didClose`; that window reads disk for that path again. Closing or crashing a window ends its connection; `handle` of `Gone` drops all of that window's buffers. The other window is unaffected.

A URI with neither a buffer for the asking window nor a `DiskFile` is JSON `null` for tokens and formatting. A non-`file:` URI is `null`. A `file:` URI whose path is not UTF-8, or a request before the config directory is interned, is an error response (`UnknownErrorCode`) naming the cause; the daemon does not panic.

`isograph send` is unchanged. A send connection never sends `didOpen`, so it has no buffers.

## Changes

Six changes, prefactors first. Change 5 ships only if open decision 4 picks (a). Change 6 does not depend on change 5.

1. Typed path conversion.
2. Tracked counters exist from construction; tracked-map writes only bump on a key change; `disk_file` tracks its miss.
3. `HostLanguage::extract_iso_literals_from_text`.
4. One `clients` map replacing `LiveSessions`.
5. `SendLspResponseEffect` is data.
6. Clients and their open files.

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

`file_path` is unchanged and stays private. `relative_from_uri` is `pub(crate)` because change 6's document-sync handlers call it.

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

Open decision 2 picks how the first is fixed. This change is option (a).

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

The TypeScript extract memo looks up a `DiskFile` and runs the regex. Change 6 runs that regex on whatever text the client reads. This change splits lookup from extract-from-text. Disk extract still looks up `DiskFile`. Highlighting is unchanged.

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

## Change 4: one `clients` map replacing `LiveSessions`

`IsographState` holds one map from `ClientId` to that client's state. Today the only per-client state is the LSP session, so `ClientState` has one field, `live_session`. Every client today is an LSP connection, so `live_session` is not an `Option`. Change 6 adds per-client state to this map's value. `ClientState` stays a named-field struct, not a newtype, because a second field is planned.

`reply`, `before_compute`, and `after_compute` move from `LiveSessions` to `Clients` with the same signatures. Each body reads `client_state.live_session` where it read `session`. They stay in `event.rs` because `BeforeCompute` and `AfterCompute` have private fields that only `event.rs` constructs.

The production readers are the ones `LiveSessions` has today: `handle_client_connection` (`Started`, `Gone`), `dispatch_lsp` (`reply`), `dispatch_lsp_request` (`before_compute`), and `send_lsp_response` (`after_compute`). This change ships alone.

### Types

Before:

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug)]
pub struct LiveSession {
    pub sender: crossbeam::channel::Sender<lsp_server::Message>,
    #[cfg_attr(not(test), expect(dead_code))]
    pub capabilities: lsp_types::ClientCapabilities,
    pub outstanding: std::collections::HashSet<lsp_server::RequestId>,
}

#[derive(Debug, Default)]
pub struct LiveSessions(pub std::collections::HashMap<ClientId, LiveSession>);
```

After:

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug)]
pub struct LiveSession {
    pub sender: crossbeam::channel::Sender<lsp_server::Message>,
    #[cfg_attr(not(test), expect(dead_code))]
    pub capabilities: lsp_types::ClientCapabilities,
    pub outstanding: std::collections::HashSet<lsp_server::RequestId>,
}

/// The daemon's state for one client.
#[derive(Debug)]
pub struct ClientState {
    pub live_session: LiveSession,
}

/// Every client that has `Started` and is not yet `Gone`.
#[derive(Debug, Default)]
pub struct Clients(pub std::collections::HashMap<ClientId, ClientState>);
```

`LiveSession` is unchanged; `ClientStartedEvent.session` and `run_session`'s construction of it (`lsp_socket.rs:185-195`) are unchanged. `ClientState` derives `Debug` because `Clients` does. Nothing clones a `ClientState`.

Before:

```rust
// from crates/isograph_cli/src/state.rs
pub struct IsographState<THostLanguage: HostLanguage> {
    pub db: IsographDb<THostLanguage>,
    pub live_sessions: LiveSessions,
    pub(crate) hello_world_debounce: Option<freddie_timer::TimerGuard>,
}
```

After:

```rust
// from crates/isograph_cli/src/state.rs
pub struct IsographState<THostLanguage: HostLanguage> {
    pub db: IsographDb<THostLanguage>,
    pub clients: Clients,
    pub(crate) hello_world_debounce: Option<freddie_timer::TimerGuard>,
}
```

### `Clients` methods

Before:

```rust
// from crates/isograph_cli/src/event.rs
impl LiveSessions {
    pub fn reply(&mut self, client: ClientId, id: lsp_server::RequestId) -> Option<BeforeCompute> {
        let session = self.0.get_mut(&client)?;
        session.outstanding.insert(id.clone());
        BeforeCompute { client, id }.wrap_some()
    }

    pub fn before_compute(&mut self, before: BeforeCompute) -> Option<AfterCompute> {
        let session = self.0.get_mut(&before.client)?;
        if !session.outstanding.contains(&before.id) {
            return None;
        }
        AfterCompute {
            client: before.client,
            id: before.id,
        }
        .wrap_some()
    }

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
        // ... unchanged
    }
}
```

After:

```rust
// from crates/isograph_cli/src/event.rs
impl Clients {
    pub fn reply(&mut self, client: ClientId, id: lsp_server::RequestId) -> Option<BeforeCompute> {
        let client_state = self.0.get_mut(&client)?;
        client_state.live_session.outstanding.insert(id.clone());
        BeforeCompute { client, id }.wrap_some()
    }

    pub fn before_compute(&mut self, before: BeforeCompute) -> Option<AfterCompute> {
        let client_state = self.0.get_mut(&before.client)?;
        if !client_state.live_session.outstanding.contains(&before.id) {
            return None;
        }
        AfterCompute {
            client: before.client,
            id: before.id,
        }
        .wrap_some()
    }

    #[expect(clippy::type_complexity)]
    pub fn after_compute(
        &mut self,
        after: AfterCompute,
        body: Result<serde_json::Value, lsp_server::ResponseError>,
    ) -> Option<Box<dyn FnOnce(&crate::effect::SideEffectToken) + Send>> {
        let client_state = self.0.get_mut(&after.client)?;
        if !client_state.live_session.outstanding.remove(&after.id) {
            return None;
        }
        let sender = client_state.live_session.sender.clone();
        // ... unchanged
    }
}
```

### `handle_client_connection`

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
            state.clients.0.insert(
                started.client,
                ClientState {
                    live_session: started.session,
                },
            );
            no_effects()
        }
        ClientConnectionEvent::Gone(client) => {
            state.clients.0.remove(&client);
            no_effects()
        }
    }
```

A second `Started` for one id replaces that client's whole `ClientState`, as it replaces the `LiveSession` today (`client_started_twice_for_the_same_id_keeps_the_second_session`). A `Gone` of an id not in the map is a no-op, as today.

### Request path

Before:

```rust
// from crates/isograph_cli/src/state.rs
        crate::event::LspEvent::Request(incoming) => {
            let client = incoming.client;
            let id = incoming.request.id.clone();
            let Some(before_compute) = state.live_sessions.reply(client, id) else {
                return no_effects();
            };
            dispatch_lsp_request(state, incoming, before_compute)
        }
// ...
    let request = incoming.request;
    let Some(after) = state.live_sessions.before_compute(before) else {
        return no_effects();
    };
// ...
    match get_response() {
        ControlFlow::Break(response) => {
            send_lsp_response(&mut state.live_sessions, after, response_body(response))
        }
        ControlFlow::Continue(request) => {
            method_not_found(&mut state.live_sessions, after, request)
        }
    }
}

fn send_lsp_response(
    sessions: &mut crate::event::LiveSessions,
    after: crate::event::AfterCompute,
    body: Result<serde_json::Value, lsp_server::ResponseError>,
) -> Vec<IsographEffect> {
    let Some(send) = sessions.after_compute(after, body) else {
        return no_effects();
    };
    // ... unchanged
}

fn method_not_found(
    sessions: &mut crate::event::LiveSessions,
    after: crate::event::AfterCompute,
    request: lsp_server::Request,
) -> Vec<IsographEffect> {
    send_lsp_response(
        sessions,
        after,
        // ... unchanged
    )
}
```

After:

```rust
// from crates/isograph_cli/src/state.rs
        crate::event::LspEvent::Request(incoming) => {
            let client = incoming.client;
            let id = incoming.request.id.clone();
            let Some(before_compute) = state.clients.reply(client, id) else {
                return no_effects();
            };
            dispatch_lsp_request(state, incoming, before_compute)
        }
// ...
    let request = incoming.request;
    let Some(after) = state.clients.before_compute(before) else {
        return no_effects();
    };
// ...
    match get_response() {
        ControlFlow::Break(response) => {
            send_lsp_response(&mut state.clients, after, response_body(response))
        }
        ControlFlow::Continue(request) => method_not_found(&mut state.clients, after, request),
    }
}

fn send_lsp_response(
    clients: &mut crate::event::Clients,
    after: crate::event::AfterCompute,
    body: Result<serde_json::Value, lsp_server::ResponseError>,
) -> Vec<IsographEffect> {
    let Some(send) = clients.after_compute(after, body) else {
        return no_effects();
    };
    // ... unchanged
}

fn method_not_found(
    clients: &mut crate::event::Clients,
    after: crate::event::AfterCompute,
    request: lsp_server::Request,
) -> Vec<IsographEffect> {
    send_lsp_response(
        clients,
        after,
        // ... unchanged
    )
}
```

`dispatch_lsp_notification` does not read the map in this change; change 6 adds its liveness check against `state.clients`.

### Construction and imports

Every `IsographState` literal writes `clients: Clients::default()` where it wrote `live_sessions: LiveSessions::default()`, and imports `Clients` from `crate::event` instead of `LiveSessions`:

- `daemon::run` (`daemon.rs:80-84`, import `daemon.rs:13`)
- `daemon.rs` tests `empty_state` (`daemon.rs:249-253`, import `daemon.rs:241`)
- `lsp_socket.rs` tests (`lsp_socket.rs:289-293`, import `lsp_socket.rs:228-231`)
- `watch.rs` tests (`watch.rs:702-706`, import `watch.rs:635`)
- `state.rs` tests `empty` (`state.rs:282-288`, import `state.rs:273-276`, which also imports `ClientState` and drops `LiveSessions`)
- `state.rs` imports `Clients` and `ClientState` instead of `LiveSessions` (`state.rs:15-17`)

### Tests

Facts unchanged. Only the path to the session changes.

`state.rs`:

- `start_client` and `dummy_session` are unchanged.
- `insert_live_session` becomes `insert_client_state`:

```rust
// from crates/isograph_cli/src/state.rs
    fn insert_client_state(
        clients: &mut Clients,
        client: ClientId,
    ) -> crossbeam::channel::Receiver<lsp_server::Message> {
        let (tx, rx) = crossbeam::channel::unbounded();
        clients.0.insert(
            client,
            ClientState {
                live_session: dummy_session(tx, lsp_types::ClientCapabilities::default()),
            },
        );
        rx
    }
```

- `reply_of_a_client_that_is_not_in_the_map_is_none`, `reply_then_after_compute_of_a_live_client_sends_the_closed_over_id`, `before_compute_after_removing_the_client_is_none`, `after_compute_after_removing_the_client_is_none`, `before_compute_after_removing_the_id_is_none`, `after_compute_after_removing_the_id_is_none`: `let mut sessions = LiveSessions::default();` becomes `let mut clients = Clients::default();`; `insert_live_session(&mut sessions, client)` becomes `insert_client_state(&mut clients, client)`; `sessions.0.get(&client).expect("the client is live").outstanding` and the `get_mut` form become `clients.0.get(&client).expect("the client is live").live_session.outstanding`; `sessions.0.remove(&client)` becomes `clients.0.remove(&client)`; method calls go through `clients`.
- `client_started_stores_the_session` and `client_started_twice_for_the_same_id_keeps_the_second_session`: `let session = state.live_sessions.0.get(&client).expect(..);` becomes `let session = &state.clients.0.get(&client).expect(..).live_session;`; the `len()` assertion reads `state.clients.0.len()`.
- `client_gone_removes_the_session`: `!state.clients.0.contains_key(&client)`. `client_gone_of_an_unknown_id_is_a_noop`: `state.clients.0.is_empty()`.
- The three `outstanding.is_empty()` assertions after a response (`state.rs:562-570`, `state.rs:610-618`, `state.rs:659-667`) read `state.clients.0.get(&client).expect("the client is live").live_session.outstanding`.

### Call sites

- `handle_client_connection` `Started` -> `clients.0.insert(client, ClientState { live_session })`
- `handle_client_connection` `Gone` -> `clients.0.remove`
- `dispatch_lsp` -> `Clients::reply`
- `dispatch_lsp_request` -> `Clients::before_compute`, `send_lsp_response` / `method_not_found` -> `Clients::after_compute`

### Docs this change amends

`refactors/pending/lsp-diagnostics.md` names `LiveSessions` (`IsographState.live_sessions`, `publish(live: &LiveSessions, ..)`, "on `LiveSessions`"). Those become `Clients` / `clients`, and a per-session field it adds is a field of `LiveSession` reached through `ClientState.live_session`.

## Change 5: `SendLspResponseEffect` is data

Ships only if open decision 4 picks (a). Change 6 does not depend on it.

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
        let client_state = self.0.get_mut(&after.client)?;
        if !client_state.live_session.outstanding.remove(&after.id) {
            return None;
        }
        let sender = client_state.live_session.sender.clone();
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
        let client_state = self.0.get_mut(&after.client)?;
        if !client_state.live_session.outstanding.remove(&after.id) {
            return None;
        }
        let (result, error) = match body {
            Ok(value) => (value.wrap_some(), None),
            Err(error) => (None, error.wrap_some()),
        };
        crate::effect::SendLspResponseEffect::new(
            client_state.live_session.sender.clone(),
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
    clients: &mut crate::event::Clients,
    after: crate::event::AfterCompute,
    body: Result<serde_json::Value, lsp_server::ResponseError>,
) -> Vec<IsographEffect> {
    let Some(effect) = clients.after_compute(after, body) else {
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

- `send_lsp_response` -> `Clients::after_compute` -> `SendLspResponseEffect::new`
- `perform` -> `SendLspResponseEffect::send`
- tests -> `SendLspResponseEffect::response`

## Change 6: clients and their open files

After this change:

- `IsographDb` holds each client's open files in pico. A tracked `clients` map holds one `SourceId<ClientOpenFiles>` per client. That client's `ClientOpenFiles` source maps each open path to that client's `OpenFile` source. `handle` of `ClientConnectionEvent::Started` creates an empty `ClientOpenFiles`; `Gone` removes it and every `OpenFile` it lists.
- `didOpen` / `didChange` / `didClose` write that client's sources.
- Every read of file text is `file_contents(db, client, path)`, called inside a memo of the client-keyed family. `file_contents` does the open check: it reads the client's `ClientOpenFiles`, then that client's `OpenFile` of `path` on a hit, otherwise disk.
- `semanticTokens/full`, `textDocument/formatting`, and `textDocument/willSaveWaitUntil` read with the asking client's `ClientId`.
- `initialize` advertises full document sync.
- `HostLanguage::extract_iso_literals` is deleted; its readers use the client-keyed family. `IsographDb::disk_file` (change 2) stays; `file_contents` calls it.

### Where the client lives

`IsographState.clients: Clients(HashMap<ClientId, ClientState>)` with `ClientState { live_session: LiveSession }` (change 4) holds connectedness and the reply channel. `LiveSession` stays outside pico: it holds a crossbeam `Sender`, which no memo reads. Open-file state is pico state on `IsographDb`, because memos read it. `IsographDb.clients` is the one database map keyed by `ClientId`; its value is the `SourceId` of that client's `ClientOpenFiles`, and per-client data that memos read is added to `ClientOpenFiles`. The value is a `SourceId` and not the struct, so a memo looks up its own client untracked and `db.get`s the hit; a map of plain structs would need a tracked read, and any client's change would re-invoke every client's memos. `handle_client_connection` writes `IsographState.clients` and `IsographDb.clients` in one arm.

### Types

Most important first.

```rust
// from crates/isograph_compiler/src/database.rs
/// A reader of file text.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ClientId(pub u64);
```

This is `crates/isograph_cli/src/event.rs`'s `ClientId`, moved to `isograph_compiler` because the database keys open-file state and memos by it, and made `pub`. Delta: `pub` instead of `pub(crate)`, and the doc comment. `isograph_compiler` re-exports it; `isograph_cli` imports `isograph_compiler::ClientId`.

Before:

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ClientId(pub u64);
```

After: `event.rs` has no `ClientId` definition and starts with `use isograph_compiler::ClientId;`. `LspRequest.client`, `LspNotification.client`, the `Clients` key, `BeforeCompute.client`, `AfterCompute.client`, `Clients::reply`, `ClientStartedEvent.client`, and `ClientConnectionEvent::Gone` keep the type name `ClientId` and now name `isograph_compiler::ClientId`.

An LSP connection's `ClientId` is minted by `accept_loop` from its local counter. The file-based compiler will be another client with its own `ClientId`, reading through `file_contents` like any client. It gets an empty `ClientOpenFiles` when it registers, as an LSP client does at `Started`, and holds open files like any client (empty unless something sets them). How the compiler gets its `ClientId` is out of scope for this doc.

```rust
// from crates/isograph_compiler/src/database.rs
#[derive(Debug, Db)]
pub struct IsographDb<THostLanguage: HostLanguage> {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
    #[tracked]
    clients: HashMap<ClientId, SourceId<ClientOpenFiles>>,
    phantom_data: PhantomData<THostLanguage>,
}

/// The buffers one client has open.
#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct ClientOpenFiles {
    #[key]
    pub client: ClientId,
    pub open_files: HashMap<RelativePathToSourceFile, SourceId<OpenFile>>,
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct OpenFileKey {
    pub client: ClientId,
    pub path: RelativePathToSourceFile,
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct OpenFile {
    #[key]
    pub key: OpenFileKey,
    pub contents: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("client {0:?} has no open-file state")]
pub struct UnknownClientError(pub ClientId);
```

`#[tracked]` on `clients` gives `get_clients()` / `get_clients_mut()` and the counter singleton `ClientsCounter` (`crates/pico_macros/src/db_macro.rs:60-95`). The field is the map itself, with no newtype; the derive takes any field type.

pico `#[key]` is one field. `OpenFile`'s key is `(client, path)` because two clients can hold different text for one path; a path-only key would make the second client's `set` replace the first's.

`SourceId<T>` derives `Debug`, `PartialEq`, `Eq` with `T` bounds (`crates/pico/src/source.rs:16`); `OpenFile` derives all three, so `ClientOpenFiles` derives them. `SourceId`'s `Hash`, `Clone`, and `Copy` are hand-written without bounds (`source.rs:26-38`).

Which writes touch which pico node, and which memos re-invoke:

- `didChange` of an open path: `set` one `OpenFile`. Memos that read that `OpenFile` re-invoke: that client's family memos for that path. No tracked map and no `ClientOpenFiles` is written.
- `didOpen`, or `didChange` of a path not yet open: `set` one `OpenFile` and that client's `ClientOpenFiles`.
- `didClose`: `set` that client's `ClientOpenFiles` and `remove` one `OpenFile`.
- An open or close by client A re-invokes every memo that called `file_contents` with A, for every path, because each read A's `ClientOpenFiles`: `extract_iso_literals(A, _)`, `text_through_last_iso_literal(A, _)`, `literal_id_at_location(A, _, _)`. Each re-runs its body. For a path whose text A reads did not change, the result is `==` and keeps its old `time_updated` (`crates/pico/src/execute_memoized_function.rs:162-174`), so `parsed_iso_literals_in_file` and `lsp_semantic_tokens_for_file` of that path do not re-run.
- No memo for client B reads A's `ClientOpenFiles` or A's `OpenFile`s, so nothing A does re-invokes B's memos.
- `Started` / `Gone`: `IsographDb.clients` (tracked), that client's `ClientOpenFiles`, and on `Gone` its `OpenFile`s. The `clients` counter re-invokes only memos that read the map tracked, which are memos that missed (no state for their client).
- `DiskChanged`: one `DiskFile`, and `disk_file_map` only when the key set changes (change 2). Memos that read that `DiskFile` re-invoke: family memos of every client without a buffer at that path.

`lib.rs`:

```rust
// from crates/isograph_compiler/src/lib.rs
pub use database::{
    ClientId, DiskFile, DiskFileMap, IsographDb, UnknownClientError, file_contents,
};
pub use iso_literals::{
    LineChar, LiteralId, extract_iso_literals, iso_literal_extraction, literal_id_at_location,
    parsed_iso_literal, parsed_iso_literals_in_file, text_through_last_iso_literal,
};
```

`OpenFile`, `OpenFileKey`, and `ClientOpenFiles` are not exported; nothing outside `isograph_compiler` reads them.

### `IsographDb::default`

```rust
// from crates/isograph_compiler/src/database.rs
impl<THostLanguage: HostLanguage> Default for IsographDb<THostLanguage> {
    fn default() -> Self {
        let mut db = Self {
            storage: Storage::default(),
            disk_file_map: DiskFileMap::default(),
            clients: HashMap::new(),
            phantom_data: PhantomData,
        };
        // pico registers no dependency on a counter that was never written.
        db.get_disk_file_map_mut().tracked();
        db.get_clients_mut().tracked();
        db
    }
}
```

### Writers

Called only by `handle` (through the adapter handlers and `handle_client_connection`), never inside a memo.

```rust
// from crates/isograph_compiler/src/database.rs
impl<THostLanguage: HostLanguage> IsographDb<THostLanguage> {
    pub fn insert_client(&mut self, client: ClientId) {
        let source_id = self.set(ClientOpenFiles {
            client,
            open_files: HashMap::new(),
        });
        self.get_clients_mut().tracked().insert(client, source_id);
    }

    pub fn remove_client(&mut self, client: ClientId) {
        let Some(open_files_id) = self.get_clients().untracked().get(&client).copied() else {
            return;
        };
        self.get_clients_mut().tracked().remove(&client);
        let open_file_ids: Vec<SourceId<OpenFile>> =
            self.get(open_files_id).open_files.values().copied().collect();
        self.remove(open_files_id);
        open_file_ids
            .into_iter()
            .for_each(|open_file_id| self.remove(open_file_id));
    }

    pub fn set_open_file(
        &mut self,
        client: ClientId,
        path: RelativePathToSourceFile,
        contents: String,
    ) -> Result<(), UnknownClientError> {
        let open_files_id = self
            .get_clients()
            .untracked()
            .get(&client)
            .copied()
            .ok_or(UnknownClientError(client))?;
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
        client: ClientId,
        path: RelativePathToSourceFile,
    ) -> Result<(), UnknownClientError> {
        let open_files_id = self
            .get_clients()
            .untracked()
            .get(&client)
            .copied()
            .ok_or(UnknownClientError(client))?;
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

`remove_client` removes the `ClientOpenFiles` source and each `OpenFile` it lists explicitly. `Drop` cannot do it: `Database::remove` takes `&mut self` (`crates/pico/src/database.rs:30`), and pico does not collect sources (`crates/pico/src/garbage_collection.rs:20`).

`insert_client` of an id already present replaces its state with an empty one. `accept_loop` mints each id once at TCP accept from a local counter (`crates/isograph_cli/src/lsp_socket.rs:96-101`), and `run_session` posts `Started` once, only after a successful `initialize` / `initialized` handshake (`lsp_socket.rs:185`), so that does not occur. Within one connection the order is `Started`, then that connection's messages, then `Gone`: `run_session` forwards messages only after it posts `Started`, and `session` posts `Gone` after `run_session` returns.

`remove_client` of an id with no state is a no-op: the keyed untracked lookup misses and the function returns before any write, `get`, or `remove`. There is no `expect`, panic, or `UnexpectedError` on that path. That case occurs on every `isograph start`: `session` posts `Gone` after every connection that got a transport, including one that never initialized (`lsp_socket.rs:134`), and the readiness probe in `start.rs:68-70` connects and drops without initializing (pinned by `drop_before_initialize_is_gone_without_started`, `lsp_socket.rs:1192`). `handle` already treats an unknown `Gone` as a no-op for `IsographState.clients` (change 4's `Gone` arm, `client_gone_of_an_unknown_id_is_a_noop`). The untracked lookup first also keeps that no-op from bumping the `clients` counter.

`set_open_file` `set`s `ClientOpenFiles` only when the path is new. pico `set_source` of an equal value moves `time_updated` to the current epoch (`crates/pico/src/database.rs:348-350`), which can report a change to a memo verified at an earlier epoch; writing only on a new path avoids that.

`set_open_file` matches on `std::collections::hash_map::Entry`. `set_open_file` and `remove_open_file` read the map and the client's source outside a memo. No dependency is recorded there.

`open_file_ids` in `remove_client` is collected because `self.remove` needs `&mut self` after the `self.get` borrow ends.

### Reads

```rust
// from crates/isograph_compiler/src/database.rs
impl<THostLanguage: HostLanguage> IsographDb<THostLanguage> {
    /// `client`'s `ClientOpenFiles`. Reads that source, so the calling memo depends on it.
    fn client_open_files(&self, client: ClientId) -> Option<&ClientOpenFiles> {
        let source_id = match self.get_clients().untracked().get(&client).copied() {
            Some(source_id) => source_id,
            None => self.get_clients().tracked().get(&client).copied()?,
        };
        self.get(source_id).wrap_some()
    }
}

/// The text of `path` as `client` reads it: that client's open buffer, otherwise disk.
pub fn file_contents<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<&str> {
    let open_file_id = db
        .client_open_files(client)
        .and_then(|client_open_files| client_open_files.open_files.get(&path).copied());
    match open_file_id {
        Some(open_file_id) => db.get(open_file_id).contents.as_str().wrap_some(),
        None => db.disk_file(path).map(|disk_file| disk_file.contents.as_str()),
    }
}
```

`file_contents` is not a memo. It returns a `&str` into a source, which a memo cannot return (`pico.md` Memo: "A memo cannot return a `&str` into a `DiskFile`"). Its reads are recorded on the memo that calls it.

Hit on an open buffer: the calling memo depends on that client's `ClientOpenFiles` and that `OpenFile`. It does not read `DiskFile`, so a disk change does not re-invoke it. The buffer wins until `didClose` or `Gone`.

Miss on `open_files`: the calling memo depends on that client's `ClientOpenFiles` and on the `DiskFile` (or, when the path has no `DiskFile`, on `disk_file_map` tracked, change 2). A later `didOpen` of that path rewrites `ClientOpenFiles`; the memo re-runs and reads the buffer.

`didClose` of a path writes that client's `ClientOpenFiles` and removes the `OpenFile` in the same `handle` call. On the next call, re-verification sees both as changed (`source_node_changed_since` is true for a removed node, `crates/pico/src/execute_memoized_function.rs:202-207`), and the body re-runs. The new `ClientOpenFiles` has no entry for that path, so the body does not `db.get` the removed id. No memo receives a `SourceId<OpenFile>` as an argument, so no memo is re-verified with a removed id as its key.

Miss on `IsographDb.clients` (no state for that client): `client_open_files` reads the map tracked and returns `None`, and `file_contents` reads disk. Production never reaches it (next section).

### Every client a memo reads has a `ClientOpenFiles`

`db.get` of a missing source panics (`crates/pico/src/database.rs:123-128`), and the miss arm above reads disk instead of reading the client's state. Both are avoided by calling memos only for clients that have a `ClientOpenFiles`:

- `handle_client_connection` is the only production writer. `Started` calls `insert_client` (an empty `ClientOpenFiles`) and inserts the `ClientState` in one arm. `Gone` removes both in one arm.
- Request path: `dispatch_lsp` calls `Clients::reply`, which returns `None` for a client not in `IsographState.clients`, and `dispatch_lsp_request` calls `Clients::before_compute`, which returns `None` again for such a client. The memos run inside `get_response`, in the same `handle` call, after both checks; nothing between them writes `IsographState.clients` or the database.
- Notification path: `dispatch_lsp` drops a notification from a client not in `IsographState.clients` before `on_did_open` / `on_did_change` / `on_did_close`. Those call writers, not memos. A writer reached for a client with no `ClientOpenFiles` returns `UnknownClientError`, which the handler logs; that occurs only if `IsographState.clients` and `IsographDb.clients` disagree.
- lsp-diagnostics.md's per-client publish iterates `IsographState.clients`.
- The file-based compiler client gets an empty `ClientOpenFiles` from `insert_client` when it registers.
- Tests that call a family memo directly call `insert_client` first.

A memo reached for a client with no `ClientOpenFiles` is neither `UnexpectedError` nor a panic: memos return `Option` and cannot return `LSPRuntimeError`, and the miss arm reads disk. The invariant is held by `handle_client_connection` writing both maps; the type system does not express it, because a `ClientId` memo argument carries no proof that a source exists.

### The client-keyed family

`HostLanguage::extract_iso_literals` is deleted from the trait and from `TypeScriptHostLanguage`, `TestHostLanguage`, and `InHost`. `IsographDb::disk_file` (change 2) stays; `file_contents` calls it for the disk read. The family memos below call `file_contents` where they called `extract_iso_literals` or `disk_file`.

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
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::SemanticToken>> {
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

### Formatting and `willSaveWaitUntil`

With open decision 3 (a), format-file-from-segments.md not landed:

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn formatting_response<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    params: lsp_types::DocumentFormattingParams,
) -> LSPRuntimeResult<<lsp_types::request::Formatting as lsp_types::request::Request>::Result> {
    let path = relative_from_uri(db, params.text_document.uri.reference()).map_err(lsp_error)?;
    format_iso_literals(db, client, path).wrap_ok()
}

pub(crate) fn will_save_wait_until_response<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    params: lsp_types::WillSaveTextDocumentParams,
) -> LSPRuntimeResult<
    <lsp_types::request::WillSaveWaitUntil as lsp_types::request::Request>::Result,
> {
    let path = relative_from_uri(db, params.text_document.uri.reference()).map_err(lsp_error)?;
    format_iso_literals(db, client, path).wrap_ok()
}

fn format_iso_literals<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::TextEdit>> {
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

With open decision 3 (b), format-file-from-segments.md landed first, `format_iso_literals` is that doc's version with the client and `file_contents`:

```rust
// from crates/isograph_cli/src/adapter.rs
fn format_iso_literals<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::TextEdit>> {
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

`semantic_tokens_response` gains `client: ClientId` and passes it to `lsp_semantic_tokens_for_file`. Its body is otherwise change 1's.

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
    let Some(after) = state.clients.before_compute(before) else {
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
    let Some(after) = state.clients.before_compute(before) else {
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
    pub client: ClientId,
    pub request: lsp_server::Request,
}

#[derive(Clone, Debug)]
pub struct LspNotification {
    pub client: ClientId,
    pub notification: lsp_server::Notification,
}

#[derive(Clone, Debug)]
pub enum LspEvent {
    Request(LspRequest),
    Notification(LspNotification),
    Response(lsp_server::Response),
}
```

`LspEvent` derives `Clone` and `Debug`, so `LspNotification` derives both. `run_event_loop`'s `tracing::debug!(?event, "event")` logs each event with its derived `Debug`, which includes the full text of every `didOpen` / `didChange` and, as today, the full contents of every `DiskChanged`; that is accepted behavior, and `run_event_loop` and `Presence` are unchanged. `ClientId` in `event.rs` is `isograph_compiler::ClientId` (see Types).

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
                let client = isograph_compiler::ClientId(next);
// ...
            lsp_server::Message::Notification(notification) => {
                crate::event::LspEvent::Notification(crate::event::LspNotification {
                    client,
                    notification,
                })
            }
```

`session` and `run_session` take `client: isograph_compiler::ClientId`.

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

`dispatch_lsp` drops a notification whose client is not in `clients`, the same as it drops a request (through `Clients::reply`, change 4). Given the per-connection order above, no notification arrives from such a client today; the check makes `handle` total over that input instead of relying on the socket's ordering.

Before:

```rust
// from crates/isograph_cli/src/state.rs
        crate::event::LspEvent::Notification(notification) => {
            dispatch_lsp_notification(state, notification)
        }
```

After:

```rust
// from crates/isograph_cli/src/state.rs
        crate::event::LspEvent::Notification(notification) => {
            if !state.clients.0.contains_key(&notification.client) {
                return no_effects();
            }
            dispatch_lsp_notification(state, notification)
        }
```

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
    UnknownClient(UnknownClientError),
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
    client: ClientId,
    params: lsp_types::DidOpenTextDocumentParams,
) -> Vec<IsographEffect> {
    did_open(db, client, params).unwrap_or_else(|error| warn!(%error, "didOpen"));
    no_effects()
}

pub(crate) fn on_did_change<THostLanguage: HostLanguage>(
    db: &mut IsographDb<THostLanguage>,
    client: ClientId,
    params: lsp_types::DidChangeTextDocumentParams,
) -> Vec<IsographEffect> {
    did_change(db, client, params).unwrap_or_else(|error| warn!(%error, "didChange"));
    no_effects()
}

pub(crate) fn on_did_close<THostLanguage: HostLanguage>(
    db: &mut IsographDb<THostLanguage>,
    client: ClientId,
    params: lsp_types::DidCloseTextDocumentParams,
) -> Vec<IsographEffect> {
    did_close(db, client, params).unwrap_or_else(|error| warn!(%error, "didClose"));
    no_effects()
}

fn did_open<THostLanguage: HostLanguage>(
    db: &mut IsographDb<THostLanguage>,
    client: ClientId,
    params: lsp_types::DidOpenTextDocumentParams,
) -> Result<(), DocumentSyncError> {
    let path = relative_from_uri(db, params.text_document.uri.reference())?;
    db.set_open_file(client, path, params.text_document.text)?;
    ().wrap_ok()
}

fn did_change<THostLanguage: HostLanguage>(
    db: &mut IsographDb<THostLanguage>,
    client: ClientId,
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
    client: ClientId,
    params: lsp_types::DidCloseTextDocumentParams,
) -> Result<(), DocumentSyncError> {
    let path = relative_from_uri(db, params.text_document.uri.reference())?;
    db.remove_open_file(client, path)?;
    ().wrap_ok()
}
```

`warn!` is `tracing::warn!`, the same as `on_notification_sync`'s params error. `change.text` is moved out of the one-element array, not cloned. `language_id` and `version` are not stored; nothing reads them.

`didChange` of a path the client never opened stores it as open. `dispatch_lsp` has already dropped notifications from a client not in `clients`, and `handle` of `Started` / `Gone` writes `IsographState.clients` and `IsographDb.clients` in the same arm, so `UnknownClientError` fires only if those two disagree. It is logged.

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

Before (change 4's after):

```rust
// from crates/isograph_cli/src/state.rs
    match event {
        ClientConnectionEvent::Started(started) => {
            state.clients.0.insert(
                started.client,
                ClientState {
                    live_session: started.session,
                },
            );
            no_effects()
        }
        ClientConnectionEvent::Gone(client) => {
            state.clients.0.remove(&client);
            no_effects()
        }
    }
```

After:

```rust
// from crates/isograph_cli/src/state.rs
    match event {
        ClientConnectionEvent::Started(started) => {
            state.db.insert_client(started.client);
            state.clients.0.insert(
                started.client,
                ClientState {
                    live_session: started.session,
                },
            );
            no_effects()
        }
        ClientConnectionEvent::Gone(client) => {
            state.clients.0.remove(&client);
            state.db.remove_client(client);
            no_effects()
        }
    }
```

Memo garbage collection is out of scope; nothing in i2 calls `run_garbage_collection` today.

### Tests

All feature behavior is asserted at the `handle` seam in `crates/isograph_cli/src/state.rs`: one process, no socket, no filesystem, no clock. Disk arrives as `InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged { path, presence: Presence::Present(..) }))`. Existing helpers: `with_config` (interns `/tmp/proj`), `start_client`, `lsp_request`, `formatting_params`, `perform_send`. Added helpers build `LspEvent::Notification(LspNotification { client, notification })` for `didOpen` / `didChange` / `didClose`, and `response_to(state, rx, client, id, method, params) -> lsp_server::Response`, which calls `handle`, passes the one `SendLspResponse` effect to `perform_send`, and `recv`s the response on the client's crossbeam receiver, the pattern of `lsp_formatting_of_a_live_client_without_a_file_is_null` (`state.rs:574`). `KEYWORD` is token type 15. `ClientId(1)` and `ClientId(2)` are test ids.

`state.rs`:

- disk `src/x.ts` is `const x = 1;\n`; client 1 `didOpen` `file:///tmp/proj/src/x.ts` with `iso(\`field Query.x{id}\`)`; Formatting from client 1: exactly one `TextEdit`, `new_text == "\n  field Query.x {\n    id,\n  }\n"`
- the same setup; `willSaveWaitUntil` from client 1 returns the same edits as Formatting
- disk `src/Home.ts` is `export const Home = iso(\`entrypoint Query.HomeRoute\`)`; client 1 `didOpen` with `iso(\`field Query.x{id}\`)`; `semanticTokens/full` from client 1: first token type `KEYWORD`, length 5
- then client 1 `didChange` (one change, no range) back to `iso(\`entrypoint Query.A\`)`: first token length 10
- then client 1 `didClose`: tokens are disk's (first token length 10, from `entrypoint Query.HomeRoute`; assert the encoded data equals a client with no buffer)
- clients 1 and 2 `didOpen` the same URI with `iso(\`field Query.x{id}\`)` and `iso(\`entrypoint Query.A\`)`: client 1's first token length 5, client 2's length 10
- client 1 has a buffer; client 2 never opened the URI: client 2's tokens equal disk's
- client 1 `didOpen` then `Gone(1)`: `clients` lacks 1; `file_contents(&state.db, ClientId(1), path)` is disk's text; client 2 started afterwards reads disk
- open-only file: no `DiskFile` at `src/new.ts`; client 1 `didOpen` with an iso literal: tokens non-null; `didClose`: JSON `null`
- `didChange` with `contentChanges: []`, with two full changes, and with one ranged change: `file_contents` for client 1 is the `didOpen` text after each
- `didOpen` of `https://example.net/a.ts`: no effects, no open file
- `#[cfg(unix)]` `didOpen` of `file:///tmp/proj/%FF.ts`: no effects, no panic
- `didOpen` from a client that never `Started`: no effects (`dispatch_lsp` drops it); `file_contents` for that client is disk's text
- `Gone` of a client that never `Started` leaves state unchanged: disk `src/x.ts` is `const x = 1;\n`; client 2 `Started` and `didOpen` with `iso(\`field Query.x{id}\`)`; `Gone(ClientId(7))`: no effects, no panic; `state.clients` holds only client 2; `file_contents` for client 2 is its buffer
- `isograph/event` HelloWorld as an `LspNotification` from a started client still returns `Timer`; an unknown notification from a started client still returns no effects. `event_notification` takes `client: ClientId`; `lsp_isograph_event_hello_world_returns_a_timer`, `lsp_isograph_event_hello_world_then_its_timer_returns_log_hello_world`, and `lsp_unknown_notification_returns_no_effects` call `start_client` first
- `isograph/event` HelloWorld from a client that never `Started`: no effects, `hello_world_debounce` is `None`
- an empty buffer: disk has an iso literal; client 1 `didOpen` with `""`: `file_contents` is `Some("")`; tokens are `Some(vec![])`, not disk's

`crates/isograph_compiler/src/database.rs` (the `DiskFile` test shape; intern paths, call writers, read `file_contents`; `TestHostLanguage`):

- `insert_client(ClientId(1))`; `set_open_file(ClientId(1), path, "open")`: `file_contents(ClientId(1), path)` is `"open"`; `file_contents(ClientId(2), path)` is disk's text, or `None` with no `DiskFile`
- `set_open_file` for a client with no state is `Err(UnknownClientError(ClientId(9)))`; `file_contents(ClientId(9), path)` is disk's text
- a second `set_open_file` of the same path replaces the text
- two clients, one path, different text: each reads its own
- `remove_open_file` then `file_contents(ClientId(1), path)` is disk's text; `remove_open_file` of a path never opened is `Ok(())`
- `remove_client(ClientId(1))` with two open paths: both read disk for client 1; client 2's buffer at one of those paths is unchanged
- `remove_client` of a never-inserted id: no-op; `get_clients().untracked().len()` unchanged
- `remove_disk_file` of a path client 1 has open: `ClientId(1)` still reads the buffer; `ClientId(2)` reads `None`
- inside a memo (`#[cfg(test)] #[memo] fn contents_of(db, client: ClientId, path) -> Option<String>` calling `file_contents`): `ClientId(1)` reads disk; `set_open_file`; reads the buffer; `remove_open_file`; reads disk; `remove_disk_file`; `None`, no panic. This pins the miss-then-hit transitions and that `didClose` does not reach the removed `OpenFile`.
- inside the same memo on a fresh database with `insert_client(ClientId(1))` and no `DiskFile`: `ClientId(1)` reads `None`; `insert_disk_file`; reads the text. This pins the tracked miss on `disk_file_map` (open decision 2).
- inside the same memo with no client inserted and a `DiskFile`: `ClientId(1)` reads disk's text; `insert_client(ClientId(1))`, `set_open_file`; reads the buffer. This pins the tracked miss on `IsographDb.clients`.
- inside the same memo with clients 1 and 2 inserted, both reading `path`: client 1 `set_open_file` of `path` then of a second path; client 2 still reads disk's text at `path`, and client 1 reads its buffer at `path`

`isograph_extract_typescript` memo tests: `TypeScriptHostLanguage::extract_iso_literals(&db, path)` becomes `extract_iso_literals(&db, client(), path)` with `fn client() -> ClientId { ClientId(1) }`, and each test calls `db.insert_client(client())` before the first memo call; no test opens a file, so every read is disk's; `parsed_iso_literals_in_file`, `text_through_last_iso_literal`, `literal_id_at_location` gain `client()`; `LiteralId { path, index }` becomes `LiteralId { client: client(), path, index }`; `file_literals` gains `client()`. Facts unchanged. Added: with `ClientId(1)` and `ClientId(2)` inserted, a `LiteralId` from `literal_id_at_location` with `ClientId(1)` over a buffer names that buffer's literal in `iso_literal_extraction`, while `ClientId(2)`, with no buffer, over disk names disk's.

`crates/isograph_lsp/src/file_semantic_tokens.rs`: tests call `db.insert_client(ClientId(1))` and pass `ClientId(1)`; facts unchanged. Added: a buffer whose iso text differs from disk (disk has no iso, buffer has `entrypoint`): first token type 15, length 10; `remove_open_file` then tokens equal disk's.

`crates/isograph_cli/src/adapter.rs`: `db_with` calls `db.insert_client(client())` with `fn client() -> ClientId { ClientId(1) }`; `format_iso_literals(&db, client(), path())`.

- `pretty_disk_edits_applied_to_a_buffer_with_an_extra_blank_line_leave_an_extra_brace` is renamed `edits_for_a_client_without_a_buffer_applied_to_a_longer_buffer_leave_an_extra_brace`. The client has no open file, so the edits are disk ranges; the fact is unchanged.
- added `edits_for_a_client_with_that_buffer_open_are_pretty_when_applied_to_it`: disk is `PRETTY`; for each `spaces` in `0..=4`, `set_open_file(client(), path(), source_with_blank_spaces(spaces))`; the edits applied to `source_with_blank_spaces(spaces)` equal `PRETTY`
- `formatting_disk_applied_to_a_buffer_with_a_different_blank_width_is_pretty` and `applying_the_same_edits_twice_with_stale_ranges_drops_the_closing_template` keep their facts with a client that has no buffer

`crates/isograph_cli/src/lsp_socket.rs`:

- `listen_for_events` and `listen_and_reply` drop the `tokio::time::sleep(SETTLE)` before returning: the listener is bound before the function returns, so a connect queues in the backlog. Other existing sleeps stay.
- `initialize_advertises_formatting_and_will_save_wait_until` also asserts `textDocumentSync.openClose == true` and `textDocumentSync.change == 1`
- added, on one connection, no sleep: `initialize`; an `isograph/event` `DiskChanged` notification for `/tmp/proj/src/Home.ts` with `export const Home = iso(\`entrypoint Query.HomeRoute\`)`; `didOpen` of that URI with `iso(\`field Query.x{id}\`)`; `semanticTokens/full`: first token length 5. One reader thread per connection and one channel to `run_event_loop` deliver the three in order.
- tests that match `LspEvent::Notification(notification)` match `LspEvent::Notification(LspNotification { notification, .. })`; `client_gone` and `take_gone` return `isograph_compiler::ClientId`; the test modules of `lsp_socket.rs` and `state.rs` drop `ClientId` from their `crate::event::{...}` imports and import `isograph_compiler::ClientId`

`watch.rs`: `contents` keeps reading `state.db.disk_file(interned(path))`; `InHost` drops `extract_iso_literals`.

`ts_graphql_react_isograph_cli/tests/cli.rs` e2e send / semantic-tokens tests keep their facts: send never opens a file.

No test asserts re-invoke counts; which memo re-runs is pico's behavior.

### Call sites

- `accept_loop` -> `isograph_compiler::ClientId(next)` -> `session` / `run_session` -> `LspRequest.client`, `LspNotification.client`, `ClientStartedEvent.client`, `ClientConnectionEvent::Gone`
- `handle_client_connection` `Started` -> `insert_client`, `clients.0.insert`
- `handle_client_connection` `Gone` -> `clients.0.remove`, `remove_client`
- `dispatch_lsp` -> `clients.0.contains_key`, then `dispatch_lsp_notification`
- `dispatch_lsp_notification` -> `handle_internal_event` (function item), `on_did_open` / `on_did_change` / `on_did_close` (closures over `client`)
- `on_did_open` / `on_did_change` -> `relative_from_uri`, `set_open_file`
- `on_did_close` -> `relative_from_uri`, `remove_open_file`
- `dispatch_lsp_request` -> `semantic_tokens_response(db, client, params)` -> `lsp_semantic_tokens_for_file(db, client, path)` -> `parsed_iso_literals_in_file` / `text_through_last_iso_literal` (with `client`) -> `extract_iso_literals` -> `file_contents` -> `client_open_files`, then `db.get` of the `OpenFile` or `disk_file`
- `dispatch_lsp_request` -> `formatting_response` / `will_save_wait_until_response` (with `client`) -> `format_iso_literals(db, client, path)` -> the same family
- `file_literals`, `literal_id_at_location`, `iso_literal_extraction` -> `extract_iso_literals` / `file_contents` with the caller's `ClientId`

### Docs this change amends

`docs-website/docs/design-docs/pico.md` Source (the `OpenFile` snippet): replace the path-keyed `OpenFile` with `OpenFileKey`, `OpenFile`, and `ClientOpenFiles` as in this change, and the Database snippet gains `#[tracked] clients: HashMap<ClientId, SourceId<ClientOpenFiles>>`. Replace "A path may have a `DiskFile`, an `OpenFile`, both, or neither. Artifact generation reads `DiskFile`. The LSP reads `OpenFile` when that path has one, otherwise `DiskFile`. Those are different memos. One overlay used by both, as isograph's `read_iso_literals_source` does, makes artifact generation depend on editor buffers." with: "A path may have a `DiskFile`, one `OpenFile` per client that has it open, both, or neither. Every read of file text is `file_contents(db, client, path)`. Client `c` reads `c`'s `OpenFile` if present, otherwise `DiskFile`. The file-based compiler will be another client with its own `ClientId`; it holds open files like any client (empty unless something sets them). The memos are keyed by `ClientId`, so a client with no open files reads memo slots no other client's buffer reaches; isograph's `read_iso_literals_source` overlays buffers for every reader."

`pico.md` Tracked maps: change 2's sentence "Extract calls it." becomes "`file_contents` calls it." Add: "A source's value can itself be a map of `SourceId`s. Open-file state is one `ClientOpenFiles` source per client (`#[key] client: ClientId`, `open_files: HashMap<RelativePathToSourceFile, SourceId<OpenFile>>`), and the tracked field `clients: HashMap<ClientId, SourceId<ClientOpenFiles>>` maps each client to it. A memo keyed by `client` looks up `clients` untracked and `db.get`s the hit, which records a dependency on that one client's `ClientOpenFiles`; another client's `Started`, `Gone`, open, or close does not re-invoke it. On a miss it reads `clients` tracked. A keystroke `set`s one `OpenFile` and does not rewrite `ClientOpenFiles`. Opening or closing a path rewrites that client's `ClientOpenFiles`, which re-invokes that client's memos that read it, for every path; each backdates when its result is `==`. `Gone` removes the `clients` entry, the `ClientOpenFiles`, and each `OpenFile` it lists; pico does not collect sources."

`pico.md` Key: "Public APIs take one of: `path`, `path` and `LineChar`, ..." lists `client` and `path`, and `client`, `path`, and `LineChar`, for the APIs that read file text; the `EntityName` keys are unchanged. "Source text is `DiskFile.contents` under `path`. A function that needs the file takes `path` and calls `db.disk_file(path)`." becomes "Source text is what `file_contents(db, client, path)` returns: that client's `OpenFile` of `path`, otherwise `DiskFile.contents`. A function that needs the file takes `client` and `path` and calls `file_contents`."

`docs-website/docs/design-docs/event-model.md`:

- Building blocks: "`DiskFile`, and `OpenFile`" stays; add `ClientId`.
- State: drop the path-keyed `struct OpenFile { path, contents }` and "`OpenFile` and its map land with the adapter." Use this change's types. "Artifact generation and watch mode read `DiskFile`. They do not read `OpenFile`." becomes "Watch mode writes `DiskFile`. Artifact generation will read through `file_contents` with its own `ClientId`; it holds open files like any client (empty unless something sets them)." "The LSP adapter reads `OpenFile` when it exists for that path, otherwise `DiskFile`." becomes "The LSP adapter reads with the asking client's `ClientId`."
- Event: `EditorChanged` and its `Buffer` are not added. "`InternalEvent` values come from outside `handle`: `DiskChanged`, `EditorChanged`." drops `EditorChanged`. "`EditorChanged` is an editor-buffer fact..." is deleted.
- Dispatch: "`EditorChanged` with `Open` sets `OpenFile`. `EditorChanged` with `Closed` removes it." becomes "`DidOpenTextDocument` and `DidChangeTextDocument` (one change, no range) call `set_open_file` for the notification's client. `DidCloseTextDocument` calls `remove_open_file`. `ClientConnectionEvent::Started` calls `insert_client`; `Gone` calls `remove_client`."
- The test-harness sentence "A test constructs a `DiskChanged` or `EditorChanged`" becomes "A test constructs a `DiskChanged` or an `LspNotification`".

`refactors/pending/format-file-from-segments.md`: "Independent of lsp-open-files.md. Does not add `OpenFile`." becomes the ordering from open decision 3. Its `format_iso_literals` takes `client: ClientId` and reads `isograph_compiler::file_contents(db, client, path)`; `state.disk_file(path)` no longer exists after change 6. Its adapter-test section names the renamed extra-`}` test.

`refactors/pending/lsp-diagnostics.md`: `report_diagnostics`, for each `ClientId` in `clients` whose `PublishDiagnosticsCap` is `Yes`, reads `file_literals(db, client, path)` over the disk paths and that client's open paths, and publishes that client's set to that client only. `didOpen` / `didChange` arm the debounce like `DiskChanged`.
