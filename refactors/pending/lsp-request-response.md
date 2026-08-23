# LSP request/response as event and effect

Requires lsp-port.md (landed). Independent of lsp-tokens.md, lsp-sessions.md, lsp-dispatch.md, filesystem-watcher.md.

Today `run_session` writes `MethodNotFound` on `connection.sender`. Domain requests need `IsographState`, which the session thread does not own. This slice sends the request through `handle` as an event that carries the JSON-RPC id, and writes the reply as an effect. `handle` of that event this slice is still `MethodNotFound`. That is a reader: every unknown request goes through it.

The session does not block on a reply channel. It posts the event and keeps pumping. The effect loop writes `Message::Response` on the `connection.sender` clone carried in the effect. No writer map. lsp-sessions.md is still later (`ClientCapabilities`, `publishDiagnostics` to N clients).

JSON-RPC ids are per connection. Two clients can both use id `1`. Outstanding keys are `(LspClientId, RequestId)`.

Origin of MethodNotFound text: isograph `server.rs` unhandled arm. Origin of `handle` / effects: landed event loop. Delta: `IsographEvent::LspRequest`, `IsographEffect::LspRespond`, outstanding set on the event loop, `LspClientGone` when the session ends.

One shippable change. Existing CLI send tests stay green. Existing `lsp_socket` MethodNotFound tests stay green (the bytes do not change).

`docs-website/docs/design-docs/event-model.md` Inner: `handle` matches `LspRequest` and returns `LspRespond`. It still does not touch the socket. The effect loop writes. `handle` is request/response in that sense; it is not IO.

## What the user does

Same as today. `isograph send` still initialize + `isograph/event`. An unknown request after initialize is still `MethodNotFound`.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct LspClientId(u64);

pub(crate) struct LspRequest {
    pub client: LspClientId,
    pub request: lsp_server::Request,
    pub reply: crossbeam::channel::Sender<lsp_server::Message>,
}

pub(crate) struct LspClientGone {
    pub client: LspClientId,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, derive_more::From)]
#[serde(tag = "kind", content = "value")]
pub enum IsographEvent {
    HelloWorld,
    Quit,
    DiskChanged(DiskChanged),
    #[serde(skip)]
    #[from]
    LspRequest(LspRequest),
    #[serde(skip)]
    #[from]
    LspClientGone(LspClientGone),
}
```

`--file` JSON is unchanged. `LspRequest` / `LspClientGone` are not wire kinds. Drop `PartialEq` / `Eq` on `IsographEvent`: `Sender` does not implement them. Tests use `matches!` or compare `DiskChanged` fields.

`reply` is `connection.sender.clone()`. The effect loop sends on it. Request id is `request.id`. Do not duplicate it as a field.

```rust
// from crates/isograph_cli/src/effect.rs
pub(crate) struct LspRespond {
    pub client: LspClientId,
    pub id: lsp_server::RequestId,
    pub reply: crossbeam::channel::Sender<lsp_server::Message>,
    pub response: lsp_server::Response,
}

#[derive(Debug)]
pub enum IsographEffect {
    LogHelloWorld,
    Kill,
    LspRespond(LspRespond),
}

impl PartialEq for IsographEffect {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::LogHelloWorld, Self::LogHelloWorld) => true,
            (Self::Kill, Self::Kill) => true,
            _ => false,
        }
    }
}
```

`Eq` is not derived. Daemon unit tests keep `assert_eq` on `LogHelloWorld` / `Kill`. Tests of `LspRespond` match the variant and `response.error` code.

```rust
// from crates/isograph_cli/src/daemon.rs
struct OutstandingLspRequests {
    inner: std::collections::HashSet<(LspClientId, lsp_server::RequestId)>,
}
```

Not pico. Not on `IsographState`. Lives in `run_event_loop`. Insert when an `LspRequest` is received. Remove when that id's `LspRespond` is forwarded to `effect_tx`. `LspClientGone` retains only other clients. A `LspRespond` whose pair is absent is dropped (client gone, or a later cancel). `handle` is synchronous this slice, so the set is empty except during that turn; it is still the table cancel/async replies will use.

### `handle`

```rust
// from crates/isograph_cli/src/state.rs
        IsographEvent::LspRequest(request) => method_not_found(request).wrap_vec(),
        IsographEvent::LspClientGone(_) => Vec::new(),

