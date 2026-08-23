# Per-connection LSP session state

Requires lsp-port.md (landed) and lsp-outstanding.md. Independent of lsp-tokens.md.

The session already exists: `Connection::initialize`, pump, `drop(connection)` then `join`. After lsp-request-response.md, requests go to `answer_request`. This slice keeps that. After `initialize`, the session posts `Client::Started` with that client's `ClientCapabilities` and a clone of `connection.sender`. The effect loop owns `LiveSessions`. Send is ephemeral: the map holds the row for the life of the connection, then `Client::Gone` removes it. An editor is the same, for the life of the window.

`connection.initialize` already returns serialized `InitializeParams`. Today they are discarded. This slice deserializes them and puts `params.capabilities` on `ClientStarted`.

Cleanup is the thread ending when the socket ends (`exit`, EOF, reset). `session` already posts `Client::Gone` (outstanding.md). No idle timeout. VS Code keeps the language client up while the window is open. A 5 minute inactivity close looks like a crash and the client will reconnect. Dead-peer TCP is socket keepalive, not an LSP timer.

`Kill` still unlinks the port file then `process::exit(0)`. This slice does not drain clients.

Origin: LSP `InitializeParams.capabilities`; isograph `Connection::initialize` then one `connection.sender`. Delta: N connections share one daemon, so connect/disconnect are `Client` events and the effect loop holds `id -> sender + capabilities`.

One shippable change.

## What the user does

No new CLI. Send still `initialize`s with `{"capabilities":{}}` and omits `processId`. A VS Code client that advertised `textDocument.publishDiagnostics` can later receive that notification (lsp-diagnostics.md). A send client that already dropped cannot.

## Types

Most important first.

outstanding.md has `Client::Gone`. This slice adds `Started`.

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ClientId(u64);

#[derive(Clone, Debug)]
pub struct ClientStarted {
    pub client: ClientId,
    pub sender: crossbeam::channel::Sender<lsp_server::Message>,
    pub capabilities: lsp_types::ClientCapabilities,
}

#[derive(Clone, Debug)]
pub enum Client {
    Started(ClientStarted),
    Gone(ClientId),
}

#[derive(Clone, Debug, derive_more::From)]
pub enum IsographEvent {
    #[from]
    Lsp(Lsp),
    #[from]
    Internal(Internal),
    #[from]
    Client(Client),
}
```

`Client` is not `Internal`: it carries a `Sender` on `Started`, so it is not on the `isograph/event` wire.

```rust
// from crates/isograph_cli/src/effect.rs
#[derive(Debug)]
pub enum IsographEffect {
    LogHelloWorld,
    Kill,
    SendLspResponse(Box<SendLspResponse>),
    Client(crate::event::Client),
}
```

`handle` of `IsographEvent::Client` returns `IsographEffect::Client` with the same payload. No `#[from]` on the effect variant: `Client::Started(...).to()` is the event.

```rust
// from crates/isograph_cli/src/daemon.rs
struct LiveSession {
    sender: crossbeam::channel::Sender<lsp_server::Message>,
    capabilities: lsp_types::ClientCapabilities,
}

struct LiveSessions(std::collections::HashMap<ClientId, LiveSession>);
```

The id is the map key. `LiveSession` does not repeat it. Clone `connection.sender` onto `ClientStarted`; that clone is how the effect loop writes notifications. The session still writes request replies on `connection.sender`.

`LiveSessions` is a local in `run_effect_loop`. Session threads send on `event_tx`.

### `handle`

```rust
// from crates/isograph_cli/src/state.rs
        IsographEvent::Lsp(lsp) => dispatch_lsp(state, lsp),
        IsographEvent::Internal(internal) => handle_internal(state, internal),
        IsographEvent::Client(client) => {
            crate::effect::IsographEffect::Client(client).wrap_vec()
        }
```

outstanding.md: `handle` of `Client` is `Vec::new()`. This slice returns `IsographEffect::Client`.

### `run_event_loop`

outstanding.md's match on `&event` is exhaustive for `Client::Gone` only. Add `Started`:

