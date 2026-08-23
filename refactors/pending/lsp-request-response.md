# LSP request/response through the event loop

Requires lsp-port.md (landed). Independent of lsp-tokens.md, lsp-sessions.md, filesystem-watcher.md.

Today `run_session` answers every request with `MethodNotFound` on `connection.sender`. `IsographState` is owned by `run_event_loop`. Domain requests (tokens, hover) need that state. This slice moves request answering onto the event loop. The session only forwards the `lsp_server::Request` and writes the `lsp_server::Response`. `handle` is unchanged. Notifications stay `isograph/event` -> `event_tx`.

This is the outer adapter: request/response, not `handle`. The production handler this slice is still `MethodNotFound`. That is a reader: every unknown request goes through it. Tokens later adds a method arm in `answer_request`. Do not add a test-only method.

Independent of lsp-sessions.md: the session already has `connection.sender` for the reply. No writer map. No caps.

Origin of the session write: landed `run_session`. Origin of MethodNotFound text: isograph `server.rs` unhandled arm. Delta: the Response is produced on the event-loop thread; the session blocks on a `std::sync::mpsc` recv then `connection.sender.send`.

One shippable change. Existing CLI send tests stay green. Existing `lsp_socket` MethodNotFound tests stay green.

## What the user does

Same as today. `isograph send` still initialize + `isograph/event`. An unknown request after initialize is still `MethodNotFound`. The bytes on the socket do not change.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
pub(crate) struct LspRequest {
    pub request: lsp_server::Request,
    pub reply: std::sync::mpsc::SyncSender<lsp_server::Response>,
}
```

One outstanding reply per `LspRequest`. `sync_channel(0)`: `reply.send` waits for the session `recv`. The session is blocked on that `recv`. They meet. The tokio query channel is unbounded: `query_tx.send` does not wait for the event loop.

```rust
// from crates/isograph_cli/src/daemon.rs
pub(crate) async fn run_event_loop<THostLanguage: HostLanguage>(
    mut state: IsographState<THostLanguage>,
    mut event_rx: UnboundedReceiver<IsographEvent>,
    mut query_rx: UnboundedReceiver<LspRequest>,
    effect_tx: UnboundedSender<IsographEffect>,
) {
    loop {
        tokio::select! {
            event = event_rx.recv() => {
                let Some(event) = event else {
                    break;
                };
                for effect in handle(&mut state, event) {
                    let _ = effect_tx.send(effect);
                }
            }
            query = query_rx.recv() => {
                let Some(query) = query else {
                    break;
                };
                let response = answer_request(&state, query.request);
                let _ = query.reply.send(response);
            }
        }
    }
}

fn answer_request<THostLanguage: HostLanguage>(
    _state: &IsographState<THostLanguage>,
    request: lsp_server::Request,
) -> lsp_server::Response {
    lsp_server::Response {
        id: request.id,
        result: None,
        error: lsp_server::ResponseError {
            code: lsp_server::ErrorCode::MethodNotFound as i32,
            data: None,
            message: format!("No handler registered for method '{}'", request.method),
        }
        .wrap_some(),
    }
}
```

`_state` is unused this slice. Tokens reads it. Keep the parameter so that slice is an arm in `answer_request`, not a new signature.

Either channel closing `break`s. Production holds `event_tx` (`_hold_events`) and `query_tx` (`_hold_queries` plus `accept_loop` clones) until `process::exit`. Unit tests of `run_event_loop` drop `event_tx` to end the loop as today and keep `query_tx` (`let _hold_queries = query_tx`).

### `serve`

```rust
// from crates/isograph_cli/src/daemon.rs
    let (query_tx, query_rx) = unbounded_channel::<crate::lsp_socket::LspRequest>();
    let _hold_events = event_tx.clone();
    let _hold_queries = query_tx.clone();
    let mut state = IsographState::<THostLanguage>::default();
    intern_config_directory(&mut state, config_path.reference());
    tokio::select! {
        () = run_event_loop(state, event_rx, query_rx, effect_tx) => {}
        () = run_effect_loop(effect_rx) => {}
        () = crate::lsp_socket::accept_loop(listener, event_tx, query_tx) => {}
    }
```

`accept_loop` / `session` / `run_session` take `query_tx: UnboundedSender<LspRequest>`. Clone `query_tx` per connection, same as `event_tx`.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn run_session(
    connection: &lsp_server::Connection,
    event_tx: tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
    query_tx: tokio::sync::mpsc::UnboundedSender<LspRequest>,
) {
    // initialize as today
    for message in &connection.receiver {
        match message {
            lsp_server::Message::Request(request) => {
                let (reply, rx) = std::sync::mpsc::sync_channel(0);
                match query_tx.send(LspRequest { request, reply }) {
                    Ok(()) => {
                        if let Ok(response) = rx.recv() {
                            let _ = connection
                                .sender
                                .send(lsp_server::Message::Response(response));
                        }
                    }
                    Err(_) => {}
                }
            }
            lsp_server::Message::Notification(notification)
                if notification.method == IsographEventNotification::METHOD =>
            {
                match serde_json::from_value::<crate::event::IsographEvent>(notification.params) {
                    Ok(event) => {
                        let _ = event_tx.send(event);
                    }
                    Err(e) => warn!(error = %e, "isograph/event params"),
                }
            }
            lsp_server::Message::Notification(_) | lsp_server::Message::Response(_) => {}
        }
    }
}
```

If `query_tx.send` fails, the event loop is gone. Do not `recv`. If `rx.recv` fails, the event loop dropped the sender without sending (it does not). Session writes nothing; the client times out. Do not invent a fallback `MethodNotFound` on the session: one place answers requests.

`session` passes `query_tx` into `run_session`. `accept_loop` clones it at accept, same as `event_tx`.

### Daemon unit tests

```rust
// from crates/isograph_cli/src/daemon.rs
        let (query_tx, query_rx) = unbounded_channel();
        let _hold_queries = query_tx;
        run_event_loop(..., event_rx, query_rx, effect_tx).await;
```

Three existing tests each gain that pair. Drop `event_tx` still ends the loop.

## Tests

`lsp_socket.rs`: same listen helper, pass `query_tx` into `accept_loop`. Existing MethodNotFound / shutdown / ServerNotInitialized / HelloWorld tests stay. `ServerNotInitialized` is still `Connection::initialize`, not this channel.

Add: initialize, `notify` HelloWorld, settle, then `textDocument/hover` on that same connection is `MethodNotFound` and `event_rx` is still only HelloWorld. Proves a request through the event loop does not become an `IsographEvent`.

`cli.rs` unchanged.

## Call sites

- `serve` -> `query_tx` -> `accept_loop` / `_hold_queries` / `run_event_loop`
- `run_session` Request -> `LspRequest` -> `answer_request` -> `connection.sender`
- `run_session` `isograph/event` -> `event_tx` -> `handle` (unchanged)
- lsp-tokens.md -> an arm in `answer_request`
