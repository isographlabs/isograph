# Reply function at insert

Requires side-effect-token.md (landed). Independent of timer-effects.md and lsp-open-files.md.

`LiveSessions::reply(client, id)` inserts the id and returns `Option<FnOnce(serde_json::Value) -> FnOnce(&SideEffectToken)>`. `dispatch_lsp` of `Request` checks that `Option` before `dispatch_lsp_request`. `None` is no effects; `get_response` does not run. `Some(reply)` closes over that id and that session's `sender`. `ControlFlow::Break` with `error: None` and `result: Some(value)` calls `reply(value)`. That callback builds `Message::Response(Response { id, result: Some(value), error: None })` and returns the send. `send_lsp_response` wraps it as `SendLspResponseEffect`. `take_reply` is gone.

Origin: landed `take_reply` / insert-then-`dispatch_lsp_request(..., id)` / `send_lsp_response` `take_reply`. Delta: `reply` replaces `take_reply`; the `Option` is checked before `get_response`; the outer `FnOnce` takes the `Break` JSON value, not a `Message` or `Response`.

One shippable change.

The inner `FnOnce` takes `&SideEffectToken`. `perform` has one token for the loop. Nested `impl FnOnce` returning `impl FnOnce` is not a representable return type; the inner function is `Box<dyn FnOnce(&SideEffectToken) + Send>`, which is `SendLspResponseEffect.send`.

## What the user does

Same verbs. Live `Formatting` is still `[]`. A not-live request is still no effects.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
// before
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
// after
impl LiveSessions {
    pub fn reply(
        &mut self,
        client: ClientId,
        id: lsp_server::RequestId,
    ) -> Option<
        impl FnOnce(serde_json::Value) -> Box<dyn FnOnce(&crate::effect::SideEffectToken) + Send>
            + Send
            + use<>,
    > {
        let session = self.0.get_mut(&client)?;
        session.outstanding.insert(id.clone());
        let sender = session.sender.clone();
        (move |value| {
            let message = lsp_server::Message::Response(lsp_server::Response {
                id,
                result: value.wrap_some(),
                error: None,
            });
            (move |_token: &crate::effect::SideEffectToken| {
                let _ = sender.send(message);
            })
            .boxed()
        })
        .wrap_some()
    }
}
```

Delta: `reply` replaces `take_reply`. `None` is only `get_mut` miss. Insert stays. The outer `FnOnce` closes over `sender` and `id`. It takes the `Break` JSON value, which has no id. It creates `Message::Response` with the closed-over id. That is when the id is put on the message this process sends. The inner `FnOnce` sends that message. Neither looks up `LiveSessions`. Calling either twice does not compile. `use<>` so the `impl Trait` does not capture `&mut self`.

```rust
// from crates/isograph_cli/src/state.rs
// before
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
// after
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
// before
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
// after
fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &IsographState<THostLanguage>,
    incoming: crate::event::LspRequest,
    reply: impl FnOnce(serde_json::Value) -> Box<dyn FnOnce(&crate::effect::SideEffectToken) + Send>,
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
        ControlFlow::Break(lsp_server::Response {
            result: Some(value),
            error: None,
            id: _,
        }) => send_lsp_response(reply, value),
        ControlFlow::Continue(request) => method_not_found(request),
        ControlFlow::Break(_) => no_effects(),
    }
}

fn send_lsp_response(
    reply: impl FnOnce(serde_json::Value) -> Box<dyn FnOnce(&crate::effect::SideEffectToken) + Send>,
    value: serde_json::Value,
) -> Vec<IsographEffect> {
    crate::effect::IsographEffect::SendLspResponse(
        crate::effect::SendLspResponseEffect {
            send: reply(value),
        }
        .boxed(),
    )
    .wrap_vec()
}
```

Delta: `dispatch_lsp_request` takes `&IsographState` and the outer `FnOnce`. It does not take `id`. It always runs `get_response`. The `Break` arm that is `error: None` and `result: Some(value)` calls `send_lsp_response(reply, value)`. `send_lsp_response` is `reply(value)` then `SendLspResponseEffect { send }`. No `take_reply`. This callback does not write `Response.error`. `handle_client_connection` is unchanged. `SendLspResponseEffect` is unchanged. The `SendLspResponse` arm of `perform` stays `(respond.send)(token)`. timer-effects.md adds `event_tx` to `perform` and `run_effect_loop`; this slice passes that sender through. `run_event_loop` is unchanged.

The `get_mut` borrow of `live_sessions` in `reply` ends before `dispatch_lsp_request`. `get_response` uses `&state.db`.

`ControlFlow::Continue` is leftover / MethodNotFound. `ControlFlow::Break` with `error: Some(_)` is `on_request_sync` of bad params or `UnexpectedError`. This callback does not send those.

## Tests

`handle` of `Formatting` for a live client: JSON `[]`. `Response.id` equals the request id. `error` is `None`. `perform(&SideEffectToken::mint(), effect)`: dummy receiver gets the response.

Never `Started`, `Started(A)` then request from `B`, `Started` then `Gone` then request, `Formatting` from a not-live client: empty effects. No `perform`. `get_response` does not run (no `SendLspResponseEffect`).

`LiveSessions::reply` of a client that is not in the map: `None`.

`LiveSessions::reply` of a live client: `Some`. After `reply`, that id is in `outstanding`. Invoking the outer `FnOnce` with `serde_json::json!([])`, wrapping as `SendLspResponseEffect`, `perform`: delivered `Response.id` is the id passed to `reply`, `result` is `[]`, `error` is `None`.

Drop `take_reply_of_an_id_that_is_not_outstanding_is_none` and `take_reply_of_an_outstanding_id_is_some_once`.

`Started` twice for the same `ClientId`: one row, the second session, outstanding empty.

`run_effect_loop` still ends on `Kill`. After timer-effects.md, `run_event_loop` HelloWorld emits `Timer`, not `LogHelloWorld`. This slice does not change that.

Do not add a production API only tests call. `reply` is the same function `dispatch_lsp` uses. `mint` is the same function `run_effect_loop` uses.

## Call sites

- `dispatch_lsp` of `Request` -> `LiveSessions::reply`; `None` does not call `dispatch_lsp_request` (no `get_response`)
- `dispatch_lsp_request` -> `get_response`; `Break` with `Some(value)` and `error: None` -> `send_lsp_response(reply, value)`
- `send_lsp_response` -> `SendLspResponseEffect { send: reply(value) }`
- outer `FnOnce` -> create `Message::Response(Response { id, result: Some(value), error: None })`; return inner `FnOnce(&SideEffectToken)`
- `perform` of `SendLspResponse` -> `(respond.send)(token)`; `let _ = sender.send(message)` (dropped receiver is a no-op)
- later cancel -> `outstanding.remove` and drop the outer `FnOnce` without calling it
- later async reply -> hold the outer `FnOnce` from `reply`; `send_lsp_response(reply, value)`
- existing `take_reply` tests -> `reply` of not-live is `None`; `reply` of live, invoke outer with a JSON value, delivered `id` is the one passed to `reply`
