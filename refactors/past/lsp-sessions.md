# Per-connection client writers

Requires lsp-port.md (landed). Independent of lsp-tokens.md and lsp-outstanding.md.

A request already carries `LspRequest.reply`. `textDocument/publishDiagnostics` is a server-to-client notification: there is no request, so there is no reply sender on the event. After `initialize`, the session posts `ClientConnectionEvent::Started`. `handle` stores `LiveSession` on CLI `State`. When the socket ends, `ClientConnectionEvent::Gone` removes it. handle of the later diagnostics event clones senders whose caps include `publishDiagnostics` onto that effect. `perform` writes the notification and does not see `LiveSessions`.

Delta: `ClientId`, `LiveSession`, `LiveSessions`, `ClientConnectionEvent`, CLI `State` (`db` + `live_sessions`), handle writes the map and returns no effects.

One shippable change.

## What the user does

No new CLI. Send still `initialize`s with `{"capabilities":{}}`. A VS Code client that advertised `textDocument.publishDiagnostics` can later receive that notification (lsp-diagnostics.md). Dropping the editor removes that row.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ClientId(pub u64);

#[derive(Clone, Debug)]
pub struct LiveSession {
    pub sender: crossbeam::channel::Sender<lsp_server::Message>,
    pub capabilities: lsp_types::ClientCapabilities,
}

#[derive(Debug, Default)]
pub struct LiveSessions(pub std::collections::HashMap<ClientId, LiveSession>);

#[derive(Clone, Debug)]
pub struct ClientStartedEvent {
    pub client: ClientId,
    pub session: LiveSession,
}

#[derive(Clone, Debug)]
pub enum ClientConnectionEvent {
    Started(ClientStartedEvent),
    Gone(ClientId),
}

#[derive(Clone, Debug, derive_more::From)]
pub enum IsographEvent {
    #[from]
    Lsp(LspEvent),
    #[from]
    Internal(InternalEvent),
    #[from]
    Client(ClientConnectionEvent),
}
```

Delta: the `Client` types and variant. `LiveSession` is the map value and the `Started` payload.

```rust
// from crates/isograph_cli/src/state.rs
pub struct State<THostLanguage: HostLanguage> {
    pub db: IsographState<THostLanguage>,
    pub live_sessions: LiveSessions,
}

pub fn handle<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
    event: IsographEvent,
) -> Vec<IsographEffect> {
    match event {
        IsographEvent::Lsp(lsp) => dispatch_lsp(&mut state.db, lsp),
        IsographEvent::Internal(internal) => handle_internal(&mut state.db, internal),
        IsographEvent::Client(ClientConnectionEvent::Started(started)) => {
            state.live_sessions.0.insert(started.client, started.session);
            Vec::new()
        }
        IsographEvent::Client(ClientConnectionEvent::Gone(client)) => {
            state.live_sessions.0.remove(&client);
            Vec::new()
        }
    }
}
```

Delta: `State`; handle takes `&mut State`; `dispatch_lsp` / `handle_internal` still take `&mut IsographState`. `remove` of an unknown id is a no-op. Ids are monotonic on `accept_loop`, so `insert` does not collide with a live row.

`IsographState` in `isograph_compiler` is unchanged. `perform` is unchanged.

```rust
// from crates/isograph_cli/src/daemon.rs
pub(crate) async fn run_event_loop<THostLanguage: HostLanguage>(
    mut state: crate::state::State<THostLanguage>,
    mut event_rx: UnboundedReceiver<IsographEvent>,
    effect_tx: UnboundedSender<IsographEffect>,
) {
    while let Some(event) = event_rx.recv().await {
        let effects = handle(&mut state, event);
        for effect in effects {
            let _ = effect_tx.send(effect);
        }
    }
}
```

`serve` builds `State { db: IsographState::default(), live_sessions: LiveSessions::default() }` and passes `intern_config_directory` `&mut state.db`.

handle of a later diagnostics event clones `session.sender` from each row whose `capabilities.text_document.publish_diagnostics` is `Some` onto that effect. `perform` of that effect sends `Message::Notification` on those senders.

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
    let _ = event_tx.send(crate::event::ClientConnectionEvent::Gone(client).to());
    drop(connection);
    let _ = io_threads.join();
}
```

