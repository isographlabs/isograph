# Per-client open files

Requires lsp-sessions.md and lsp-outstanding.md (landed). Diagnostics (lsp-diagnostics.md) builds on this file.

A client is a reader of file text. Each LSP connection (one VS Code window) is a client with its own open buffers: the text it sent with `didOpen` / `didChange` and has not closed. `IsographDb` holds every client's buffers in one tracked map keyed by `OpenFileKey { client, path }`. Every read of file text is the memo `file_contents(db, client, path)`: that client's open buffer for that path if it has one, otherwise the `DiskFile`. Highlighting, Format Document, and `willSaveWaitUntil` read through `file_contents` with the asking client. The file-based compiler will be another client with its own `ClientId`, reading through `file_contents` like any client; how it gets its `ClientId` is out of scope for this doc.

Origin of the open-file map and handlers: isograph `OpenFileMap` / `OpenFileSource` / `insert_open_file` / `remove_open_file` (`crates/isograph_schema/src/isograph_database.rs`) and `crates/isograph_lsp/src/text_document.rs`. Origin of overlay-then-disk: isograph `read_iso_literals_source` (`crates/isograph_schema/src/validated_isograph_schema/isograph_literals.rs`). Delta from isograph: the map key is `OpenFileKey { client, path }`, not the path alone, because two clients can hold different text for one path; the source is `OpenFile { #[key] key: OpenFileKey, contents }`; a lookup is `db.try_get(OpenFile::id_for_key(&key))`, as upstream isograph #1003 reads `OpenFileSource::id_for_key`; i2 keeps `open_file_map` for the writers, where #1003 deletes upstream's map; a write of an already-present key does not write the map; `didChange` accepts one full-text change and logs any other shape with `warn!`; no `expect` on a URI.

## Open decisions

These are not approved.

1. A tracked map whose counter was never written (change 3). Resolved without change 3 by change 1 of `refactors/pending/pico-missing-key-dependency.md` (upstream isograph #994). See Known gap.
2. Order relative to format-file-from-segments.md. Decided: (a); change 7 landed first and format-file-from-segments.md reads `file_contents`. (a) This doc first: change 7's `format_iso_literals` uses `parsed_iso_literals_in_file` and `text_through_last_iso_literal` with the client, and format-file-from-segments.md then reads `file_contents(db, client, path)` instead of `state.disk_file(path)`. (b) format-file-from-segments.md first: change 7's `format_iso_literals` is that doc's version with `file_contents` replacing `disk_file` (second snippet in change 7). Either order ships; (b) makes change 7's format diff smaller, (a) makes format-file-from-segments.md read the buffer from the start.
3. `SendLspResponseEffect` as data (optional change). The feature does not need it. Tests read a response by calling `perform` and `recv`ing on the crossbeam receiver behind the `LiveSession` (`lsp_formatting_of_a_live_client_without_a_file_is_null`, `crates/isograph_cli/src/state.rs`), and change 6's tests use that pattern. (a) Ship the optional change. (b) Do not ship it; `Box<dyn FnOnce(&SideEffectToken) + Send>` stays.

## What the user does

Highlighting. The user types in an iso literal and does not save. vscode-languageclient 9.0.1 holds full-sync `didChange` for up to 250ms, and `sendRequest` flushes pending full-text changes before sending any request (`vscode-extension/node_modules/vscode-languageclient/lib/common/client.js:335-345`). The daemon handles events in order on one loop. So `semanticTokens/full` colors the text the editor has, not the last save. Typing `iso(\`field Query.x{id}\`)` over a saved `iso(\`entrypoint Query.HomeRoute\`)` colors `field` as a keyword without a save.

Formatting. Format Document and save (`willSaveWaitUntil`) compute edits from that window's buffer. Applying them to that buffer yields the formatted file even when disk differs; the extra `}` from disk ranges applied to a longer buffer no longer happens.

Two VS Code windows on one project are two `isograph lsp` processes, two connections, and two `ClientId`s on one daemon. Each window has its own buffer for a file both have open. Window A's unsaved edits color and format only in A. Window B colors and formats B's buffer. When A saves, the watcher posts `DiskChanged`; B's buffer is still B's text. If B has no unsaved changes, VS Code reloads the file into B and sends `didChange`; B then shows the new text. A window that does not have the file open reads disk. The daemon does not push `workspace/semanticTokens/refresh`; B recolors on its next tokens request.

Closing a tab sends `didClose`; that window reads disk for that path again. Closing or crashing a window ends its connection; `handle` of `Gone` drops all of that window's buffers. The other window is unaffected.

A URI with neither a buffer for the asking window nor a `DiskFile` is JSON `null` for tokens and formatting. A non-`file:` URI is `null`. A `file:` URI whose path is not UTF-8, or a request before the config directory is interned, is an error response (`UnknownErrorCode`) naming the cause; the daemon does not panic. A `didOpen` / `didChange` / `didClose` with such a URI is logged with `warn!` and changes nothing.

`isograph send` is unchanged. A send connection never sends `didOpen`, so it has no buffers.

## Panics

This doc adds no panic and no `UnexpectedError`. The `db.get` calls on ids found in `open_file_map` or `disk_file_map` are the existing pico pattern: `handle` writes the map in the same step that it `set`s or `remove`s the source, and a stale id panics in pico (`crates/pico/src/database.rs:123-128`), which `pico.md` Source allows.

## Changes

Prefactors first.

1. Notifications carry the client; handlers receive it as a parameter. Dispatch drops a notification from a client not in `LiveSessions`. Landed in `9e0dcde0`.
2. Requests and responses carry the client; request handlers receive it as a parameter. `LSPRequestDispatch` moves from `isograph_lsp` to `isograph_cli`, next to `LSPNotificationDispatch`. Dispatch drops a response from a client not in `LiveSessions`. Landed in `c74128a3` (move) and `862b68fc`.
3. A tracked counter that was never written. Not needed; change 1 of `refactors/pending/pico-missing-key-dependency.md` fixes it in pico. See Known gap.
4. `ClientId` moves to `isograph_compiler`. Landed in `65d4cce4`.
5. `open_file_map`, the `file_contents` memo, the writers, and unit tests. Landed in `a42d7cad`; `file_contents` reads both keys with `try_get(id_for_key)` since `27b023a1`.
6. `didOpen` / `didChange` / `didClose`, advertised capabilities, `Gone` cleanup, and `handle` tests. Landed in `0ecc7e2f` (typed path conversion) and `a7846d69` (handlers, capabilities, `Gone`); design docs in `220e64ac`.
7. Semantic tokens and formatting read through `file_contents`. Landed in `1bdffe19` (`extract_iso_literals_from_text`), `0a9cf53c` (client-keyed memos, semantic tokens, formatting, `willSaveWaitUntil`), `76d666c5` (multi-literal formatting tests), and `77c5a992` (`IsographDb::disk_file` deleted).

## Known gap

Closed. Change 3 did not ship. Upstream isograph #994 (change 1 of `refactors/pending/pico-missing-key-dependency.md`), as reworked by upstream isograph #1000, makes a memo that reads a tracked map whose counter was never written depend on the counter's key at the initial epoch, and the first counter write advances the epoch, so a `file_contents(client, path)` miss before the first `set_open_file` or `insert_disk_file` sees that later write. `read_before_first_open_file_write_sees_the_later_open` (`crates/isograph_compiler/src/database.rs`) asserts the buffer text after the open. The `file_contents`, `sources`, `compiler_errors_in_file`, and `extract_iso_literals` tests no longer write an unrelated map entry before the first read.

Deltas from the snippets below, as landed:

- `remove_open_files_of_client` removes the collected ids with a `for` loop; clippy `needless_for_each` (workspace lints) rejects `.into_iter().for_each(...)`.
- `DidChangeError::Uri` is `Uri(#[from] UriPathError)` with no `derive_more::From`; thiserror rejects a variant-level `#[from]`, and the repo's thiserror enums put `#[from]` on the field (`send.rs`, `lsp_stdio.rs`, `parse_error.rs`).
- `DiskChangedPathError` is `#[error("{}: {error}", .path.display())]`; thiserror names a field in an extra format argument with a leading `.`.
- The request handlers keep `_client: ClientId` from change 2; change 7 renames it to `client`.
- The `handle` tests of a `didChange` that is not one full change assert the buffer is unchanged and no effects; they do not observe the `warn!`.
- Change 7: `file_contents` is the only reader of `DiskFile.contents` and `OpenFile.contents`. `IsographDb::disk_file` is deleted; its remaining callers were tests (`database.rs`, `watch.rs`), and those tests and the `state.rs` `contents` helper read `file_contents(db, ClientId(1), path)`. Other uses of `disk_file_map` are writers and map-size assertions.
- Change 7: `adapter::tests` is `pub(crate) mod tests`. `state.rs` `handle` tests reuse `apply_text_edits`, `assert_edits_replace_disjoint_interiors`, and the `TWO_LITERALS` / `THREE_LITERALS` fixtures. `apply_text_edits` resolves every range against the original text (treating `\n`, `\r\n`, and `\r` as line terminators, UTF-16 columns) and applies from the last start to the first.
- Change 7 added formatting tests beyond the list below: two and three literals where the first grows, the second already pretty, the first shrinking, a failed literal between two good ones, multi-byte text before a literal on its line, CRLF outside the literals; each also formats its result again unchanged. `state.rs` runs the two- and three-literal cases through Formatting and `willSaveWaitUntil`. All passed against the change 7 code; no formatting bug was found.
- The iso tokenizer rejects a string character above U+FFFF (`ErrorUnsupportedStringCharacter`), so the multi-byte fixture keeps the emoji in the JavaScript and BMP characters in the description.
- The `isograph_extract_typescript` `tests` module calls `extract_iso_literals_from_text` directly; `two_literals_in_one_file` already covered two calls in source order.
- `event-model.md` amended: artifact generation reads through `file_contents` as its own client.

The optional change (`SendLspResponseEffect` is data) ships only if open decision 3 picks (a); no other change depends on it.

## Change 1: notifications carry the client

After this change every LSP notification reaches its handler with the `ClientId` of the connection that sent it. Nothing reads the client yet except `dispatch_lsp`, which drops a notification from a client not in `LiveSessions`, the same as it drops a request (`LiveSessions::reply` returns `None`). Within one connection `run_session` forwards messages only after it posts `Started`, and `session` posts `Gone` after `run_session` returns, so no notification arrives from such a client today; the check makes `handle` total over that input instead of relying on the socket's ordering, and change 6 relies on it so that a `didOpen` cannot write buffers for a client whose `Gone` was already handled.

### Types

Before:

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug)]
pub enum LspEvent {
    Request(LspRequest),
    Notification(lsp_server::Notification),
    Response(lsp_server::Response),
}
```

After:

```rust
// from crates/isograph_cli/src/event.rs
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

