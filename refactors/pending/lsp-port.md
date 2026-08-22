# The daemon port speaks LSP

Requires send-events.md (landed).

The TCP port in `{slug}.port` is LSP JSON-RPC (`Content-Length`). Same crate isograph uses: `lsp-server` 0.7.8 `Connection`, `lsp-types` 0.97.

isograph: `Connection::stdio()`, `server::initialize` (`connection.initialize(server_capabilities)`), loop on messages, unhandled requests `MethodNotFound`. That is the server. We do the same.

`isograph send` is an LSP client of that port: `Connection::connect`, initialize request, `initialized`, one notification, drop. The notification is `isograph/event`. Its params are the same JSON as `--file` today (`HelloWorld` / `Quit` / `DiskChanged`).

Each accepted TCP connection is one LSP client. `accept_loop` accepts forever. Each client is one `session`. Each `isograph send` is one client: connect, handshake, one notification, drop.

`session` is the per-client LSP: `Connection` from the stream, `connection.initialize`, then a pump. After initialize, every `lsp_server::Message` becomes `IsographEvent::Lsp` on `event_tx` (the message plus that client's id). Requests also get `MethodNotFound` on that connection in this slice (no writer map yet; lsp-sessions.md). Domain meaning of `isograph/event` is `handle`.

Not every `IsographEvent` is LSP. SIGTERM still posts `Quit` in-process. Tests and the later watcher still post `HelloWorld` / `DiskChanged` / `Quit` in-process. The wire is always `Lsp`.

One shippable change. Existing CLI send tests stay green.

## What is not isograph

isograph is one process per editor on stdio. i2 is one daemon per config. Send and later editors share the TCP port that send-events.md already binds. Everything below is that, or a later slice.

- `Connection::stdio` / `Connection::listen` are one client. We accept N streams on one bind, then `Connection { sender, receiver }` from each `TcpStream`. `socket_transport` is `pub(crate)`. Copy `lsp-server` 0.7.8 `socket.rs` `socket_transport` / `make_reader` / `make_write` and `stdio.rs` `make_io_threads` / `IoThreads`. Delta: none. The `unwrap`s stay; they are the library's.
- Session is a std thread per stream. `Connection::initialize` blocks. N of those cannot sit on the daemon's current-thread runtime.
- `tokio::net::TcpListener` then `into_std` + `set_nonblocking(false)`. Serve is already tokio (event loop, effect loop, SIGTERM). `into_std` streams are non-blocking; `Message::read` is not.
- Code lives in `isograph_cli`, not `isograph_lsp`. The daemon is already `isograph_cli`. i2 `isograph_lsp` is encoding.
- Empty `ServerCapabilities`. isograph's `initialize` advertises tokens, hover, and the rest. Those land with lsp-tokens.md / later docs.
- No `LspState`, no `LSPNotificationDispatch` / `LSPRequestDispatch`. isograph chains many methods. This slice pumps messages onto `event_tx` and answers requests with `MethodNotFound`. Copy the dispatch types when `handle` has a chain.
- No `bridge_crossbeam_to_tokio`. isograph bridges so one `select!` can mix LSP, watcher, and debounce. We already have `run_event_loop`. The session thread iterates `connection.receiver`.
- `isograph/event`. isograph has no CLI event socket. send-events.md already sends `HelloWorld` / `DiskChanged` / `Quit`; the port is now LSP, so that JSON is a notification's params. The channel event is `Lsp`, not those variants.
- Send uses `Connection::connect` (public), then writes the initialize request itself. `Connection::initialize` is server-only. There is no client initialize in `lsp-server`. `tokio-lsp` / `lsp-client-rs` / `async-lsp-client` are other stacks (stdio child process, their own runtime). A second client crate is not isograph. Do not add one.

`shutdown` is `MethodNotFound`, same as isograph (no `handle_shutdown`). Dropping the TCP connection ends that session. `exit` ends it because copied `make_reader` stops on `exit`. Neither `Quit`s the daemon. SIGTERM still sends `IsographEvent::Quit` in-process.

## What the user does

```
$ printf '%s\n' '{"kind":"HelloWorld"}' > /tmp/hello.json
$ isograph send --file /tmp/hello.json
```

The log has `hello world`. The socket sees `initialize`, `initialized`, then `isograph/event` with params `{"kind":"HelloWorld"}`. Send does not wait for a result of that notification.

```
$ printf '%s\n' '{"kind":"DiskChanged","value":{"path":"/tmp/proj/src/a.ts","presence":{"Present":"export const a = 1;\n"}}}' > /tmp/disk.json
$ isograph send --file /tmp/disk.json
```

Send is not in `--help`. The daemon not running is the same error as today.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LspClientId(u64);

#[derive(Debug)]
struct Lsp {
    client: LspClientId,
    message: lsp_server::Message,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum IsographEvent {
    HelloWorld,
    Quit,
    DiskChanged(DiskChanged),
    #[serde(skip)]
    Lsp(Lsp),
}

impl From<Lsp> for IsographEvent {
    fn from(lsp: Lsp) -> Self {
        Self::Lsp(lsp)
    }
}
```

`--file` JSON is `HelloWorld` / `Quit` / `DiskChanged`. `Lsp` is not on the wire as a `kind`. `LspClientId` is monotonic per daemon process, never reused. Origin of `From`: postfix `.to()`. Drop `PartialEq` / `Eq` on `IsographEvent`: `lsp_server::Message` does not implement them. Tests use `matches!`.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
use lsp_types::notification::Notification;

#[derive(Debug)]
pub enum IsographEventNotification {}

impl Notification for IsographEventNotification {
    type Params = crate::event::IsographEvent;
    const METHOD: &'static str = "isograph/event";
}
```

`Params` is the `--file` enum. Deserializing params never yields `Lsp` (`serde(skip)`). Origin of the empty enum: `lsp-types` 0.97 `notification::Initialized`.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn connection_from_stream(
    stream: std::net::TcpStream,
) -> (lsp_server::Connection, IoThreads) {
    let (sender, receiver, io_threads) = socket_transport(stream);
    (lsp_server::Connection { sender, receiver }, io_threads)
}
```

Origin: `lsp-server` 0.7.8 `Connection::listen` after `accept`. Delta: the `TcpStream` is already accepted.

`socket_transport`, `make_reader`, `make_write`: verbatim `lsp-server` 0.7.8 `src/socket.rs`. `make_io_threads`, `IoThreads`: verbatim `src/stdio.rs`. Do not retype. Do not change `unwrap`s.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn initialize(
    connection: &lsp_server::Connection,
) -> Result<(), lsp_server::ProtocolError> {
    let server_capabilities =
        serde_json::to_value(lsp_types::ServerCapabilities::default())
            .map_err(|e| lsp_server::ProtocolError::new(e.to_string()))?;
    let _params = connection.initialize(server_capabilities)?;
    ().wrap_ok()
}
```

Origin: isograph `server.rs` `initialize`. Delta: default capabilities; ignore returned `InitializeParams` (lsp-sessions.md). There is no `handshake` function.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
pub(crate) async fn accept_loop(
    listener: tokio::net::TcpListener,
    event_tx: tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
) {
    let next_client = std::sync::atomic::AtomicU64::new(1);
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let event_tx = event_tx.clone();
                let client = crate::event::LspClientId(
                    next_client.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                );
                std::thread::spawn(move || match stream.into_std() {
                    Ok(std_stream) => {
                        if let Err(e) = std_stream.set_nonblocking(false) {
                            debug!(error = %e, %peer, "could not set the lsp stream blocking");
                            return;
                        }
                        session(std_stream, event_tx, client);
                    }
                    Err(e) => debug!(error = %e, %peer, "could not take the lsp stream"),
                });
            }
            Err(e) => debug!(error = %e, "accept failed"),
        }
    }
}

