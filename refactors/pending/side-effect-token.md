# Side-effect token and take_reply

Requires lsp-outstanding.md. Independent of lsp-tokens.md and timer-effects.md.

`LiveSessions::reply(client)` yields a send capability for that connection, not for a request id. `handle` can still respond for an id that was cancelled, already answered, or never inserted. The token only gates *when* the send runs.

This slice puts outstanding ids on `LiveSession`. The only way to obtain a send `FnOnce` is `take_reply(client, id)`, which removes that id. Missing id is `None`. `Gone` drops the session, so the ids go with it. The `FnOnce` takes `&SideEffectToken`. `run_effect_loop` mints the token and passes it to `perform`. `handle` does not have a token.

`run_event_loop` and `run_effect_loop` share one current-thread tokio runtime (`select!`). There is not a second OS thread. `SideEffectToken` is a capability of `perform`, not thread affinity. It is not a field on `State`. It is not an argument of `handle`.

Origin: outstanding.md `LiveSessions::reply` / `send_lsp_response`; landed `LiveSession` / `SendLspResponseEffect` / `perform`. Delta: `outstanding` on `LiveSession`, `take_reply` replaces `reply`, `SideEffectToken`, `SendLspResponseEffect.send`, `perform(&SideEffectToken, IsographEffect)`.

One shippable change. MethodNotFound bytes do not change.

## What the user does

Same verbs. Hover is still MethodNotFound. A not-live request is still no effects. A second send for the same id, or a send after `Gone` / cancel has removed the id, is no effects.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug)]
pub struct LiveSession {
    pub sender: crossbeam::channel::Sender<lsp_server::Message>,
    pub capabilities: lsp_types::ClientCapabilities,
    pub outstanding: std::collections::HashSet<lsp_server::RequestId>,
}
```

Delta: `outstanding`. `Gone` is still `live_sessions.0.remove(&client)`. `Started` twice replaces the session and drops that client's ids. No second map on `State`.

`run_session` `Started` payload and handle-test dummy sessions gain `outstanding: std::collections::HashSet::new()`. Drop `#[cfg_attr(not(test), expect(dead_code))]` on `sender`.

```rust
// from crates/isograph_cli/src/effect.rs
pub struct SideEffectToken {
    _not_send: std::marker::PhantomData<*const ()>,
}

impl SideEffectToken {
    pub(crate) fn mint() -> Self {
        Self {
            _not_send: std::marker::PhantomData,
        }
    }
}
```

`!Send` `!Sync`. The private field is not constructible outside this module. `mint` is `pub(crate)` because `run_effect_loop` and tests that call `perform` are in this crate. `handle` does not call `mint`.

```rust
// from crates/isograph_cli/src/event.rs
impl LiveSessions {
    pub fn take_reply(
        &mut self,
        client: ClientId,
        id: lsp_server::RequestId,
    ) -> Option<impl FnOnce(&crate::effect::SideEffectToken, lsp_server::Message) + Send> {
        let session = self.0.get_mut(&client)?;
        if !session.outstanding.remove(&id) {
            return None;
        }
        let sender = session.sender.clone();
        (move |_token, message| {
            let _ = sender.send(message);
        })
        .wrap_some()
    }
}
```

Delta: replaces `reply`. No `FnOnce` unless that id was in the session's set. Later cancel removes the id and does not send. Later async always calls `send_lsp_response`; `None` is no effects.

```rust
// from crates/isograph_cli/src/effect.rs
pub struct SendLspResponseEffect {
    pub send: Box<dyn FnOnce(&SideEffectToken) + Send>,
}

impl std::fmt::Debug for SendLspResponseEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SendLspResponseEffect")
    }
}

#[derive(Debug)]
pub enum IsographEffect {
    LogHelloWorld,
    Kill,
    SendLspResponse(Box<SendLspResponseEffect>),
}
```

Delta: no `reply`, no `response` on the struct. Those are captured in `send`. Manual `Debug`; `FnOnce` has none. `IsographEffect` still derives `Debug`.

