# Debounced `publishDiagnostics`

Requires lsp-port.md, lsp-sessions.md, and timer-effects.md. Independent of lsp-tokens.md.

A change that should refresh squiggles arms a 100ms timer (timer-effects.md) and stores the guard. Another change before it fires overwrites the guard, which cancels the earlier sleep. `handle` of the matching `TimerFired` reads parse/host errors and returns `ReportDiagnostics`. The effect loop sends `textDocument/publishDiagnostics` to live sessions whose `PublishDiagnosticsCap` is `Yes`.

`handle` does not sleep. HelloWorld and Quit are not this kind of change. `isograph send` is usually already gone; it does not receive the notification.

Origin: isograph `server.rs` debounce then `publish_new_diagnostics_and_clear_old_diagnostics`. Origin of the effect: `docs-website/docs/design-docs/event-model.md` `ReportDiagnostics`. Origin of the timer: timer-effects.md. Delta: the debounce is a timer guard on state, not `ResetDiagnosticsDebounce` / a sleep the effect loop owns; N writers from lsp-sessions.md; parse-of-interned-files first, not `validate_entire_schema`.

One shippable change.

## What the user does

An editor that advertised `publishDiagnostics` has a file interned (send or, later, `didOpen`). After the debounce, a parse error on `iso(\`entrypoint\`)` is a red squiggle. A second change before the timer fires publishes once, for the later contents.

## Types

`IsographEvent::Timer` and `IsographEffect::Timer` are timer-effects.md. This slice adds the guard field and `ReportDiagnostics`.

```rust
// from crates/isograph_cli/src/state.rs
pub struct State<THostLanguage: HostLanguage> {
    pub db: IsographState<THostLanguage>,
    pub live_sessions: LiveSessions,
    pub debounce: Option<freddie::TimerGuard>,
}
```

`State` is lsp-sessions.md. Delta: `debounce`. 100ms, same as isograph `SHORT_DEBOUNCE_TIME`.

```rust
// from crates/isograph_cli/src/effect.rs
struct ReportDiagnostics {
    pub by_uri: Vec<(lsp_types::Uri, Vec<lsp_types::Diagnostic>)>,
}
```

```rust
// from crates/isograph_cli/src/state.rs
        IsographEvent::Internal(InternalEvent::DiskChanged(change)) => {
            handle_disk_changed(state, change);
            let (guard, effect) = freddie::timer_effect_and_guard(
                DIAGNOSTICS_DEBOUNCE,
                crate::event::timer_fired,
            );
            state.debounce = guard.wrap_some();
            IsographEffect::Timer(effect).wrap_vec()
        }
        IsographEvent::Timer(fired) => {
            if state
                .debounce
                .as_ref()
                .map(freddie::TimerGuard::trigger)
                .is_matching(&fired)
            {
                state.debounce = None;
                report_diagnostics(state).wrap_vec()
            } else {
                Vec::new()
            }
        }
```

`use bind::EventTrigger` is at the top of `state.rs` (`bind` git dep, `default-features = false`, same rev as `freddie`).

`report_diagnostics` walks interned `DiskFile`s, maps `file_literals` errors to `lsp_types::Diagnostic` (origin lsp-parse-diagnostics.md, now this file). URI from config directory + relative path (`file://`). Empty diagnostic list for a URI that had errors last time and has none now is required so the client clears squiggles. Keep the previous URI set on `IsographState` or on `LiveSessions`. A `HashSet<Uri>` on a new tracked field is compiler state; put it on a session-less `DiagnosticsUris` singleton source, or compute the clear set in the effect loop from the last `ReportDiagnostics`. Last published URIs on `LiveSessions` is enough: effect loop remembers `last: HashSet<Uri>`, new report, send empty lists for `last - new`.

```rust
// from crates/isograph_cli/src/daemon.rs
        IsographEffect::ReportDiagnostics(report) => {
            publish(live.reference(), report);
            ControlFlow::Continue(())
        }
```

`perform` has no debounce arm. Arming is `IsographEffect::Timer`; cancelling is dropping the guard.

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

- intern a file with `iso(\`entrypoint\`)` via `isograph/event` DiskChanged. A live session with `PublishDiagnosticsCap::Yes` receives `publishDiagnostics` whose `diagnostics` is non-empty. Park on that notification; do not `sleep` then `try_recv`.
- two disk changes 10ms apart: one publish, contents of the second file. Inject a later event after the second change; the first firing must not publish.
- `PublishDiagnosticsCap::No`: no notification.
- drop the session before the timer fires: no panic, no send to a dead socket.

## Call sites

- `DiskChanged` -> `Timer` effect plus guard -> matching `TimerFired` -> `ReportDiagnostics` -> `publish`
- session writer thread -> socket
