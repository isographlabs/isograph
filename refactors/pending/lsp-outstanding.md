# Outstanding LSP requests

Requires lsp-request-response.md. Independent of lsp-dispatch.md, lsp-tokens.md, lsp-sessions.md.

`handle` of `Lsp` whose message is a request always returns `SendLspResponse` in the same turn. After disconnect, `Lsp` events can still sit on `event_rx`. This slice drops those replies: a per-connection id, `Client::Gone` when the session ends, and a `HashSet<(ClientId, RequestId)>` on the event loop.

JSON-RPC ids are per connection. Two clients can both use id `1`. Keys are `(ClientId, RequestId)`.

`handle` stays synchronous. The set is empty except while a request is in `handle`, and for ids still on `event_rx` after `Client::Gone`. Cancel and async replies are later readers of the same table.

Origin: landed `Lsp` / `SendLspResponse`. Delta: `ClientId` on the session and on `Lsp` / `SendLspResponse`, `Client::Gone`, outstanding on `run_event_loop`.

One shippable change. MethodNotFound bytes do not change. Send tests stay green.

## What the user does

Same as today. Dropping a client while a request is queued does not write a response on a dead socket.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ClientId(u64);

#[derive(Clone, Debug)]
pub enum Client {
    Gone(ClientId),
}

pub struct LspRequest {
    pub client: ClientId,
    pub request: lsp_server::Request,
    pub reply: crossbeam::channel::Sender<lsp_server::Message>,
}

pub enum Lsp {
    Request(LspRequest),
    Notification(lsp_server::Notification),
    Response(lsp_server::Response),
}

#[derive(Debug, derive_more::From)]
pub enum IsographEvent {
    #[from]
    Lsp(Lsp),
    #[from]
    Internal(Internal),
    #[from]
    Client(Client),
}
```

`LspRequest` gains `client`. `#[from]` on `Client`. `Client` is not `Internal`: it is not on the `isograph/event` wire.

```rust
// from crates/isograph_cli/src/effect.rs
pub(crate) struct SendLspResponse {
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

```rust
// from crates/isograph_cli/src/state.rs
        IsographEvent::Client(_) => Vec::new(),
```

`method_not_found` sets `client: lsp.client` and `id` from the request on `SendLspResponse`.

### `run_event_loop`

```rust
// from crates/isograph_cli/src/daemon.rs
    let mut outstanding = OutstandingLspRequests {
        inner: std::collections::HashSet::new(),
    };
    while let Some(event) = event_rx.recv().await {
        match &event {
            IsographEvent::Lsp(crate::event::Lsp::Request(request)) => {
                outstanding
                    .inner
                    .insert((request.client, request.request.id.clone()));
            }
            IsographEvent::Lsp(_) => {}
            IsographEvent::Client(crate::event::Client::Gone(client)) => {
                outstanding.inner.retain(|(id, _)| *id != *client);
            }
            IsographEvent::Internal(_) => {}
        }
        let effects = handle(&mut state, event);
        for effect in effects {
            let forward = match &effect {
                IsographEffect::SendLspResponse(respond) => outstanding
                    .inner
                    .remove(&(respond.client, respond.id.clone())),
                IsographEffect::LogHelloWorld | IsographEffect::Kill => true,
            };
            if forward {
                let _ = effect_tx.send(effect);
            }
        }
    }
```

`remove` is whether the id was still open. `SendLspResponse` whose pair is absent is not forwarded.

### Session

`accept_loop` assigns `ClientId` with a `mut u64` on that task (`next += 1` before `spawn`). Pass it into `session`.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn session(
    stream: std::net::TcpStream,
    event_tx: tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
    client: crate::event::ClientId,
) {
    let (connection, io_threads) = match connection_from_stream(stream) {
        Ok(pair) => pair,
        Err(e) => {
            debug!(error = %e, "lsp transport");
            return;
        }
    };
    run_session(&connection, event_tx.clone(), client);
    let _ = event_tx.send(crate::event::Client::Gone(client).to());
    drop(connection);
    let _ = io_threads.join();
}
```

`run_session` takes `client` and puts it on `Lsp`.

## Tests

`handle` of `Client::Gone` is no effects.

`lsp_socket.rs`: drop a connection after initialize. `event_rx` receives `Client::Gone`. A second connection HelloWorld still works. Two connections that both drop produce two `Client::Gone` with different `ClientId`s.

`handle` still returns `SendLspResponse` in the same turn as an `Lsp` request. A dropped in-flight reply is not observable until a later slice can defer the response. Do not assert it here.

Do not add a production API only tests call.

## Call sites

- `accept_loop` -> `ClientId` -> `session`
- `run_session` end -> `Client::Gone`
- `run_event_loop` -> outstanding insert / retain / remove
- later cancel / async reply -> the same set
