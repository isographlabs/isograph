# Outstanding LSP requests

Requires lsp-request-response.md and lsp-sessions.md (both landed). Independent of lsp-tokens.md.

`run_event_loop` is recv, `handle`, send effects.

`LspRequest` today carries `reply`, a clone of `connection.sender` minted on the session thread. That clone still sends after `Gone`. `contains_key` on `live_sessions` is discipline: the clone is already on the event.

`LiveSession` is the drop guard. It owns the writer. `Gone` drops it. The only way to put a sender on `SendLspResponseEffect` is `LiveSessions::reply` at `handle` time. After `Gone`, `reply` is `None`. `LspRequest` does not carry `reply`.

JSON-RPC ids are per connection. Two clients can both use id `1`. Outstanding is `HashMap<ClientId, HashSet<RequestId>>`. Cancel and async replies are later readers: they call `send_lsp_response` only if that id is still in the map.

Two shippable changes. Change 1 is the prefactor.

## What the user does

Same verbs. A request `handle`d after that client is `Gone` does not write a response: there is no sender. The session posts every request before `Gone`, so the production channel does not deliver that order; `handle` does not depend on it. A request `handle`d while the client is still live still replies; `perform` ignores a failed `reply.send` if the TCP side is already down.

## Change 1: reply from the live session

Origin: landed `LspRequest.reply`; sessions.md `LiveSession.sender` / `handle_client_connection`. Delta: `client` on `LspRequest`, drop `reply` from `LspRequest`, `LiveSessions::reply`, `dispatch_lsp` takes `&mut State`, `send_lsp_response` clones the sender from the map or returns no effects.

### Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
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

`Gone` is `live_sessions.0.remove`. That drops the `LiveSession`, which drops its sender. `reply` of that `client` is then `None`.

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
        crate::event::LspEvent::Request(incoming) => dispatch_lsp_request(state, incoming),
        crate::event::LspEvent::Notification(notification) => {
            dispatch_lsp_notification(&mut state.db, notification)
        }
        crate::event::LspEvent::Response(_response) => Vec::new(),
    }
}

fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &State<THostLanguage>,
    incoming: crate::event::LspRequest,
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
        ControlFlow::Break(response) => send_lsp_response(state, client, response),
        ControlFlow::Continue(request) => method_not_found(
            state,
            crate::event::LspRequest { client, request },
        ),
    }
}

fn send_lsp_response<THostLanguage: HostLanguage>(
    state: &State<THostLanguage>,
    client: crate::event::ClientId,
    response: lsp_server::Response,
) -> Vec<IsographEffect> {
    let Some(reply) = state.live_sessions.reply(client) else {
        return Vec::new();
    };
    crate::effect::IsographEffect::SendLspResponse(
        crate::effect::SendLspResponseEffect { reply, response }.boxed(),
    )
    .wrap_vec()
}

