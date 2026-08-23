# LSP dispatch

Requires lsp-request-response.md (landed). Independent of lsp-sessions.md, filesystem-watcher.md. lsp-tokens.md is the first domain request arm. didOpen / didChange / didClose are later notification arms.

isograph's server loop runs `LSPRequestDispatch` and `LSPNotificationDispatch` the same way: `?` chain, first match wins, leftover is MethodNotFound (request) or drop (notification). `initialize` / `initialized` / `exit` stay outside (`Connection::initialize`, reader stop on `Exit`).

Those structs already exist. This slice puts them in `isograph_lsp` and calls them from `handle`. It does not write a second dispatcher.

After lsp-port.md, the session special-cases `isograph/event` and ignores every other notification. After lsp-request-response.md, a request is `LspRequest` and `handle` is `method_not_found`. This slice makes the session a pump: every request is `LspRequest`, every notification is `LspNotification`. `handle` of `LspRequest` is `method_not_found` (no `on_request_sync`). `handle` of `LspNotification` is isograph `dispatch_notification`. The notification chain has `isograph/event`. Tokens adds the first `.on_request_sync`.

Origin: isograph `crates/isograph_lsp/src/lsp_request_dispatch.rs`, `lsp_notification_dispatch.rs`, `lsp_runtime_error.rs`, `server.rs` `dispatch_request` / `dispatch_notification`. Delta on the three files: none. They are copied into `crates/isograph_lsp/src/` under the same names, including `#[cfg(test)]`. `lib.rs` has `pub mod` for all three (isograph's `lsp_request_dispatch` is `mod`; the call site here is `isograph_cli`). Call-site delta: `TState` is `IsographState` not `LspState`; this slice there is no `on_request_sync`; `dispatch_lsp_request` is `method_not_found`, which returns `Vec<IsographEffect>` (one immediate `SendLspResponse`); notification `TState` is `(&mut IsographState, &mut Vec<IsographEffect>)` because the existing handler returns `LSPRuntimeResult<()>` and `handle` returns effects.

One shippable change. Unknown requests are still `MethodNotFound`. `isograph send` still initialize + `isograph/event`. Existing CLI send tests stay green.

`docs-website/docs/design-docs/event-model.md` Inner: `handle` of `LspRequest` is `method_not_found`, which returns `Vec<IsographEffect>` (one immediate `SendLspResponse`). No request is dispatched this slice. `handle` of `LspNotification` runs `LSPNotificationDispatch`; leftover is no effects. The session does not interpret methods after `initialize`.

## What the user does

Same as after lsp-request-response.md.

## Types

Most important first.

Copy these isograph files into `crates/isograph_lsp/src/` under the same names, including `#[cfg(test)]` modules and doc comments. Byte-for-byte with the origin besides path. Do not edit them. Do not reimplement `on_request_sync` / `on_notification_sync` / `convert_to_lsp_response` / `extract_request_params` / `extract_notification_params` / `LSPRuntimeError` in `isograph_cli`.

- `crates/isograph_lsp/src/lsp_runtime_error.rs` from isograph `crates/isograph_lsp/src/lsp_runtime_error.rs`
- `crates/isograph_lsp/src/lsp_request_dispatch.rs` from isograph `crates/isograph_lsp/src/lsp_request_dispatch.rs`
- `crates/isograph_lsp/src/lsp_notification_dispatch.rs` from isograph `crates/isograph_lsp/src/lsp_notification_dispatch.rs`

`crates/prelude/src/postfix_constructors.rs` `walk_rust` skips those three filenames. They use `Ok` / `Err` / `Some` / `.into()` as isograph wrote them.

```rust
// from crates/isograph_lsp/src/lsp_runtime_error.rs
pub type LSPRuntimeResult<T> = std::result::Result<T, LSPRuntimeError>;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum LSPRuntimeError {
    ExpectedError,
    UnexpectedError(String),
}
```

`From<LSPRuntimeError> for Option<lsp_server::ResponseError>` is in that file.

```rust
// from crates/isograph_lsp/src/lsp_request_dispatch.rs
pub struct LSPRequestDispatch<'state, TState> {
    request: lsp_server::Request,
    state: &'state TState,
}

impl<'state, TState> LSPRequestDispatch<'state, TState> {
    pub fn new(request: lsp_server::Request, state: &'state TState) -> Self;

    pub fn on_request_sync<TRequest: lsp_types::request::Request>(
        self,
        handler: fn(&TState, TRequest::Params) -> LSPRuntimeResult<TRequest::Result>,
    ) -> ControlFlow<lsp_server::Response, Self>;

    pub fn request(self) -> lsp_server::Request;
}

pub(crate) fn convert_to_lsp_response(
    id: lsp_server::RequestId,
    result: LSPRuntimeResult<serde_json::Value>,
) -> lsp_server::Response;

fn extract_request_params<R: lsp_types::request::Request>(
    req: lsp_server::Request,
) -> LSPRuntimeResult<(lsp_server::RequestId, R::Params)>;
```

```rust
// from crates/isograph_lsp/src/lsp_notification_dispatch.rs
pub struct LSPNotificationDispatch<'state, TState> {
    notification: lsp_server::Notification,
    state: &'state mut TState,
}

impl<'state, TState> LSPNotificationDispatch<'state, TState> {
    pub fn new(notification: lsp_server::Notification, state: &'state mut TState) -> Self;

    pub fn on_notification_sync<TNotification: lsp_types::notification::Notification>(
        self,
        handler: fn(&mut TState, TNotification::Params) -> LSPRuntimeResult<()>,
    ) -> ControlFlow<Option<LSPRuntimeError>, Self>;

    pub fn notification(self) -> lsp_server::Notification;
}

fn extract_notification_params<N: lsp_types::notification::Notification>(
    notification: lsp_server::Notification,
) -> N::Params;
```

`extract_notification_params` `expect`s. A notification whose method matched and whose params fail to deserialize panics. Same as isograph. Request extract `catch_unwind`s that panic and returns `UnexpectedError` with id `"default-lsp-id"`. Same as isograph.

Do not copy `lsp_command_dispatch.rs`. That is `ExecuteCommand` later.

```rust
// from crates/isograph_cli/src/event.rs
use lsp_server::Message;

#[derive(Clone, Debug)]
pub struct LspRequest {
    pub request: lsp_server::Request,
    pub reply: crossbeam::channel::Sender<Message>,
}

#[derive(Clone, Debug)]
pub struct LspNotification(pub lsp_server::Notification);

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, derive_more::From)]
#[serde(tag = "kind", content = "value")]
pub enum IsographEvent {
    HelloWorld,
    Quit,
    DiskChanged(DiskChanged),
    #[serde(skip)]
    #[from]
    LspRequest(LspRequest),
    #[serde(skip)]
    #[from]
    LspNotification(LspNotification),
}
```

`LspNotification` is a newtype: one field. `LspRequest` stays a struct because of `reply`. Neither is a `--file` kind.

### `handle`

```rust
// from crates/isograph_cli/src/state.rs
        IsographEvent::LspRequest(incoming) => dispatch_lsp_request(incoming),
        IsographEvent::LspNotification(incoming) => {
            dispatch_lsp_notification(state, incoming.0)
        }
```

```rust
fn dispatch_lsp_request(incoming: crate::event::LspRequest) -> Vec<IsographEffect> {
    method_not_found(incoming)
}

fn method_not_found(incoming: crate::event::LspRequest) -> Vec<IsographEffect> {
    // Immediate SendLspResponse this slice. Async later: a timer plus an event; handle of
    // that event is an immediate SendLspResponse.
    crate::effect::IsographEffect::SendLspResponse(
        crate::effect::SendLspResponse {
            reply: incoming.reply,
            response: lsp_server::Response {
                id: incoming.request.id,
                result: None,
                error: lsp_server::ResponseError {
                    code: lsp_server::ErrorCode::MethodNotFound as i32,
                    data: None,
                    message: format!(
                        "No handler registered for method '{}'",
                        incoming.request.method
                    ),
                }
                .wrap_some(),
            },
        }
        .boxed(),
    )
    .wrap_vec()
}

fn dispatch_lsp_notification<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    notification: lsp_server::Notification,
) -> Vec<IsographEffect> {
    let mut effects = Vec::new();
    let _ = dispatch_notification(state, &mut effects, notification);
    effects
}

fn dispatch_notification<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    effects: &mut Vec<IsographEffect>,
    notification: lsp_server::Notification,
) -> ControlFlow<Option<isograph_lsp::lsp_runtime_error::LSPRuntimeError>, ()> {
    isograph_lsp::lsp_notification_dispatch::LSPNotificationDispatch::new(
        notification,
        &mut (state, effects),
    )
    .on_notification_sync::<crate::lsp_socket::IsographEventNotification>(
        on_isograph_event::<THostLanguage>,
    )?
    .notification();
    ControlFlow::Continue(())
}

fn on_isograph_event<THostLanguage: HostLanguage>(
    (state, effects): &mut (
        &mut IsographState<THostLanguage>,
        &mut Vec<IsographEffect>,
    ),
    event: crate::event::IsographEvent,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<()> {
    effects.extend(handle(state, event));
    ().wrap_ok()
}
```

`dispatch_notification` is isograph `server.rs`. `dispatch_lsp_request` this slice has no `on_request_sync`. The only request handler is `method_not_found`, which returns `Vec<IsographEffect>`: one immediate `SendLspResponse`. Tokens inserts the `LSPRequestDispatch` `?` chain and keeps `method_not_found` as Continue. Notification leftover is `ControlFlow::Continue(())`; `dispatch_lsp_notification` then returns the effects the handler pushed.

The notification chain has `isograph/event`. `on_isograph_event` calls `handle` with the extracted event (`HelloWorld` / `Quit` / `DiskChanged`). `LspRequest` / `LspNotification` are `#[serde(skip)]`, so they are not a `--file` / `isograph/event` payload. One-level re-entry.

The existing notification handler is `fn(&mut TState, Params) -> LSPRuntimeResult<()>`. It cannot return effects. `TState` is therefore `(&mut IsographState, &mut Vec<IsographEffect>)`. Request `TState` is `&IsographState`, same as isograph's `&LspState`.

`method_not_found` returns `Vec<IsographEffect>`.

No `crates/isograph_cli/src/lsp_dispatch.rs`.

```rust
// from crates/isograph_lsp/src/lib.rs
pub mod lsp_notification_dispatch;
pub mod lsp_request_dispatch;
pub mod lsp_runtime_error;
```

```toml
# from crates/isograph_lsp/Cargo.toml
lsp-server = { workspace = true }
serde_json = { workspace = true }
```

```toml
# from crates/isograph_cli/Cargo.toml
isograph_lsp = { path = "../isograph_lsp" }
```

```rust
// from crates/prelude/src/postfix_constructors.rs
            } else if path.extension().is_some_and(|e| e == "rs") {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if matches!(
                    name,
                    "lsp_request_dispatch.rs"
                        | "lsp_notification_dispatch.rs"
                        | "lsp_runtime_error.rs"
                ) {
                    continue;
                }
                lint_rust_file(path.reference(), hits);
            }
```

### Session

```rust
// from crates/isograph_cli/src/lsp_socket.rs
            lsp_server::Message::Request(request) => {
                let _ = event_tx.send(
                    crate::event::LspRequest {
                        request,
                        reply: connection.sender.clone(),
                    }
                    .to(),
                );
            }
            lsp_server::Message::Notification(notification) => {
                let _ = event_tx.send(crate::event::LspNotification(notification).to());
            }
            lsp_server::Message::Response(_) => {}
```

The session does not match `IsographEventNotification::METHOD`. `Connection::initialize` still consumes `initialize` and `initialized`. The reader still stops on `Exit`. Those stay outside dispatch, same as isograph.

## Tests

`state.rs`: `handle` of `LspRequest` (hover) is still `SendLspResponse` / `MethodNotFound`. Same assertion as lsp-request-response.md (`id`, `result` `None`, code, message, `perform` delivers that `Response`). Do not add a tests-only request handler.

`handle` of `LspNotification` whose method is `isograph/event` and params are `HelloWorld`: `[LogHelloWorld]`.

`handle` of `LspNotification` whose method is `isograph/event` and params are `Quit`: `[Kill]`.

`handle` of `LspNotification` whose method is `isograph/event` and params are `DiskChanged` Present, with a config directory interned: no effects, file interned. Same facts as the existing `DiskChanged` test.

`handle` of `LspNotification` whose method is `window/logMessage`: no effects.

Do not `handle` an `isograph/event` whose params fail to deserialize. `extract_notification_params` `expect`s; that is a panic. `bad_event_params_are_not_an_event` stays on `listen_for_events` (no `handle`): the session posts `LspNotification`; params do not deserialize as `IsographEvent`; then `HelloWorld` on the same connection is a second `LspNotification` whose params are `HelloWorld`.

`lsp_socket.rs`: `listen_for_events` sees `LspNotification`, not `HelloWorld` / `DiskChanged` / `Quit`. Assert `notification.0.method` is `IsographEventNotification::METHOD` and `serde_json::from_value::<IsographEvent>(notification.0.params)` is that event. Same for the existing HelloWorld / DiskChanged present-then-absent / multi-connection tests.

`unknown_notification_is_not_an_event`: `window/logMessage` arrives as `LspNotification`. Then `HelloWorld` as today.

MethodNotFound / shutdown / ServerNotInitialized tests stay. `listen_and_reply` tees non-`LspRequest` events. After hover MethodNotFound, `isograph/event` HelloWorld is `LspNotification` on that tee; the loops still run `handle`.

`ServerNotInitialized` is still `Connection::initialize`. A request before initialize is not `LspRequest`.

`daemon.rs` existing tests stay. They do not send `LspRequest` / `LspNotification`.

`cli.rs` unchanged.

The copied `#[cfg(test)]` modules in `isograph_lsp` stay. They are isograph's tests of those structs.

Do not add a production API only tests call.

## Call sites

- `run_session` Request -> `LspRequest` -> `handle` -> `method_not_found` -> `[SendLspResponse]` -> `perform`
- `run_session` Notification -> `LspNotification` -> `handle` -> `LSPNotificationDispatch` -> `isograph/event` `on_isograph_event` -> `handle` of `HelloWorld` / `Quit` / `DiskChanged`, or leftover empty
- lsp-tokens.md -> `.on_request_sync::<SemanticTokensFullRequest>(...)?` on `LSPRequestDispatch`
- later didOpen / didChange / didClose -> `.on_notification_sync` on `LSPNotificationDispatch`
