# LSP dispatch

Requires lsp-request-response.md (landed). Independent of lsp-sessions.md, filesystem-watcher.md. lsp-tokens.md is the first domain request arm. didOpen / didChange / didClose are later notification arms.

isograph's server loop matches `Request` / `Notification` / `Response`, then `LSPRequestDispatch` / `LSPNotificationDispatch`. `initialize` / `initialized` / `exit` stay outside (`Connection::initialize`, reader stop on `Exit`).

After lsp-port.md, the session special-cases `isograph/event` and ignores every other notification. After lsp-request-response.md, `IsographEvent` has `HelloWorld` / `Quit` / `DiskChanged` / `LspRequest`. This slice makes `IsographEvent` two variants: `Lsp` and `Internal`. `Lsp` is `Request` (request plus reply) / `Notification` / `Response`. `Internal` is `HelloWorld` / `Quit` / `DiskChanged` (the `--file` JSON). The session posts every message after initialize as `Lsp`. `handle` of `Lsp` is isograph's server-loop match. `dispatch_lsp_request` is `LSPRequestDispatch` with no `on_request_sync`; leftover is `method_not_found`. `dispatch_lsp_notification` returns `Vec<IsographEffect>`; it does not return `SendLspResponse` (a notification has no id). Tokens adds the first `.on_request_sync`.

Origin: isograph `crates/isograph_lsp/src/lsp_request_dispatch.rs`, `lsp_notification_dispatch.rs`, `lsp_runtime_error.rs`, `server.rs` `dispatch_request` / `dispatch_notification`.

Delta on `lsp_runtime_error.rs` and `lsp_request_dispatch.rs`: none. Copy into `crates/isograph_lsp/src/` under the same names, including `#[cfg(test)]`. `lib.rs` has `pub mod` for all three (isograph's request dispatch is `mod`; the call site here is `isograph_cli`).

Delta on `lsp_notification_dispatch.rs`: extract is `Result` (isograph `expect`s); handler returns `Vec<IsographEffect>` (isograph returns `LSPRuntimeResult<()>`); `JsonError` logs and `Break`s with an empty vec; leftover is empty vec. `TState` is `&mut IsographState`. Do not thread a `Vec` of effects through `TState`. isograph's notification params are protocol-typed; ours include `isograph/event`, a JSON DSL the session already recovers. Socket-supplied JSON is a typed error or a recovered region. `expect` on extract is neither.

Call-site delta: `IsographEvent` is `Lsp` | `Internal`; `handle` of `Lsp` matches `Lsp` the way isograph's server loop matches `Message`; `dispatch_lsp_request` is the `LSPRequestDispatch` `?` chain with zero handlers, Continue `method_not_found` returning `Vec<IsographEffect>` (one immediate `SendLspResponse`); `--file` / `isograph/event` params are `Internal` (same JSON as today's `IsographEvent` kinds).

One shippable change. Unknown requests are still `MethodNotFound`. `isograph send` still initialize + `isograph/event`. Existing CLI send tests stay green.

`docs-website/docs/design-docs/event-model.md` Inner: `IsographEvent` is `Lsp` | `Internal`. `handle` of `Lsp` matches `Request` / `Notification` / `Response` and runs isograph request/notification dispatch. A request leftover is `method_not_found` (`Vec<IsographEffect>`, one immediate `SendLspResponse`). A notification leftover is no effects. A notification that matches returns `Vec<IsographEffect>` and not `SendLspResponse`. A response is no effects. `Internal` is `HelloWorld` / `Quit` / `DiskChanged`. The session does not interpret methods after `initialize`. Bad `isograph/event` params are leftover (empty effects). The connection and the event loop stay up.

## What the user does

Same as after lsp-request-response.md. `isograph send` of `{"kind":"Nope"}` still fails on the client (`NotEvent`). A socket that posts `isograph/event` with those params does not kill the daemon.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
use lsp_server::Message;

#[derive(Clone, Debug)]
pub struct LspRequest {
    pub request: lsp_server::Request,
    pub reply: crossbeam::channel::Sender<Message>,
}

#[derive(Clone, Debug)]
pub enum Lsp {
    Request(LspRequest),
    Notification(lsp_server::Notification),
    Response(lsp_server::Response),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, derive_more::From)]
#[serde(tag = "kind", content = "value")]
pub enum Internal {
    HelloWorld,
    Quit,
    DiskChanged(DiskChanged),
}

