# Per-client open files

Requires lsp-sessions.md and lsp-outstanding.md (treat both as landed). Independent of lsp-diagnostics.md.

The session already has `LspClientId`, posts `LspClientGone` when the connection ends, and puts `client` on `LspRequest`. Notifications are still a bare `lsp_server::Notification`. `semanticTokens/full` reads `DiskFile` only. Unsaved buffer text is leftover.

This file is the open-file path. A `didOpen` / `didChange` / `didClose` writes an `OpenFile` source keyed by that client and path. `LspClientGone` removes every `OpenFile` for that client. Every LSP read of file text goes through one function: that client's `OpenFile` if present, otherwise `DiskFile`. Artifact generation still reads `DiskFile` only.

Origin of the map and the handlers: isograph `OpenFileMap` / `OpenFileSource` / `insert_open_file` / `remove_open_file` and `crates/isograph_lsp/src/text_document.rs`. Origin of overlay-then-disk: isograph `file_text_at_location`. Origin of `LspClientId` / `LspClientGone`: lsp-outstanding.md.

Delta from isograph: the source key includes the client, so two connections can have different unsaved text for the same path; disconnect garbage-collects that client's sources. No `expect` on the URI. Full document sync; a `didChange` that is not one full-text change is a no-op, not a panic. `Internal::EditorChanged` is not added. The notification arms write the map.

Delta from `docs-website/docs/design-docs/event-model.md` and `pico.md`: `OpenFile` is not a path-only overlay shared by every editor. It is per `LspClientId`. `EditorChanged` as an `Internal` event is not the write path.

Three shippable changes. Change 1 can land without changing highlighting. Change 3 is when an unsaved buffer is what `semanticTokens/full` colors.

## What the user does

After change 1, VS Code (or any client that honors `textDocumentSync`) sends `didOpen` / `didChange` / `didClose`. The daemon stores that text per connection. Highlighting is still the last interned `DiskFile` (save, or `isograph send`). Dropping the editor drops that connection's stored buffers.

After change 3, `semanticTokens/full` for a URI uses the asking client's buffer when that client has the file open, otherwise the interned disk contents. An unsaved edit of `iso(\`entrypoint Query.HomeRoute\`)` colors `entrypoint` without a save. A second editor on the same daemon, looking at the on-disk file, still sees disk. Closing the buffer, or dropping the client, falls back to disk. A URI with neither an open file for that client nor a `DiskFile` is still JSON `null`.

`isograph send` of `DiskChanged` is unchanged. Send does not hold a buffer.

## Change 1: per-client `OpenFile`, document sync, GC

`Lsp::Notification` carries the client. Notification handlers receive it. `didOpen` / `didChange` / `didClose` insert or remove `OpenFile` for that client. `handle` of `LspClientGone` removes that client's entries. `initialize` advertises full document sync. `semanticTokens/full` still reads `DiskFile`.

### Types

Most important first.

pico `#[key]` is one field. Two clients on one path cannot share a path-only source: `db.set` would collide, and removing one client would `remove` the source the other still points at. The key is `(client, path)`.

`LspClientId` today lives in the CLI (`event.rs` from lsp-outstanding.md, `lsp_socket.rs` from lsp-sessions.md). `OpenFile` sits on `IsographState` in `isograph_compiler`. Move the newtype to the compiler crate. The CLI uses that type. One definition.

```rust
// from crates/isograph_compiler/src/database.rs
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LspClientId(pub u64);
```

`event.rs` and `lsp_socket.rs` drop their local `LspClientId`. They use `isograph_compiler::LspClientId`. `crates/isograph_compiler/src/lib.rs` re-exports it next to `DiskFile`.

```rust
// from crates/isograph_compiler/src/database.rs
use std::collections::HashMap;
use std::marker::PhantomData;

use common_lang_types::RelativePathToSourceFile;
use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source};

#[derive(Debug, Db)]
pub struct IsographState<THostLanguage: HostLanguage> {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
    #[tracked]
    open_file_map: OpenFileMap,
    phantom_data: PhantomData<THostLanguage>,
}

#[derive(Debug, Default)]
pub struct OpenFileMap(
    pub HashMap<LspClientId, HashMap<RelativePathToSourceFile, SourceId<OpenFile>>>,
);

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
```

The tracked field is nested: outer key is the client, inner map is that client's open paths. `handle` of `LspClientGone` drops one outer key. A memo keyed by `(client, path)` looks up that pair; it does not iterate the outer map.

`Default` for `IsographState` also sets `open_file_map: OpenFileMap::default()`.

