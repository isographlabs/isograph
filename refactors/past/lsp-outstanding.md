# Request client and live-session replies

Requires lsp-request-response.md and lsp-sessions.md (both landed). Independent of lsp-tokens.md.

`LspRequest` carries `reply`, a clone of `connection.sender` minted on the session thread. That clone still sends after `Gone`. This slice puts `client` on `LspRequest`, drops `reply` from the event, and obtains the sender from `LiveSessions::reply` in `dispatch_lsp`. `dispatch_lsp_request` is only called with that sender. It always runs `get_response`.

Origin: landed `LspRequest.reply`; sessions.md `LiveSession.sender` / `handle_client_connection`. Delta: `client` on `LspRequest`, no `reply` on `LspRequest`, `LiveSessions::reply`, `dispatch_lsp` takes `&mut State`, `dispatch_lsp_request` takes the sender.

One shippable change. MethodNotFound bytes do not change.

## What the user does

Same verbs. A request `handle`d after that client is `Gone` does not write a response. The session posts every request before `Gone`, so the production channel does not deliver that order; `handle` does not depend on it.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug)]
pub struct LspRequest {
    pub client: ClientId,
    pub request: lsp_server::Request,
}
```

Delta: `client`; no `reply`. Notifications stay a bare `lsp_server::Notification`.

```rust
// from crates/isograph_cli/src/event.rs
impl LiveSessions {
    pub fn reply(
        &self,
        client: ClientId,
    ) -> Option<crossbeam::channel::Sender<lsp_server::Message>> {
        self.0.get(&client).map(|session| session.sender.clone())
    }
}
```

`Gone` is `live_sessions.0.remove`. That drops the `LiveSession`. `reply` of that `client` is then `None`. `LiveSession.sender` is read here; drop `#[cfg_attr(not(test), expect(dead_code))]` on `sender`. `capabilities` still has it.

```rust
// from crates/isograph_cli/src/state.rs
pub fn handle<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
    event: IsographEvent,
) -> Vec<IsographEffect> {
    match event {
        IsographEvent::Lsp(lsp) => dispatch_lsp(state, lsp),
        IsographEvent::Internal(internal) => handle_internal(&mut state.db, internal),
        IsographEvent::ClientConnection(client) => handle_client_connection(state, client),
    }
}

fn dispatch_lsp<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
    lsp: crate::event::LspEvent,
) -> Vec<IsographEffect> {
    match lsp {
        crate::event::LspEvent::Request(incoming) => {
            let Some(reply) = state.live_sessions.reply(incoming.client) else {
                return Vec::new();
            };
            dispatch_lsp_request(&state.db, incoming, reply)
        }
        crate::event::LspEvent::Notification(notification) => {
            dispatch_lsp_notification(&mut state.db, notification)
        }
        crate::event::LspEvent::Response(_response) => Vec::new(),
    }
}

fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &IsographState<THostLanguage>,
    incoming: crate::event::LspRequest,
    reply: crossbeam::channel::Sender<lsp_server::Message>,
) -> Vec<IsographEffect> {
    let request = incoming.request;
    let get_response = || {
        let request =
            isograph_lsp::lsp_request_dispatch::LSPRequestDispatch::new(request, state)
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
    match get_response() {
        ControlFlow::Break(response) => send_lsp_response(reply, response),
        ControlFlow::Continue(request) => method_not_found(reply, request),
    }
}

fn send_lsp_response(
    reply: crossbeam::channel::Sender<lsp_server::Message>,
    response: lsp_server::Response,
) -> Vec<IsographEffect> {
    crate::effect::IsographEffect::SendLspResponse(
        crate::effect::SendLspResponseEffect { reply, response }.boxed(),
    )
    .wrap_vec()
}

fn method_not_found(
    reply: crossbeam::channel::Sender<lsp_server::Message>,
    request: lsp_server::Request,
) -> Vec<IsographEffect> {
    send_lsp_response(
        reply,
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
        },
    )
}
```

Delta: `handle` of `Lsp` is `dispatch_lsp(state, lsp)`. `dispatch_lsp` of `Request` calls `LiveSessions::reply`. `None` does not call `dispatch_lsp_request`. `Some(reply)` is passed in. `dispatch_lsp_request` always runs `get_response`. `handle_client_connection` is unchanged. `dispatch_lsp_notification` / `handle_internal` still take `&mut IsographState`. `SendLspResponseEffect` still has `reply`. `perform` is unchanged. `run_event_loop` is unchanged.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
            lsp_server::Message::Request(request) => {
                crate::event::LspEvent::Request(crate::event::LspRequest { client, request })
            }
```

Delta: `client`; no `reply`. `run_session` already has `client`.

## Tests

`handle` of `Lsp` request when that `client` is in `live_sessions` (a prior `Started` whose `LiveSession.sender` is a dummy `unbounded` sender): one `SendLspResponseEffect`, MethodNotFound bytes unchanged for `textDocument/hover`. `perform` of that effect: the dummy receiver gets the response.

`handle` of `Formatting` for that same live client: one `SendLspResponseEffect`, `result` is `[]`.

`handle` of `Lsp` request when that `client` was never `Started`: empty effects.

`handle` of `Started` for `A`, then `Lsp` request from `B`: empty effects.

`handle` of `Started`, then `Gone`, then `Lsp` request for that `client`: empty effects. The dummy receiver gets nothing.

`handle` of `Formatting` from a client that is not live: empty effects.

Existing `lsp_request_returns_method_not_found` `Started`s first. The dummy `(tx, rx)` is the `LiveSession.sender`, not a field on `LspRequest`.

`lsp_socket.rs` (`listen_for_events`): initialize, then a `textDocument/hover` request. First event `ClientConnectionEvent::Started`. Next event `LspEvent::Request` whose `client` equals that `Started` client. Two connections: two `Started` with different `ClientId`s; each connection's request carries that connection's id.

`listen_and_reply`: hover after initialize is still MethodNotFound. Drop one of two connections; the remaining connection's hover is still MethodNotFound.

## Call sites

- `run_session` -> `client` on `LspRequest`; no `reply`
- `dispatch_lsp` of `Request` -> `LiveSessions::reply`; `None` does not call `dispatch_lsp_request`
- `dispatch_lsp_request` -> `get_response`; `reply` is a parameter
- `send_lsp_response` -> the `reply` already obtained
- `handle` of `Lsp` -> `dispatch_lsp(state, lsp)`
- existing `LspRequest { request, reply }` constructions (handle tests) -> `Started` plus `LspRequest { client, request }`
- lsp-open-files.md -> `client` already on `LspRequest`; notifications still need `client`