#[derive(Clone, Debug, derive_more::From)]
pub enum IsographEvent {
    #[from]
    Lsp(Lsp),
    #[from]
    Internal(Internal),
}
```

`LspRequest` is landed. `reply` exists only on `Request`. Session clones `connection.sender` only for requests.

`IsographEvent` is not on the wire. `--file` and `isograph/event` params are `Internal` (same JSON as today's `{"kind":"HelloWorld"}` / `DiskChanged` / `Quit`).

```rust
// from crates/isograph_lsp/src/lsp_runtime_error.rs
pub type LSPRuntimeResult<T> = std::result::Result<T, LSPRuntimeError>;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum LSPRuntimeError {
    ExpectedError,
    UnexpectedError(String),
}
```

Copied from isograph `crates/isograph_lsp/src/lsp_runtime_error.rs`, including `From<LSPRuntimeError> for Option<lsp_server::ResponseError>` and the `#[cfg(test)]` modules of the request file. Hand-written `From`, no `thiserror`. Request extract failure answers with id `"default-lsp-id"` (copied isograph bug). No request handler this slice calls extract.

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
```

Copied from isograph `crates/isograph_lsp/src/lsp_request_dispatch.rs` including `convert_to_lsp_response`, `extract_request_params`, and `#[cfg(test)]`.

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

    pub fn on_notification_sync<TNotification: lsp_types::notification::Notification>(
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
                ControlFlow::Break(Vec::new())
            }
        }
    }

    pub fn notification(self) -> lsp_server::Notification {
        self.notification
    }
}
```

`isograph_lsp` cannot name `crate::effect::IsographEffect`. Handler return is `Vec<isograph_cli::effect::IsographEffect>` only if `isograph_lsp` depends on `isograph_cli` (cycle: `isograph_cli` depends on `isograph_lsp`). Put the notification dispatcher in `crates/isograph_cli/src/lsp_notification_dispatch.rs`. Request dispatch and `LSPRuntimeError` stay in `isograph_lsp` (no cycle: they return `Response` / `LSPRuntimeError`).

Origin of the struct and `?` chain: isograph `lsp_notification_dispatch.rs`. Delta: extract is `Result`; handler returns `Vec<IsographEffect>`; `JsonError` is leftover (empty vec), same drop as today's session `from_value` failure plus `warn`. Do not copy isograph's `#[cfg(test)]` module (handler signature changed).

Do not copy `lsp_command_dispatch.rs`. That is `ExecuteCommand` later.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
impl Notification for IsographEventNotification {
    type Params = crate::event::Internal;
    const METHOD: &'static str = "isograph/event";
}
```

```rust
// from crates/isograph_cli/src/send.rs
    let internal: crate::event::Internal =
        serde_json::from_str(frame.trim()).map_err(SendError::NotEvent)?;
```

`notify` takes `Internal`.

```rust
// from crates/isograph_cli/src/daemon.rs
                    let _ = event_tx.send(crate::event::Internal::Quit.to());
```

### `handle`

```rust
// from crates/isograph_cli/src/state.rs
        IsographEvent::Lsp(lsp) => dispatch_lsp(state, lsp),
        IsographEvent::Internal(internal) => handle_internal(state, internal),

fn dispatch_lsp<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    lsp: crate::event::Lsp,
) -> Vec<IsographEffect> {
    match lsp {
        crate::event::Lsp::Request(incoming) => dispatch_lsp_request(state, incoming),
        crate::event::Lsp::Notification(notification) => {
            dispatch_lsp_notification(state, notification)
        }
        crate::event::Lsp::Response(_) => Vec::new(),
    }
}

fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &IsographState<THostLanguage>,
    incoming: crate::event::LspRequest,
) -> Vec<IsographEffect> {
    let crate::event::LspRequest { request, reply } = incoming;
    let get_response = || {
        let request =
            isograph_lsp::lsp_request_dispatch::LSPRequestDispatch::new(request, state).request();
        ControlFlow::Continue(request)
    };
    match get_response() {
        ControlFlow::Break(response) => crate::effect::IsographEffect::SendLspResponse(
            crate::effect::SendLspResponse { reply, response }.boxed(),
        )
        .wrap_vec(),
        ControlFlow::Continue(request) => method_not_found(crate::event::LspRequest { request, reply }),
    }
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

fn handle_internal<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    internal: crate::event::Internal,
) -> Vec<IsographEffect> {
    match internal {
        crate::event::Internal::HelloWorld => IsographEffect::LogHelloWorld.wrap_vec(),
        crate::event::Internal::Quit => IsographEffect::Kill.wrap_vec(),
        crate::event::Internal::DiskChanged(change) => handle_disk_changed(state, change),
    }
}