```rust
// from crates/isograph_compiler/src/database.rs
impl<THostLanguage: HostLanguage> IsographState<THostLanguage> {
    pub fn insert_open_file(
        &mut self,
        client: LspClientId,
        path: RelativePathToSourceFile,
        contents: String,
    ) {
        let key = OpenFileKey { client, path };
        let source_id = self.set(OpenFile { key, contents });
        self.get_open_file_map_mut()
            .tracked()
            .0
            .entry(client)
            .or_default()
            .insert(path, source_id);
    }

    pub fn remove_open_file(&mut self, client: LspClientId, path: RelativePathToSourceFile) {
        let source_id = {
            let map = self.get_open_file_map_mut().tracked();
            let Some(files) = map.0.get_mut(&client) else {
                return;
            };
            let Some(source_id) = files.remove(&path) else {
                return;
            };
            if files.is_empty() {
                map.0.remove(&client);
            }
            source_id
        };
        self.remove(source_id);
    }

    pub fn remove_open_files_for_client(&mut self, client: LspClientId) {
        let Some(files) = self.get_open_file_map_mut().tracked().0.remove(&client) else {
            return;
        };
        for source_id in files.into_values() {
            self.remove(source_id);
        }
    }

    pub fn open_file(
        &self,
        client: LspClientId,
        path: RelativePathToSourceFile,
    ) -> Option<&OpenFile> {
        let source_id = match self
            .get_open_file_map()
            .untracked()
            .0
            .get(&client)
            .and_then(|files| files.get(&path))
            .copied()
        {
            Some(source_id) => source_id,
            None => self
                .get_open_file_map()
                .tracked()
                .0
                .get(&client)
                .and_then(|files| files.get(&path))
                .copied()?,
        };
        self.get(source_id).wrap_some()
    }
}
```

Same miss pattern as `TypeScriptHostLanguage::extract_iso_literals` on `disk_file_map`. Hit: `untracked` then `db.get`, so another client's insert does not re-invoke. Miss: `tracked` the map, so a later insert of this pair is seen. A miss also re-invokes on an unrelated client's `didOpen`. That is the disk-map miss pattern.

`insert_open_file` of a pair that is already present replaces the `OpenFile` (`set` on the same `OpenFileKey`) and the inner map entry. `remove_open_file` of a pair that is not present is a no-op. Removing the last path for a client drops the outer key. `remove_open_files_for_client` of a client that has no entry is a no-op. An empty string is present, not absent.

`open_file` is a lookup of that client's buffer. It does not fall back to disk. Change 3 is the fallback.

### `Lsp::Notification` carries the client

Before (lsp-dispatch.md, after outstanding's `client` on `LspRequest` only):

```rust
// from crates/isograph_cli/src/event.rs
pub enum Lsp {
    Request(LspRequest),
    Notification(lsp_server::Notification),
    Response(lsp_server::Response),
}
```

After:

```rust
// from crates/isograph_cli/src/event.rs
pub struct LspNotification {
    pub client: isograph_compiler::LspClientId,
    pub notification: lsp_server::Notification,
}

pub enum Lsp {
    Request(LspRequest),
    Notification(LspNotification),
    Response(lsp_server::Response),
}
```

Session already has `client` for requests. Notifications get the same field.

Before:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
            lsp_server::Message::Notification(notification) => {
                crate::event::Lsp::Notification(notification)
            }
```

After:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
            lsp_server::Message::Notification(notification) => {
                crate::event::Lsp::Notification(crate::event::LspNotification {
                    client,
                    notification,
                })
            }
```

`posted_internal` and the other `lsp_socket.rs` tests that match `Lsp::Notification(notification)` match `Lsp::Notification(LspNotification { notification, .. })`.

### Notification dispatch receives the client

`on_notification_sync` takes a function pointer, so a handler cannot capture `client`. Every notification handler takes it. `isograph/event` does not read it.

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
pub struct LSPNotificationDispatch<'state, TState> {
    notification: lsp_server::Notification,
    state: &'state mut TState,
    client: isograph_compiler::LspClientId,
}

impl<'state, TState> LSPNotificationDispatch<'state, TState> {
    pub fn new(
        notification: lsp_server::Notification,
        state: &'state mut TState,
        client: isograph_compiler::LspClientId,
    ) -> Self {
        Self {
            notification,
            state,
            client,
        }
    }

