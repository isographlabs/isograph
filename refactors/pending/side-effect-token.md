# Side-effect token and reply as `FnOnce`

Requires lsp-outstanding.md. Independent of lsp-tokens.md and timer-effects.md.

`LiveSessions::reply` returns a `Sender`. `send_lsp_response` puts that sender on `SendLspResponseEffect`. `handle` could send on it. This slice makes `reply` return an `FnOnce` that takes `&SideEffectToken`. `handle` does not have a token. `run_effect_loop` mints one and passes it to `perform`. The send runs only there.

`run_event_loop` and `run_effect_loop` share one current-thread tokio runtime (`select!`). There is not a second OS thread. `SideEffectToken` is a capability of `perform`, not thread affinity. It is not a field on `State`. It is not an argument of `handle`.

Origin: outstanding.md `LiveSessions::reply` / `send_lsp_response`; landed `SendLspResponseEffect` / `perform`. Delta: `SideEffectToken`, `reply` returns `FnOnce`, `SendLspResponseEffect.send`, `perform(&SideEffectToken, IsographEffect)`.

One shippable change. MethodNotFound bytes do not change.

## What the user does

Same verbs. Hover is still MethodNotFound. A not-live request is still no effects.

## Types

Most important first.

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
    pub fn reply(
        &self,
        client: ClientId,
    ) -> Option<impl FnOnce(&crate::effect::SideEffectToken, lsp_server::Message) + Send>
    {
        let sender = self.0.get(&client)?.sender.clone();
        (move |_token, message| {
            let _ = sender.send(message);
        })
        .wrap_some()
    }
}
```

Delta: the return is an `FnOnce`, not a `Sender`. The token is unused in the body except as proof that `perform` is running.

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
    let Some(send) = state.live_sessions.reply(client) else {
        return Vec::new();
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
        ControlFlow::Break(response) => send_lsp_response(send, response),
        ControlFlow::Continue(request) => method_not_found(send, request),
    }

fn send_lsp_response(
    send: impl FnOnce(&crate::effect::SideEffectToken, lsp_server::Message) + Send + 'static,
    response: lsp_server::Response,
) -> Vec<IsographEffect> {
    crate::effect::IsographEffect::SendLspResponse(
        crate::effect::SendLspResponseEffect {
            send: (move |token| send(token, lsp_server::Message::Response(response))).boxed(),
        }
        .boxed(),
    )
    .wrap_vec()
}

fn method_not_found(
    send: impl FnOnce(&crate::effect::SideEffectToken, lsp_server::Message) + Send + 'static,
    request: lsp_server::Request,
) -> Vec<IsographEffect> {
    send_lsp_response(
        send,
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

Delta: `reply(client)` still runs before `get_response`. `None` returns. `Some(send)` is the `FnOnce`. `send_lsp_response` boxes `|token| send(token, Message::Response(response))`.

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

## Tests

Outstanding.md handle tests that `Started` then hover and `perform`: `perform(&SideEffectToken::mint(), effect)`. The dummy receiver still gets MethodNotFound.

`Formatting` for a live client: same, `result` is `[]`.

Never `Started`, `Started(A)` then request from `B`, `Started` then `Gone` then request, `Formatting` from a not-live client: empty effects. No `perform`.

`run_effect_loop` still ends on `Kill`. `run_event_loop` HelloWorld still emits `LogHelloWorld`.

Do not add a production API only tests call. `mint` is the same function `run_effect_loop` uses.

## Call sites

- `LiveSessions::reply` -> `FnOnce(&SideEffectToken, Message)`
- `send_lsp_response` -> box `|token| send(token, Message::Response(response))`
- `run_effect_loop` -> `SideEffectToken::mint`, `perform(&token, effect)`
- `perform` of `SendLspResponse` -> `(respond.send)(token)`
- handle tests that call `perform` -> `perform(&SideEffectToken::mint(), effect)`
- later effects that send, write, or log from a captured capability -> the same `FnOnce(&SideEffectToken)` shape
