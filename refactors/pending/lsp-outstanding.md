# Outstanding LSP requests

Requires lsp-request-response.md. Independent of lsp-dispatch.md, lsp-tokens.md, lsp-sessions.md.

`handle` of `LspRequest` always returns `LspRespond` in the same turn. After disconnect, `LspRequest` events can still sit on `event_rx`. This slice drops those replies: a per-connection id, `LspClientGone` when the session ends, and a `HashSet<(LspClientId, RequestId)>` on the event loop.

JSON-RPC ids are per connection. Two clients can both use id `1`. Keys are `(LspClientId, RequestId)`.

`handle` stays synchronous. The set is empty except while a request is in `handle`, and for ids still on `event_rx` after `LspClientGone`. Cancel and async replies are later readers of the same table.

Origin: landed `LspRequest` / `LspRespond`. Delta: `LspClientId` on the session and on those two structs, `LspClientGone`, outstanding on `run_event_loop`.

One shippable change. MethodNotFound bytes do not change. Send tests stay green.

## What the user does

Same as today. Dropping a client while a request is queued does not write a response on a dead socket.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct LspClientId(u64);

pub(crate) struct LspClientGone {
    pub client: LspClientId,
}

pub(crate) struct LspRequest {
    pub client: LspClientId,
    pub request: lsp_server::Request,
    pub reply: crossbeam::channel::Sender<lsp_server::Message>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, derive_more::From)]
#[serde(tag = "kind", content = "value")]
pub enum IsographEvent {
    HelloWorld,
    Quit,
    DiskChanged(DiskChanged),
    #[serde(skip)]
    #[from]
    LspRequest(LspRequest),
    #[serde(skip)]
    #[from]
    LspClientGone(LspClientGone),
}
```

`LspRequest` gains `client`. `#[from]` on `LspClientGone` as well.

```rust
// from crates/isograph_cli/src/effect.rs
pub(crate) struct LspRespond {
    pub client: LspClientId,
    pub id: lsp_server::RequestId,
    pub reply: crossbeam::channel::Sender<lsp_server::Message>,
    pub response: lsp_server::Response,
}
```

`method_not_found` fills `client` and `id` from the request. If lsp-dispatch.md has landed, `respond` in `lsp_dispatch.rs` gains the same two fields.

```rust
// from crates/isograph_cli/src/daemon.rs
struct OutstandingLspRequests {
    inner: std::collections::HashSet<(LspClientId, lsp_server::RequestId)>,
}
```

Not pico. Not on `IsographState`. Lives in `run_event_loop`.

### `handle`

```rust
// from crates/isograph_cli/src/state.rs
        IsographEvent::LspClientGone(_) => Vec::new(),
```

`method_not_found` sets `client: request.client` and `id: request.request.id.clone()` on `LspRespond`.

### `run_event_loop`

```rust
// from crates/isograph_cli/src/daemon.rs
    let mut outstanding = OutstandingLspRequests {
        inner: std::collections::HashSet::new(),
    };
    while let Some(event) = event_rx.recv().await {
        match &event {
            IsographEvent::LspRequest(request) => {
                outstanding
                    .inner
                    .insert((request.client, request.request.id.clone()));
            }
            IsographEvent::LspClientGone(gone) => {
                outstanding.inner.retain(|(client, _)| *client != gone.client);
            }
            IsographEvent::HelloWorld
            | IsographEvent::Quit
            | IsographEvent::DiskChanged(_) => {}
        }
        let effects = handle(&mut state, event);
        for effect in effects {
            let forward = match &effect {
                IsographEffect::LspRespond(respond) => outstanding
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

`remove` is whether the id was still open. `LspRespond` whose pair is absent is not forwarded.

### Session

`accept_loop` assigns `LspClientId` with a `mut u64` on that task (`next += 1` before `spawn`). Pass it into `session`.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn session(
    stream: std::net::TcpStream,
    event_tx: tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
    client: crate::event::LspClientId,
) {
    let (connection, io_threads) = match connection_from_stream(stream) {
        Ok(pair) => pair,
        Err(e) => {
            debug!(error = %e, "lsp transport");
            return;
        }
    };
    run_session(&connection, event_tx.clone(), client);
    let _ = event_tx.send(crate::event::LspClientGone { client }.to());
    drop(connection);
    let _ = io_threads.join();
}
```

`run_session` takes `client` and puts it on `LspRequest`.

## Tests

`handle` of `LspClientGone` is no effects.

`lsp_socket.rs`: drop a connection after initialize. `event_rx` receives `LspClientGone`. A second connection HelloWorld still works. Two connections that both drop produce two `LspClientGone` with different `LspClientId`s.

`handle` still returns `LspRespond` in the same turn as `LspRequest`. A dropped in-flight reply is not observable until a later slice can defer the response. Do not assert it here.

Do not add a production API only tests call.

## Call sites

- `accept_loop` -> `LspClientId` -> `session`
- `run_session` end -> `LspClientGone`
- `run_event_loop` -> outstanding insert / retain / remove
- later cancel / async reply -> the same set