    pub fn on_notification_sync<TNotification: Notification>(
        self,
        handler: fn(
            &mut TState,
            isograph_compiler::LspClientId,
            TNotification::Params,
        ) -> Vec<IsographEffect>,
    ) -> ControlFlow<Vec<IsographEffect>, Self> {
        if self.notification.method != TNotification::METHOD {
            return ControlFlow::Continue(self);
        }
        match self
            .notification
            .extract::<TNotification::Params>(TNotification::METHOD)
        {
            Ok(params) => ControlFlow::Break(handler(self.state, self.client, params)),
            Err(lsp_server::ExtractError::MethodMismatch(notification)) => {
                ControlFlow::Continue(Self {
                    notification,
                    state: self.state,
                    client: self.client,
                })
            }
            Err(lsp_server::ExtractError::JsonError { method, error }) => {
                warn!(method = method.as_str(), error = %error, "notification params");
                ControlFlow::Break(Vec::new())
            }
        }
    }
}
```

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
        crate::lsp_notification_dispatch::LSPNotificationDispatch::new(
            notification,
            state,
            client,
        )
        .on_notification_sync::<crate::lsp_socket::IsographEventNotification>(
            on_isograph_event::<THostLanguage>,
        )?
        .on_notification_sync::<lsp_types::notification::DidOpenTextDocument>(
            crate::adapter::on_did_open::<THostLanguage>,
        )?
        .on_notification_sync::<lsp_types::notification::DidChangeTextDocument>(
            crate::adapter::on_did_change::<THostLanguage>,
        )?
        .on_notification_sync::<lsp_types::notification::DidCloseTextDocument>(
            crate::adapter::on_did_close::<THostLanguage>,
        )?
        .notification();
        ControlFlow::Continue(())
    };
    match dispatch() {
        ControlFlow::Break(effects) => effects,
        ControlFlow::Continue(()) => Vec::new(),
    }
}

fn on_isograph_event<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    _client: isograph_compiler::LspClientId,
    internal: Internal,
) -> Vec<IsographEffect> {
    handle_internal(state, internal)
}
```

`state.rs` tests that construct `Lsp::Notification(lsp_server::Notification { ... })` construct `LspNotification { client: LspClientId(1), notification }`. `LspClientId(1)` is a test id; those tests do not go through `accept_loop`.

### Document sync handlers

Origin: isograph `text_document.rs`. Delta: client on insert/remove; URI via `file_path` (already in `adapter.rs`), not `expect`; empty or ranged `didChange` is a no-op; no diagnostics publish (lsp-diagnostics.md).

`file_path` stays in `adapter.rs` and becomes `pub(crate)` so the handlers and `semantic_tokens_response` share it.

A relative path needs `CurrentWorkingDirectory`. DiskChanged already `expect`s it. The LSP arms do not: missing cwd or a non-file URI is no effects.

```rust
// from crates/isograph_cli/src/adapter.rs
use isograph_compiler::{HostLanguage, LspClientId};
use lsp_types::notification::Notification;
use prelude::Postfix;

pub(crate) fn on_did_open<THostLanguage: HostLanguage>(
    state: &mut isograph_compiler::IsographState<THostLanguage>,
    client: LspClientId,
    params: <lsp_types::notification::DidOpenTextDocument as Notification>::Params,
) -> Vec<crate::effect::IsographEffect> {
    let Some(path) = relative_from_uri(state, params.text_document.uri.reference()) else {
        return Vec::new();
    };
    state.insert_open_file(client, path, params.text_document.text);
    Vec::new()
}

pub(crate) fn on_did_change<THostLanguage: HostLanguage>(
    state: &mut isograph_compiler::IsographState<THostLanguage>,
    client: LspClientId,
    params: <lsp_types::notification::DidChangeTextDocument as Notification>::Params,
) -> Vec<crate::effect::IsographEffect> {
    let [change] = params.content_changes.as_slice() else {
        return Vec::new();
    };
    if change.range.is_some() {
        return Vec::new();
    }
    let Some(path) = relative_from_uri(state, params.text_document.uri.reference()) else {
        return Vec::new();
    };
    state.insert_open_file(client, path, change.text.clone());
    Vec::new()
}

pub(crate) fn on_did_close<THostLanguage: HostLanguage>(
    state: &mut isograph_compiler::IsographState<THostLanguage>,
    client: LspClientId,
    params: <lsp_types::notification::DidCloseTextDocument as Notification>::Params,
) -> Vec<crate::effect::IsographEffect> {
    let Some(path) = relative_from_uri(state, params.text_document.uri.reference()) else {
        return Vec::new();
    };
    state.remove_open_file(client, path);
    Vec::new()
}

fn relative_from_uri<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographState<THostLanguage>,
    uri: &lsp_types::Uri,
) -> Option<common_lang_types::RelativePathToSourceFile> {
    let absolute = file_path(uri)?;
    let cwd = *state.get_singleton::<common_lang_types::CurrentWorkingDirectory>()?;
    common_lang_types::relative_path_from_absolute_and_working_directory(cwd, absolute.reference())
        .wrap_some()
}
```

