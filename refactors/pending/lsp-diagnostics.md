# Debounced `publishDiagnostics`

Requires lsp-port.md and lsp-sessions.md. Independent of lsp-tokens.md.

A change that should refresh squiggles returns `ResetDiagnosticsDebounce`. The effect loop drops the previous timer and starts a new one. When it fires, that is `DiagnosticsDebounceFired`. `handle` of that event reads parse/host errors and returns `ReportDiagnostics`. The effect loop sends `textDocument/publishDiagnostics` to live sessions whose `PublishDiagnosticsCap` is `Yes`.

`handle` does not sleep. HelloWorld and Quit are not this kind of change. `isograph send` is usually already gone; it does not receive the notification.

Origin: isograph `server.rs` debounce then `publish_new_diagnostics_and_clear_old_diagnostics`. Origin of the effect: `docs-website/docs/design-docs/event-model.md` `ReportDiagnostics`. Delta: the timer is an effect because the worker must not block; N writers from lsp-sessions.md; parse-of-interned-files first, not `validate_entire_schema`.

One shippable change.

## What the user does

An editor that advertised `publishDiagnostics` has a file interned (send or, later, `didOpen`). After the debounce, a parse error on `iso(\`entrypoint\`)` is a red squiggle. A second change before the timer fires publishes once, for the later contents.

## Types

```rust
// from crates/isograph_cli/src/event.rs
enum IsographEvent {
    HelloWorld,
    Quit,
    DiskChanged(DiskChanged),
    DiagnosticsDebounceFired,
}

// from crates/isograph_cli/src/effect.rs
enum IsographEffect {
    LogHelloWorld,
    Kill,
    ResetDiagnosticsDebounce,
    ReportDiagnostics(ReportDiagnostics),
}

struct ReportDiagnostics {
    pub by_uri: Vec<(lsp_types::Uri, Vec<lsp_types::Diagnostic>)>,
}
```

`ResetDiagnosticsDebounce` has no payload. `DiagnosticsDebounceFired` has no payload; `handle` reads current files.

```rust
// from crates/isograph_cli/src/state.rs
        IsographEvent::DiskChanged(change) => {
            handle_disk_changed(state, change);
            crate::effect::IsographEffect::ResetDiagnosticsDebounce.wrap_vec()
        }
        IsographEvent::DiagnosticsDebounceFired => report_diagnostics(state).wrap_vec(),
```

`report_diagnostics` walks interned `DiskFile`s, maps `file_literals` errors to `lsp_types::Diagnostic` (origin lsp-parse-diagnostics.md, now this file). URI from config directory + relative path (`file://`). Empty diagnostic list for a URI that had errors last time and has none now is required so the client clears squiggles. Keep the previous URI set on `IsographState` or on `LiveSessions`. A `HashSet<Uri>` on a new tracked field is compiler state; put it on a session-less `DiagnosticsUris` singleton source, or compute the clear set in the effect loop from the last `ReportDiagnostics`. Last published URIs on `LiveSessions` is enough: effect loop remembers `last: HashSet<Uri>`, new report, send empty lists for `last - new`.

```rust
// from crates/isograph_cli/src/daemon.rs
        IsographEffect::ResetDiagnosticsDebounce => {
            debounce.cancel();
            debounce.start();
            ControlFlow::Continue(())
        }
        IsographEffect::ReportDiagnostics(report) => {
            publish(live.reference(), report);
            ControlFlow::Continue(())
        }
```

`debounce` is a `tokio::time::Sleep` pinned in `serve`'s `select!`, or a task the effect loop owns. 100ms, same as isograph `SHORT_DEBOUNCE_TIME`. Firing sends `IsographEvent::DiagnosticsDebounceFired` on `event_tx`. A new `ResetDiagnosticsDebounce` before fire drops that sleep.

The session already writes responses on `connection.sender`. Server-to-client `publishDiagnostics` uses the writer registered in lsp-sessions.md (`LiveSession.sender`). Do not add a second writer thread here if lsp-sessions.md already clones `connection.sender`; `Message::Notification` on that sender is enough. `bounded(0)` on the IO writer still applies: send blocks until the writer thread takes the message.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn publish(live: &LiveSessions, report: crate::effect::ReportDiagnostics) {
    let inner = live.inner.lock().unwrap_or_else(|e| e.into_inner());
    for session in inner.iter() {
        match session.publish_diagnostics {
            PublishDiagnosticsCap::No => continue,
            PublishDiagnosticsCap::Yes => {
                for (uri, diagnostics) in report.by_uri.iter() {
                    let _ = session.notify.send(lsp_server::Notification {
                        method: lsp_types::notification::PublishDiagnostics::METHOD.to_owned(),
                        params: serde_json::to_value(lsp_types::PublishDiagnosticsParams {
                            uri: uri.clone(),
                            diagnostics: diagnostics.clone(),
                            version: None,
                        })
                        .unwrap_or(serde_json::Value::Null),
                    });
                }
            }
        }
    }
}
```

Do not `unwrap` `to_value`. If encode fails, skip that URI and `warn`. `PublishDiagnostics` is the `lsp_types::notification::Notification` impl.

The session thread already writes responses. It also reads `notify` and writes those notifications on the same socket. A `select` on the session thread cannot easily mix blocking `Message::read` and a tokio mpsc. Use `std::sync::mpsc` for `notify`, and `read` with a timeout, or put the writer on its own thread from lsp-port.md’s cloned `TcpStream`. Writer thread: blocking recv on `std::sync::mpsc::Receiver<lsp_server::Message>`, `Message::write`. Session read thread stays as today. Register the writer’s sender as `LiveSession.notify`. Drop of the session joins by dropping the sender; the writer sees disconnect and exits.

That writer thread should land in lsp-sessions.md if not already implied. lsp-sessions.md’s `notify: UnboundedSender<Notification>` is this sender. This doc is the first producer.

## Tests

- intern a file with `iso(\`entrypoint\`)` via `isograph/event` DiskChanged. After 100ms+settle, a live session with `PublishDiagnosticsCap::Yes` receives `publishDiagnostics` whose `diagnostics` is non-empty.
- two disk changes 10ms apart: one publish, contents of the second file.
- `PublishDiagnosticsCap::No`: no notification.
- drop the session before the timer fires: no panic, no send to a dead socket.

## Call sites

- `DiskChanged` -> `ResetDiagnosticsDebounce` -> timer -> `DiagnosticsDebounceFired` -> `ReportDiagnostics` -> `publish`
- session writer thread -> socket