fn dispatch_lsp_notification<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    notification: lsp_server::Notification,
) -> Vec<IsographEffect> {
    let dispatch = || {
        crate::lsp_notification_dispatch::LSPNotificationDispatch::new(notification, state)
            .on_notification_sync::<crate::lsp_socket::IsographEventNotification>(
                on_isograph_event::<THostLanguage>,
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
    internal: crate::event::Internal,
) -> Vec<IsographEffect> {
    handle_internal(state, internal)
}
```

`handle_disk_changed` already returns `Vec<IsographEffect>` (empty after intern or remove).

`dispatch_lsp` is isograph `server.rs` `match`. `dispatch_lsp_request` is isograph `dispatch_request` with zero `on_request_sync`. Continue is `method_not_found`. Tokens inserts `.on_request_sync::<SemanticTokensFullRequest>(...)?` before `.request()`. Notification leftover is empty vec. A handler that matches returns its `Vec<IsographEffect>`. Nothing type-enforces that vec contains no `SendLspResponse`; do not construct one from a notification.

`on_request_sync` has no production caller this slice. `LSPRequestDispatch::new(...).request()` is the leftover path. Continue is the reader.

```rust
// from crates/isograph_cli/src/lib.rs
mod lsp_notification_dispatch;
```

```rust
// from crates/isograph_lsp/src/lib.rs
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
                let skip = path.ends_with("crates/isograph_lsp/src/lsp_request_dispatch.rs")
                    || path.ends_with("crates/isograph_lsp/src/lsp_runtime_error.rs");
                if skip {
                    continue;
                }
                lint_rust_file(path.reference(), hits);
            }
```

Skip those two paths, not any file of that basename.

### Session

```rust
// from crates/isograph_cli/src/lsp_socket.rs
    for message in &connection.receiver {
        let event = match message {
            lsp_server::Message::Request(request) => crate::event::Lsp::Request(
                crate::event::LspRequest {
                    request,
                    reply: connection.sender.clone(),
                },
            ),
            lsp_server::Message::Notification(notification) => {
                crate::event::Lsp::Notification(notification)
            }
            lsp_server::Message::Response(response) => crate::event::Lsp::Response(response),
        };
        let _ = event_tx.send(event.to());
    }
```

The session does not match methods. `Connection::initialize` still consumes `initialize` and `initialized`. The reader still stops on `Exit`. Those stay outside dispatch, same as isograph.

## Tests

`state.rs`: `handle` of `Lsp::Request` hover is still `SendLspResponse` / `MethodNotFound`. Same assertion as lsp-request-response.md (`id`, `result` `None`, code, message, `perform` delivers that `Response`). Do not add a tests-only request handler.

`handle` of `Lsp::Notification` `isograph/event` with `HelloWorld` params: `[LogHelloWorld]`.

`handle` of `Lsp::Notification` `isograph/event` with `Quit` params: `[Kill]`.

`handle` of `Lsp::Notification` `isograph/event` with `DiskChanged` Present, with a config directory interned: no effects, file interned. Same facts as the existing `DiskChanged` test.

`handle` of `Lsp::Notification` `window/logMessage`: no effects.

`handle` of `Lsp::Response`: no effects.

`handle` of `Lsp::Notification` `isograph/event` with params `{"kind":"Nope"}`: no effects. Does not panic. This is `JsonError` leftover. Same recovery as today's session `from_value` failure. Run `handle`, not `listen_for_events`.

`lsp_socket.rs`: `listen_for_events` sees `Lsp::Notification`, not `Internal::HelloWorld` / `DiskChanged` / `Quit`. Assert `method` is `IsographEventNotification::METHOD` and `serde_json::from_value::<Internal>(params)` is that event. Same for the existing HelloWorld / DiskChanged present-then-absent / multi-connection tests.

`unknown_notification_is_not_an_event`: `window/logMessage` arrives as `Lsp::Notification`. Then `HelloWorld` as today.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
        tokio::spawn(async move {
            while let Some(event) = session_rx.recv().await {
                if !matches!(event, IsographEvent::Lsp(crate::event::Lsp::Request(_))) {
                    let _ = test_tx.send(event.clone());
                }
                let _ = loop_tx.send(event);
            }
        });
```

Tee hides `Lsp::Request` so MethodNotFound tests that then send HelloWorld `try_recv` a notification, not the hover request. Loops still run `handle`. After hover MethodNotFound, `isograph/event` HelloWorld is `Lsp::Notification` on that tee.

`ServerNotInitialized` is still `Connection::initialize`. A request before initialize is not `Lsp`.

```rust
// from crates/isograph_cli/src/daemon.rs
        event_tx
            .send(crate::event::Internal::HelloWorld.to())
            .expect("the test sends HelloWorld");
```

Same for `Quit`: `Internal::Quit.to()`. They do not send `Lsp`.

`send.rs`: `--file` deserializes as `Internal`. Same JSON as today.

`cli.rs` unchanged.

`state.rs` existing `HelloWorld` / `Quit` / `DiskChanged` tests construct `IsographEvent::Internal(...)`.

Copied `#[cfg(test)]` in `lsp_request_dispatch.rs` stays.

Do not add a production API only tests call.

## Call sites

- `run_session` any message -> `Lsp` -> `handle` -> `dispatch_lsp`
- `Lsp::Request` -> `LSPRequestDispatch` (zero handlers) -> Continue `method_not_found` -> `[SendLspResponse]`
- `Lsp::Notification` -> `LSPNotificationDispatch` -> `Vec<IsographEffect>`, not `SendLspResponse`
- `isograph/event` -> `on_isograph_event` -> `handle_internal`
- `isograph/event` `JsonError` -> empty vec, `warn`, daemon stays up
- `isograph send --file` -> `Internal` JSON -> `isograph/event`
- SIGTERM -> `Internal::Quit`
- lsp-tokens.md -> `.on_request_sync::<SemanticTokensFullRequest>(...)?` on `LSPRequestDispatch`
- later didOpen / didChange / didClose -> `.on_notification_sync` on `LSPNotificationDispatch`