`LspNotification` mirrors `LspRequest { client, request }`. `LspEvent` derives `Clone` and `Debug`, so `LspNotification` derives both.

### `run_session`

Before:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
            lsp_server::Message::Notification(notification) => {
                crate::event::LspEvent::Notification(notification)
            }
```

After:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
            lsp_server::Message::Notification(notification) => {
                crate::event::LspEvent::Notification(crate::event::LspNotification {
                    client,
                    notification,
                })
            }
```

### `LSPNotificationDispatch`

The dispatch struct carries the client and passes it to the handler. The handler type stays a `fn` pointer; the `ControlFlow` chain is unchanged.

Before:

```rust
// from crates/isograph_cli/src/lsp_notification_dispatch.rs
pub struct LSPNotificationDispatch<'state, TState> {
    notification: lsp_server::Notification,
    state: &'state mut TState,
}

impl<'state, TState> LSPNotificationDispatch<'state, TState> {
    pub fn new(notification: lsp_server::Notification, state: &'state mut TState) -> Self {
        Self {
            notification,
            state,
        }
    }

    pub fn on_notification_sync<TNotification: Notification>(
        self,
        handler: fn(&mut TState, TNotification::Params) -> Vec<IsographEffect>,
    ) -> ControlFlow<Vec<IsographEffect>, Self> {
        if self.notification.method != TNotification::METHOD {
            return ControlFlow::Continue(self);
        }
        match self
            .notification
            .extract::<TNotification::Params>(TNotification::METHOD)
        {
            Ok(params) => ControlFlow::Break(handler(self.state, params)),
            Err(lsp_server::ExtractError::MethodMismatch(notification)) => {
                ControlFlow::Continue(Self {
                    notification,
                    state: self.state,
                })
            }
            Err(lsp_server::ExtractError::JsonError { method, error }) => {
                warn!(method = method.as_str(), error = %error, "notification params");
                ControlFlow::Break(no_effects())
            }
        }
    }
}
```

After:

```rust
// from crates/isograph_cli/src/lsp_notification_dispatch.rs
pub struct LSPNotificationDispatch<'state, TState> {
    client: ClientId,
    notification: lsp_server::Notification,
    state: &'state mut TState,
}

impl<'state, TState> LSPNotificationDispatch<'state, TState> {
    pub fn new(incoming: LspNotification, state: &'state mut TState) -> Self {
        Self {
            client: incoming.client,
            notification: incoming.notification,
            state,
        }
    }

    pub fn on_notification_sync<TNotification: Notification>(
        self,
        handler: fn(&mut TState, ClientId, TNotification::Params) -> Vec<IsographEffect>,
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
                    client: self.client,
                    notification,
                    state: self.state,
                })
            }
            Err(lsp_server::ExtractError::JsonError { method, error }) => {
                warn!(method = method.as_str(), error = %error, "notification params");
                ControlFlow::Break(no_effects())
            }
        }
    }
}
```

`notification(self) -> lsp_server::Notification` is unchanged.

### `dispatch_lsp` and `dispatch_lsp_notification`

Before:

```rust
// from crates/isograph_cli/src/state.rs
        crate::event::LspEvent::Notification(notification) => {
            dispatch_lsp_notification(state, notification)
        }
```

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
        crate::event::LspEvent::Notification(notification) => {
            if !state.live_sessions.0.contains_key(&notification.client) {
                return no_effects();
            }
            dispatch_lsp_notification(state, notification)
        }
```

```rust
// from crates/isograph_cli/src/state.rs
fn dispatch_lsp_notification<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    notification: crate::event::LspNotification,
) -> Vec<IsographEffect> {
    let dispatch = || {
        crate::lsp_notification_dispatch::LSPNotificationDispatch::new(notification, state)
            .on_notification_sync::<crate::lsp_socket::IsographEventNotification>(
                on_isograph_event_notification::<THostLanguage>,
            )?
            .notification();
        ControlFlow::Continue(())
    };
    match dispatch() {
        ControlFlow::Break(effects) => effects,
        ControlFlow::Continue(()) => no_effects(),
    }
}