`didChange` accepts one change with `range: None`. That is full document sync. Zero changes, two changes, or a ranged (incremental) change: no-op. Do not treat `change.text` as the whole file when `range` is `Some`.

`language_id` and `version` are not stored.

### `initialize` advertises full sync

Before: `ServerCapabilities` with only `semantic_tokens_provider`. After, also:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
            text_document_sync: lsp_types::TextDocumentSyncCapability::Options(
                lsp_types::TextDocumentSyncOptions {
                    open_close: true.wrap_some(),
                    change: lsp_types::TextDocumentSyncKind::FULL.wrap_some(),
                    ..Default::default()
                },
            )
            .wrap_some(),
```

`open_close` / `change` are `lsp_types` fields. `FULL` is why `on_did_change` requires one rangeless change.

### `LspClientGone` garbage-collects

lsp-outstanding.md: `handle` of `LspClientGone` is `Vec::new()`. This slice is the first writer of sources on that event.

```rust
// from crates/isograph_cli/src/state.rs
        IsographEvent::LspClientGone(gone) => {
            state.remove_open_files_for_client(gone.client);
            Vec::new()
        }
```

`handle` is the only writer of `OpenFile`, same as `DiskFile`. `SessionGuard` already drops the live-session writer. It does not touch pico sources.

### Tests

Compiler, `database.rs` (same shape as the `DiskFile` tests; intern paths, call insert/remove, no `PathBuf`):

- `insert_open_file` then `open_file` is the contents
- a second insert of the same client and path replaces contents; outer map length 1, inner length 1
- empty string is stored
- two paths for one client are two inner entries
- two clients, same path, different contents: each `open_file` returns that client's text; they are two `OpenFile` sources
- `remove_open_file` then `open_file` is `None`
- `remove_open_file` of a never-present pair is a no-op
- removing the last path for a client drops the outer key (`get_open_file_map().untracked().0.get(&client)` is `None`)
- `remove_open_files_for_client` removes both paths; the other client's path remains
- `remove_open_files_for_client` of a never-present client is a no-op
- `insert_disk_file` of the same path does not change `open_file`; `insert_open_file` does not change `disk_file`

`handle`, `state.rs` (`with_config()`, `LspClientId(1)`, helpers that build `DidOpen` / `DidChange` / `DidClose` notifications as `LspNotification`):

- `didOpen` of `file:///tmp/proj/src/a.ts` with text `unsaved` then `open_file(client, interned("src/a.ts")).contents` is `unsaved`. No effects.
- `didChange` with one rangeless change replaces the text
- `didClose` then `open_file` is `None`
- `didOpen` of a non-file URI: no entry, no effects
- `didChange` with `contentChanges: []`: open text unchanged
- `didChange` with `range` set: open text unchanged
- `didChange` with two full changes: open text unchanged
- `LspClientGone` after two `didOpen`s: both gone; a `DiskFile` at one of those paths is still there
- `isograph/event` HelloWorld still returns `LogHelloWorld` with the new handler signature
- unknown notification still no effects

`lsp_socket.rs` (listen_and_reply / listen_for_events, intern config as today):

- initialize result has `textDocumentSync.change == 1` (`FULL`) and `openClose == true`
- two connections, each `didOpen` the same URI with different text: after settle, each client's `open_file` is that connection's text. Inspect via `handle` on a harness `IsographState` is not available from listen_and_reply's private state. Drive this fact through `handle` tests above; the socket test is: drop one connection, the remaining connection's later `didChange` still succeeds (no panic). After both drop, a third connection HelloWorld still works.
- drop a connection after `didOpen`: `event_rx` receives `LspClientGone` (already an outstanding test). This slice adds: a subsequent `didOpen` from a new connection of the same URI is that new client's file only (no leftover from the dropped id). Observable after change 3 via tokens; this slice asserts via `handle` + `LspClientGone` as above.

Do not add a production function only tests call.

### Call sites

- `accept_loop` -> `LspClientId` -> `session` -> `LspNotification.client`
- `didOpen` / `didChange` -> `insert_open_file`
- `didClose` -> `remove_open_file`
- `LspClientGone` -> `remove_open_files_for_client`
- `isograph/event` -> `handle_internal` (client unused)
- later change 3 -> `open_file` / `lsp_file_contents`

