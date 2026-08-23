# Per-connection LSP session state

Requires lsp-port.md (landed) and lsp-outstanding.md. Independent of lsp-tokens.md.

The session already exists: `Connection::initialize`, pump, `drop(connection)` then `join`. After lsp-request-response.md, requests go to `answer_request`. This slice keeps that. After `initialize`, the session posts `LspClientStarted` with that client's `ClientCapabilities` and a clone of `connection.sender`. The effect loop owns `HashMap<LspClientId, LiveSession>`. It does not put capabilities on `IsographState`. Send is ephemeral: the map holds the row for the life of the connection, then `LspClientGone` removes it. An editor is the same, for the life of the window.

`connection.initialize` already returns serialized `InitializeParams`. Today they are discarded. This slice deserializes them and puts `params.capabilities` on `LspClientStarted`.

Cleanup is the thread ending when the socket ends (`exit`, EOF, reset). `session` already posts `LspClientGone` (outstanding.md). No idle timeout. VS Code keeps the language client up while the window is open. A 5 minute inactivity close looks like a crash and the client will reconnect. Dead-peer TCP is socket keepalive, not an LSP timer.

`Kill` still unlinks the port file then `process::exit(0)`. This slice does not drain clients.

Origin: LSP `InitializeParams.capabilities`; isograph `Connection::initialize` then one `connection.sender`. Delta: N connections share one daemon, so connect/disconnect are events and the effect loop holds `id -> sender + capabilities`.

One shippable change.

## What the user does

No new CLI. Send still `initialize`s with `{"capabilities":{}}` and omits `processId`. A VS Code client that advertised `textDocument.publishDiagnostics` can later receive that notification (lsp-diagnostics.md). A send client that already dropped cannot.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug)]
pub(crate) struct LspClientStarted {
    pub client: LspClientId,
    pub sender: crossbeam::channel::Sender<lsp_server::Message>,
    pub capabilities: lsp_types::ClientCapabilities,
}

#[derive(Clone, Debug, derive_more::From)]
pub enum IsographEvent {
    #[from]
    Lsp(Lsp),
    #[from]
    Internal(Internal),
    #[from]
    LspClientGone(LspClientGone),
    #[from]
    LspClientStarted(LspClientStarted),
}
```

`LspClientId` and `LspClientGone` are outstanding.md. `#[from]` on `LspClientStarted`. `LspClientStarted` is not `Internal`: it carries a `Sender`, so it is not on the `isograph/event` wire.

```rust
// from crates/isograph_cli/src/effect.rs
#[derive(Debug)]
pub enum IsographEffect {
    LogHelloWorld,
    Kill,
    SendLspResponse(Box<SendLspResponse>),
    RegisterLspSession(LspClientStarted),
    UnregisterLspSession(LspClientId),
}
```

```rust
// from crates/isograph_cli/src/daemon.rs
struct LiveSession {
    sender: crossbeam::channel::Sender<lsp_server::Message>,
    capabilities: lsp_types::ClientCapabilities,
}

struct LiveSessions {
    inner: std::collections::HashMap<LspClientId, LiveSession>,
}
```

The id is the map key. `LiveSession` does not repeat it. Clone `connection.sender` onto `LspClientStarted`; that clone is how the effect loop writes notifications. The session still writes request replies on `connection.sender`.

`LiveSessions` lives in `run_effect_loop`. Not pico. Not `Arc`. Not a mutex. Session threads do not touch it.

### `handle`

```rust
// from crates/isograph_cli/src/state.rs
        IsographEvent::Lsp(lsp) => dispatch_lsp(state, lsp),
        IsographEvent::Internal(internal) => handle_internal(state, internal),
        IsographEvent::LspClientStarted(started) => {
            crate::effect::IsographEffect::RegisterLspSession(started).wrap_vec()
        }
        IsographEvent::LspClientGone(gone) => {
            crate::effect::IsographEffect::UnregisterLspSession(gone.client).wrap_vec()
        }
```

