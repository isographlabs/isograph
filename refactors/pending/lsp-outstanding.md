# Outstanding LSP requests

Requires lsp-request-response.md and lsp-sessions.md. Independent of lsp-dispatch.md and lsp-tokens.md.

`handle` of `Lsp` whose message is a request always returns `SendLspResponseEffect` in the same turn. After disconnect, `Lsp` events can still sit on `event_rx`. This slice drops those replies: `ClientId` and `ClientEvent::Gone` are already on the session (lsp-sessions.md). A `HashSet<(ClientId, RequestId)>` on the event loop is this slice.

JSON-RPC ids are per connection. Two clients can both use id `1`. Keys are `(ClientId, RequestId)`.

`handle` stays synchronous. The set is empty except while a request is in `handle`, and for ids still on `event_rx` after `ClientEvent::Gone`. Cancel and async replies are later readers of the same table.

Origin: landed `Lsp` / `SendLspResponseEffect`; lsp-sessions.md `ClientId` / `ClientEvent::Gone`. Delta: `client` on `LspRequest` and on `SendLspResponseEffect`, outstanding on `run_event_loop`.

One shippable change. MethodNotFound bytes do not change. Send tests stay green.

## What the user does

Same as today. Dropping a client while a request is queued does not write a response on a dead socket.

## Types

Most important first. `ClientId` and `Client` are lsp-sessions.md.

```rust
// from crates/isograph_cli/src/event.rs
pub struct LspRequest {
    pub client: ClientId,
    pub request: lsp_server::Request,
    pub reply: crossbeam::channel::Sender<lsp_server::Message>,
}
```

`LspRequest` gains `client`. `run_session` already has `client` (sessions.md). Put it on every `Lsp::Request`.

```rust
// from crates/isograph_cli/src/effect.rs
pub(crate) struct SendLspResponseEffect {
    pub client: ClientId,
    pub id: lsp_server::RequestId,
    pub reply: crossbeam::channel::Sender<lsp_server::Message>,
    pub response: lsp_server::Response,
}
```

`method_not_found` fills `client` and `id` from the request. If lsp-dispatch.md has landed, `respond` in `lsp_dispatch.rs` gains the same two fields.

```rust
// from crates/isograph_cli/src/daemon.rs
struct OutstandingLspRequests {
    inner: std::collections::HashSet<(ClientId, lsp_server::RequestId)>,
}
```

Lives in `run_event_loop`.

### `handle`

`handle` of `Client` stays the sessions.md map to `IsographEffect::Client` (`ClientEffect`). This slice does not change that.

`method_not_found` sets `client: lsp.client` and `id` from the request on `SendLspResponseEffect`.

### `run_event_loop`

```rust
// from crates/isograph_cli/src/daemon.rs
    let mut outstanding = OutstandingLspRequests {
        inner: std::collections::HashSet::new(),
    };
    while let Some(event) = event_rx.recv().await {
        match &event {
            IsographEvent::Lsp(crate::event::LspEvent::Request(request)) => {
                outstanding
                    .inner
                    .insert((request.client, request.request.id.clone()));
            }
            IsographEvent::Lsp(_) => {}
            IsographEvent::Client(crate::event::ClientEvent::Gone(client)) => {
                outstanding.inner.retain(|(id, _)| *id != *client);
            }
            IsographEvent::Client(crate::event::ClientEvent::Started(_)) => {}
            IsographEvent::Internal(_) => {}
        }
        let effects = handle(&mut state, event);
        for effect in effects {
            let forward = match &effect {
                IsographEffect::SendLspResponse(respond) => outstanding
                    .inner
                    .remove(&(respond.client, respond.id.clone())),
                IsographEffect::LogHelloWorld
                | IsographEffect::Kill
                | IsographEffect::Client(_) => true,
            };
            if forward {
                let _ = effect_tx.send(effect);
            }
        }
    }
```

`remove` is whether the id was still open. `SendLspResponseEffect` whose pair is absent is not forwarded. `ClientEffect` still forwards.

### Session

`accept_loop` already assigns `ClientId` and `session` already posts `ClientEvent::Gone` (sessions.md). `run_session` takes `client` and puts it on `LspEvent::Request`.

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

## Tests

`lsp_socket.rs`: drop a connection after initialize. `event_rx` receives `ClientEvent::Gone` (sessions.md). A second connection HelloWorld still works. Two connections that both drop produce two `ClientEvent::Gone` with different `ClientId`s.

`handle` still returns `SendLspResponseEffect` in the same turn as an `Lsp` request. A dropped in-flight reply is not observable until a later slice can defer the response. Do not assert it here.

Do not add a production API only tests call.

## Call sites

- `run_session` -> `client` on `LspRequest`
- `run_event_loop` -> outstanding insert / retain / remove
- later cancel / async reply -> the same set