### Docs this change amends

`docs-website/docs/design-docs/pico.md` `OpenFile`:

```rust
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
struct OpenFileKey {
    client: LspClientId,
    path: RelativePathToSourceFile,
}

#[derive(Clone, PartialEq, Eq, Source)]
struct OpenFile {
    #[key]
    key: OpenFileKey,
    contents: String,
}
```

A path may have a `DiskFile`, any number of `OpenFile`s (one per client), both, or neither. Artifact generation reads `DiskFile`. The LSP reads that client's `OpenFile` when it exists, otherwise `DiskFile`. `LspClientGone` removes every `OpenFile` whose key contains that client.

`docs-website/docs/design-docs/event-model.md` State: drop the path-only `OpenFile { path, contents }`. Same `OpenFile` / `OpenFileKey` / `OpenFileMap` as above. `handle` of `didOpen` / `didChange` / `didClose` is the writer. There is no `Internal::EditorChanged`.

`docs-website/docs/design-docs/event-model.md` Dispatch: `EditorChanged` sentences become: `DidOpenTextDocument` / `DidChangeTextDocument` (one rangeless change) call `insert_open_file`. `DidCloseTextDocument` calls `remove_open_file`. `LspClientGone` calls `remove_open_files_for_client`.

`docs-website/docs/design-docs/event-model.md` Event: `EditorChanged` / `Buffer` types are not added.

`refactors/pending/event-model.md`: after lsp-sessions.md, this file's three changes, then lsp-diagnostics.md. lsp-diagnostics.md currently walks interned `DiskFile`s; when it lands after this, it uses change 3's `lsp_file_contents` per live session, not a global overlay.

`refactors/pending/lsp-outstanding.md`: `handle` of `LspClientGone` is no longer empty; it is `remove_open_files_for_client`.

## Change 2: `extract_iso_literals_from_text`

The TypeScript extract memo looks up a `DiskFile` and runs the regex. Change 3 needs that regex on overlay text. Split lookup from extract-from-text. Disk extract still looks up `DiskFile`. No `OpenFile` reads. Highlighting unchanged.

Origin: the regex body of `TypeScriptHostLanguage::extract_iso_literals`. Delta: the body after the lookup is an associated function on `HostLanguage` that takes `&str`.

```rust
// from crates/isograph_compiler/src/host_language.rs
    fn extract_iso_literals_from_text(contents: &str) -> Vec<IsoLiteralExtraction<Self>>;

    fn extract_iso_literals(
        db: &IsographState<Self>,
        path: RelativePathToSourceFile,
    ) -> &Option<Vec<IsoLiteralExtraction<Self>>>;
```

`extract_iso_literals_from_text` is not a memo. Missing file is the lookup. Empty string is `vec![]`.

TypeScript impl: move the `captures_iter` body into `extract_iso_literals_from_text`. The memo becomes:

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    #[memo]
    fn extract_iso_literals(
        db: &IsographState<Self>,
        path: RelativePathToSourceFile,
    ) -> Option<Vec<IsoLiteralExtraction<Self>>> {
        let source_id = match db.get_disk_file_map().untracked().0.get(&path).copied() {
            Some(source_id) => source_id,
            None => db.get_disk_file_map().tracked().0.get(&path).copied()?,
        };
        let contents = db.get(source_id).contents.reference();
        Self::extract_iso_literals_from_text(contents).wrap_some()
    }

    fn extract_iso_literals_from_text(contents: &str) -> Vec<IsoLiteralExtraction<Self>> {
        EXTRACT_ISO_LITERAL
            .captures_iter(contents)
            .filter_map(|captures| { /* same body as today */ })
            .collect()
    }
```

`TestHostLanguage` and `InHost`:

```rust
        fn extract_iso_literals_from_text(_contents: &str) -> Vec<IsoLiteralExtraction<Self>> {
            Vec::new()
        }