fn on_isograph_event_notification<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    _client: ClientId,
    internal: InternalEvent,
) -> Vec<IsographEffect> {
    handle_internal_event(state, internal)
}
```

`handle_internal_event` keeps its signature; `handle` calls it directly for `IsographEvent::Internal`.

### Tests

`crates/isograph_cli/src/state.rs` (`handle` seam):

- `event_notification` takes `client: ClientId` and builds `LspEvent::Notification(LspNotification { client, notification })`
- `lsp_isograph_event_hello_world_returns_a_timer`, `lsp_isograph_event_hello_world_then_its_timer_returns_log_hello_world`, `lsp_isograph_event_quit_returns_kill`, `lsp_isograph_event_disk_changed_interns`, `lsp_unknown_notification_returns_no_effects`, and `lsp_isograph_event_bad_params_returns_no_effects` start a client first; facts unchanged
- added `lsp_notification_from_a_client_that_never_started_returns_no_effects`: `isograph/event` HelloWorld from `ClientId(1)` with no `Started`: no effects, `hello_world_debounce` is `None`
- added `lsp_notification_disk_changed_from_a_client_that_never_started_does_not_intern`: on `with_config()`, `isograph/event` `DiskChanged` for `/tmp/proj/src/a.ts` from a client that never started: no effects, no `DiskFile` at `src/a.ts`
- added `lsp_notification_after_gone_returns_no_effects`: `Started(1)`, `Gone(1)`, then HelloWorld from client 1: no effects, `hello_world_debounce` is `None`

`crates/isograph_cli/src/lsp_socket.rs`: tests that bind `LspEvent::Notification(notification)` and read `notification.method` / `.params` bind `LspEvent::Notification(LspNotification { notification, .. })`. Facts unchanged.

### Call sites

- `run_session` -> `LspEvent::Notification(LspNotification { client, notification })`
- `dispatch_lsp` -> `live_sessions.0.contains_key`, then `dispatch_lsp_notification` -> `LSPNotificationDispatch::new` -> `on_notification_sync` -> `on_isograph_event_notification` -> `handle_internal_event`

### Docs this change amends

`docs-website/docs/design-docs/event-model.md` Dispatch: add "An LSP notification carries the `ClientId` of its connection. `handle` drops a notification from a client not in `LiveSessions`."

## Change 2: requests and responses carry the client

After this change every LSP request reaches its handler with the `ClientId` of the connection that sent it, and every LSP response reaches `dispatch_lsp` tagged with that `ClientId`. No handler reads the client yet; each request handler takes `_client: ClientId`. Change 7 reads it in the request handlers.

Every handler takes the client as a separate parameter, not a tuple and not a struct: requests `fn(&TState, ClientId, TRequest::Params) -> LSPRuntimeResult<TRequest::Result>`, notifications (change 1) `fn(&mut TState, ClientId, TNotification::Params) -> Vec<IsographEffect>`. `LspRequest`, `LspNotification`, and `LspResponse` are only the payloads `run_session` sends on the event channel; dispatch destructures each and passes `client` and the message separately. A handler's output never contains a `ClientId`.

The request dispatcher moves to `isograph_cli`, next to the notification dispatcher, because its struct carries `ClientId`. `ClientId` is `pub(crate)` in `crates/isograph_cli/src/event.rs`, and `isograph_lsp` cannot name it: `isograph_cli` depends on `isograph_lsp` (`crates/isograph_cli/Cargo.toml`), not the reverse. With both dispatchers in `isograph_cli`, `ClientId` stays where it is until change 4.

Today `dispatch_lsp` drops every response (`crates/isograph_cli/src/state.rs:79`, `crate::event::LspEvent::Response(_response) => no_effects()`), and `lsp_response_returns_no_effects` (`state.rs:1007`) pins that. Nothing else matches `LspEvent::Response`. No response handler exists, and this change does not add one. After this change the only reader of `LspResponse.client` is a liveness check, the same as for notifications: `dispatch_lsp` drops a response from a client not in `LiveSessions`. A response from a live client is also no effects, so the check changes nothing a test can observe until a response handler exists. The arm binds `response: _response`, as today's arm binds `_response`; a struct field that no pattern binds is `dead_code`. The client becomes necessary once the daemon sends server-to-client requests (for example `workspace/configuration` or `workspace/semanticTokens/refresh`): the response to such a request names a `RequestId` the daemon minted for one connection, and matching it requires knowing which connection answered.

The change ships as two commits: the move, then the client.

### Move `LSPRequestDispatch` to `isograph_cli`

`crates/isograph_lsp/src/lsp_request_dispatch.rs` moves to `crates/isograph_cli/src/lsp_request_dispatch.rs` with its two test modules. Delta: the `LSPRuntimeError` / `LSPRuntimeResult` import names `isograph_lsp`, which keeps `lsp_runtime_error`.

Before:

```rust
// from crates/isograph_lsp/src/lsp_request_dispatch.rs
use crate::lsp_runtime_error::{LSPRuntimeError, LSPRuntimeResult};
```

```rust
// from crates/isograph_lsp/src/lsp_request_dispatch.rs
    use super::LSPRequestDispatch;
    use crate::lsp_runtime_error::LSPRuntimeResult;
```

After:

```rust
// from crates/isograph_cli/src/lsp_request_dispatch.rs
use isograph_lsp::lsp_runtime_error::{LSPRuntimeError, LSPRuntimeResult};
```

```rust
// from crates/isograph_cli/src/lsp_request_dispatch.rs
    use super::LSPRequestDispatch;
    use isograph_lsp::lsp_runtime_error::LSPRuntimeResult;
```

Before:

```rust
// from crates/isograph_lsp/src/lib.rs
mod file_semantic_tokens;
pub mod lsp_request_dispatch;
pub mod lsp_runtime_error;
mod semantic_tokens;
```

After:

```rust
// from crates/isograph_lsp/src/lib.rs
mod file_semantic_tokens;
pub mod lsp_runtime_error;
mod semantic_tokens;
```

`crates/isograph_cli/src/lib.rs` adds `mod lsp_request_dispatch;` after `mod lsp_notification_dispatch;`.

`crates/isograph_lsp/Cargo.toml` drops `serde_json`; `lsp_request_dispatch.rs` was its only user. `isograph_cli` already depends on `lsp-server`, `lsp-types`, `serde_json`, and `isograph_lsp`.

`crates/prelude/src/postfix_constructors.rs` `walk_rust` skips the file at its new path. The file keeps its upstream `Ok` / `Err` / `Some` constructors; converting them is not part of a move.

Before:

```rust
// from crates/prelude/src/postfix_constructors.rs
                let skip = path.ends_with("crates/isograph_lsp/src/lsp_request_dispatch.rs")
                    || path.ends_with("crates/isograph_lsp/src/lsp_runtime_error.rs");
```

After:

```rust
// from crates/prelude/src/postfix_constructors.rs
                let skip = path.ends_with("crates/isograph_cli/src/lsp_request_dispatch.rs")
                    || path.ends_with("crates/isograph_lsp/src/lsp_runtime_error.rs");
```

Before:

```rust
// from crates/isograph_cli/src/state.rs
        let request =
            isograph_lsp::lsp_request_dispatch::LSPRequestDispatch::new(request, &state.db)
```

After:

```rust
// from crates/isograph_cli/src/state.rs
        let request = crate::lsp_request_dispatch::LSPRequestDispatch::new(request, &state.db)
```

`dispatch_lsp_request` is the only caller of `LSPRequestDispatch` outside its own tests. No other crate names `lsp_request_dispatch`, `convert_to_lsp_response`, or `extract_request_params`.

### Types

Before:

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug)]
pub enum LspEvent {
    Request(LspRequest),
    Notification(LspNotification),
    Response(lsp_server::Response),
}
```

After:

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug)]
pub struct LspResponse {
    pub client: ClientId,
    pub response: lsp_server::Response,
}

#[derive(Clone, Debug)]
pub enum LspEvent {
    Request(LspRequest),
    Notification(LspNotification),
    Response(LspResponse),
}
```

`LspResponse` mirrors `LspRequest { client, request }` and `LspNotification { client, notification }`.

### `run_session`

Before:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
            lsp_server::Message::Response(response) => crate::event::LspEvent::Response(response),
```

After:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
            lsp_server::Message::Response(response) => {
                crate::event::LspEvent::Response(crate::event::LspResponse { client, response })
            }
