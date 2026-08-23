# LSP dispatch

Requires lsp-request-response.md (landed). Independent of lsp-sessions.md, filesystem-watcher.md. lsp-tokens.md is the first domain request arm. didOpen / didChange / didClose are later notification arms.

isograph's server loop treats request and notification the same: `LSPRequestDispatch` / `LSPNotificationDispatch` chains, `?` until the first match, leftover is MethodNotFound (request) or drop (notification). `initialize` / `initialized` / `exit` stay outside those chains (`Connection::initialize`, reader stop on `Exit`).

After lsp-port.md, the session special-cases `isograph/event` (deserialize, post `HelloWorld` / `Quit` / `DiskChanged`) and ignores every other notification. After lsp-request-response.md, a request is `LspRequest` and `handle` is `method_not_found`. This slice copies both isograph dispatchers onto `handle` and makes the session a pump: every request is `LspRequest`, every notification is `LspNotification`. `isograph/event` is the first notification arm. The request chain is empty. Tokens adds one `.on_request_sync`.

Origin: isograph `crates/isograph_lsp/src/lsp_request_dispatch.rs`, `lsp_notification_dispatch.rs`, `server.rs` `dispatch_request` / `dispatch_notification`. Delta: extract is `Result` (isograph `expect`s; request extract is also `catch_unwind`); no `LSPRuntimeError`; request handler returns `Result<TRequest::Result, lsp_server::ResponseError>`; notification handler returns `Vec<IsographEffect>` (isograph returns `LSPRuntimeResult<()>`; i2's `handle` returns effects); request `JsonError` uses the real id (isograph `"default-lsp-id"`); the dispatcher holds `lsp_server::Request` / `lsp_server::Notification`, not the reply sender; `handle` wraps a request `Response` as `SendLspResponse`.

One shippable change. Unknown requests are still `MethodNotFound`. `isograph send` still initialize + `isograph/event`. Existing CLI send tests stay green.

`docs-website/docs/design-docs/event-model.md` Inner: `handle` of `LspRequest` runs the request chain; leftover is `MethodNotFound`. `handle` of `LspNotification` runs the notification chain; leftover is no effects. The session does not interpret methods after `initialize`.

## What the user does

Same as after lsp-request-response.md. `isograph send` still initialize + `isograph/event`. An unknown request after initialize is still `MethodNotFound`. An unknown notification is still dropped.

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

```rust
// from crates/isograph_cli/src/lsp_dispatch.rs
use std::ops::ControlFlow;

use lsp_server::ExtractError;
use lsp_types::request::Request;
use prelude::Postfix;
use tracing::warn;

pub struct LspRequestDispatch<'state, TState> {
    request: lsp_server::Request,
    state: &'state TState,
}

impl<'state, TState> LspRequestDispatch<'state, TState> {
    pub fn new(request: lsp_server::Request, state: &'state TState) -> Self {
        Self { request, state }
    }

    pub fn on_request_sync<TRequest: Request>(
        self,
        handler: fn(&TState, TRequest::Params) -> Result<TRequest::Result, lsp_server::ResponseError>,
    ) -> ControlFlow<lsp_server::Response, Self> {
        if self.request.method != TRequest::METHOD {
            return ControlFlow::Continue(self);
        }
        let id = self.request.id.clone();
        match self.request.extract::<TRequest::Params>(TRequest::METHOD) {
            Ok((_, params)) => {
                let response = match handler(self.state, params) {
                    Ok(result) => match serde_json::to_value(result) {
                        Ok(value) => lsp_server::Response {
                            id,
                            result: value.wrap_some(),
                            error: None,
                        },
                        Err(e) => {
                            warn!(error = %e, "could not encode request result");
                            lsp_server::Response::new_err(
                                id,
                                lsp_server::ErrorCode::InternalError as i32,
                                "could not encode request result".to_owned(),
                            )
                        }
                    },
                    Err(error) => lsp_server::Response {
                        id,
                        result: None,
                        error: error.wrap_some(),
                    },
                };
                ControlFlow::Break(response)
            }
            Err(ExtractError::MethodMismatch(request)) => ControlFlow::Continue(Self {
                request,
                state: self.state,
            }),
            Err(ExtractError::JsonError { method, error }) => {
                warn!(method = method.as_str(), error = %error, "request params");
                ControlFlow::Break(lsp_server::Response::new_err(
                    id,
                    lsp_server::ErrorCode::InvalidParams as i32,
                    "invalid request params".to_owned(),
                ))
            }
        }
    }

    pub fn request(self) -> lsp_server::Request {
        self.request
    }
}
```

Origin: isograph `LSPRequestDispatch::on_request_sync` and `convert_to_lsp_response`. Delta: extract is `Result`; no `LSPRuntimeError`, no `catch_unwind`, no `"default-lsp-id"`; `JsonError` is `InvalidParams` with the real `id` cloned before `extract` consumes the request; postfix. `Break` is `lsp_server::Response`, same as isograph. After the method-string check, `extract` returning `MethodMismatch` is restore-and-Continue. Do not `unwrap`.

```rust
// from crates/isograph_cli/src/lsp_dispatch.rs
use lsp_types::notification::Notification;

pub struct LspNotificationDispatch<'state, TState> {
    notification: lsp_server::Notification,
    state: &'state mut TState,
}

impl<'state, TState> LspNotificationDispatch<'state, TState> {
    pub fn new(notification: lsp_server::Notification, state: &'state mut TState) -> Self {
        Self {
            notification,
            state,
        }
    }

    pub fn on_notification_sync<TNotification: Notification>(
        self,
        handler: fn(&mut TState, TNotification::Params) -> Vec<crate::effect::IsographEffect>,
    ) -> ControlFlow<Vec<crate::effect::IsographEffect>, Self> {
        if self.notification.method != TNotification::METHOD {
            return ControlFlow::Continue(self);
        }
        match self.notification.extract::<TNotification::Params>(TNotification::METHOD) {
            Ok(params) => ControlFlow::Break(handler(self.state, params)),
            Err(ExtractError::MethodMismatch(notification)) => ControlFlow::Continue(Self {
                notification,
                state: self.state,
            }),
            Err(ExtractError::JsonError { method, error }) => {
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

Origin: isograph `LSPNotificationDispatch::on_notification_sync`. Delta: extract is `Result`; handler returns `Vec<IsographEffect>` not `LSPRuntimeResult<()>` (`handle` returns effects; isograph mutates `LspState` and has no effect layer); `JsonError` logs and `Break`s with no effects (isograph `expect`s); postfix. `&mut TState` is isograph. `Break` carries the handler's effects instead of `Option<LSPRuntimeError>`.

Do not copy `LspIsographCommandDispatch`. That is `ExecuteCommand` later.

### `handle`

```rust
// from crates/isograph_cli/src/state.rs
        IsographEvent::LspRequest(incoming) => {
            let response = dispatch_lsp_request(state, incoming.request);
            crate::effect::IsographEffect::SendLspResponse(
                crate::effect::SendLspResponse {
                    reply: incoming.reply,
                    response,
                }
                .boxed(),
            )
            .wrap_vec()
        }
        IsographEvent::LspNotification(incoming) => {
            dispatch_lsp_notification(state, incoming.0)
        }

fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &IsographState<THostLanguage>,
    request: lsp_server::Request,
) -> lsp_server::Response {
    let get_response = || {
        let request = crate::lsp_dispatch::LspRequestDispatch::new(request, state).request();
        ControlFlow::Continue(request)
    };
    match get_response() {
        ControlFlow::Break(response) => response,
        ControlFlow::Continue(request) => method_not_found(request),
    }
}