```

Their `extract_iso_literals` bodies stay returning `&NONE`.

### Tests

Existing extract / `parsed_iso_literals_in_file` / tokens tests stay green.

Added:

- `extract_iso_literals_from_text` of `export const Home = iso(\`entrypoint Query.HomeRoute\`)` is one extraction, text `entrypoint Query.HomeRoute`
- of `""` is empty
- of `const x = 1;` is empty
- of two `iso(\`...\`)` interiors is two extractions in source order
- `TypeScriptHostLanguage::extract_iso_literals(db, path)` of an interned `DiskFile` with that same text equals `extract_iso_literals_from_text` of that text wrapped in `Some`

### Call sites

- `HostLanguage::extract_iso_literals` (disk memo) -> `extract_iso_literals_from_text`
- change 3 `lsp_extract_iso_literals` -> `extract_iso_literals_from_text`

### Docs this change amends

`docs-website/docs/design-docs/pico.md` extract snippet: the memo looks up `DiskFile`, then `extract_iso_literals_from_text`. The regex is not inlined in the memo.

## Change 3: LSP file text is open then disk

One function. Every LSP read of file text calls it. Compile still uses `DiskFile` / `extract_iso_literals(db, path)`.

Request handlers receive `LspClientId`. `semanticTokens/full` uses the asking client.

### `lsp_file_contents`

```rust
// from crates/isograph_compiler/src/database.rs
impl<THostLanguage: HostLanguage> IsographState<THostLanguage> {
    fn disk_file_source_id(
        &self,
        path: RelativePathToSourceFile,
    ) -> Option<SourceId<DiskFile>> {
        match self.get_disk_file_map().untracked().0.get(&path).copied() {
            Some(source_id) => source_id.wrap_some(),
            None => self.get_disk_file_map().tracked().0.get(&path).copied(),
        }
    }

    pub fn disk_file(&self, path: RelativePathToSourceFile) -> Option<&DiskFile> {
        self.get(self.disk_file_source_id(path)?).wrap_some()
    }

    pub fn lsp_file_contents(
        &self,
        client: LspClientId,
        path: RelativePathToSourceFile,
    ) -> Option<&str> {
        if let Some(open) = self.open_file(client, path) {
            return open.contents.reference().wrap_some();
        }
        self.disk_file_source_id(path)
            .map(|source_id| self.get(source_id).contents.reference())
    }
}
```

`disk_file` today `untracked` then `?`, so a miss never `tracked`s the map and a later `Present` of that path is not seen. `extract_iso_literals` already uses the miss-tracked pattern. This slice makes `disk_file` use that pattern. `lsp_file_contents` uses `disk_file_source_id` on overlay miss, so a later disk `Present` of a path this client does not have open is seen.

If `open_file` hits, this function does not read `DiskFile`. A later disk change does not re-invoke a memo that took the overlay. The buffer still wins until `didClose` or `LspClientGone`.

`TypeScriptHostLanguage::extract_iso_literals` can keep its own lookup or call `disk_file`. Either way it does not call `lsp_file_contents`.

Nothing in `isograph_lsp` or `adapter.rs` calls `disk_file`, `get_disk_file_map`, or `extract_iso_literals(db, path)` after this slice. Those are compile / disk tests.

### LSP extract and the tokens pipeline

```rust
// from crates/isograph_compiler/src/iso_literals.rs
#[memo]
pub fn lsp_extract_iso_literals<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    client: LspClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<IsoLiteralExtraction<THostLanguage>>> {
    let contents = db.lsp_file_contents(client, path)?;
    THostLanguage::extract_iso_literals_from_text(contents).wrap_some()
}

fn with_parsed_literals<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    extractions: &[IsoLiteralExtraction<THostLanguage>],
) -> Vec<(IsoLiteralExtraction<THostLanguage>, ParsedIsoLiteral)> {
    extractions
        .iter()
        .map(|extraction| {
            (
                extraction.clone(),
                parsed_iso_literal(db, extraction.iso_literal_text.clone()).clone(),
            )
        })
        .collect()
}

#[memo]
pub fn parsed_iso_literals_in_file<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: RelativePathToSourceFile,
) -> Option<Vec<(IsoLiteralExtraction<THostLanguage>, ParsedIsoLiteral)>> {
    let extractions = THostLanguage::extract_iso_literals(db, path).as_ref()?;
    with_parsed_literals(db, extractions).wrap_some()
}

#[memo]
pub fn lsp_parsed_iso_literals_in_file<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    client: LspClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<(IsoLiteralExtraction<THostLanguage>, ParsedIsoLiteral)>> {
    let extractions = lsp_extract_iso_literals(db, client, path).as_ref()?;
    with_parsed_literals(db, extractions).wrap_some()
}

#[memo]
pub fn text_through_last_iso_literal<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: RelativePathToSourceFile,
) -> Option<String> {
    let extractions = THostLanguage::extract_iso_literals(db, path).as_ref()?;
    let contents = db.disk_file(path)?.contents.reference();
    text_through_last(contents, extractions).wrap_some()
}

#[memo]
pub fn lsp_text_through_last_iso_literal<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    client: LspClientId,
    path: RelativePathToSourceFile,
) -> Option<String> {
    let extractions = lsp_extract_iso_literals(db, client, path).as_ref()?;
    let contents = db.lsp_file_contents(client, path)?;
    text_through_last(contents, extractions).wrap_some()
}

fn text_through_last<THostLanguage: HostLanguage>(
    contents: &str,
    extractions: &[IsoLiteralExtraction<THostLanguage>],
) -> String {
    let end = extractions
        .last()
        .map(|last| last.span().as_usize_range().end)
        .unwrap_or(0);
    contents[..end].to_owned()
}
```

`parsed_iso_literals_in_file` and `text_through_last_iso_literal` stay disk-only. They are the compile / existing-test path. Tokens stop calling them.

`literal_id_at_location` and `iso_literal_extraction` still read disk. Hover is not this slice. When hover lands, it takes `LspClientId` and goes through `lsp_file_contents` / `lsp_extract_iso_literals`. Do not leave hover on `disk_file`.

```rust
// from crates/isograph_lsp/src/file_semantic_tokens.rs
#[memo]
pub fn lsp_semantic_tokens_for_file<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    client: LspClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::SemanticToken>> {
    let literals = lsp_parsed_iso_literals_in_file(db, client, path).as_ref()?;
    let page_content = lsp_text_through_last_iso_literal(db, client, path).as_ref()?;
    lsp_semantic_tokens(
        page_content,
        literals.iter().map(|(extraction, parsed)| {
            (extraction.iso_literal_start_index, parsed.tokens.as_slice())
        }),
    )
    .wrap_some()
}
```

Existing tokens tests intern a `DiskFile` and pass `LspClientId(1)` with no `OpenFile`. Overlay miss falls back to disk. Those tests keep their facts.

`lib.rs` re-exports `lsp_extract_iso_literals`, `lsp_parsed_iso_literals_in_file`, `lsp_text_through_last_iso_literal`.

### Request dispatch passes the client

`LSPRequestDispatch` is a function-pointer chain, same capture problem as notifications. Handlers take `LspClientId`.

Before:

```rust
// from crates/isograph_lsp/src/lsp_request_dispatch.rs
    pub fn new(request: lsp_server::Request, state: &'state TState) -> Self { ... }

    pub fn on_request_sync<TRequest: Request>(
        self,
        handler: fn(&TState, TRequest::Params) -> LSPRuntimeResult<TRequest::Result>,
    ) -> ControlFlow<Response, Self> {
        ...
                Ok((request_id, params)) => {
                    let response = handler(self.state, params).and_then(|handler_result| {
```

After:

```rust
// from crates/isograph_lsp/src/lsp_request_dispatch.rs
pub struct LSPRequestDispatch<'state, TState> {
    request: lsp_server::Request,
    state: &'state TState,
    client: isograph_compiler::LspClientId,
}

impl<'state, TState> LSPRequestDispatch<'state, TState> {
    pub fn new(
        request: lsp_server::Request,
        state: &'state TState,
        client: isograph_compiler::LspClientId,
    ) -> Self {
        LSPRequestDispatch {
            request,
            state,
            client,
        }
    }

    pub fn on_request_sync<TRequest: Request>(
        self,
        handler: fn(
            &TState,
            isograph_compiler::LspClientId,
            TRequest::Params,
        ) -> LSPRuntimeResult<TRequest::Result>,
    ) -> ControlFlow<Response, Self> {
```

`handler(self.state, self.client, params)`. `MethodMismatch` / `Continue` copies `client` the way change 1 copies it on notifications.

Dispatch tests (`AtomicI32` handlers) gain `LspClientId` on `new` and on the handler signatures. They do not read it.

```rust
// from crates/isograph_cli/src/state.rs
fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &IsographState<THostLanguage>,
    incoming: crate::event::LspRequest,
) -> Vec<IsographEffect> {
    let crate::event::LspRequest {
        client,
        request,
        reply,
    } = incoming;
    let get_response = || {
        let request = isograph_lsp::lsp_request_dispatch::LSPRequestDispatch::new(
            request,
            state,
            client,
        )
        .on_request_sync::<lsp_types::request::SemanticTokensFullRequest>(
            crate::adapter::semantic_tokens_response::<THostLanguage>,
        )?
        .request();
        ControlFlow::Continue(request)
    };
    ...
}
```

`LspRequest.client` already exists from lsp-outstanding.md. This slice is the first handler that reads it.

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn semantic_tokens_response<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographState<THostLanguage>,
    client: LspClientId,
    params: lsp_types::SemanticTokensParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::SemanticTokensFullRequest as lsp_types::request::Request>::Result,
> {
    let Some(absolute) = file_path(params.text_document.uri.reference()) else {
        return isograph_lsp::lsp_runtime_error::LSPRuntimeError::ExpectedError.wrap_err();
    };
    let tokens = semantic_tokens(state, client, absolute.reference());
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
    state: &isograph_compiler::IsographState<THostLanguage>,
    client: LspClientId,
    absolute: &std::path::Path,
) -> Option<Vec<lsp_types::SemanticToken>> {
    let cwd = state.get_singleton::<common_lang_types::CurrentWorkingDirectory>()?;
    let path = common_lang_types::relative_path_from_absolute_and_working_directory(*cwd, absolute);
    isograph_lsp::lsp_semantic_tokens_for_file::<THostLanguage>(state, client, path).clone()
}
```

Missing cwd, non-file URI, no `OpenFile` for this client and no `DiskFile`: JSON `null`, same as today.

### Tests

`database.rs`:

- `lsp_file_contents` with an `OpenFile` and a different `DiskFile` at that path is the open text
- with no `OpenFile` and a `DiskFile` is the disk text
- with neither is `None`
- client B does not see client A's `OpenFile`; B with no open file of its own sees disk
- after `remove_open_file`, `lsp_file_contents` is disk
- after `remove_open_files_for_client`, `lsp_file_contents` is disk
- open-only (no `DiskFile`) is the open text
- `disk_file` after a miss, then `insert_disk_file`, then `disk_file` is `Some` (the miss-tracked fix)

`file_semantic_tokens.rs` (existing tests pass `LspClientId(1)` and intern disk only):

- `didOpen`-equivalent `insert_open_file` of a buffer whose iso text differs from disk: first token is from the buffer (e.g. disk has no iso, buffer has `entrypoint`, first token type 15 length 10)
- two clients, same path, different buffers: each `lsp_semantic_tokens_for_file` matches that client's buffer
- no open file: same encoded tokens as today's disk test
- `remove_open_file` then tokens match disk
- open-only file with an iso literal: `Some` non-empty; after `remove_open_files_for_client`: `None`

`handle` / `lsp_socket.rs` listen_and_reply:

- intern disk `export const Home = iso(\`entrypoint Query.HomeRoute\`)`. `didOpen` the same URI with `iso(\`field Query.Unsaved\`)`. `semanticTokens/full` from that connection: first token is `field` (KEYWORD, length 5), not `entrypoint` (length 10)
- a second initialized connection that did not `didOpen` that URI: first token is still `entrypoint` from disk
- `didClose` then `semanticTokens/full` on the first connection: `entrypoint` again
- `didOpen` a URI with no `DiskFile` and an iso literal: non-null tokens. `didClose`: JSON `null`
- drop the connection after `didOpen` of unsaved text; a new connection `semanticTokens/full` without `didOpen`: disk (or `null` if no disk)

`cli.rs` e2e-send-semantic-tokens tests stay green: send never `didOpen`s, overlay miss, disk as today.

### Call sites

- `Lsp::Request` -> `semantic_tokens_response(state, client, params)` -> `lsp_semantic_tokens_for_file(state, client, path)` -> `lsp_parsed_iso_literals_in_file` / `lsp_text_through_last_iso_literal` -> `lsp_extract_iso_literals` -> `lsp_file_contents` -> `extract_iso_literals_from_text`
- later hover / diagnostics / goto -> `lsp_file_contents` (and `lsp_extract_iso_literals` where they need extractions)
- `HostLanguage::extract_iso_literals` / `parsed_iso_literals_in_file` / `text_through_last_iso_literal` / `literal_id_at_location` -> still disk, not this function

### Docs this change amends

`docs-website/docs/design-docs/pico.md`: "The LSP reads `OpenFile` when that path has one, otherwise `DiskFile`" becomes: the LSP calls `lsp_file_contents(client, path)`. Compile memos call `disk_file` / `extract_iso_literals(db, path)`. Those are different memos. One overlay used by both, as isograph's `read_iso_literals_source` does, makes artifact generation depend on editor buffers.

`refactors/past/lsp-tokens.md` / `refactors/past/file-semantic-tokens.md` / `refactors/past/vscode-extension.md`: unsaved buffer text is what that client colors. Tokens are `lsp_semantic_tokens_for_file` of `lsp_file_contents`, not of `DiskFile` alone.

`refactors/pending/lsp-diagnostics.md`: `report_diagnostics` per live session uses `lsp_file_contents(session.id, path)`, not a walk of `DiskFile` only.