```rust
// from crates/isograph_cli/src/daemon.rs
            IsographEvent::Client(crate::event::Client::Gone(client)) => {
                outstanding.inner.retain(|(id, _)| *id != *client);
            }
            IsographEvent::Client(crate::event::Client::Started(_)) => {}
            IsographEvent::Internal(_) => {}
```

Then `handle` as today. outstanding.md's effect forward match is exhaustive; add the new arm and forward it:

```rust
// from crates/isograph_cli/src/daemon.rs
            let forward = match &effect {
                IsographEffect::SendLspResponse(respond) => outstanding
                    .inner
                    .remove(&(respond.client, respond.id.clone())),
                IsographEffect::LogHelloWorld
                | IsographEffect::Kill
                | IsographEffect::Client(_) => true,
            };
```

### `run_effect_loop` / `perform`

```rust
// from crates/isograph_cli/src/daemon.rs
pub(crate) async fn run_effect_loop(mut effect_rx: UnboundedReceiver<IsographEffect>) {
    let mut live = LiveSessions(std::collections::HashMap::new());
    while let Some(effect) = effect_rx.recv().await {
        if perform(effect, &mut live).is_break() {
            break;
        }
    }
}

pub fn perform(effect: IsographEffect, live: &mut LiveSessions) -> ControlFlow<()> {
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
            let _ = respond
                .reply
                .send(lsp_server::Message::Response(respond.response));
            ControlFlow::Continue(())
        }
        IsographEffect::Client(crate::event::Client::Started(started)) => {
            live.0.insert(
                started.client,
                LiveSession {
                    sender: started.sender,
                    capabilities: started.capabilities,
                },
            );
            ControlFlow::Continue(())
        }
        IsographEffect::Client(crate::event::Client::Gone(client)) => {
            live.0.remove(&client);
            ControlFlow::Continue(())
        }
    }
}
```

`accept_loop` already assigns `ClientId` (outstanding.md). Ids are monotonic and not reused, so `insert` cannot collide with a live row. `remove` of an unknown id is a no-op (`initialize` failed, no `Client::Started`).

### Session

`run_session` after successful `initialize`:

```rust
    let params = match serde_json::from_value::<lsp_types::InitializeParams>(params) {
        Ok(params) => params,
        Err(e) => {
            debug!(error = %e, "initialize params");
            lsp_types::InitializeParams::default()
        }
    };
    let _ = event_tx.send(
        crate::event::Client::Started(crate::event::ClientStarted {
            client,
            sender: connection.sender.clone(),
            capabilities: params.capabilities,
        })
        .to(),
    );
```

`processId` is not watched. `session` still posts `Client::Gone` after `run_session` returns (outstanding.md). Same thread, same `event_tx`: `Client::Started` is enqueued before `Client::Gone`.

Request answering lives in `answer_request` after lsp-request-response.md. This slice does not move it. `handle` stays inner.

## Tests

`handle` of `Client::Started`: one `IsographEffect::Client(Client::Started(_))` with that `client` and `capabilities`. Dummy sender from `crossbeam::channel::unbounded()`.

`handle` of `Client::Gone`: one `IsographEffect::Client(Client::Gone(id))` with that `ClientId`.

`lsp_socket.rs` (`listen_for_events`):

- initialize with `{"capabilities":{}}`: `event_rx` receives `Client::Started` whose `capabilities == ClientCapabilities::default()`. Drop the client. `Client::Gone` for that `ClientId` (already an outstanding test).
- initialize with `textDocument.publishDiagnostics: {}` and `textDocument.hover: {}`: `ClientStarted.capabilities` equals the deserialized `InitializeParams.capabilities` (`publish_diagnostics` and `hover` both `Some`). Drop.
- two connections: two `Client::Started` with different `ClientId`s. Drop one: one `Client::Gone`. The other connection still answers a request.

`perform` of `Client::Gone` of an id that was never registered: `Continue`, no panic.

Do not add a production function only tests call.

## Call sites

- `run_session` after `initialize` -> `Client::Started`
- `session` end -> `Client::Gone` (outstanding.md)
- `handle` -> `IsographEffect::Client`
- `perform` -> `HashMap` insert / remove
- lsp-diagnostics.md -> iterate `live.0.values()`; send `publishDiagnostics` when `capabilities.text_document.publish_diagnostics` is `Some`