fn session(
    stream: std::net::TcpStream,
    event_tx: tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
    client: crate::event::LspClientId,
) {
    let (connection, io_threads) = connection_from_stream(stream);
    if let Err(e) = initialize(&connection) {
        debug!(error = %e, "lsp initialize");
        let _ = io_threads.join();
        return;
    }
    for message in &connection.receiver {
        if let lsp_server::Message::Request(request) = &message {
            let _ = connection.sender.send(lsp_server::Message::Response(
                lsp_server::Response {
                    id: request.id.clone(),
                    result: None,
                    error: lsp_server::ResponseError {
                        code: lsp_server::ErrorCode::MethodNotFound as i32,
                        data: None,
                        message: format!(
                            "No handler registered for method '{}'",
                            request.method
                        ),
                    }
                    .wrap_some(),
                },
            ));
        }
        let _ = event_tx.send(crate::event::Lsp { client, message }.to());
    }
    let _ = io_threads.join();
}
```

Origin of the loop: isograph `server.rs` `run` matching `Message`. Origin of the `MethodNotFound` body: isograph's unhandled arm. Delta: no `LspState`; every request is that arm, and every message (including that request) is `IsographEvent::Lsp`. `initialized` is consumed by `Connection::initialize`. `run_event_loop` still recvs `IsographEvent`.

```rust
// from crates/isograph_cli/src/state.rs
        IsographEvent::Lsp(crate::event::Lsp { message, client: _ }) => match message {
            lsp_server::Message::Notification(notification)
                if notification.method == crate::lsp_socket::IsographEventNotification::METHOD =>
            {
                match serde_json::from_value::<IsographEvent>(notification.params) {
                    Ok(event) => handle(state, event),
                    Err(e) => {
                        warn!(error = %e, "isograph/event params");
                        Vec::new()
                    }
                }
            }
            _ => Vec::new(),
        },