fn method_not_found(request: lsp_server::Request) -> lsp_server::Response {
    lsp_server::Response {
        id: request.id,
        result: None,
        error: lsp_server::ResponseError {
            code: lsp_server::ErrorCode::MethodNotFound as i32,
            data: None,
            message: format!(
                "No handler registered for method '{}'",
                request.method
            ),
        }
        .wrap_some(),
    }
}

fn dispatch_lsp_notification<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    notification: lsp_server::Notification,
) -> Vec<IsographEffect> {
    let dispatch = || {
        crate::lsp_dispatch::LspNotificationDispatch::new(notification, state)
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
    event: crate::event::IsographEvent,
) -> Vec<IsographEffect> {
    handle(state, event)
}
```

`dispatch_request` / `dispatch_notification` are isograph `server.rs`. Request leftover is MethodNotFound (same text). Notification leftover is no effects (isograph `ControlFlow::Continue(())`).

This slice the request chain has zero `on_request_sync` calls. Tokens inserts `.on_request_sync::<SemanticTokensFullRequest>(...)?` before `.request()`.

The notification chain has `isograph/event`. `on_isograph_event` calls `handle` with the extracted event (`HelloWorld` / `Quit` / `DiskChanged`). `LspRequest` / `LspNotification` are `#[serde(skip)]`, so they are not a `--file` / `isograph/event` payload. One-level re-entry.

`method_not_found` returns `Response`. `handle` wraps it with `incoming.reply` as `SendLspResponse`. The dispatcher does not hold the reply sender.

`lib.rs`: `mod lsp_dispatch;`

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

`handle` of `LspNotification` whose method is `isograph/event` and params are `{"kind":"Nope"}`: no effects.

`lsp_socket.rs`: `listen_for_events` sees `LspNotification`, not `HelloWorld` / `DiskChanged` / `Quit`. Assert `notification.0.method` is `IsographEventNotification::METHOD` and `serde_json::from_value::<IsographEvent>(notification.0.params)` is that event. Same for the existing HelloWorld / DiskChanged present-then-absent / multi-connection tests.

`bad_event_params_are_not_an_event`: the session posts `LspNotification`. Params do not deserialize as `IsographEvent`. Then `HelloWorld` on the same connection is a second `LspNotification` whose params are `HelloWorld`.

`unknown_notification_is_not_an_event`: `window/logMessage` arrives as `LspNotification`. Then `HelloWorld` as today.

MethodNotFound / shutdown / ServerNotInitialized tests stay. `listen_and_reply` tees non-`LspRequest` events. After hover MethodNotFound, `isograph/event` HelloWorld is `LspNotification` on that tee; the loops still run `handle`.

`ServerNotInitialized` is still `Connection::initialize`. A request before initialize is not `LspRequest`.

`daemon.rs` existing tests stay. They do not send `LspRequest` / `LspNotification`.

`cli.rs` unchanged.

Do not add a production API only tests call.

## Call sites

- `run_session` Request -> `LspRequest` -> `handle` -> `dispatch_lsp_request` -> Continue `method_not_found` or Break `Response` -> `SendLspResponse` -> `perform`
- `run_session` Notification -> `LspNotification` -> `handle` -> `dispatch_lsp_notification` -> `isograph/event` `on_isograph_event` -> `handle` of `HelloWorld` / `Quit` / `DiskChanged`, or leftover empty
- lsp-tokens.md -> `.on_request_sync::<SemanticTokensFullRequest>(semantic_tokens_response)?`
- later didOpen / didChange / didClose -> `.on_notification_sync` on the same chain
