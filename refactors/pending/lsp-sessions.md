# Per-connection LSP session state

Requires lsp-port.md (landed). Independent of lsp-tokens.md.

The session already exists: `Connection::initialize`, pump, `MethodNotFound` on the session thread, `drop(connection)` then `join`. This slice keeps that. After `initialize`, the session stores that client's `ClientCapabilities` and registers a writer for server-to-client messages. It does not put capabilities on `IsographState`. Send is ephemeral: it stores caps for the life of the thread, then drops. An editor is the same, for the life of the window.

`connection.initialize` already returns serialized `InitializeParams`. Today they are discarded. This slice keeps them.

Cleanup is the thread ending when the socket ends (`exit`, EOF, reset). No idle timeout. VS Code keeps the language client up while the window is open. A 5 minute inactivity close looks like a crash and the client will reconnect. Dead-peer TCP is socket keepalive, not an LSP timer.

`Kill` still unlinks the port file then `process::exit(0)`. This slice does not drain clients.

Origin: LSP `InitializeParams.capabilities`; isograph `Connection::initialize` then one `connection.sender`. Delta: N connections share one daemon, so each session holds its own caps and a clone of `connection.sender`.

One shippable change.

## What the user does

No new CLI. Send still `initialize`s with `{"capabilities":{}}` and omits `processId`. A VS Code client that advertised `textDocument.publishDiagnostics` can later receive that notification (lsp-diagnostics.md). A send client that already dropped cannot.

## Types

```rust
// from crates/isograph_cli/src/lsp_socket.rs
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LspClientId(u64);

#[derive(Copy, Clone)]
enum PublishDiagnosticsCap {
    Yes,
    No,
}

struct LiveSession {
    id: LspClientId,
    sender: crossbeam::channel::Sender<lsp_server::Message>,
    publish_diagnostics: PublishDiagnosticsCap,
}
```

Clone `connection.sender` into `LiveSession`. Session still uses `connection.sender` for `MethodNotFound`. The clone is how the effect loop writes notifications.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
struct LiveSessions {
    inner: std::sync::Mutex<Vec<LiveSession>>,
}

struct SessionGuard {
    id: LspClientId,
    sessions: std::sync::Arc<LiveSessions>,
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        let mut inner = self.sessions.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.retain(|session| session.id != self.id);
    }
}
```

`lock().unwrap_or_else(into_inner)` is poison recovery. Poison means a session thread panicked (and the daemon panic hook would already have aborted in production). Prefer `std`.

`LspClientId` is assigned in `accept_loop` (one task: `let mut next = 1u64`). Pass it into `session`. Do not use `AtomicU64`.

`run_session` after successful `initialize`:

```rust
    let params = match serde_json::from_value::<lsp_types::InitializeParams>(params) {
        Ok(params) => params,
        Err(e) => {
            debug!(error = %e, "initialize params");
            lsp_types::InitializeParams::default()
        }
    };
    let publish_diagnostics = publish_diagnostics_cap(&params.capabilities);
    let _guard = SessionGuard {
        id: client,
        sessions: live.clone(),
    };
    {
        let mut inner = live.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.push(LiveSession {
            id: client,
            sender: connection.sender.clone(),
            publish_diagnostics,
        });
    }
```

`PublishDiagnosticsCap::Yes` when `capabilities.text_document` → `publish_diagnostics` is `Some`. Missing that field is `No`. `processId` is not watched.

`serve` creates `Arc<LiveSessions>` and passes it to `accept_loop`. lsp-diagnostics.md is the first production reader of the writers besides tests.

`MethodNotFound` stays in `run_session` this slice. Domain request dispatch (lsp-tokens.md) stays in the session too: the session owns the writer. `handle` stays inner.

## Tests

- initialize with no `textDocument.publishDiagnostics`: after handshake the live set has one session, `PublishDiagnosticsCap::No`. Drop the client (and settle). The live set is empty.
- initialize with `textDocument.publishDiagnostics: {}`: `Yes`. Drop. Set empty.
- two connections, drop one: set length 1, the remaining `LspClientId`.

Do not add a production function only tests call. Tests use `LiveSessions` the same way `serve` does: pass `Arc` into `accept_loop`, inspect after settle.

## Call sites

- `accept_loop` -> `LspClientId` -> `session`
- `initialize` -> store caps, `SessionGuard`
- thread exit -> `Drop` removes the writer
- lsp-diagnostics.md -> iterate `LiveSession` with `PublishDiagnosticsCap::Yes`