```

`handle` already matches `HelloWorld` / `Quit` / `DiskChanged`. The `Lsp` arm of `isograph/event` deserializes those and calls `handle` with them. Other LSP messages are no-ops this slice. `client` is unused until a later slice writes back (lsp-tokens.md / lsp-sessions.md). Recursion does not see `Lsp`: params cannot deserialize to it.

### `serve`

```rust
// from crates/isograph_cli/src/daemon.rs
    let listener = match tokio::net::TcpListener::bind(std::net::SocketAddr::from((
        std::net::Ipv4Addr::LOCALHOST,
        0,
    )))
    .await
    {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!(error = %e, "could not bind the lsp socket");
            return;
        }
    };
    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(e) => {
            tracing::error!(error = %e, "could not read the lsp socket address");
            return;
        }
    };
    // write port file, SIGTERM, intern_config_directory as today
    tokio::select! {
        () = run_event_loop(state, event_rx, effect_tx) => {}
        () = run_effect_loop(effect_rx) => {}
        () = crate::lsp_socket::accept_loop(listener, event_tx) => {}
    }
```

The listen callback today is `freddie_event_socket`. Drop it. `event_tx` is cloned into `accept_loop` and `_hold_events` as today. Delete `external.rs`. `on_message` goes away.

### `send`

```rust
// from crates/isograph_cli/src/send.rs
    let event: crate::event::IsographEvent =
        serde_json::from_str(frame.trim()).map_err(SendError::NotEvent)?;
    let (connection, io_threads) = lsp_server::Connection::connect((
        std::net::Ipv4Addr::LOCALHOST,
        port,
    ))
    .map_err(|source| SendError::Connect(Connect { port, source }))?;
    let result = notify(&connection, event);
    let _ = io_threads.join();
    result
```

```rust
// from crates/isograph_cli/src/send.rs
fn notify(
    connection: &lsp_server::Connection,
    event: crate::event::IsographEvent,
) -> Result<(), SendError> {
    let id = lsp_server::RequestId::from(1);
    connection
        .sender
        .send(lsp_server::Message::Request(lsp_server::Request {
            id: id.clone(),
            method: lsp_types::request::Initialize::METHOD.to_owned(),
            params: serde_json::json!({ "capabilities": {} }),
        }))
        .map_err(|_| SendError::Closed)?;
    wait_for_initialize_result(&connection.receiver, id.reference())?;
    connection
        .sender
        .send(lsp_server::Message::Notification(
            lsp_server::Notification {
                method: lsp_types::notification::Initialized::METHOD.to_owned(),
                params: serde_json::json!({}),
            },
        ))
        .map_err(|_| SendError::Closed)?;
    let params = serde_json::to_value(&event).map_err(SendError::Encode)?;
    connection
        .sender
        .send(lsp_server::Message::Notification(
            lsp_server::Notification {
                method: crate::lsp_socket::IsographEventNotification::METHOD.to_owned(),
                params,
            },
        ))
        .map_err(|_| SendError::Closed)?;
    ().wrap_ok()
}

