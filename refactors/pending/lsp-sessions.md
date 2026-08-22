# Per-connection LSP session state

Requires lsp-port.md. Independent of lsp-tokens.md.

Each TCP connection is one LSP client. After `initialize`, the session keeps that client’s `ClientCapabilities` and a writer for server-to-client messages. It does not put capabilities on `IsographState`. Send is ephemeral: it stores caps for the life of the thread, then drops. An editor is the same, for the life of the window.

Cleanup is the thread ending when the socket ends (`exit`, EOF, reset). No idle timeout. VS Code keeps the language client up while the window is open. A 5 minute “inactivity” close looks like a crash and the client will reconnect. Dead-peer TCP (sleep, a proxy that did not RST) is socket keepalive, not an LSP timer.

Origin: LSP `InitializeParams.capabilities`; isograph `Connection::initialize` then one `connection.sender`. Delta: N connections share one daemon, so each session holds its own caps and writer; membership is the thread.

One shippable change.

## What the user does

No new CLI. Send still `initialize`s with `{"capabilities":{}}` and omits `processId`. A VS Code client that advertised `textDocument.publishDiagnostics` can later receive that notification (lsp-diagnostics.md). A send client that already dropped cannot.

## Types

```rust
// from crates/isograph_cli/src/lsp_socket.rs
struct SessionState {
    phase: Session,
    capabilities: Option<lsp_types::ClientCapabilities>,
}

struct LiveSession {
    notify: tokio::sync::mpsc::UnboundedSender<lsp_server::Notification>,
    publish_diagnostics: PublishDiagnosticsCap,
}

#[derive(Copy, Clone)]
enum PublishDiagnosticsCap {
    Yes,
    No,
}
```

`initialize` extracts `InitializeParams` (ignore failure of unknown fields via serde default). Store `params.capabilities`. `processId` is not watched. `PublishDiagnosticsCap::Yes` when `capabilities.text_document` → `publish_diagnostics` is `Some`. Missing that field is `No`.

On `Running` after successful initialize, the session registers `LiveSession` with a process-wide set. The session thread owns a receiver and writes `Message::Notification` on the socket. On `step` `End` or read EOF, drop `notify`; the set entry is gone. Do not store `LiveSession` on `IsographState`.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
struct LiveSessions {
    inner: std::sync::Mutex<Vec<LiveSession>>,
}
```

`Mutex` so the effect loop (later) and session threads can register. `register` pushes. Drop of `LiveSessionNotify` (a guard the session holds) retains only senders whose `send` would still succeed, or the guard removes its index on `Drop`. Index-in-vec is a collision if we compact; hold an `id: u64` instead.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
struct SessionGuard {
    id: u64,
    sessions: std::sync::Arc<LiveSessions>,
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        let mut inner = self.sessions.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.retain(|session| session.id != self.id);
    }
}
```

`lock().unwrap_or_else(into_inner)` is poison recovery, not a panic on a missing invariant. Raise: `Mutex::lock` returns `Result`; poison means a session thread panicked. `into_inner` is the remaining map. If that is unacceptable, `parking_lot::Mutex` which does not poison. Prefer `std` and poison recovery.

`LiveSession` gains `id: u64`. Allocate from an `AtomicU64` on register.

`serve` creates `Arc<LiveSessions>` and passes it to `accept_loop`. lsp-diagnostics.md is the first reader besides tests.

## Tests

- initialize with no `textDocument.publishDiagnostics`: session `PublishDiagnosticsCap::No`. Drop the client. The live set is empty.
- initialize with `textDocument.publishDiagnostics: {}`: `Yes`. Drop. Set empty.
- two connections, drop one: set length 1, the remaining id.

Do not add a production function only tests call. Tests use `LiveSessions` the same way `serve` does.

## Call sites

- `initialize` -> store caps, `SessionGuard`
- thread exit -> `Drop` removes the writer
- lsp-diagnostics.md -> iterate `LiveSession` with `PublishDiagnosticsCap::Yes`
