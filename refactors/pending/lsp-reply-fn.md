# Reply function at insert

Requires side-effect-token.md (landed). Independent of timer-effects.md and lsp-open-files.md.

`LiveSessions::reply(client, id)` inserts the id and returns `Option<FnOnce(Response) -> FnOnce(&SideEffectToken)>`. `dispatch_lsp` of `Request` checks that `Option` before `dispatch_lsp_request`. `None` is no effects; `get_response` does not run. `Some(reply)` is a callback that closes over that id and that session's `sender`. After `get_response`, `send_lsp_response` calls `reply(response)` and wraps the inner `FnOnce(&SideEffectToken)` as `SendLspResponseEffect`. The inner function is the send; `perform` runs it with the token. `take_reply` is gone.

Origin: landed `take_reply` / insert-then-`dispatch_lsp_request(..., id)` / `send_lsp_response` `take_reply`. Delta: `reply` replaces `take_reply`; the `Option` is checked before `get_response`; `send_lsp_response` is `reply(response)` then wrap.

One shippable change. MethodNotFound bytes do not change.

The inner `FnOnce` takes `&SideEffectToken`. `perform` has one token for the loop. Nested `impl FnOnce` returning `impl FnOnce` is not a representable return type; the inner function is `Box<dyn FnOnce(&SideEffectToken) + Send>`, which is `SendLspResponseEffect.send`.

## What the user does

Same verbs. Hover is still MethodNotFound. Live `Formatting` is still `[]`. A not-live request is still no effects.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
impl LiveSessions {
    pub fn take_reply(
        &mut self,
        client: ClientId,
        id: lsp_server::RequestId,
    ) -> Option<
        impl for<'a> FnOnce(&'a crate::effect::SideEffectToken, lsp_server::Message) + Send + use<>,
    > {
        let session = self.0.get_mut(&client)?;
        if !session.outstanding.remove(&id) {
            return None;
        }
        let sender = session.sender.clone();
        (move |_token: &crate::effect::SideEffectToken, message| {
            let _ = sender.send(message);
        })
        .wrap_some()
    }
}
```

```rust
// from crates/isograph_cli/src/event.rs
impl LiveSessions {
    pub fn reply(
        &mut self,
        client: ClientId,
        id: lsp_server::RequestId,
    ) -> Option<
        impl FnOnce(
                lsp_server::Response,
            ) -> Box<dyn FnOnce(&crate::effect::SideEffectToken) + Send>
            + Send
            + use<>,
    > {
        let session = self.0.get_mut(&client)?;
        session.outstanding.insert(id.clone());
        let sender = session.sender.clone();
        (move |mut response: lsp_server::Response| {
            response.id = id;
            (move |_token: &crate::effect::SideEffectToken| {
                let _ = sender.send(lsp_server::Message::Response(response));
            })
            .boxed()
        })
        .wrap_some()
    }
}
```

Delta: `reply` replaces `take_reply`. `None` is only `get_mut` miss. Insert stays; remove of the id from `outstanding` does not happen in `reply`. The outer `FnOnce` closes over `sender` and `id`. Calling it assigns `response.id` from the closed-over id and returns the inner send. That assignment is when the id is put on the `Response` this process sends. Neither `FnOnce` looks up `LiveSessions`. Calling either twice does not compile. `use<>` so the `impl Trait` does not capture `&mut self`.

`get_response` still returns a `lsp_server::Response` (`convert_to_lsp_response` fills `id` there because the struct requires it). The outer callback overwrites `response.id`.

```rust
// from crates/isograph_cli/src/state.rs
        crate::event::LspEvent::Request(incoming) => {
            let client = incoming.client;
            let id = incoming.request.id.clone();
            {
                let Some(session) = state.live_sessions.0.get_mut(&client) else {
                    return no_effects();
                };
                session.outstanding.insert(id.clone());
            }
            dispatch_lsp_request(state, incoming, id)
        }
```

```rust
// from crates/isograph_cli/src/state.rs
        crate::event::LspEvent::Request(incoming) => {
            let client = incoming.client;
            let id = incoming.request.id.clone();
            let Some(reply) = state.live_sessions.reply(client, id) else {
                return no_effects();
            };
            dispatch_lsp_request(state, incoming, reply)
        }
```

Delta: insert and the live-session check are `reply`. `None` does not call `dispatch_lsp_request`.

```rust
// from crates/isograph_cli/src/state.rs
fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    incoming: crate::event::LspRequest,
    id: lsp_server::RequestId,
) -> Vec<IsographEffect> {
    let crate::event::LspRequest { client, request } = incoming;
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
    match get_response() {
        ControlFlow::Break(response) => send_lsp_response(state, client, id, response),
        ControlFlow::Continue(request) => method_not_found(state, client, id, request),
    }
}

fn send_lsp_response<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    client: crate::event::ClientId,
    id: lsp_server::RequestId,
    response: lsp_server::Response,
) -> Vec<IsographEffect> {
    let Some(send) = state.live_sessions.take_reply(client, id) else {
        return no_effects();
    };
    crate::effect::IsographEffect::SendLspResponse(
        crate::effect::SendLspResponseEffect {
            send: (move |token: &crate::effect::SideEffectToken| {
                send(token, lsp_server::Message::Response(response))
            })
            .boxed(),
        }
        .boxed(),
    )
    .wrap_vec()
}

