# Per-connection LSP session state

Requires lsp-port.md (landed). Independent of lsp-tokens.md and lsp-outstanding.md.

The session already exists: `Connection::initialize`, pump, `drop(connection)` then `join`. After lsp-request-response.md, requests go to `answer_request`. This slice keeps that. After `initialize`, the session posts `Client::Started` with that client's `ClientCapabilities` and a clone of `connection.sender`. The effect loop owns `LiveSessions`. Send is ephemeral: the map holds the row for the life of the connection, then `Client::Gone` removes it. An editor is the same, for the life of the window.

`connection.initialize` already returns serialized `InitializeParams`. Today they are discarded. This slice deserializes them and puts `params.capabilities` on `ClientStarted`.

Cleanup is the thread ending when the socket ends (`exit`, EOF, reset). `session` posts `Client::Gone` after `run_session` returns. No idle timeout. VS Code keeps the language client up while the window is open. A 5 minute inactivity close looks like a crash and the client will reconnect. Dead-peer TCP is socket keepalive, not an LSP timer.

`Kill` still unlinks the port file then `process::exit(0)`. This slice does not drain clients.

Origin: LSP `InitializeParams.capabilities`; isograph `Connection::initialize` then one `connection.sender`. Delta: N connections share one daemon, so connect/disconnect are `Client` events and the effect loop holds `id -> sender + capabilities`.

One shippable change.

## What the user does

No new CLI. Send still `initialize`s with `{"capabilities":{}}` and omits `processId`. A VS Code client that advertised `textDocument.publishDiagnostics` can later receive that notification (lsp-diagnostics.md). A send client that already dropped cannot.

## Types

Most important first.

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

`Client` is not `Internal`: it carries a `Sender` on `Started`, so it is not on the `isograph/event` wire. `LspRequest` does not gain `client`. Request replies still ride on `LspRequest.reply`.

```rust
// from crates/isograph_cli/src/effect.rs
#[derive(Debug)]
pub enum ClientEffect {
    Started(crate::event::ClientStarted),
    Gone(crate::event::ClientId),
}

#[derive(Debug, derive_more::From)]
pub enum IsographEffect {
    LogHelloWorld,
    Kill,
    SendLspResponseEffect(Box<SendLspResponseEffect>),
    #[from]
    ClientEffect(ClientEffect),
}
```

`handle` of `IsographEvent::Client` returns `IsographEffect::ClientEffect` with the matching variant. `#[from]` on the effect variant: `ClientEffect::Started(...).to()` is the effect. `Client::Started(...).to()` is the event.

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
        IsographEvent::Client(client) => crate::effect::IsographEffect::ClientEffect(
            match client {
                crate::event::Client::Started(started) => {
                    crate::effect::ClientEffect::Started(started)
                }
                crate::event::Client::Gone(client) => crate::effect::ClientEffect::Gone(client),
            },
        )
        .wrap_vec(),
```

### `run_event_loop`

Unchanged. `handle` returns `IsographEffect::ClientEffect`; the existing loop forwards every effect.

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
        IsographEffect::SendLspResponseEffect(respond) => {
            let _ = respond
                .reply
                .send(lsp_server::Message::Response(respond.response));
            ControlFlow::Continue(())
        }
        IsographEffect::ClientEffect(crate::effect::ClientEffect::Started(started)) => {
            live.0.insert(
                started.client,
                LiveSession {
                    sender: started.sender,
                    capabilities: started.capabilities,
                },
            );
            ControlFlow::Continue(())
        }
        IsographEffect::ClientEffect(crate::effect::ClientEffect::Gone(client)) => {
            live.0.remove(&client);
            ControlFlow::Continue(())
        }
    }
}
```

Ids are monotonic and not reused, so `insert` cannot collide with a live row. `remove` of an unknown id is a no-op (`initialize` failed, no `Client::Started`).

### Session

`accept_loop` assigns `ClientId` with a `mut u64` on that task (`next += 1` before `spawn`). Pass it into `session`. Do not use `AtomicU64`.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
pub(crate) async fn accept_loop(
    listener: tokio::net::TcpListener,
    event_tx: tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
) {
    let mut next = 1u64;
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let client = crate::event::ClientId(next);
                next += 1;
                let event_tx = event_tx.clone();
                let std_stream = match stream.into_std() {
                    Ok(std_stream) => std_stream,
                    Err(e) => {
                        debug!(error = %e, %peer, "could not take the lsp stream");
                        continue;
                    }
                };
                if let Err(e) = std_stream.set_nonblocking(false) {
                    debug!(error = %e, %peer, "could not set the lsp stream blocking");
                    continue;
                }
                std::thread::spawn(move || session(std_stream, event_tx, client));
            }
            Err(e) => debug!(error = %e, "accept failed"),
        }
    }
}

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

`processId` is not watched. Same thread, same `event_tx`: `Client::Started` is enqueued before `Client::Gone`.

Request answering lives in `answer_request` after lsp-request-response.md. This slice does not move it. `handle` stays inner. `run_session` still puts `reply: connection.sender.clone()` on `LspRequest` and does not put `client` on `Lsp`.

## Tests

`handle` of `Client::Started`: one `IsographEffect::ClientEffect(ClientEffect::Started(_))` with that `client` and `capabilities`. Dummy sender from `crossbeam::channel::unbounded()`.

`handle` of `Client::Gone`: one `IsographEffect::ClientEffect(ClientEffect::Gone(id))` with that `ClientId`.

`lsp_socket.rs` (`listen_for_events`):

- initialize with `{"capabilities":{}}`: `event_rx` receives `Client::Started` whose `capabilities == ClientCapabilities::default()`. Drop the client. `Client::Gone` for that `ClientId`.
- initialize with `textDocument.publishDiagnostics: {}` and `textDocument.hover: {}`: `ClientStarted.capabilities` equals the deserialized `InitializeParams.capabilities` (`publish_diagnostics` and `hover` both `Some`). Drop.
- two connections: two `Client::Started` with different `ClientId`s. Drop one: one `Client::Gone`. The other connection still answers a request.

`perform` of `ClientEffect::Gone` of an id that was never registered: `Continue`, no panic.

Do not add a production function only tests call.

## Call sites

- `accept_loop` -> `ClientId` -> `session`
- `run_session` after `initialize` -> `Client::Started`
- `session` end -> `Client::Gone`
- `handle` -> `IsographEffect::ClientEffect`
- `perform` -> `HashMap` insert / remove
- lsp-diagnostics.md -> iterate `live.0.values()`; send `publishDiagnostics` when `capabilities.text_document.publish_diagnostics` is `Some`
- lsp-outstanding.md -> `ClientId` on `LspRequest` / `SendLspResponseEffect`; `Client::Gone` drops outstanding ids