```rust
// from crates/isograph_cli/src/state.rs
        crate::event::LspEvent::Request(incoming) => {
            let client = incoming.client;
            let id = incoming.request.id.clone();
            {
                let Some(session) = state.live_sessions.0.get_mut(&client) else {
                    return Vec::new();
                };
                session.outstanding.insert(id.clone());
            }
            dispatch_lsp_request(state, incoming, id)
        }
```

```rust
fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
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
    state: &mut State<THostLanguage>,
    client: crate::event::ClientId,
    id: lsp_server::RequestId,
    response: lsp_server::Response,
) -> Vec<IsographEffect> {
    let Some(send) = state.live_sessions.take_reply(client, id) else {
        return Vec::new();
    };
    crate::effect::IsographEffect::SendLspResponse(
        crate::effect::SendLspResponseEffect {
            send: (move |token| send(token, lsp_server::Message::Response(response))).boxed(),
        }
        .boxed(),
    )
    .wrap_vec()
}

fn method_not_found<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
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

Delta: `dispatch_lsp` of `Request` `get_mut`s, inserts the id, and only then calls `dispatch_lsp_request`. `dispatch_lsp_request` always runs `get_response`. `send_lsp_response` always `take_reply`s. `None` is no effects. No `UnexpectedError`. Later readers call `send_lsp_response` the same way.

`get_response` uses `&state.db`. The `get_mut` borrow of `live_sessions` ends before that.

```rust
// from crates/isograph_cli/src/daemon.rs
pub(crate) async fn run_effect_loop(mut effect_rx: UnboundedReceiver<IsographEffect>) {
    let token = crate::effect::SideEffectToken::mint();
    while let Some(effect) = effect_rx.recv().await {
        tracing::debug!(?effect, "effect");
        if perform(&token, effect).is_break() {
            break;
        }
    }
}

pub fn perform(token: &crate::effect::SideEffectToken, effect: IsographEffect) -> ControlFlow<()> {
    match effect {
        IsographEffect::LogHelloWorld => {
            tracing::info!("hello world");
            ControlFlow::Continue(())
        }
        IsographEffect::Kill => {
            tracing::info!("kill: exiting");
            ControlFlow::Break(())
        }
        IsographEffect::SendLspResponse(respond) => {
            (respond.send)(token);
            ControlFlow::Continue(())
        }
    }
}
```

Delta: one token for the loop. `perform` takes `&SideEffectToken`. `run_event_loop` is unchanged.

HashSet `insert` of an id that is already present collapses. Sequential same-id requests in this slice take before the next insert. Two in-flight requests with the same id are not distinguished.

## Tests

`handle` of hover for a live client: MethodNotFound bytes unchanged. `perform(&SideEffectToken::mint(), effect)`: dummy receiver gets the response. That client's `outstanding` is empty.

`handle` of `Formatting` for that live client: `result` is `[]`. `outstanding` empty.

Never `Started`, `Started(A)` then request from `B`, `Started` then `Gone` then request, `Formatting` from a not-live client: empty effects. No `perform`.

`LiveSessions::take_reply` of a live client whose set does not contain `id`: `None`. After `insert` of that `id`, `take_reply` is `Some`; a second `take_reply` of the same id is `None`.

`Started` twice for the same `ClientId`: one row, the second session, outstanding empty.

`run_effect_loop` still ends on `Kill`. `run_event_loop` HelloWorld still emits `LogHelloWorld`.

Do not add a production API only tests call. `mint` is the same function `run_effect_loop` uses.

## Call sites

- `run_session` / dummy `LiveSession` -> `outstanding: HashSet::new()`
- `dispatch_lsp` of `Request` -> `get_mut` insert; no `dispatch_lsp_request` if not live
- `dispatch_lsp_request` -> `get_response`; then `send_lsp_response`
- `send_lsp_response` -> `take_reply`; `None` is no effects
- later cancel -> `session.outstanding.remove(&id)` only
- later async reply -> `send_lsp_response` (always)
- `run_effect_loop` -> `SideEffectToken::mint`, `perform(&token, effect)`
- `perform` of `SendLspResponse` -> `(respond.send)(token)`
- handle tests that call `perform` -> `perform(&SideEffectToken::mint(), effect)`
- later effects that send, write, or log from a captured capability -> the same `FnOnce(&SideEffectToken)` shape