fn method_not_found<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    client: crate::event::ClientId,
    id: lsp_server::RequestId,
    request: lsp_server::Request,
) -> Vec<IsographEffect> {
    send_lsp_response(
        state,
        client,
        id,
        lsp_server::Response {
            id: request.id,
            result: None,
            error: lsp_server::ResponseError {
                code: lsp_server::ErrorCode::MethodNotFound as i32,
                data: None,
                message: format!("No handler registered for method '{}'", request.method),
            }
            .wrap_some(),
        },
    )
}
```

```rust
// from crates/isograph_cli/src/state.rs
fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &IsographState<THostLanguage>,
    incoming: crate::event::LspRequest,
    reply: impl FnOnce(
        lsp_server::Response,
    ) -> Box<dyn FnOnce(&crate::effect::SideEffectToken) + Send>,
) -> Vec<IsographEffect> {
    let request = incoming.request;
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
    match get_response() {
        ControlFlow::Break(response) => send_lsp_response(reply, response),
        ControlFlow::Continue(request) => method_not_found(reply, request),
    }
}

fn send_lsp_response(
    reply: impl FnOnce(
        lsp_server::Response,
    ) -> Box<dyn FnOnce(&crate::effect::SideEffectToken) + Send>,
    response: lsp_server::Response,
) -> Vec<IsographEffect> {
    crate::effect::IsographEffect::SendLspResponse(
        crate::effect::SendLspResponseEffect {
            send: reply(response),
        }
        .boxed(),
    )
    .wrap_vec()
}

fn method_not_found(
    reply: impl FnOnce(
        lsp_server::Response,
    ) -> Box<dyn FnOnce(&crate::effect::SideEffectToken) + Send>,
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
                message: format!("No handler registered for method '{}'", request.method),
            }
            .wrap_some(),
        },
    )
}
```

Delta: `dispatch_lsp_request` takes `&IsographState` and the outer `FnOnce`. It does not take `id`. It always runs `get_response`. `send_lsp_response` is `reply(response)` then `SendLspResponseEffect { send }`. No `take_reply`. `method_not_found` still builds the MethodNotFound error body; `id: request.id` is only because `Response` requires the field. The outer callback overwrites it. `handle_client_connection` is unchanged. `SendLspResponseEffect` is unchanged. `perform` is unchanged (`(respond.send)(token)`). `run_event_loop` is unchanged. `run_effect_loop` is unchanged.

The `get_mut` borrow of `live_sessions` in `reply` ends before `dispatch_lsp_request`. `get_response` uses `&state.db`.

## Tests

`handle` of hover for a live client: MethodNotFound bytes unchanged. The dummy receiver's `Response.id` equals the request id. `perform(&SideEffectToken::mint(), effect)`: dummy receiver gets the response.

`handle` of `Formatting` for that live client: `result` is `[]`. `Response.id` equals the request id.

Never `Started`, `Started(A)` then request from `B`, `Started` then `Gone` then request, `Formatting` from a not-live client: empty effects. No `perform`. `get_response` does not run (no `SendLspResponseEffect`).

`LiveSessions::reply` of a client that is not in the map: `None`.

`LiveSessions::reply` of a live client: `Some`. After `reply`, that id is in `outstanding`. Invoking the outer `FnOnce` with a `Response` whose `id` is a different value, wrapping as `SendLspResponseEffect`, `perform`: delivered `Response.id` is the id passed to `reply`.

Drop `take_reply_of_an_id_that_is_not_outstanding_is_none` and `take_reply_of_an_outstanding_id_is_some_once`.

`Started` twice for the same `ClientId`: one row, the second session, outstanding empty.

`run_effect_loop` still ends on `Kill`. `run_event_loop` HelloWorld still emits `LogHelloWorld`.

Do not add a production API only tests call. `reply` is the same function `dispatch_lsp` uses. `mint` is the same function `run_effect_loop` uses.

## Call sites

- `dispatch_lsp` of `Request` -> `LiveSessions::reply`; `None` does not call `dispatch_lsp_request` (no `get_response`)
- `dispatch_lsp_request` -> `get_response`; then `send_lsp_response(reply, response)`
- `send_lsp_response` -> `SendLspResponseEffect { send: reply(response) }`
- outer `FnOnce` -> `response.id = id`; return inner `FnOnce(&SideEffectToken)`
- `perform` of `SendLspResponse` -> `(respond.send)(token)`; `let _ = sender.send(...)` (dropped receiver is a no-op)
- `method_not_found` -> `send_lsp_response(reply, Response { error: MethodNotFound, ... })`
- later cancel -> `outstanding.remove` and drop the outer `FnOnce` without calling it
- later async reply -> hold the outer `FnOnce` from `reply`; `send_lsp_response(reply, response)`
- existing `take_reply` tests -> `reply` of not-live is `None`; `reply` of live, invoke outer with a `Response`, delivered `id` is the one passed to `reply`
