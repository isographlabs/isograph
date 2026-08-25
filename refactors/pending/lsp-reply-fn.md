# Reply function at insert

Requires side-effect-token.md (landed). Independent of timer-effects.md and lsp-open-files.md.

`dispatch_lsp` of `Request` inserts `id` into `outstanding`, then `dispatch_lsp_request` takes that `id`, then `send_lsp_response` `take_reply`s. `take_reply` of a missing id is `None` and no effects. In this handle that `None` cannot happen: the client was just live, `get_response` is synchronous, nothing else removes the id. `None` is a representable invalid state.

`LiveSessions::reply(client, id)` does the insert and returns a `FnOnce` that builds `SendLspResponseEffect`. `None` is only a client that is not live. `dispatch_lsp_request` takes that `FnOnce`, not the id. `send_lsp_response` calls it. There is no second lookup. `take_reply` is gone.

Origin: side-effect-token.md `LiveSessions::take_reply` / insert-then-`dispatch_lsp_request(..., id)` / `send_lsp_response` `take_reply`. Landed `LiveSession.outstanding` / `SendLspResponseEffect.send` / `SideEffectToken`. Delta: `reply` replaces `take_reply` and the insert block, `dispatch_lsp_request` takes the `FnOnce`, `send_lsp_response` has no `Option`.

One shippable change. MethodNotFound bytes do not change. `Response.id` is still copied from the request in `convert_to_lsp_response` and in `method_not_found`.

## What the user does

Same verbs. Hover is still MethodNotFound. Live `Formatting` is still `[]`. A not-live request is still no effects.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
impl LiveSessions {
    pub fn reply(
        &mut self,
        client: ClientId,
        id: lsp_server::RequestId,
    ) -> Option<
        impl FnOnce(lsp_server::Response) -> Vec<crate::effect::IsographEffect> + Send + use<>,
    > {
        let session = self.0.get_mut(&client)?;
        session.outstanding.insert(id);
        let sender = session.sender.clone();
        session.outstanding.remove(&id);
        (move |response: lsp_server::Response| {
            crate::effect::IsographEffect::SendLspResponse(
                crate::effect::SendLspResponseEffect {
                    send: (move |_token: &crate::effect::SideEffectToken| {
                        let _ = sender.send(lsp_server::Message::Response(response));
                    })
                    .boxed(),
                }
                .boxed(),
            )
            .wrap_vec()
        })
        .wrap_some()
    }
}
```

Delta: replaces `take_reply`. `None` is only `get_mut` miss (that client is not live). Insert then remove transfers the outstanding slot into the `FnOnce`. After `reply` returns, that id is not in `outstanding`. The `FnOnce` always builds one `SendLspResponseEffect`. Calling it twice does not compile. `use<>` so the `impl Trait` does not capture `&mut self`.

```rust
// from crates/isograph_cli/src/state.rs
        crate::event::LspEvent::Request(incoming) => {
            let client = incoming.client;
            let id = incoming.request.id.clone();
            let Some(reply) = state.live_sessions.reply(client, id) else {
                return no_effects();
            };
            dispatch_lsp_request(&state.db, incoming, reply)
        }
```

```rust
// from crates/isograph_cli/src/state.rs
fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &IsographState<THostLanguage>,
    incoming: crate::event::LspRequest,
    reply: impl FnOnce(lsp_server::Response) -> Vec<IsographEffect>,
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
    reply: impl FnOnce(lsp_server::Response) -> Vec<IsographEffect>,
    response: lsp_server::Response,
) -> Vec<IsographEffect> {
    reply(response)
}

fn method_not_found(
    reply: impl FnOnce(lsp_server::Response) -> Vec<IsographEffect>,
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

Delta: `dispatch_lsp` of `Request` calls `LiveSessions::reply`. `None` does not call `dispatch_lsp_request`. `dispatch_lsp_request` takes `&IsographState` and the `FnOnce`. It does not take `id`. It always runs `get_response`. `send_lsp_response` calls the `FnOnce`. No `take_reply`. No `Option` on send. `method_not_found` still puts `request.id` on the `Response`. `handle_client_connection` is unchanged. `SendLspResponseEffect` is unchanged. `perform` is unchanged. `run_event_loop` is unchanged. `run_effect_loop` is unchanged.

The `get_mut` borrow of `live_sessions` ends before `dispatch_lsp_request`.

## Tests

`handle` of hover for a live client: MethodNotFound bytes unchanged. `perform(&SideEffectToken::mint(), effect)`: dummy receiver gets the response. That client's `outstanding` is empty.

`handle` of `Formatting` for that live client: `result` is `[]`. `outstanding` empty.

Never `Started`, `Started(A)` then request from `B`, `Started` then `Gone` then request, `Formatting` from a not-live client: empty effects. No `perform`.

`LiveSessions::reply` of a client that is not in the map: `None`.

`LiveSessions::reply` of a live client: `Some`. Invoking that `FnOnce` with a `Response` is one `SendLspResponseEffect`. `perform` of that effect: the dummy receiver gets that `Response`. `outstanding` empty.

Drop `take_reply_of_an_id_that_is_not_outstanding_is_none` and `take_reply_of_an_outstanding_id_is_some_once`.

`Started` twice for the same `ClientId`: one row, the second session, outstanding empty.

`run_effect_loop` still ends on `Kill`. `run_event_loop` HelloWorld still emits `LogHelloWorld`.

Do not add a production API only tests call. `reply` is the same function `dispatch_lsp` uses. `mint` is the same function `run_effect_loop` uses.

## Call sites

- `dispatch_lsp` of `Request` -> `LiveSessions::reply`; `None` does not call `dispatch_lsp_request`
- `dispatch_lsp_request` -> `get_response`; then `send_lsp_response(reply, response)`
- `send_lsp_response` -> `reply(response)`
- `method_not_found` -> `send_lsp_response(reply, Response { id: request.id, ... })`
- later cancel -> drop the `FnOnce`
- later async reply -> hold the `FnOnce` from `reply`; `send_lsp_response(reply, response)`
- existing `take_reply` tests -> `reply` of not-live is `None`; `reply` of live invoked with a `Response` is `SendLspResponseEffect`