fn wait_for_initialize_result(
    receiver: &crossbeam_channel::Receiver<lsp_server::Message>,
    expected: &lsp_server::RequestId,
) -> Result<(), SendError> {
    loop {
        let message = receiver.recv().map_err(|_| SendError::Closed)?;
        let lsp_server::Message::Response(response) = message else {
            continue;
        };
        if &response.id != expected {
            return SendError::Lsp(format!(
                "initialize response id {} wanted {}",
                response.id, expected
            ))
            .wrap_err();
        }
        match response.error {
            None => return ().wrap_ok(),
            Some(error) => return SendError::Lsp(error.message).wrap_err(),
        }
    }
}
```

Initialize params omit `processId`. `Connect.source` is `io::Error`. Drop tungstenite. `SendError` gains `Encode`, `Closed`, `Lsp(String)`. Send does not wait after the notification and does not send `shutdown`.

The loop skips non-responses (`window/logMessage`, `$/` notifications). Send has one outstanding request, id `1`. A response with another id is a protocol bug, not a case we wait through. It does not happen in this handshake unless the server is wrong.

`CliVerb::Send` doc comment: `Encode one IsographEvent as an LSP notification to the running daemon. Not for typing: tests and CI.`

### Cargo

```toml
# from crates/isograph_cli/Cargo.toml
# drop freddie_event_socket, tungstenite, tokio-tungstenite, futures-util
lsp-server = { workspace = true }
lsp-types = { workspace = true }
tokio = { workspace = true, features = ["rt", "macros", "signal", "sync", "time", "net"] }
```

`lib.rs`: `mod lsp_socket;`

`crossbeam-channel` comes from `lsp-server` (send `wait_for_initialize_result`). Do not add a direct dep unless the compiler requires it.

### Design doc

`docs-website/docs/design-docs/event-model.md` Inner: `handle` matches `IsographEvent::Lsp`. Outer / Ports: the TCP port is LSP; send is an LSP client; the event on the wire is notification `isograph/event`; the channel event is `Lsp` (message plus `LspClientId`). Drop `freddie_event_socket` and `{slug}.lsp`. SIGTERM, tests, and the later watcher still post non-`Lsp` variants in-process.

## Tests

`lsp_socket.rs`: bind `accept_loop`, 250ms settle. The test client is `Connection::connect`, then `notify`.

- initialize then `isograph/event` `HelloWorld`: `event_rx` is `Lsp` whose notification params deserialize to `HelloWorld`; `handle` of that event is `LogHelloWorld`
- initialize then `isograph/event` DiskChanged present, then absent
- `isograph/event` before initialize: not an event; initialize then a second HelloWorld arrives
- unknown notification after initialize: `event_rx` is `Lsp`; `handle` of it is no effects; then `isograph/event` HelloWorld arrives
- unknown request after initialize: `MethodNotFound` on the socket; `event_rx` is `Lsp`; then `isograph/event` HelloWorld arrives
- request before initialize: `ServerNotInitialized`
- `shutdown` is `MethodNotFound` and does not `Quit`; a following `isograph/event` HelloWorld arrives
- a second connection can initialize + HelloWorld; the two `Lsp` events have different `LspClientId`s
- malformed payload ends that connection; a second connection can initialize + HelloWorld
- two connections both HelloWorld

`state.rs`: existing `HelloWorld` / `Quit` / `DiskChanged` in-process tests stay. Add `handle` of `Lsp` wrapping `isograph/event` HelloWorld is `LogHelloWorld`. Add `handle` of `Lsp` wrapping an unknown notification is no effects.

`send.rs`: `parse_port` / `read_port` stay.

`cli.rs`: HelloWorld, DiskChanged present then absent, daemon stopped, not json, unknown kind, not in help stay.

## Call sites

- `serve` -> `event_tx` -> `accept_loop` / SIGTERM `Quit` / `run_event_loop`
- `Running` -> `event_tx.send(Lsp { client, message }.to())` -> `handle`
- `isograph/event` inside `handle` -> `HelloWorld` / `DiskChanged` / `Quit`
- `isograph send` -> `Connection::connect` -> initialize -> `isograph/event`
- watcher (later) -> `event_tx.send(DiskChanged)` -> `handle`, never the wire
