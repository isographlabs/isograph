# Reply function at insert

Requires side-effect-token.md (landed). Independent of lsp-open-files.md.

`LiveSessions::reply(client, id)` inserts the id and returns `Option<BeforeCompute>`. `dispatch_lsp` of `Request` checks that `Option` before `dispatch_lsp_request`. `LiveSessions::before_compute` looks up the session and `contains` the id. `None` skips `get_response`. After `get_response`, `LiveSessions::after_compute` takes `Result<serde_json::Value, lsp_server::ResponseError>`: session still live and `outstanding.remove`? `None` does not send. `Some` creates `Message::Response` with the token's id and returns `FnOnce(&SideEffectToken)`. `send_lsp_response` wraps that as `SendLspResponseEffect`. MethodNotFound and `Break` with `error: Some(_)` also go through `after_compute`. `take_reply` is gone.

`outstanding` is the revoke table for a capability held outside the session. This slice's `handle` holds the tokens on the stack and calls both stages before returning. Later async holds `BeforeCompute` until it is ready to compute, then holds `AfterCompute` until it is ready to send. Later cancel is `outstanding.remove` of that id. `Gone` drops the row. Either stage then returns `None`. `before_compute` uses `contains`, not `remove`, so a cancel between the stages still prevents the send.

Origin: landed `take_reply` / insert-then-`dispatch_lsp_request(..., id)` / `send_lsp_response` `take_reply`. Delta: `reply` / `before_compute` / `after_compute` replace `take_reply`; two token types; three `Option`s; the body has no id; the message is created in `after_compute`.

One shippable change.

The tokens carry `client` and `id`, not `sender`. Each stage takes `&mut LiveSessions`. Handle already holds that map. A token cannot see `Gone` without a lookup. The send `FnOnce` takes `&SideEffectToken` and is boxed: `Box<dyn FnOnce(&SideEffectToken) + Send>`, which is `SendLspResponseEffect.send`.

## What the user does

Same verbs. Live `Formatting` is still `[]`. Hover is still MethodNotFound. Bad params and `UnexpectedError` still send an error response. A not-live request is still no effects. A stage invoked after that client is `Gone`, or after that id was removed from `outstanding`, is no effects and does not send.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
pub struct BeforeCompute {
    client: ClientId,
    id: lsp_server::RequestId,
}

pub struct AfterCompute {
    client: ClientId,
    id: lsp_server::RequestId,
}
```

Private fields. Not `Clone`. Only `reply` constructs `BeforeCompute`. Only `before_compute` constructs `AfterCompute`.

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
    ) -> Option<BeforeCompute> {
        let session = self.0.get_mut(&client)?;
        session.outstanding.insert(id.clone());
        BeforeCompute { client, id }.wrap_some()
    }

    pub fn before_compute(&mut self, before: BeforeCompute) -> Option<AfterCompute> {
        let session = self.0.get_mut(&before.client)?;
        if !session.outstanding.contains(&before.id) {
            return None;
        }
        AfterCompute {
            client: before.client,
            id: before.id,
        }
        .wrap_some()
    }

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
        (move |_token: &crate::effect::SideEffectToken| {
            let _ = sender.send(message);
        })
        .boxed()
        .wrap_some()
    }
}
```

Delta: `reply` replaces `take_reply`. First `Option`: client is not live. Insert then return `BeforeCompute`. Second `Option`: `before_compute`; session gone or id not in `outstanding` is `None`. Third `Option`: `after_compute`; session gone or id already removed is `None`. A successful third step `remove`s the id, clones `sender`, creates the message with the token's id. That is when the id is put on the message. The send `FnOnce` does not take a `Message`. `Ok` is `result: Some(value)`, `error: None`. `Err` is `result: None`, `error: Some(error)`.

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
            let Some(before_compute) = state.live_sessions.reply(client, id) else {
                return no_effects();
            };
            dispatch_lsp_request(state, incoming, before_compute)
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
    state: &mut IsographState<THostLanguage>,
    incoming: crate::event::LspRequest,
    before: crate::event::BeforeCompute,
) -> Vec<IsographEffect> {
    let request = incoming.request;
    let Some(after) = state.live_sessions.before_compute(before) else {
        return no_effects();
    };
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
            send_lsp_response(&mut state.live_sessions, after, response_body(response))
        }
        ControlFlow::Continue(request) => {
            method_not_found(&mut state.live_sessions, after, request)
        }
    }
}

fn send_lsp_response(
    sessions: &mut crate::event::LiveSessions,
    after: crate::event::AfterCompute,
    body: Result<serde_json::Value, lsp_server::ResponseError>,
) -> Vec<IsographEffect> {
    let Some(send) = sessions.after_compute(after, body) else {
        return no_effects();
    };
    crate::effect::IsographEffect::SendLspResponse(
        crate::effect::SendLspResponseEffect { send }.boxed(),
    )
    .wrap_vec()
}

