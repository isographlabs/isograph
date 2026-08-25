# Reply function at insert

Requires side-effect-token.md (landed). Independent of timer-effects.md and lsp-open-files.md.

Starting a request inserts its id into that client's `outstanding` and returns a `FnOnce` that closes over that id and `client`. Calling the `FnOnce` later writes that id onto the `Response` and sends it on that client's `sender`. If the session is gone, the `FnOnce` returns no effects.

`take_reply` of a missing id in the same handle as the insert is a representable invalid state. This slice deletes `take_reply`. `None` from `reply` is only a client that is not live. `None` from the `FnOnce` is only a sender that has been dropped.

Origin: side-effect-token.md `LiveSessions::take_reply` / insert-then-`dispatch_lsp_request(..., id)` / `send_lsp_response` `take_reply`. Landed `LiveSession.outstanding` / `SendLspResponseEffect.send` / `SideEffectToken`. Delta: `reply` replaces `take_reply` and the insert block, the `FnOnce` closes over `id` and assigns `response.id`, `dispatch_lsp_request` takes the `FnOnce` not the id.

One shippable change. MethodNotFound bytes do not change.

## What the user does

Same verbs. Hover is still MethodNotFound. Live `Formatting` is still `[]`. A not-live request is still no effects. A callback invoked after that client is `Gone` is no effects.

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
        impl for<'a> FnOnce(
                &'a mut LiveSessions,
                lsp_server::Response,
            ) -> Vec<crate::effect::IsographEffect>
            + Send
            + use<>,
    > {
        let session = self.0.get_mut(&client)?;
        session.outstanding.insert(id.clone());
        (move |sessions: &mut LiveSessions, mut response: lsp_server::Response| {
            let Some(session) = sessions.0.get_mut(&client) else {
                return crate::effect::no_effects();
            };
            session.outstanding.remove(&id);
            response.id = id;
            let sender = session.sender.clone();
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

Delta: replaces `take_reply`. `None` is only `get_mut` miss at start (that client is not live). The `FnOnce` closes over `client` and `id`. It does not clone `sender` at start. After `reply` returns, `id` is in `outstanding`.

Calling the `FnOnce`: no session is no effects (the sender was dropped). A live session removes `id`, assigns `response.id` from the closed-over id, clones `sender`, and builds one `SendLspResponseEffect`. That assignment is when the id is put on the `Response` this process sends. Calling the `FnOnce` twice does not compile. `use<>` so the `impl Trait` does not capture `&mut self`.

`get_response` still builds a `Response` through `LSPRequestDispatch` (`convert_to_lsp_response` copies `request.id` there). The callback overwrites `response.id` with the id it closed over. `method_not_found` still fills `result` / `error`; it does not own the id on the wire.

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

```rust
// from crates/isograph_cli/src/state.rs
fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
    incoming: crate::event::LspRequest,
    reply: impl FnOnce(&mut crate::event::LiveSessions, lsp_server::Response) -> Vec<IsographEffect>,
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
        ControlFlow::Break(response) => {
            send_lsp_response(&mut state.live_sessions, reply, response)
        }
        ControlFlow::Continue(request) => {
            method_not_found(&mut state.live_sessions, reply, request)
        }
    }
}

fn send_lsp_response(
    sessions: &mut crate::event::LiveSessions,
    reply: impl FnOnce(&mut crate::event::LiveSessions, lsp_server::Response) -> Vec<IsographEffect>,
    response: lsp_server::Response,
) -> Vec<IsographEffect> {
    reply(sessions, response)
}

fn method_not_found(
    sessions: &mut crate::event::LiveSessions,
    reply: impl FnOnce(&mut crate::event::LiveSessions, lsp_server::Response) -> Vec<IsographEffect>,
    request: lsp_server::Request,
) -> Vec<IsographEffect> {
    send_lsp_response(
        sessions,
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

Delta: `dispatch_lsp` of `Request` calls `LiveSessions::reply`. `None` does not call `dispatch_lsp_request`. `dispatch_lsp_request` takes `&mut State` and the `FnOnce`. It does not take `id`. It always runs `get_response`. `send_lsp_response` calls the `FnOnce` with `live_sessions`. No `take_reply`. `method_not_found` still builds the MethodNotFound error body. The callback overwrites `response.id`. `handle_client_connection` is unchanged. `SendLspResponseEffect` is unchanged. `perform` is unchanged. `run_event_loop` is unchanged. `run_effect_loop` is unchanged.

The `get_mut` borrow of `live_sessions` in `reply` ends before `dispatch_lsp_request`. `get_response` borrows `state.db` and ends before `send_lsp_response` mutably borrows `live_sessions`.

## Tests

`handle` of hover for a live client: MethodNotFound bytes unchanged. The dummy receiver's `Response.id` equals the request id. `perform(&SideEffectToken::mint(), effect)`: dummy receiver gets the response. That client's `outstanding` is empty.

`handle` of `Formatting` for that live client: `result` is `[]`. `Response.id` equals the request id. `outstanding` empty.

Never `Started`, `Started(A)` then request from `B`, `Started` then `Gone` then request, `Formatting` from a not-live client: empty effects. No `perform`.

`LiveSessions::reply` of a client that is not in the map: `None`.

`LiveSessions::reply` of a live client: `Some`. After `reply`, that id is in `outstanding`. Invoking the `FnOnce` with the `LiveSessions` and a `Response`: one `SendLspResponseEffect`, `outstanding` empty, `perform` delivers a `Response` whose `id` is the id passed to `reply` (not whatever `id` the `Response` argument carried).

`LiveSessions::reply` of a live client, then remove that client, then invoke the `FnOnce`: no effects. The dummy receiver gets nothing.

Drop `take_reply_of_an_id_that_is_not_outstanding_is_none` and `take_reply_of_an_outstanding_id_is_some_once`.

`Started` twice for the same `ClientId`: one row, the second session, outstanding empty.

`run_effect_loop` still ends on `Kill`. `run_event_loop` HelloWorld still emits `LogHelloWorld`.

Do not add a production API only tests call. `reply` is the same function `dispatch_lsp` uses. `mint` is the same function `run_effect_loop` uses.

## Call sites

- `dispatch_lsp` of `Request` -> `LiveSessions::reply`; `None` does not call `dispatch_lsp_request`
- `dispatch_lsp_request` -> `get_response`; then `send_lsp_response(sessions, reply, response)`
- `send_lsp_response` -> `reply(sessions, response)`
- the `FnOnce` -> `response.id = id` (the closed-over id); clone `sender` if the session is live; else no effects
- `method_not_found` -> `send_lsp_response(sessions, reply, Response { error: MethodNotFound, ... })`
- later cancel -> `outstanding.remove` and drop the `FnOnce` without calling it
- later async reply -> hold the `FnOnce` from `reply`; `send_lsp_response(sessions, reply, response)`
- existing `take_reply` tests -> `reply` of not-live is `None`; `reply` of live leaves the id in `outstanding` until the `FnOnce` is invoked; invoke after the session is gone is no effects