fn method_not_found<THostLanguage: HostLanguage>(
    state: &State<THostLanguage>,
    incoming: crate::event::LspRequest,
) -> Vec<IsographEffect> {
    send_lsp_response(
        state,
        incoming.client,
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

Delta: `handle` of `Lsp` is `dispatch_lsp(state, lsp)`. `send_lsp_response` is the only place a request handler obtains a sender. Missing client is no effects, not `UnexpectedError`. `handle_client_connection` is unchanged. `dispatch_lsp_notification` / `handle_internal` still take `&mut IsographState`. `SendLspResponseEffect` still has `reply`. `perform` is unchanged.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
            lsp_server::Message::Request(request) => {
                crate::event::LspEvent::Request(crate::event::LspRequest { client, request })
            }
```

Delta: `client`; no `reply`.

### Tests

`handle` of `Lsp` request when that `client` is in `live_sessions` (a prior `Started` whose `LiveSession.sender` is a dummy `unbounded` sender): one `SendLspResponseEffect`, MethodNotFound bytes unchanged for `textDocument/hover`. `perform` of that effect: the dummy receiver gets the response.

`handle` of `Lsp` request when that `client` was never `Started`: empty effects.

`handle` of `Started`, then `Gone`, then `Lsp` request for that `client`: empty effects. The dummy receiver gets nothing.

Existing `lsp_request_returns_method_not_found` `Started`s first. The dummy `(tx, rx)` is the `LiveSession.sender`, not a field on `LspRequest`.

`lsp_socket.rs` (`listen_for_events`): initialize, then a `textDocument/hover` request. First event `ClientConnectionEvent::Started`. Next event `LspEvent::Request` whose `client` equals that `Started` client and which has no `reply` field. Two connections: two `Started` with different `ClientId`s; each connection's request carries that connection's id.

`listen_and_reply`: hover after initialize is still MethodNotFound. Drop one of two connections; the remaining connection's hover is still MethodNotFound.

### Call sites

- `run_session` -> `client` on `LspRequest`; no `reply`
- `send_lsp_response` -> `LiveSessions::reply`
- `handle` of `Lsp` -> `dispatch_lsp(state, lsp)`
- existing `LspRequest { request, reply }` constructions (handle tests) -> `Started` plus `LspRequest { client, request }`

## Change 2: outstanding ids

Origin: change 1. Delta: `OutstandingLspRequests` on `State`, insert in `dispatch_lsp_request`, take in `send_lsp_response`, `handle_client_connection` of `Gone` removes that client's entry.

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
        ClientConnectionEvent::Gone(client) => {
            state.live_sessions.0.remove(&client);
            state.outstanding.0.remove(&client);
            Vec::new()
        }
```

Delta: `outstanding.0.remove(&client)` next to the live-session remove.

```rust
// from crates/isograph_cli/src/state.rs
    let crate::event::LspRequest { client, request } = incoming;
    let id = request.id.clone();
    state
        .outstanding
        .0
        .entry(client)
        .or_default()
        .insert(id.clone());
    let get_response = || {
        let request =
            isograph_lsp::lsp_request_dispatch::LSPRequestDispatch::new(request, &state.db)
```

Delta: insert the id under `client` before dispatch, only if that client is in `live_sessions`. `send_lsp_response` is still the only place that clones a sender.

```rust
// from crates/isograph_cli/src/state.rs
fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
    incoming: crate::event::LspRequest,
) -> Vec<IsographEffect> {
    let crate::event::LspRequest { client, request } = incoming;
    if !state.live_sessions.0.contains_key(&client) {
        return Vec::new();
    }
    let id = request.id.clone();
    state
        .outstanding
        .0
        .entry(client)
        .or_default()
        .insert(id.clone());
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
        ControlFlow::Continue(request) => method_not_found(
            state,
            id,
            crate::event::LspRequest { client, request },
        ),
    }
}

fn send_lsp_response<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
    client: crate::event::ClientId,
    id: lsp_server::RequestId,
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
            id: id.clone(),
            result: None,
            error: isograph_lsp::lsp_runtime_error::LSPRuntimeError::UnexpectedError(
                "send_lsp_response of a pair that was not outstanding".to_owned(),
            )
            .to(),
        }
    };
    let Some(reply) = state.live_sessions.reply(client) else {
        return Vec::new();
    };
    crate::effect::IsographEffect::SendLspResponse(
        crate::effect::SendLspResponseEffect { reply, response }.boxed(),
    )
    .wrap_vec()
}
```

Delta: `dispatch_lsp_request` takes `&mut State`. Insert only if `reply(client)` is `Some`. `send_lsp_response` takes the id; missing id is `UnexpectedError` on the response, then `reply` still has to succeed to send it. If `reply` is `None`, no effects (the drop guard already dropped). Later cancel / async: if the id is gone, do not call `send_lsp_response`.

`method_not_found` passes `id` through to `send_lsp_response`.

### Tests

Change 1 tests still pass. After a live hover, `state.outstanding.0` is empty.

`handle` of `Gone` when `outstanding` already has `client -> {id}` and `other -> {id}`: `client` is absent from `live_sessions` and from outstanding; `other`'s id remains. Empty effects. Construct `State { outstanding: OutstandingLspRequests(...), .. }` with those two keys.

`handle` of `Gone` of a client that was never registered: empty effects, outstanding empty.

### Call sites

- `dispatch_lsp_request` -> insert id under client
- `send_lsp_response` -> take id, then `LiveSessions::reply`
- `handle_client_connection` of `Gone` -> outstanding `remove`
- `serve` / `empty` / `empty_state` / `listen_and_reply` / watch `Harness` -> `State { .., outstanding }`
- later cancel / async reply -> if the id is still in `state.outstanding`, then `send_lsp_response`
- lsp-open-files.md -> `client` already on `LspRequest`; notifications still need `client`
