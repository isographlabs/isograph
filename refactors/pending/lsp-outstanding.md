# Outstanding LSP requests

Requires lsp-request-response.md and lsp-sessions.md (both landed). Independent of lsp-tokens.md.

`run_event_loop` is recv, `handle`, send effects. It does not match on the event. It does not match on the effect. Outstanding lives on CLI `State` next to `live_sessions`. `handle` writes it.

JSON-RPC ids are per connection. Two clients can both use id `1`. Keys are `(ClientId, RequestId)`.

A request whose `client` is not in `live_sessions` returns no effects. A live client's request inserts `id` under that client, then this slice's handlers still reply in the same turn and `send_lsp_response` removes that id. `remove` returning false is `LSPRuntimeError::UnexpectedError`: `send_lsp_response` runs only after that insert in the same `handle`. `handle_client_connection` of `Gone` removes the session and that client's outstanding map entry. Cancel and async replies are later readers of the same map: they do not call `send_lsp_response` if the id is already gone.

Origin: landed `LspRequest` / `SendLspResponseEffect`; sessions.md `ClientId` / `LiveSessions` / `handle_client_connection` / CLI `State`. Delta: `client` on `LspRequest`, `OutstandingLspRequests` on `State`, `dispatch_lsp_request` and `handle_client_connection` of `Gone` write the map. `SendLspResponseEffect` is unchanged (`response.id` is the request id). `run_event_loop` is unchanged. `perform` is unchanged.

One shippable change. MethodNotFound bytes do not change.

## What the user does

Same verbs. A request handled after that client is `Gone` does not write a response. The session posts every request before `Gone`, so the production channel does not deliver that order; `handle` does not depend on it.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
pub struct LspRequest {
    pub client: ClientId,
    pub request: lsp_server::Request,
    pub reply: crossbeam::channel::Sender<Message>,
}
```

Delta: `client`. `run_session` already has `client` (sessions.md). Put it on every `Lsp::Request`. Notifications stay a bare `lsp_server::Notification`.

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Debug, Default)]
pub struct OutstandingLspRequests(
    pub std::collections::HashMap<ClientId, std::collections::HashSet<lsp_server::RequestId>>,
);
```

```rust
// from crates/isograph_cli/src/state.rs
pub struct State<THostLanguage: HostLanguage> {
    pub db: IsographState<THostLanguage>,
    pub live_sessions: LiveSessions,
    pub outstanding: OutstandingLspRequests,
}
```

Delta: `outstanding`. `serve`, `empty`, `empty_state`, `listen_and_reply`, and the watch `Harness` pass `OutstandingLspRequests::default()`.

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

fn handle_client_connection<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
    event: ClientConnectionEvent,
) -> Vec<IsographEffect> {
    match event {
        ClientConnectionEvent::Started(started) => {
            state
                .live_sessions
                .0
                .insert(started.client, started.session);
            Vec::new()
        }
        ClientConnectionEvent::Gone(client) => {
            state.live_sessions.0.remove(&client);
            state.outstanding.0.remove(&client);
            Vec::new()
        }
    }
}

fn dispatch_lsp<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
    lsp: crate::event::LspEvent,
) -> Vec<IsographEffect> {
    match lsp {
        crate::event::LspEvent::Request(incoming) => dispatch_lsp_request(state, incoming),
        crate::event::LspEvent::Notification(notification) => {
            dispatch_lsp_notification(&mut state.db, notification)
        }
        crate::event::LspEvent::Response(_response) => Vec::new(),
    }
}

fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
    incoming: crate::event::LspRequest,
) -> Vec<IsographEffect> {
    let crate::event::LspRequest {
        client,
        request,
        reply,
    } = incoming;
    if !state.live_sessions.0.contains_key(&client) {
        return Vec::new();
    }
    let id = request.id.clone();
    state.outstanding.0.entry(client).or_default().insert(id.clone());
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
        ControlFlow::Break(response) => send_lsp_response(state, client, id, reply, response),
        ControlFlow::Continue(request) => method_not_found(
            state,
            crate::event::LspRequest {
                client,
                request,
                reply,
            },
        ),
    }
}

fn send_lsp_response<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
    client: crate::event::ClientId,
    id: lsp_server::RequestId,
    reply: crossbeam::channel::Sender<lsp_server::Message>,
    response: lsp_server::Response,
) -> Vec<IsographEffect> {
    let present = match state.outstanding.0.entry(client) {
        std::collections::hash_map::Entry::Occupied(mut occupied) => {
            let present = occupied.get_mut().remove(&id);
            if occupied.get().is_empty() {
                occupied.remove();
            }
            present
        }
        std::collections::hash_map::Entry::Vacant(_) => false,
    };
    let response = if present {
        response
    } else {
        lsp_server::Response {
            id,
            result: None,
            error: isograph_lsp::lsp_runtime_error::LSPRuntimeError::UnexpectedError(
                "send_lsp_response of a pair that was not outstanding".to_owned(),
            )
            .to(),
        }
    };
    crate::effect::IsographEffect::SendLspResponse(
        crate::effect::SendLspResponseEffect { reply, response }.boxed(),
    )
    .wrap_vec()
}