fn method_not_found(request: crate::event::LspRequest) -> IsographEffect {
    IsographEffect::LspRespond(crate::effect::LspRespond {
        client: request.client,
        id: request.request.id.clone(),
        reply: request.reply,
        response: lsp_server::Response {
            id: request.request.id,
            result: None,
            error: lsp_server::ResponseError {
                code: lsp_server::ErrorCode::MethodNotFound as i32,
                data: None,
                message: format!(
                    "No handler registered for method '{}'",
                    request.request.method
                ),
            }
            .wrap_some(),
        },
    })
}
```

Dispatch (`on_request_sync`) is lsp-dispatch.md. This slice is one function.

### `run_event_loop`

```rust
// from crates/isograph_cli/src/daemon.rs
pub(crate) async fn run_event_loop<THostLanguage: HostLanguage>(
    mut state: IsographState<THostLanguage>,
    mut event_rx: UnboundedReceiver<IsographEvent>,
    effect_tx: UnboundedSender<IsographEffect>,
) {
    let mut outstanding = OutstandingLspRequests {
        inner: std::collections::HashSet::new(),
    };
    while let Some(event) = event_rx.recv().await {
        match &event {
            IsographEvent::LspRequest(request) => {
                outstanding
                    .inner
                    .insert((request.client, request.request.id.clone()));
            }
            IsographEvent::LspClientGone(gone) => {
                outstanding.inner.retain(|(client, _)| *client != gone.client);
            }
            IsographEvent::HelloWorld
            | IsographEvent::Quit
            | IsographEvent::DiskChanged(_) => {}
        }
        let effects = handle(&mut state, event);
        for effect in effects {
            let forward = match &effect {
                IsographEffect::LspRespond(respond) => outstanding
                    .inner
                    .remove(&(respond.client, respond.id.clone())),
                IsographEffect::LogHelloWorld | IsographEffect::Kill => true,
            };
            if forward {
                let _ = effect_tx.send(effect);
            }
        }
    }
}
```

`remove` returns whether the id was still open. `true` forwards. `handle` of `LspClientGone` returns no `LspRespond`.

### `perform`

```rust
// from crates/isograph_cli/src/daemon.rs
        IsographEffect::LspRespond(respond) => {
            let _ = respond
                .reply
                .send(lsp_server::Message::Response(respond.response));
            ControlFlow::Continue(())
        }
```

`bounded(0)` on the copied writer: this send waits until the IO thread takes the message. Same as today's session-thread send. The effect loop is a current-thread task; it yields only at `.await`. This send is blocking. Acceptable this slice: MethodNotFound is small. Do not add a writer thread here (lsp-sessions.md / lsp-diagnostics.md if it becomes a problem).

### Session

`accept_loop` assigns `LspClientId` with a `mut u64` on that task (`next += 1` before `spawn`). Pass it into `session`.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn session(
    stream: std::net::TcpStream,
    event_tx: tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
    client: crate::event::LspClientId,
) {
    let (connection, io_threads) = match connection_from_stream(stream) {
        Ok(pair) => pair,
        Err(e) => {
            debug!(error = %e, "lsp transport");
            return;
        }
    };
    run_session(&connection, event_tx.clone(), client);
    let _ = event_tx.send(crate::event::LspClientGone { client }.to());
    drop(connection);
    let _ = io_threads.join();
}

fn run_session(
    connection: &lsp_server::Connection,
    event_tx: tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
    client: crate::event::LspClientId,
) {
    // initialize as today
    for message in &connection.receiver {
        match message {
            lsp_server::Message::Request(request) => {
                let _ = event_tx.send(
                    crate::event::LspRequest {
                        client,
                        request,
                        reply: connection.sender.clone(),
                    }
                    .to(),
                );
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

`From<LspRequest>` / `From<LspClientGone>` via `derive_more::From` with `#[from]` on those two variants only. The session uses `.to()`.

```toml
# from crates/isograph_cli/Cargo.toml
derive_more = { workspace = true }
```

No `query_tx`. No `sync_channel`. No second channel on `run_event_loop`.

`listen_for_events` in tests must still use a multi-thread runtime: `TcpStream` is blocking.

## Tests

`lsp_socket.rs`: existing MethodNotFound / shutdown / ServerNotInitialized / HelloWorld tests stay. `ServerNotInitialized` is still `Connection::initialize`.

Add: initialize, hover, `MethodNotFound`, then `isograph/event` HelloWorld on the same connection arrives. Proves a request event did not eat the notification path.

Add: `handle` of `LspRequest` (built in the test with `Connection::memory` or a `crossbeam` channel) returns one `LspRespond` whose error code is `MethodNotFound`.

`state.rs` existing in-process tests stay. They construct `HelloWorld` / `Quit` / `DiskChanged` only.

`daemon.rs` existing tests stay. They do not send `LspRequest`.

`cli.rs` unchanged.

## Call sites

- `run_session` Request -> `IsographEvent::LspRequest` -> `handle` -> `LspRespond` -> `perform` -> `reply.send`
- `run_session` end -> `LspClientGone` -> drop outstanding for that client
- `run_session` `isograph/event` -> `event_tx` -> `handle` (unchanged)
- lsp-dispatch.md -> replaces `method_not_found` with `LSPRequestDispatch`
- lsp-tokens.md -> an `on_request_sync` arm
