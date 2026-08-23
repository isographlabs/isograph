# LSP request dispatch

Requires lsp-request-response.md. Independent of lsp-sessions.md, filesystem-watcher.md. lsp-tokens.md is the first domain arm.

`handle` of `LspRequest` is `method_not_found`. This slice replaces that call with isograph's `on_request_sync` chain. The Continue arm is `method_not_found`. No new LSP method. Tokens adds one `.on_request_sync` call.

Origin: isograph `crates/isograph_lsp/src/lsp_request_dispatch.rs` and `server.rs` `dispatch_request`. Delta: extract is `Result` (isograph `expect`s); no `LSPRuntimeError`; handler returns `Result<TRequest::Result, lsp_server::ResponseError>`; Break is `IsographEffect::LspRespond`.

One shippable change. Unknown requests are still `MethodNotFound`. Existing MethodNotFound tests stay green.

## What the user does

Same as after lsp-request-response.md.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/lsp_dispatch.rs
use std::ops::ControlFlow;

use lsp_server::ExtractError;
use lsp_types::request::Request;
use prelude::Postfix;
use tracing::warn;

pub struct LspRequestDispatch<'state, TState> {
    incoming: crate::event::LspRequest,
    state: &'state TState,
}

impl<'state, TState> LspRequestDispatch<'state, TState> {
    pub fn new(incoming: crate::event::LspRequest, state: &'state TState) -> Self {
        Self { incoming, state }
    }

    pub fn on_request_sync<TRequest: Request>(
        self,
        handler: fn(&TState, TRequest::Params) -> Result<TRequest::Result, lsp_server::ResponseError>,
    ) -> ControlFlow<crate::effect::IsographEffect, Self> {
        if self.incoming.request.method != TRequest::METHOD {
            return ControlFlow::Continue(self);
        }
        let crate::event::LspRequest { request, reply } = self.incoming;
        let id = request.id.clone();
        let effect = match request.extract::<TRequest::Params>(TRequest::METHOD) {
            Ok((_, params)) => match handler(self.state, params) {
                Ok(result) => match serde_json::to_value(result) {
                    Ok(value) => respond(
                        reply,
                        lsp_server::Response {
                            id,
                            result: value.wrap_some(),
                            error: None,
                        },
                    ),
                    Err(e) => {
                        warn!(error = %e, "could not encode request result");
                        respond(
                            reply,
                            lsp_server::Response::new_err(
                                id,
                                lsp_server::ErrorCode::InternalError as i32,
                                "could not encode request result".to_owned(),
                            ),
                        )
                    }
                },
                Err(error) => respond(
                    reply,
                    lsp_server::Response {
                        id,
                        result: None,
                        error: error.wrap_some(),
                    },
                ),
            },
            Err(ExtractError::MethodMismatch(request)) => {
                return ControlFlow::Continue(Self {
                    incoming: crate::event::LspRequest { request, reply },
                    state: self.state,
                });
            }
            Err(ExtractError::JsonError { method, error }) => {
                warn!(method = method.as_str(), error = %error, "request params");
                respond(
                    reply,
                    lsp_server::Response::new_err(
                        id,
                        lsp_server::ErrorCode::InvalidParams as i32,
                        "invalid request params".to_owned(),
                    ),
                )
            }
        };
        ControlFlow::Break(effect)
    }

    pub fn request(self) -> crate::event::LspRequest {
        self.incoming
    }
}

fn respond(
    reply: crossbeam::channel::Sender<lsp_server::Message>,
    response: lsp_server::Response,
) -> crate::effect::IsographEffect {
    crate::effect::IsographEffect::LspRespond(crate::effect::LspRespond { reply, response })
}
```

After the method-string check, `extract` returning `MethodMismatch` is restore-and-Continue. Do not `unwrap`.

```rust
// from crates/isograph_cli/src/state.rs
        IsographEvent::LspRequest(request) => dispatch_lsp_request(state, request).wrap_vec(),

fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &IsographState<THostLanguage>,
    request: crate::event::LspRequest,
) -> IsographEffect {
    let get_effect = || {
        let request = crate::lsp_dispatch::LspRequestDispatch::new(request, state).request();
        ControlFlow::Continue(request)
    };
    match get_effect() {
        ControlFlow::Break(effect) => effect,
        ControlFlow::Continue(request) => method_not_found(request),
    }
}
```

This slice the chain has zero `on_request_sync` calls. `method_not_found` stays the Continue arm (one source of that error text). Tokens inserts `.on_request_sync::<SemanticTokensFullRequest>(...)?` before `.request()`.

`lib.rs`: `mod lsp_dispatch;`

## Tests

`state.rs`: `handle` of `LspRequest` (hover) is still `LspRespond` / `MethodNotFound`. Same assertion as lsp-request-response.md. Do not add a tests-only handler.

`lsp_socket.rs` MethodNotFound tests stay.

## Call sites

- `handle` `LspRequest` -> `LspRequestDispatch` -> Continue `method_not_found` or Break `LspRespond`
- lsp-tokens.md -> `.on_request_sync::<SemanticTokensFullRequest>(semantic_tokens_response)?`
