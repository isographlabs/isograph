# Per-connection client writers

Requires lsp-port.md (landed). Independent of lsp-tokens.md and lsp-outstanding.md.

A request already carries `LspRequest.reply`. `textDocument/publishDiagnostics` is a server-to-client notification: there is no request, so there is no reply sender on the event. After `initialize`, the session posts `ClientEvent::Started` with that client's `ClientCapabilities` and a clone of `connection.sender`. The effect loop stores the row in `LiveSessions`. When the socket ends, `ClientEvent::Gone` removes it. lsp-diagnostics.md iterates the map and sends on those senders.

Origin of the writer: isograph one `connection.sender`; this process has N connections. Origin of the caps: LSP `InitializeParams.capabilities`. Delta: `ClientId` + `ClientEvent` / `ClientEffect`, `LiveSessions` local in `run_effect_loop`.

One shippable change.

## What the user does

No new CLI. Send still `initialize`s with `{"capabilities":{}}`. A VS Code client that advertised `textDocument.publishDiagnostics` can later receive that notification (lsp-diagnostics.md). Dropping the editor removes that row.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/daemon.rs
struct LiveSession {
    sender: crossbeam::channel::Sender<lsp_server::Message>,
    capabilities: lsp_types::ClientCapabilities,
}

struct LiveSessions(std::collections::HashMap<ClientId, LiveSession>);
```

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
pub enum ClientEvent {
    Started(ClientStarted),
    Gone(ClientId),
}
```

```rust
// from crates/isograph_cli/src/effect.rs
#[derive(Debug)]
pub enum ClientEffect {
    Started(crate::event::ClientStarted),
    Gone(crate::event::ClientId),
}

impl From<crate::event::ClientEvent> for ClientEffect {
    fn from(client: crate::event::ClientEvent) -> Self {
        match client {
            crate::event::ClientEvent::Started(started) => Self::Started(started),
            crate::event::ClientEvent::Gone(client) => Self::Gone(client),
        }
    }
}
```

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug, derive_more::From)]
pub enum IsographEvent {
    #[from]
    Lsp(LspEvent),
    #[from]
    Internal(InternalEvent),
    #[from]
    Client(ClientEvent),
}
```

Delta: `Client` variant.

```rust
// from crates/isograph_cli/src/effect.rs
#[derive(Debug, derive_more::From)]
pub enum IsographEffect {
    LogHelloWorld,
    Kill,
    SendLspResponse(Box<SendLspResponseEffect>),
    #[from]
    Client(ClientEffect),
}
```

Delta: `Client` variant and `#[from]`.

### `handle`

```rust
// from crates/isograph_cli/src/state.rs
        IsographEvent::Lsp(lsp) => dispatch_lsp(state, lsp),
        IsographEvent::Internal(internal) => handle_internal(state, internal),
        IsographEvent::Client(client) => {
            crate::effect::IsographEffect::Client(client.to()).wrap_vec()
        }
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
        IsographEffect::Client(crate::effect::ClientEffect::Started(started)) => {
            live.0.insert(
                started.client,
                LiveSession {
                    sender: started.sender,
                    capabilities: started.capabilities,
                },
            );
            ControlFlow::Continue(())
        }
        IsographEffect::Client(crate::effect::ClientEffect::Gone(client)) => {
            live.0.remove(&client);
            ControlFlow::Continue(())
        }
    }
}
```

Delta: `live` local; `perform` takes `&mut LiveSessions`; `Client` arms. `remove` of an unknown id is a no-op (`initialize` failed). Ids are monotonic on `accept_loop`, so `insert` does not collide with a live row.

`run_event_loop` is unchanged.

### Session

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
    let _ = event_tx.send(crate::event::ClientEvent::Gone(client).to());
    drop(connection);
    let _ = io_threads.join();
}
```

Delta: `next`, `ClientId` into `session`, `ClientEvent::Gone` after `run_session`.

`run_session` after successful `initialize`:

```rust
    let params = match connection.initialize(capabilities) {
        Ok(params) => params,
        Err(e) => {
            debug!(error = %e, "lsp initialize");
            return;
        }
    };
    let params = serde_json::from_value::<lsp_types::InitializeParams>(params).unwrap_or_else(|e| {
        debug!(error = %e, "initialize params");
        lsp_types::InitializeParams::default()
    });
    let _ = event_tx.send(
        crate::event::ClientEvent::Started(crate::event::ClientStarted {
            client,
            sender: connection.sender.clone(),
            capabilities: params.capabilities,
        })
        .to(),
    );
```

Delta: keep initialize params; post `ClientEvent::Started`. `run_session` takes `client`. Same thread, same `event_tx`: `Started` is enqueued before `Gone`.

## Tests

`handle` of `ClientEvent::Started`: one `IsographEffect::Client(ClientEffect::Started(_))` with that `client` and `capabilities`. Dummy sender from `crossbeam::channel::unbounded()`.

`handle` of `ClientEvent::Gone`: one `IsographEffect::Client(ClientEffect::Gone(id))` with that `ClientId`.

`lsp_socket.rs` (`listen_for_events`):

- initialize with `{"capabilities":{}}`: `event_rx` receives `ClientEvent::Started` whose `capabilities == ClientCapabilities::default()`. Drop the client. `ClientEvent::Gone` for that `ClientId`.
- initialize with `textDocument.publishDiagnostics: {}` and `textDocument.hover: {}`: `ClientStarted.capabilities` equals the deserialized `InitializeParams.capabilities` (`publish_diagnostics` and `hover` both `Some`). Drop.
- two connections: two `ClientEvent::Started` with different `ClientId`s. Drop one: one `ClientEvent::Gone`. The other connection still answers a request.

`perform` of `ClientEffect::Gone` of an id that was never registered: `Continue`, no panic.

## Call sites

- `accept_loop` -> `ClientId` -> `session`
- `run_session` after `initialize` -> `ClientEvent::Started`
- `session` end -> `ClientEvent::Gone`
- `handle` -> `IsographEffect::Client`
- `perform` -> `HashMap` insert / remove
- lsp-diagnostics.md -> iterate `live.0.values()`; send `publishDiagnostics` when `capabilities.text_document.publish_diagnostics` is `Some`
- lsp-outstanding.md -> `ClientId` on `LspRequest` / `SendLspResponseEffect`; `ClientEvent::Gone` drops outstanding ids