```

### `LSPRequestDispatch`

The dispatch struct carries the client and passes it to the handler, the same as `LSPNotificationDispatch` in change 1. `new` takes the `LspRequest`. The handler type stays a `fn` pointer; the `ControlFlow` chain, `request`, `convert_to_lsp_response`, and `extract_request_params` are unchanged.

Before (after the move):

```rust
// from crates/isograph_cli/src/lsp_request_dispatch.rs
pub struct LSPRequestDispatch<'state, TState> {
    request: lsp_server::Request,
    state: &'state TState,
}

impl<'state, TState> LSPRequestDispatch<'state, TState> {
    pub fn new(request: lsp_server::Request, state: &'state TState) -> Self {
        LSPRequestDispatch { request, state }
    }

    pub fn on_request_sync<TRequest: Request>(
        self,
        handler: fn(&TState, TRequest::Params) -> LSPRuntimeResult<TRequest::Result>,
    ) -> ControlFlow<Response, Self> {
        if self.request.method == TRequest::METHOD {
            let id = self.request.id.clone();
            match extract_request_params::<TRequest>(self.request) {
                Ok((request_id, params)) => {
                    let response = handler(self.state, params).and_then(|handler_result| {
                        // ... unchanged
                    });
                    // ... unchanged
                }
                // ... the error arm is unchanged
            }
        }

        ControlFlow::Continue(self)
    }
}
```

After:

```rust
// from crates/isograph_cli/src/lsp_request_dispatch.rs
use crate::event::{ClientId, LspRequest};

pub struct LSPRequestDispatch<'state, TState> {
    client: ClientId,
    request: lsp_server::Request,
    state: &'state TState,
}

impl<'state, TState> LSPRequestDispatch<'state, TState> {
    pub fn new(incoming: LspRequest, state: &'state TState) -> Self {
        LSPRequestDispatch {
            client: incoming.client,
            request: incoming.request,
            state,
        }
    }

    pub fn on_request_sync<TRequest: Request>(
        self,
        handler: fn(&TState, ClientId, TRequest::Params) -> LSPRuntimeResult<TRequest::Result>,
    ) -> ControlFlow<Response, Self> {
        if self.request.method == TRequest::METHOD {
            let id = self.request.id.clone();
            match extract_request_params::<TRequest>(self.request) {
                Ok((request_id, params)) => {
                    let response =
                        handler(self.state, self.client, params).and_then(|handler_result| {
                            // ... unchanged
                        });
                    // ... unchanged
                }
                // ... the error arm is unchanged
            }
        }

        ControlFlow::Continue(self)
    }
}
```

The doc comment on `on_request_sync` is unchanged.

### Request handlers

Each handler gains `_client: ClientId` as its second parameter. The bodies are unchanged. `adapter.rs` imports `crate::event::ClientId`.

Before:

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn semantic_tokens_response<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographDb<THostLanguage>,
    params: lsp_types::SemanticTokensParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::SemanticTokensFullRequest as lsp_types::request::Request>::Result,
> {
```

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn formatting_response<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographDb<THostLanguage>,
    params: lsp_types::DocumentFormattingParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::Formatting as lsp_types::request::Request>::Result,