outstanding.md: `handle` of `LspClientGone` is `Vec::new()`. This slice returns `UnregisterLspSession`.

### `run_event_loop`

outstanding.md's match on `&event` is exhaustive. Add the new variant:

```rust
// from crates/isograph_cli/src/daemon.rs
            IsographEvent::LspClientGone(gone) => {
                outstanding.inner.retain(|(client, _)| *client != gone.client);
            }
            IsographEvent::LspClientStarted(_) => {}
            IsographEvent::Internal(_) => {}
```

Then `handle` as today. outstanding.md's effect forward match is exhaustive; add the new arms and forward them:

```rust
// from crates/isograph_cli/src/daemon.rs
            let forward = match &effect {
                IsographEffect::SendLspResponse(respond) => outstanding
                    .inner
                    .remove(&(respond.client, respond.id.clone())),
                IsographEffect::LogHelloWorld
                | IsographEffect::Kill
                | IsographEffect::RegisterLspSession(_)
                | IsographEffect::UnregisterLspSession(_) => true,
            };
```

### `run_effect_loop` / `perform`

```rust
// from crates/isograph_cli/src/daemon.rs
pub(crate) async fn run_effect_loop(mut effect_rx: UnboundedReceiver<IsographEffect>) {
    let mut live = LiveSessions {
        inner: std::collections::HashMap::new(),
    };
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
        IsographEffect::RegisterLspSession(started) => {
            live.inner.insert(
                started.client,
                LiveSession {
                    sender: started.sender,
                    capabilities: started.capabilities,
                },
            );
            ControlFlow::Continue(())
        }
        IsographEffect::UnregisterLspSession(client) => {
            live.inner.remove(&client);
            ControlFlow::Continue(())
        }
    }
}
```

`accept_loop` already assigns `LspClientId` (outstanding.md). Ids are monotonic and not reused, so `insert` cannot collide with a live row. `remove` of an unknown id is a no-op (`initialize` failed, no `LspClientStarted`).

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
        crate::event::LspClientStarted {
            client,
            sender: connection.sender.clone(),
            capabilities: params.capabilities,
        }
        .to(),
    );
```

`processId` is not watched. `session` still posts `LspClientGone` after `run_session` returns (outstanding.md). Same thread, same `event_tx`: `LspClientStarted` is enqueued before `LspClientGone`.

Request answering lives in `answer_request` after lsp-request-response.md. This slice does not move it. `handle` stays inner.

## Tests

`handle` of `LspClientStarted`: one `RegisterLspSession` with that `client` and `capabilities`. Dummy sender from `crossbeam::channel::unbounded()`.

`handle` of `LspClientGone`: one `UnregisterLspSession` with that `client`.

`lsp_socket.rs` (`listen_for_events`):

- initialize with `{"capabilities":{}}`: `event_rx` receives `LspClientStarted` whose `capabilities == ClientCapabilities::default()`. Drop the client. `LspClientGone` for that `LspClientId` (already an outstanding test).
- initialize with `textDocument.publishDiagnostics: {}` and `textDocument.hover: {}`: `LspClientStarted.capabilities` equals the deserialized `InitializeParams.capabilities` (`publish_diagnostics` and `hover` both `Some`). Drop.
- two connections: two `LspClientStarted` with different `LspClientId`s. Drop one: one `LspClientGone`. The other connection still answers a request.

`perform(UnregisterLspSession)` of an id that was never registered: `Continue`, no panic.

Do not add a production function only tests call. Do not pass a map into `accept_loop` so a test can peek at it.

## Call sites

- `run_session` after `initialize` -> `LspClientStarted`
- `session` end -> `LspClientGone` (outstanding.md)
- `handle` -> `RegisterLspSession` / `UnregisterLspSession`
- `perform` -> `HashMap` insert / remove
- lsp-diagnostics.md -> iterate `live.inner.values()`; send `publishDiagnostics` when `capabilities.text_document.publish_diagnostics` is `Some`