fn method_not_found<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
    incoming: crate::event::LspRequest,
) -> Vec<IsographEffect> {
    let id = incoming.request.id.clone();
    send_lsp_response(
        state,
        incoming.client,
        id,
        incoming.reply,
        lsp_server::Response {
            id: incoming.request.id,
            result: None,
            error: lsp_server::ResponseError {
                code: lsp_server::ErrorCode::MethodNotFound as i32,
                data: None,
                message: format!(
                    "No handler registered for method '{}'",
                    incoming.request.method
                ),
            }
            .wrap_some(),
        },
    )
}
```

Delta: `dispatch_lsp` takes `&mut State`. `dispatch_lsp_request` takes `&mut State` and `&state.db` into `LSPRequestDispatch`. A request whose client is not live returns no effects and does not insert. A live request inserts the id under that client, then `send_lsp_response` removes it and returns `SendLspResponseEffect`. `remove` of a missing id is `LSPRuntimeError::UnexpectedError` (LSP error response), not a panic. `handle_client_connection` of `Gone` also `remove`s that client from outstanding. `dispatch_lsp_notification` / `handle_internal` still take `&mut IsographState`.

`SendLspResponseEffect` does not gain `client`. `perform` still sends on `reply`.

```rust
// from crates/isograph_cli/src/daemon.rs
pub(crate) async fn run_event_loop<THostLanguage: HostLanguage>(
    mut state: crate::state::State<THostLanguage>,
    mut event_rx: UnboundedReceiver<IsographEvent>,
    effect_tx: UnboundedSender<IsographEffect>,
) {
    while let Some(event) = event_rx.recv().await {
        tracing::debug!(?event, "event");
        let effects = handle(&mut state, event);
        for effect in effects {
            let _ = effect_tx.send(effect);
        }
    }
}
```

Delta: none.

### Session

```rust
// from crates/isograph_cli/src/lsp_socket.rs
            lsp_server::Message::Request(request) => {
                crate::event::LspEvent::Request(crate::event::LspRequest {
                    client,
                    request,
                    reply: connection.sender.clone(),
                })
            }
```

Delta: `client` on `LspRequest`.

## Tests

`handle` of `Lsp` request when that `client` is in `live_sessions` (a prior `ClientConnectionEvent::Started` with a dummy sender): one `SendLspResponseEffect`, MethodNotFound bytes unchanged for `textDocument/hover`, `state.outstanding.0` empty.

`handle` of `Lsp` request when that `client` was never `Started`: empty effects, outstanding empty.

`handle` of `Started`, then `Gone`, then `Lsp` request for that `client`: empty effects, outstanding empty.

`handle` of `Gone` when `outstanding` already has `client -> {id}` and `other -> {id}`: `client` is absent from `live_sessions` and from outstanding; `other`'s id remains. Empty effects. Construct `State { outstanding: OutstandingLspRequests(...), .. }` with those two keys; this is not a production API only tests call.

`handle` of `Gone` of an id that was never registered: empty effects, outstanding empty. Same as sessions.md for `live_sessions`.

Existing `lsp_request_returns_method_not_found` `Started`s first.

`lsp_socket.rs` (`listen_for_events`): initialize, then a `textDocument/hover` request. First event `ClientConnectionEvent::Started`. Next event `LspEvent::Request` whose `client` equals that `Started` client. Two connections: two `Started` with different `ClientId`s; each connection's request carries that connection's id.

`listen_and_reply`: hover after initialize is still MethodNotFound. Drop one of two connections; the remaining connection's hover is still MethodNotFound.

## Call sites

- `run_session` -> `client` on `LspRequest`
- `handle` of `Lsp` request -> insert id under client / `send_lsp_response` remove
- `handle_client_connection` of `Gone` -> `live_sessions` remove and outstanding remove
- `serve` / `empty` / `empty_state` / `listen_and_reply` / watch `Harness` -> `State { .., outstanding }`
- later cancel / async reply -> if the pair is still in `state.outstanding`, then `send_lsp_response`; if `Gone` or cancel already dropped it, do not call `send_lsp_response`
- lsp-open-files.md -> `client` already on `LspRequest`; notifications still need `client`