> {
```

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn will_save_wait_until_response<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographDb<THostLanguage>,
    params: lsp_types::WillSaveTextDocumentParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::WillSaveWaitUntil as lsp_types::request::Request>::Result,
> {
```

After:

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn semantic_tokens_response<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographDb<THostLanguage>,
    _client: ClientId,
    params: lsp_types::SemanticTokensParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::SemanticTokensFullRequest as lsp_types::request::Request>::Result,
> {
```

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn formatting_response<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographDb<THostLanguage>,
    _client: ClientId,
    params: lsp_types::DocumentFormattingParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::Formatting as lsp_types::request::Request>::Result,
> {
```

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn will_save_wait_until_response<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographDb<THostLanguage>,
    _client: ClientId,
    params: lsp_types::WillSaveTextDocumentParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::WillSaveWaitUntil as lsp_types::request::Request>::Result,
> {
```

`method_not_found` is not a handler. It runs on the `ControlFlow::Continue` leftover and already reaches the client through `AfterCompute`; it is unchanged.

### `dispatch_lsp` and `dispatch_lsp_request`

Before (`dispatch_lsp_request` after the move):

```rust
// from crates/isograph_cli/src/state.rs
        crate::event::LspEvent::Response(_response) => no_effects(),
```

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
        let request = crate::lsp_request_dispatch::LSPRequestDispatch::new(request, &state.db)
            .on_request_sync::<lsp_types::request::SemanticTokensFullRequest>(
                crate::adapter::semantic_tokens_response::<THostLanguage>,
            )?
            // ... Formatting, WillSaveWaitUntil unchanged
            .request();
        ControlFlow::Continue(request)
    };
    // ... unchanged
}
```

After:

```rust
// from crates/isograph_cli/src/state.rs
        crate::event::LspEvent::Response(crate::event::LspResponse {
            client,
            response: _response,
        }) => {
            if !state.live_sessions.0.contains_key(&client) {
                return no_effects();
            }
            no_effects()
        }
```

```rust
// from crates/isograph_cli/src/state.rs
fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    incoming: crate::event::LspRequest,
    before: crate::event::BeforeCompute,
) -> Vec<IsographEffect> {
    let Some(after) = state.live_sessions.before_compute(before) else {
        return no_effects();
    };
    let get_response = || {
        let request = crate::lsp_request_dispatch::LSPRequestDispatch::new(incoming, &state.db)
            .on_request_sync::<lsp_types::request::SemanticTokensFullRequest>(
                crate::adapter::semantic_tokens_response::<THostLanguage>,
            )?
            // ... Formatting, WillSaveWaitUntil unchanged
            .request();
        ControlFlow::Continue(request)
    };
    // ... unchanged
}
```

The request liveness check is unchanged: `LiveSessions::reply` returns `None` for a client not in the map.

### Tests

`crates/isograph_cli/src/state.rs` (`handle` seam, no sleep):

- added helper `lsp_response(client: ClientId) -> IsographEvent` builds `LspEvent::Response(LspResponse { client, response })` with id 1 and neither result nor error
- `lsp_response_returns_no_effects` is renamed `lsp_response_from_a_live_client_returns_no_effects`; it starts a client with `start_a_client` and sends `lsp_response(client)`: no effects
- added `lsp_response_from_a_client_that_never_started_returns_no_effects`: `lsp_response(ClientId(1))` with no `Started`: no effects
- the request case is already covered: `lsp_request_without_a_live_session_returns_no_effects` sends a request from `ClientId(1)` with no `Started` and asserts no effects; `lsp_formatting_from_a_client_that_is_not_live_returns_no_effects`, `lsp_request_from_a_sibling_client_returns_no_effects`, and `lsp_request_after_gone_returns_no_effects` cover the other cases. Facts unchanged.

`crates/isograph_cli/src/lsp_request_dispatch.rs`: `calls_first_matching_request_handler` builds `LspRequest { client: ClientId(1), request }` for `LSPRequestDispatch::new`; `hover_handler` and `goto_definition_handler` take `_client: ClientId`. Facts unchanged. `test_extract_request_params_error` is unchanged.

### Call sites

- `run_session` -> `LspEvent::Response(LspResponse { client, response })`
- `dispatch_lsp` -> `live_sessions.0.contains_key` on the response's client
- `dispatch_lsp` -> `live_sessions.reply`, then `dispatch_lsp_request` -> `LSPRequestDispatch::new(incoming, &state.db)` -> `on_request_sync` -> `semantic_tokens_response` / `formatting_response` / `will_save_wait_until_response`

### Docs this change amends

`docs-website/docs/design-docs/event-model.md` Dispatch: "A response is no effects." becomes "An LSP request and an LSP response carry the `ClientId` of their connection. `handle` drops a response from a client not in `LiveSessions`; a response from a live client is no effects."

## Change 4: `ClientId` moves to `isograph_compiler`

The database keys open buffers by `ClientId` (change 5), so the type moves to the crate that owns `IsographDb` and becomes `pub`. One `ClientId` names every client.

Before:

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ClientId(pub u64);
```

After:

```rust
// from crates/isograph_compiler/src/database.rs
/// A reader of file text.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ClientId(pub u64);
```

Delta: `pub` instead of `pub(crate)`, and the doc comment.

```rust
// from crates/isograph_compiler/src/lib.rs
pub use database::{ClientId, DiskFile, DiskFileMap, IsographDb};
```

`event.rs` has no `ClientId` definition and imports `isograph_compiler::ClientId`. `LspRequest.client`, `LspNotification.client`, the `LiveSessions` key, `BeforeCompute.client`, `AfterCompute.client`, `LiveSessions::reply`, `ClientStartedEvent.client`, `ClientConnectionEvent::Gone`, `LspResponse.client`, `LSPNotificationDispatch.client`, `LSPRequestDispatch.client`, the `ClientId` parameter of every notification and request handler, and `session` / `run_session` keep the type name `ClientId` and now name `isograph_compiler::ClientId`. `accept_loop` builds `isograph_compiler::ClientId(next)`. `lsp_notification_dispatch.rs`, `lsp_request_dispatch.rs`, and `state.rs` import `isograph_compiler::ClientId` instead of `crate::event::ClientId`.

Tests: the test modules of `lsp_socket.rs` and `state.rs` drop `ClientId` from their `crate::event::{...}` imports and import `isograph_compiler::ClientId`. Facts unchanged.

## Change 5: `open_file_map`, `file_contents`, writers

Requires change 4.

### Types

```rust
// from crates/isograph_compiler/src/database.rs
#[derive(Debug, Db)]
pub struct IsographDb<THostLanguage: HostLanguage> {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
    #[tracked]
    open_file_map: OpenFileMap,
    phantom_data: PhantomData<THostLanguage>,
}

/// Every client's open buffers.
#[derive(Debug, Default)]
pub struct OpenFileMap(pub HashMap<OpenFileKey, SourceId<OpenFile>>);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OpenFileKey {
    pub client: ClientId,
    pub path: RelativePathToSourceFile,
}

/// One client's buffer of one path.
#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct OpenFile {
    #[key]
    pub key: OpenFileKey,
    pub contents: String,
}
```

`#[tracked]` on `open_file_map` gives `get_open_file_map()` / `get_open_file_map_mut()` and the counter singleton `OpenFileMapCounter` (`crates/pico_macros/src/db_macro.rs:60-95`). pico `#[key]` is one field; `OpenFile`'s key is `OpenFileKey` because two clients can hold different text for one path, and a path-only key would make the second client's `set` replace the first's.

`lib.rs`:

```rust
// from crates/isograph_compiler/src/lib.rs
pub use database::{ClientId, DiskFile, DiskFileMap, IsographDb, file_contents};
```

`OpenFile`, `OpenFileKey`, and `OpenFileMap` are not exported; nothing outside `isograph_compiler` reads them.

### `file_contents`

```rust
// from crates/isograph_compiler/src/database.rs
/// The text of `path` as `client` reads it: that client's open buffer, otherwise disk.
#[memo]
pub fn file_contents<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<String> {
    match db.try_get(OpenFile::id_for_key(&OpenFileKey { client, path })) {
        Some(open_file) => open_file.contents.clone().wrap_some(),
        None => db
            .try_get(DiskFile::id_for_key(&path))
            .map(|disk_file| disk_file.contents.clone()),
    }
}
```

`file_contents` is a memo and returns an owned `Option<String>`, which pico hands out as `&Option<String>`. A memo cannot return a `&str` into a source (`pico.md` Memo). Its callers are memos (change 7) and tests; `handle` does not call it.

The lookups follow AGENTS.md "pico tracked maps": a one-key lookup does not read the map. `try_get` records a dependency on that key whether or not a source is set there.

Dependencies of one `file_contents(client, path)` slot:

- Open buffer hit: that `OpenFile` key. A disk change does not re-invoke it. The buffer wins until `didClose` or `Gone`.
- Open miss, disk hit: that `OpenFile` key and that `DiskFile` key.
- Open miss, disk miss: the same two keys.

`didClose` of a path removes the `OpenFile`. A slot that hit re-verifies, sees the key changed, re-runs, misses, and reads disk. No memo receives a `SourceId<OpenFile>` as an argument.

An open or close re-invokes only the `file_contents` slot of that client and path. A keystroke (`didChange` of an already-open path) re-invokes only the slot that read that `OpenFile`. Each re-run clones its text and compares it with `==`; an unchanged result keeps its old `time_updated`, and nothing downstream re-runs.

### Writers

Called only by `handle` (change 6), never inside a memo.

```rust
// from crates/isograph_compiler/src/database.rs
impl<THostLanguage: HostLanguage> IsographDb<THostLanguage> {
    pub fn set_open_file(
        &mut self,
        client: ClientId,
        path: RelativePathToSourceFile,
        contents: String,
    ) {
        let key = OpenFileKey { client, path };
        let open_file_id = self.set(OpenFile { key, contents });
        if self.get_open_file_map().untracked().0.contains_key(&key) {
            return;
        }
        self.get_open_file_map_mut()
            .tracked()
            .0
            .insert(key, open_file_id);
    }

    pub fn remove_open_file(&mut self, client: ClientId, path: RelativePathToSourceFile) {
        let key = OpenFileKey { client, path };
        let Some(open_file_id) = self.get_open_file_map().untracked().0.get(&key).copied() else {
            return;
        };
        self.get_open_file_map_mut().tracked().0.remove(&key);
        self.remove(open_file_id);
    }

    pub fn remove_open_files_of_client(&mut self, client: ClientId) {
        let is_client = |key: &OpenFileKey| key.client == client;
        if !self.get_open_file_map().untracked().0.keys().any(is_client) {
            return;
        }
        let open_file_ids: Vec<SourceId<OpenFile>> = self
            .get_open_file_map_mut()
            .tracked()
            .0
            .extract_if(|key, _| is_client(key))
            .map(|(_, open_file_id)| open_file_id)
            .collect();
        open_file_ids
            .into_iter()
            .for_each(|open_file_id| self.remove(open_file_id));
    }
}
```

`set_open_file` writes the map only when the key is absent. `MutView::tracked()` increments the counter on every call, so writing a present key would re-invoke every memo that iterates `open_file_map` on each keystroke. The `SourceId` of a key is a function of the key, so re-inserting a present key would write the same id. Reading the key untracked before writing it is the shape AGENTS.md steers away from in favor of `entry`; it is required here because `MutView` has only `tracked()`, and calling it is the bump.

`remove_open_file` of an absent key returns before any write. `remove_open_files_of_client` of a client with no buffers returns before any write; that occurs on every `isograph send` connection and on the readiness probe in `start.rs`, which connect and never `didOpen`. Its scan is over every client's keys; a per-client index is backlog (`future-improvements.md`). `open_file_ids` is collected because `self.remove` needs `&mut self` after the map borrow ends. pico does not collect sources (`crates/pico/src/garbage_collection.rs:21`), so `Gone` must remove each `OpenFile` explicitly.

### Tests

`crates/isograph_compiler/src/database.rs` `#[cfg(test)]`, `TestHostLanguage`. Paths are interned; tests call writers and read `file_contents(&db, client, path)`. Observed results only.

- fresh database, no `DiskFile`, no buffer: `file_contents(ClientId(1), path)` is `None`
- `insert_disk_file(path, "disk")`: `file_contents(ClientId(1), path)` is `Some("disk")`
- `set_open_file(ClientId(1), path, "open")`: client 1 reads `"open"`; client 2 reads `"disk"`
- with no `DiskFile`, `set_open_file(ClientId(1), path, "open")`: client 1 reads `"open"`; client 2 reads `None`
- a second `set_open_file` of the same key replaces the text
- two clients, one path, different text: each reads its own
- `set_open_file(ClientId(1), path, "")`: client 1 reads `Some("")`, not disk's text
- `remove_open_file` then client 1 reads disk's text; `remove_open_file` of a key never opened leaves every read unchanged
- `remove_open_files_of_client(ClientId(1))` with two open paths: client 1 reads disk at both; client 2's buffer at one of those paths is unchanged
- `remove_open_files_of_client` of a client with no buffers leaves every read unchanged and `get_open_file_map().untracked().0.len()` unchanged
- `remove_disk_file` of a path client 1 has open: client 1 still reads the buffer; client 2 reads `None`
- miss then hit: client 1 reads disk; `set_open_file`; reads the buffer; `set_open_file` again; reads the new text; `remove_open_file`; reads disk; `remove_disk_file`; `None`, no panic
- fresh-database miss then insert: client 1 reads `None` before any write; `set_open_file(ClientId(1), path, "open")`; reads `"open"`. A second: client 1 reads `None`; `insert_disk_file`; reads disk's text. pico's missing-key dependency (upstream isograph #994, #1000) makes these pass.
- client 1 reads `path`; client 2 `set_open_file` of `path`: client 1 still reads disk's text

## Change 6: document sync handlers

Requires changes 1 and 4.

After this change `didOpen` / `didChange` / `didClose` write the sending client's buffers, `initialize` advertises full document sync, and `Gone` removes that client's buffers. Nothing reads the buffers until change 7.

### Typed path conversion

The handlers convert a URI to a `RelativePathToSourceFile`. `relative_path_from_absolute_and_working_directory` `expect`s at `crates/common_lang_types/src/location.rs:101,103`: `file:///tmp/proj/%FF.ts` percent-decodes to a non-UTF-8 path and reaches the second `expect`, and `semanticTokens/full` and formatting of that URI panic the event loop today. This section returns a typed error and gives the adapter one URI-to-path function.

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

`DiskChanged` logs instead of panicking. Before:

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

`relative_from_uri` replaces the conversion that `semantic_tokens` and `format_iso_literals` each repeat:

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

`file_path` is unchanged and stays private. A non-file URI stays JSON `null` (`ExpectedError`). A missing working directory is a broken startup invariant and is an error response. A path with no relative form is a real file the daemon cannot key and is an error response naming the cause.

Before:

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

`adapter.rs` imports `IsographDb`, `ClientId`, `CurrentWorkingDirectory`, `RelativePathToSourceFile`, `RelativePathError`, `relative_path_from_absolute_and_working_directory`, `LSPRuntimeError`, `LSPRuntimeResult`, and `tracing::warn`.

### Handlers

Origin: isograph `crates/isograph_lsp/src/text_document.rs`. Delta: the client is a parameter on every write; the URI goes through `relative_from_uri` with a typed error, not `expect`; `didChange` takes exactly one change with no range and logs any other shape; no diagnostics publish (lsp-diagnostics.md).

```rust
// from crates/isograph_cli/src/adapter.rs
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error, derive_more::From)]
pub(crate) enum DidChangeError {
    #[error("didChange carried {0} content changes; full document sync sends exactly one")]
    NotOneChange(usize),
    #[error("didChange carried a ranged change; full document sync sends the whole text")]
    RangedChange,
    #[error("{0}")]
    #[from]
    Uri(UriPathError),
}

pub(crate) fn on_did_open<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    client: ClientId,
    params: lsp_types::DidOpenTextDocumentParams,
) -> Vec<IsographEffect> {
    did_open(&mut state.db, client, params).unwrap_or_else(|error| warn!(%error, "didOpen"));
    no_effects()
}

pub(crate) fn on_did_change<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    client: ClientId,
    params: lsp_types::DidChangeTextDocumentParams,
) -> Vec<IsographEffect> {
    did_change(&mut state.db, client, params).unwrap_or_else(|error| warn!(%error, "didChange"));
    no_effects()
}

pub(crate) fn on_did_close<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    client: ClientId,
    params: lsp_types::DidCloseTextDocumentParams,
) -> Vec<IsographEffect> {
    did_close(&mut state.db, client, params).unwrap_or_else(|error| warn!(%error, "didClose"));
    no_effects()
}

fn did_open<THostLanguage: HostLanguage>(
    db: &mut IsographDb<THostLanguage>,
    client: ClientId,
    params: lsp_types::DidOpenTextDocumentParams,
) -> Result<(), UriPathError> {
    let path = relative_from_uri(db, params.text_document.uri.reference())?;
    db.set_open_file(client, path, params.text_document.text);
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
    let path = relative_from_uri(db, params.text_document.uri.reference())?;
    db.set_open_file(client, path, change.text);
    ().wrap_ok()
}

fn did_close<THostLanguage: HostLanguage>(
    db: &mut IsographDb<THostLanguage>,
    client: ClientId,
    params: lsp_types::DidCloseTextDocumentParams,
) -> Result<(), UriPathError> {
    let path = relative_from_uri(db, params.text_document.uri.reference())?;
    db.remove_open_file(client, path);
    ().wrap_ok()
}
```

The `on_did_*` functions have the change 1 handler type `fn(&mut IsographState<THostLanguage>, ClientId, Params) -> Vec<IsographEffect>`. `warn!` is `tracing::warn!`, the same as `on_notification_sync`'s params error. `change.text` is moved out of the one-element array, not cloned. `language_id` and `version` are not stored; nothing reads them. `didChange` of a path the client never opened stores it as open.

### Dispatch

Before (change 1's after):

```rust
// from crates/isograph_cli/src/state.rs
        crate::lsp_notification_dispatch::LSPNotificationDispatch::new(notification, state)
            .on_notification_sync::<crate::lsp_socket::IsographEventNotification>(
                on_isograph_event_notification::<THostLanguage>,
            )?
            .notification();
```

After:

```rust
// from crates/isograph_cli/src/state.rs
        crate::lsp_notification_dispatch::LSPNotificationDispatch::new(notification, state)
            .on_notification_sync::<crate::lsp_socket::IsographEventNotification>(
                on_isograph_event_notification::<THostLanguage>,
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
```

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

### `Gone`

Before:

```rust
// from crates/isograph_cli/src/state.rs
        ClientConnectionEvent::Gone(client) => {
            state.live_sessions.0.remove(&client);
            no_effects()
        }
```

After:

```rust
// from crates/isograph_cli/src/state.rs
        ClientConnectionEvent::Gone(client) => {
            state.live_sessions.0.remove(&client);
            state.db.remove_open_files_of_client(client);
            no_effects()
        }
```

`Started` is unchanged: a client needs no database state before its first `didOpen`. `Gone` of a client that never started, or that has no buffers, is a no-op on the database (change 5).

### Tests

All at the `handle` seam in `crates/isograph_cli/src/state.rs`: one process, no socket, no filesystem, no clock, no sleep. Disk arrives as `InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged { path, presence: Presence::Present(..) }))`. Existing helpers: `with_config` (interns `/tmp/proj`), `start_client`, `lsp_request`, `formatting_params`, `perform_send`. Added helpers build `LspEvent::Notification(LspNotification { client, notification })` for `didOpen` / `didChange` / `didClose`. Buffers are observed with `isograph_compiler::file_contents(&state.db, client, path)`, a memo call outside `handle`. Observed results only; no re-invocation counts.

- client 1 `didOpen` `file:///tmp/proj/src/x.ts` with `"open"`: no effects; `file_contents` for client 1 is `Some("open")`; for client 2 is disk's text
- then `didChange` (one change, no range) to `"changed"`: client 1 reads `"changed"`
- then `didClose`: client 1 reads disk's text
- clients 1 and 2 `didOpen` the same URI with different text: each reads its own
- client 1 `didOpen` then `Gone(1)`: client 1 reads disk's text; client 2's buffer at that path is unchanged
- `didChange` with `contentChanges: []`, with two full changes, and with one ranged change: client 1 reads the `didOpen` text after each
- `didOpen` of `https://example.net/a.ts`: no effects, no buffer
- `#[cfg(unix)]` `didOpen` of `file:///tmp/proj/%FF.ts`: no effects, no panic
- `didOpen` on `empty()` (no working directory) from a started client: no effects, no panic
- `didOpen` from a client that never `Started`: no effects; client 1 reads disk's text (change 1 drops it)
- `Gone(ClientId(7))` of a client that never started while client 2 has a buffer: no effects; client 2 reads its buffer
- `DiskChanged::File` with a relative path `src/a.ts` and `Present`: no effects, no `DiskFile` at any path, no panic
- `#[cfg(unix)]` `DiskChanged::File` with a non-UTF-8 absolute path: no effects, no panic
- `semanticTokens/full` of `file:///tmp/proj/%FF.ts` from a started client on `with_config()`: the response has `error.code == UnknownErrorCode` and `error.message == "the relative path is not UTF-8"`, read through `perform` and `rx.recv()`
- Formatting from a started client on `empty()`: the response has `error.message == "CurrentWorkingDirectory is not interned"`
- `lsp_formatting_of_a_live_client_without_a_file_is_null` uses `with_config()` instead of `empty()`; its fact is unchanged

`crates/common_lang_types/src/location.rs`:

- a relative `absolute_path` (`src/a.ts`) against `/tmp/proj` is `Err(RelativePathError::NotDiffable)`
- `#[cfg(unix)]` an absolute path with a non-UTF-8 component (`OsStr::from_bytes(b"/tmp/proj/\xFF.ts")`) is `Err(RelativePathError::NotUtf8)`
- the existing tests assert `Ok(...)` of the same values

`crates/isograph_cli/src/adapter.rs`: the tests call `format_iso_literals(&db, path())` with `fn path() -> RelativePathToSourceFile { "src/Pet.ts".intern().to() }`. `uri()` is deleted. Facts unchanged.

`crates/isograph_cli/src/lsp_socket.rs`: `initialize_advertises_formatting_and_will_save_wait_until` also asserts `textDocumentSync.openClose == true` and `textDocumentSync.change == 1`. `semantic_tokens_full_of_a_non_file_uri_is_null` stays.

### Call sites

- `dispatch_lsp_notification` -> `on_did_open` / `on_did_change` / `on_did_close` -> `relative_from_uri`, then `set_open_file` / `remove_open_file`
- `handle_client_connection` `Gone` -> `live_sessions.0.remove`, `remove_open_files_of_client`
- `handle_disk_changed` -> `apply_disk_changed` -> `relative_path_to_source_file` -> `relative_path_from_absolute_and_working_directory`
- `semantic_tokens_response` / `formatting_response` / `will_save_wait_until_response` -> `relative_from_uri`

### Docs this change amends

`docs-website/docs/design-docs/event-model.md`:

- Building blocks: "`DiskFile`, and `OpenFile`" stays; add `ClientId`.
- State: the path-keyed `struct OpenFile { path, contents }` becomes this doc's `OpenFileMap`, `OpenFileKey`, `OpenFile`, and the `open_file_map` field; "`OpenFile` and its map land with the adapter." is deleted. "The LSP adapter reads `OpenFile` when it exists for that path, otherwise `DiskFile`." becomes "The LSP adapter reads through `file_contents` with the asking client's `ClientId`."
- Event: `EditorChanged` and its `Buffer` are not added. "`InternalEvent` values come from outside `handle`: `DiskChanged`, `EditorChanged`." drops `EditorChanged`. "`EditorChanged` is an editor-buffer fact..." is deleted.
- Dispatch: "`EditorChanged` with `Open` sets `OpenFile`. `EditorChanged` with `Closed` removes it." becomes "`DidOpenTextDocument` and `DidChangeTextDocument` (one change, no range) call `set_open_file` for the notification's client. `DidCloseTextDocument` calls `remove_open_file`. `ClientConnectionEvent::Gone` calls `remove_open_files_of_client`."
- The test-harness sentence "A test constructs a `DiskChanged` or `EditorChanged`" becomes "A test constructs a `DiskChanged` or an `LspNotification`".

`docs-website/docs/design-docs/pico.md` Source: the path-keyed `OpenFile` snippet becomes `OpenFileKey` and `OpenFile` as in change 5, and the Database snippet gains `#[tracked] open_file_map: OpenFileMap`. "A path may have a `DiskFile`, an `OpenFile`, both, or neither. Artifact generation reads `DiskFile`. The LSP reads `OpenFile` when that path has one, otherwise `DiskFile`. Those are different memos. One overlay used by both, as isograph's `read_iso_literals_source` does, makes artifact generation depend on editor buffers." becomes "A path may have a `DiskFile`, one `OpenFile` per client that has it open, both, or neither. Every read of file text is `file_contents(db, client, path)`: client `c` reads `c`'s `OpenFile` if present, otherwise `DiskFile`. The file-based compiler will be another client with its own `ClientId`."

`pico.md` Tracked maps: add "`open_file_map` is keyed by `OpenFileKey { client, path }`. `file_contents` reads `OpenFile::id_for_key` of that key, then `DiskFile::id_for_key` of the path on a miss. A keystroke re-invokes only the slot that read that `OpenFile`. An open or close re-invokes only the `file_contents` slots of that client and path."

## Change 7: semantic tokens and formatting read `file_contents`

Landed. Deltas are under Known gap.

After this change the extract memo and every memo downstream of it are keyed by `(client, path)` and read text through `file_contents`. `semanticTokens/full`, `textDocument/formatting`, and `textDocument/willSaveWaitUntil` read with the asking client's `ClientId`.

### `extract_iso_literals_from_text`

The TypeScript extract memo looks up a `DiskFile` and runs the regex. This change splits lookup from extract-from-text. Origin: the regex body of `TypeScriptHostLanguage::extract_iso_literals`. Delta: the body after the lookup is an associated function on `HostLanguage` that takes `&str`, and `HostLanguage::extract_iso_literals` is deleted from the trait and from `TypeScriptHostLanguage`, `TestHostLanguage`, and `InHost`.

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait HostLanguage: Send + Sync + Sized + 'static {
    type Error: std::fmt::Display + std::error::Error + Clone + PartialEq + Eq + 'static;
    type LiteralContext: Clone + PartialEq + Eq + Debug + 'static;

    fn extract_iso_literals_from_text(contents: &str) -> Vec<IsoLiteralExtraction<Self>>;

    fn should_skip_source_file(relative_path: &Path) -> SkipSourceFile;
}
```

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    fn extract_iso_literals_from_text(contents: &str) -> Vec<IsoLiteralExtraction<Self>> {
        EXTRACT_ISO_LITERAL
            .captures_iter(contents)
            .filter_map(|captures| { /* the body from today's memo, unchanged */ })
            .collect()
    }
```

`extract_iso_literals_from_text` is not a memo. `TestHostLanguage` (`database.rs`) and `InHost` (`watch.rs`) return `Vec::new()`.

### The client-keyed family

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
    let contents = file_contents(db, client, path).as_ref()?;
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
    let contents = file_contents(db, client, path).as_ref()?;
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
    let file_content = file_contents(db, client, path).as_ref()?;
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
    let extractions = extract_iso_literals(db, literal_id.client, literal_id.path).as_ref()?;
    extractions.get(literal_id.index).cloned()
}
```

`LiteralId` carries the client because its `index` is a position in one client's text; with the client inside the id, a `LiteralId` from client A cannot be paired with client B's text.

`isograph_extract_typescript::file_literals` gains `client: ClientId` and calls `extract_iso_literals(db, client, path)`.

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

### Request handlers read the client

Change 2 already passes the client to every request handler. This change reads it: `semantic_tokens_response`, `formatting_response`, and `will_save_wait_until_response` rename `_client: ClientId` to `client: ClientId` and pass it to `lsp_semantic_tokens_for_file` / `format_iso_literals`. `LSPRequestDispatch` and `dispatch_lsp_request` are unchanged by this change.

### Formatting and `willSaveWaitUntil`

With open decision 2 (a):

```rust
// from crates/isograph_cli/src/adapter.rs
fn format_iso_literals<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::TextEdit>> {
    let literals = isograph_compiler::parsed_iso_literals_in_file(db, client, path).as_ref()?;
    let page_content =
        isograph_compiler::text_through_last_iso_literal(db, client, path).as_ref()?;
    // ... the filter_map over literals is unchanged
}
```

With open decision 2 (b), format-file-from-segments.md landed first:

```rust
// from crates/isograph_cli/src/adapter.rs
fn format_iso_literals<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::TextEdit>> {
    let literals = isograph_compiler::parsed_iso_literals_in_file(db, client, path).as_ref()?;
    let contents = isograph_compiler::file_contents(db, client, path).as_ref()?;
    let new_text = rewrite_iso_interiors(contents, literals);
    lsp_types::TextEdit {
        range: isograph_lsp::lsp_range_for_byte_range(contents, 0, contents.len() as u32),
        new_text,
    }
    .wrap_vec()
    .wrap_some()
}
```

### Tests

`crates/isograph_cli/src/state.rs` (`handle` seam, `perform` then `rx.recv()`, no sleep). `KEYWORD` is token type 15.

- disk `src/x.ts` is `const x = 1;\n`; client 1 `didOpen` with `iso(\`field Query.x{id}\`)`; Formatting from client 1: exactly one `TextEdit`, `new_text == "\n  field Query.x {\n    id,\n  }\n"`
- the same setup; `willSaveWaitUntil` from client 1 returns the same edits as Formatting
- disk `src/Home.ts` is `export const Home = iso(\`entrypoint Query.HomeRoute\`)`; client 1 `didOpen` with `iso(\`field Query.x{id}\`)`; `semanticTokens/full` from client 1: first token type `KEYWORD`, length 5
- then client 1 `didChange` to `iso(\`entrypoint Query.A\`)`: first token length 10
- then client 1 `didClose`: tokens equal those of a client with no buffer
- clients 1 and 2 `didOpen` the same URI with `iso(\`field Query.x{id}\`)` and `iso(\`entrypoint Query.A\`)`: client 1's first token length 5, client 2's length 10
- client 1 has a buffer; client 2 never opened the URI: client 2's tokens equal disk's
- open-only file: no `DiskFile` at `src/new.ts`; client 1 `didOpen` with an iso literal: tokens non-null; `didClose`: JSON `null`
- an empty buffer over a disk iso literal: tokens are `Some(vec![])`, not disk's

`isograph_extract_typescript`: `extract_iso_literals_from_text` of one `iso(\`...\`)` is one extraction; of `""` is empty; of `const x = 1;` is empty; of two calls is two in source order. The memo tests call `extract_iso_literals(&db, ClientId(1), path)` and the family with `ClientId(1)`; `LiteralId` gains `client`; facts unchanged. Added: `literal_id_at_location` with client 1 over a buffer names that buffer's literal in `iso_literal_extraction`, while client 2, with no buffer, names disk's.

`crates/isograph_lsp/src/file_semantic_tokens.rs`: tests pass `ClientId(1)`; facts unchanged. Added: a buffer whose iso text differs from disk: first token type 15, length 10; `remove_open_file` then tokens equal disk's.

`crates/isograph_cli/src/adapter.rs`: `format_iso_literals(&db, client(), path())` with `fn client() -> ClientId { ClientId(1) }`.

- `pretty_disk_edits_applied_to_a_buffer_with_an_extra_blank_line_leave_an_extra_brace` is renamed `edits_for_a_client_without_a_buffer_applied_to_a_longer_buffer_leave_an_extra_brace`; fact unchanged
- added `edits_for_a_client_with_that_buffer_open_are_pretty_when_applied_to_it`: disk is `PRETTY`; for each `spaces` in `0..=4`, `set_open_file(client(), path(), source_with_blank_spaces(spaces))`; the edits applied to `source_with_blank_spaces(spaces)` equal `PRETTY`

`crates/isograph_cli/src/lsp_socket.rs`: added, on one connection, no sleep: `initialize`; an `isograph/event` `DiskChanged` for `/tmp/proj/src/Home.ts` with `export const Home = iso(\`entrypoint Query.HomeRoute\`)`; `didOpen` of that URI with `iso(\`field Query.x{id}\`)`; `semanticTokens/full`: first token length 5. One reader thread per connection and one channel to `run_event_loop` deliver the three in order.

`ts_graphql_react_isograph_cli/tests/cli.rs` e2e send and semantic-tokens tests keep their facts: send never opens a file.

### Call sites

- `dispatch_lsp_request` -> `LSPRequestDispatch::on_request_sync` -> `semantic_tokens_response(db, client, params)` -> `lsp_semantic_tokens_for_file(db, client, path)` -> `parsed_iso_literals_in_file` / `text_through_last_iso_literal` -> `extract_iso_literals` -> `file_contents`
- `dispatch_lsp_request` -> `formatting_response` / `will_save_wait_until_response` -> `format_iso_literals(db, client, path)` -> the same family
- `file_literals`, `literal_id_at_location`, `iso_literal_extraction` -> `extract_iso_literals` / `file_contents` with the caller's `ClientId`

### Docs this change amends

`pico.md` Memo: the `extract_iso_literals` snippet becomes the free memo keyed by `client` and `path`. Parser and extract: "Extract is a memo over a `DiskFile`" becomes "Extract is a memo that reads `file_contents` and calls `HostLanguage::extract_iso_literals_from_text`."

`pico.md` Key: "Public APIs take one of: `path`, `path` and `LineChar`, ..." lists `client` and `path`, and `client`, `path`, and `LineChar`, for the APIs that read file text. "Source text is `DiskFile.contents` under `path`. A function that needs the file takes `path` and calls `db.disk_file(path)`." becomes "Source text is `file_contents(db, client, path)`. A function that needs the file takes `client` and `path` and calls `file_contents`."

`refactors/pending/format-file-from-segments.md`: "Independent of lsp-open-files.md. Does not add `OpenFile`." becomes the ordering from open decision 2. Its `format_iso_literals` takes `client: ClientId` and reads `isograph_compiler::file_contents(db, client, path)`.

`refactors/pending/lsp-diagnostics.md`: `report_diagnostics`, for each `ClientId` in `LiveSessions` whose `PublishDiagnosticsCap` is `Yes`, reads `file_literals(db, client, path)` over the disk paths and that client's open paths, and publishes that client's set to that client only. `didOpen` / `didChange` arm the debounce like `DiskChanged`.

## Optional change: `SendLspResponseEffect` is data

Ships only if open decision 3 picks (a).

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

Tests: `lsp_request_returns_method_not_found` keeps `perform` then `recv`, which pins that `perform` sends the response on the session's sender. The other `state.rs` response tests match `let [IsographEffect::SendLspResponse(effect)] = effects.as_slice() else { panic!(...) };` and assert on `effect.response().id`, `.result`, `.error`. `after_compute` tests assert `response().id` is the closed-over id.

Call sites: `send_lsp_response` -> `LiveSessions::after_compute` -> `SendLspResponseEffect::new`; `perform` -> `SendLspResponseEffect::send`; tests -> `SendLspResponseEffect::response`.