`Gone` is enqueued before `drop(connection)` and `join`. `LiveSession.sender` is a clone of `connection.sender`. The writer thread exits when every clone is dropped. `join` waits until handle of `Gone` has dropped the stored clone.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn run_session(
    connection: &lsp_server::Connection,
    event_tx: tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
    client: crate::event::ClientId,
) {
    let capabilities = match serde_json::to_value(lsp_types::ServerCapabilities {
        semantic_tokens_provider:
            lsp_types::SemanticTokensServerCapabilities::SemanticTokensOptions(
                lsp_types::SemanticTokensOptions {
                    work_done_progress_options: lsp_types::WorkDoneProgressOptions::default(),
                    legend: isograph_lsp::semantic_token_legend(),
                    range: None,
                    full: lsp_types::SemanticTokensFullOptions::Bool(true).wrap_some(),
                },
            )
            .wrap_some(),
        ..Default::default()
    }) {
        Ok(value) => value,
        Err(e) => {
            debug!(error = %e, "server capabilities");
            return;
        }
    };
    let params = match connection.initialize(capabilities) {
        Ok(params) => params,
        Err(e) => {
            debug!(error = %e, "lsp initialize");
            return;
        }
    };
    let params = match serde_json::from_value::<lsp_types::InitializeParams>(params) {
        Ok(params) => params,
        Err(e) => {
            debug!(error = %e, "initialize params");
            return;
        }
    };
    let _ = event_tx.send(
        crate::event::ClientConnectionEvent::Started(crate::event::ClientStartedEvent {
            client,
            session: crate::event::LiveSession {
                sender: connection.sender.clone(),
                capabilities: params.capabilities,
            },
        })
        .to(),
    );
    for message in &connection.receiver {
        let event = match message {
            lsp_server::Message::Request(request) => {
                crate::event::LspEvent::Request(crate::event::LspRequest {
                    request,
                    reply: connection.sender.clone(),
                })
            }
            lsp_server::Message::Notification(notification) => {
                crate::event::LspEvent::Notification(notification)
            }
            lsp_server::Message::Response(response) => crate::event::LspEvent::Response(response),
        };
        let _ = event_tx.send(event.to());
    }
}
```

Delta: `client` argument; `Started` after a successful parse of `InitializeParams`; message loop unchanged (`LspRequest` has no `client` field). Deserialize failure returns without `Started`. `session` still posts `Gone`. Same thread, same `event_tx`: `Started` is enqueued before `Gone`.

Existing `listen_for_events` tests that `initialize` then `try_recv`: first event is `ClientConnectionEvent::Started`. Then the Lsp event the test already asserted. On drop, `ClientConnectionEvent::Gone`. Tests that asserted no events after a failed handshake (`initialize_then_event_without_initialized_is_not_an_event`, `truncated_body_ends_that_connection`) receive `Gone` and no `Started`.

## Tests

`handle` of `ClientConnectionEvent::Started`: `state.live_sessions` has one row at that `client` with those `capabilities`. Dummy `(tx, rx)` from `crossbeam::channel::unbounded()`. Send `Message::Notification` on the stored sender; `rx` receives it. Empty effects.

`handle` of `ClientConnectionEvent::Gone`: that `client` is absent. Empty effects.

`handle` of `ClientConnectionEvent::Gone` of an id that was never registered: empty effects, no panic.

`handle` of `ClientConnectionEvent::Started` twice for the same `ClientId`: one row, the second `session`.

`lsp_socket.rs` (`listen_for_events`):

- initialize with `{"capabilities":{}}`: `event_rx` receives `ClientConnectionEvent::Started` whose `session.capabilities == ClientCapabilities::default()`. Drop. `ClientConnectionEvent::Gone` for that `ClientId`.
- initialize with `textDocument.publishDiagnostics: {}` and `textDocument.hover: {}`: stored `capabilities` equals the deserialized `InitializeParams.capabilities` (`publish_diagnostics` and `hover` both `Some`). Drop.
- initialize whose params JSON is not `InitializeParams`: `Gone`, no `Started`.
- drop the TCP stream before initialize: `Gone`, no `Started`.
- two connections: two `Started` with different `ClientId`s. Drop one: one `Gone`.

`listen_and_reply`: two connections, drop one, the remaining connection's `textDocument/hover` is still `MethodNotFound`.

## Call sites

- `accept_loop` -> `ClientId` -> `session`
- `run_session` after `initialize` -> `ClientConnectionEvent::Started`
- `session` end -> `ClientConnectionEvent::Gone` then `drop(connection)` then `join`
- `handle` -> `live_sessions.0.insert` / `remove`
- `serve` / `run_event_loop` / `handle` tests -> `State { db, live_sessions }`
- lsp-diagnostics.md -> handle clones senders onto the report effect; `perform` does not take `LiveSessions`
- lsp-outstanding.md -> `ClientId` on `LspRequest` / `SendLspResponseEffect`; `ClientConnectionEvent::Gone` drops outstanding ids. handle of `Client` is empty effects.
