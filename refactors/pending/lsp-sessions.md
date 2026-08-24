# Per-connection client writers

Requires lsp-port.md (landed). Independent of lsp-tokens.md and lsp-outstanding.md.

A request already carries `LspRequest.reply`. `textDocument/publishDiagnostics` is a server-to-client notification: there is no request, so there is no reply sender on the event. After `initialize`, the session posts `ClientEvent::Started` with that client's `ClientCapabilities` and a clone of `connection.sender`. `handle` stores the row on `IsographState`. When the socket ends, `ClientEvent::Gone` removes it. lsp-diagnostics.md reads the map and puts senders on the `ReportDiagnostics` effect.

Origin of the writer: isograph one `connection.sender`; this process has N connections. Origin of the caps: LSP `InitializeParams.capabilities`. Origin of "event mutates state": `docs-website/docs/design-docs/event-model.md` `handle`. Delta: `ClientId` + `ClientEvent`, `LiveSessions` on `IsographState`.

One shippable change.

## What the user does

No new CLI. Send still `initialize`s with `{"capabilities":{}}`. A VS Code client that advertised `textDocument.publishDiagnostics` can later receive that notification (lsp-diagnostics.md). Dropping the editor removes that row.

## Types

Most important first.

```rust
// from crates/isograph_compiler/src/database.rs
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ClientId(pub u64);

#[derive(Debug)]
pub struct LiveSession {
    pub sender: crossbeam::channel::Sender<lsp_server::Message>,
    pub capabilities: lsp_types::ClientCapabilities,
}

#[derive(Debug, Default)]
pub struct LiveSessions(pub std::collections::HashMap<ClientId, LiveSession>);

#[derive(Debug, Db)]
pub struct IsographState<THostLanguage: HostLanguage> {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
    live_sessions: LiveSessions,
    phantom_data: PhantomData<THostLanguage>,
}
```

Delta: `ClientId`, `LiveSession`, `LiveSessions`, `live_sessions` (not `#[tracked]`). `isograph_compiler` gains `crossbeam`, `lsp-server`, `lsp-types`. `crates/isograph_compiler/src/lib.rs` re-exports `ClientId` next to `DiskFile`. `event.rs` uses `isograph_compiler::ClientId`.

```rust
// from crates/isograph_compiler/src/database.rs
    pub fn insert_live_session(&mut self, client: ClientId, session: LiveSession) {
        self.live_sessions.0.insert(client, session);
    }

    pub fn remove_live_session(&mut self, client: ClientId) {
        self.live_sessions.0.remove(&client);
    }
```

`Default` for `IsographState` fills `live_sessions: LiveSessions::default()`.

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug)]
pub struct ClientStarted {
    pub client: isograph_compiler::ClientId,
    pub sender: crossbeam::channel::Sender<lsp_server::Message>,
    pub capabilities: lsp_types::ClientCapabilities,
}

#[derive(Clone, Debug)]
pub enum ClientEvent {
    Started(ClientStarted),
    Gone(isograph_compiler::ClientId),
}

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

Delta: `ClientStarted`, `ClientEvent`, `Client` variant.

### `handle`

```rust
// from crates/isograph_cli/src/state.rs
        IsographEvent::Lsp(lsp) => dispatch_lsp(state, lsp),
        IsographEvent::Internal(internal) => handle_internal(state, internal),
        IsographEvent::Client(crate::event::ClientEvent::Started(started)) => {
            state.insert_live_session(
                started.client,
                isograph_compiler::LiveSession {
                    sender: started.sender,
                    capabilities: started.capabilities,
                },
            );
            Vec::new()
        }
        IsographEvent::Client(crate::event::ClientEvent::Gone(client)) => {
            state.remove_live_session(client);
            Vec::new()
        }
```

`remove_live_session` of an unknown id is a no-op (`initialize` failed). Ids are monotonic on `accept_loop`, so `insert` does not collide with a live row.

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
                let client = isograph_compiler::ClientId(next);
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
    client: isograph_compiler::ClientId,
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

`handle` of `ClientEvent::Started`: `state` has one live session, that `client` and `capabilities`. Dummy sender from `crossbeam::channel::unbounded()`. Empty effects.

`handle` of `ClientEvent::Gone`: that `client` is absent. Empty effects.

`lsp_socket.rs` (`listen_for_events`):

- initialize with `{"capabilities":{}}`: `event_rx` receives `ClientEvent::Started` whose `capabilities == ClientCapabilities::default()`. Drop the client. `ClientEvent::Gone` for that `ClientId`.
- initialize with `textDocument.publishDiagnostics: {}` and `textDocument.hover: {}`: `ClientStarted.capabilities` equals the deserialized `InitializeParams.capabilities` (`publish_diagnostics` and `hover` both `Some`). Drop.
- two connections: two `ClientEvent::Started` with different `ClientId`s. Drop one: one `ClientEvent::Gone`. The other connection still answers a request.

`handle` of `ClientEvent::Gone` of an id that was never registered: empty effects, no panic.

## Call sites

- `accept_loop` -> `ClientId` -> `session`
- `run_session` after `initialize` -> `ClientEvent::Started`
- `session` end -> `ClientEvent::Gone`
- `handle` -> `insert_live_session` / `remove_live_session`
- lsp-diagnostics.md -> read `live_sessions`; send `publishDiagnostics` when `capabilities.text_document.publish_diagnostics` is `Some`
- lsp-outstanding.md -> `ClientId` on `LspRequest` / `SendLspResponseEffect`; `ClientEvent::Gone` drops outstanding ids