fn method_not_found(
    sessions: &mut crate::event::LiveSessions,
    after: crate::event::AfterCompute,
    request: lsp_server::Request,
) -> Vec<IsographEffect> {
    send_lsp_response(
        sessions,
        after,
        lsp_server::ResponseError {
            code: lsp_server::ErrorCode::MethodNotFound as i32,
            data: None,
            message: format!("No handler registered for method '{}'", request.method),
        }
        .wrap_err(),
    )
}

fn response_body(
    response: lsp_server::Response,
) -> Result<serde_json::Value, lsp_server::ResponseError> {
    match (response.result, response.error) {
        (Some(value), None) => value.wrap_ok(),
        (_, Some(error)) => error.wrap_err(),
        (None, None) => lsp_server::ResponseError {
            code: lsp_server::ErrorCode::UnknownErrorCode as i32,
            data: None,
            message: "LSP response has neither result nor error".to_owned(),
        }
        .wrap_err(),
    }
}
```

Delta: `dispatch_lsp_request` takes `&mut IsographState` and `BeforeCompute`. It does not take `id`. It calls `before_compute` before `get_response`. Every `get_response` outcome that replies calls `after_compute` (ok `Break`, error `Break`, MethodNotFound). `send_lsp_response` calls `after_compute` and wraps `Some(send)`. `None` is no send. No `take_reply`. `handle_client_connection` is unchanged. `SendLspResponseEffect` is unchanged. `perform` is unchanged (`(respond.send)(token)`). `run_event_loop` is unchanged. `run_effect_loop` is unchanged.

`ControlFlow::Continue` is leftover / MethodNotFound. `ControlFlow::Break` with `error: Some(_)` is `on_request_sync` of bad params or `UnexpectedError`. `ExpectedError` is `result: Some(Null)`, `error: None`, and is `Ok`. `(None, None)` is not produced by `convert_to_lsp_response`; `response_body` still replies so the reservation is consumed.

This slice's `dispatch_lsp_request` calls both stages in one `handle`. Later async is a different caller: hold `BeforeCompute` across `handle` returns until compute, hold `AfterCompute` until send. `Gone` or `outstanding.remove` in between either call is `None`. Dropping a token without calling the next stage leaves the id in `outstanding` until `Gone` or later cancel of that id.

## Tests

`handle` of hover for a live client: MethodNotFound bytes unchanged. `Response.id` equals the request id. `perform` of that effect (`mint`, same `event_tx` as `run_effect_loop`): dummy receiver gets the response. That client's `outstanding` is empty.

`handle` of `Formatting` for a live client: JSON `[]`. `Response.id` equals the request id. `error` is `None`. `perform` of that effect: dummy receiver gets the response. That client's `outstanding` is empty.

`handle` of `Formatting` for a live client with `{}` params: one `SendLspResponseEffect`. `result` is `None`. `error` is `Some`. `Response.id` equals the request id. That client's `outstanding` is empty.

Never `Started`, `Started(A)` then request from `B`, `Started` then `Gone` then request, `Formatting` from a not-live client: empty effects. No `perform`. `get_response` does not run.

`LiveSessions::reply` of a client that is not in the map: `None`.

`LiveSessions::reply` of a live client: `Some`. After `reply`, that id is in `outstanding`. `before_compute` then `after_compute` with `serde_json::json!([]).wrap_ok()`: `outstanding` empty, `perform` delivers `Response.id` equal to the id passed to `reply`.

`LiveSessions::reply` of a live client, then remove that client, then `before_compute`: `None`. No send.

`LiveSessions::reply` of a live client, `before_compute` succeeds, then remove that client, then `after_compute`: `None`. No send.

`LiveSessions::reply` of a live client, then `outstanding.remove` of that id, then `before_compute`: `None`. No send.

`LiveSessions::reply` of a live client, `before_compute` succeeds, then `outstanding.remove` of that id, then `after_compute`: `None`. No send.

Drop `take_reply_of_an_id_that_is_not_outstanding_is_none` and `take_reply_of_an_outstanding_id_is_some_once`.

`Started` twice for the same `ClientId`: one row, the second session, outstanding empty.

`run_effect_loop` still ends on `Kill`. `run_event_loop` HelloWorld still emits `LogHelloWorld`.

Do not add a production API only tests call. `reply` / `before_compute` / `after_compute` are the same functions `dispatch_lsp` uses. `mint` is the same function `run_effect_loop` uses.

## Call sites

- `dispatch_lsp` of `Request` -> `LiveSessions::reply`; `None` does not call `dispatch_lsp_request`
- `dispatch_lsp_request` -> `before_compute`; `None` does not call `get_response`
- `Break` -> `send_lsp_response(sessions, after, response_body(response))`
- `Continue` -> `method_not_found(sessions, after, request)` -> `after_compute` with MethodNotFound
- `send_lsp_response` -> `after_compute`; `None` does not send; `Some(send)` is `SendLspResponseEffect`
- `after_compute` `Some` -> `outstanding.remove`; create `Message::Response` with the token's id
- `perform` of `SendLspResponse` -> `(respond.send)(token)`
- later cancel -> `outstanding.remove` of that id; the next stage is `None`
- later async -> hold `BeforeCompute`, then hold `AfterCompute`, call each stage when that stage runs
