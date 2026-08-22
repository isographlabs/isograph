# The daemon port speaks LSP

Requires send-events.md (landed).

The TCP port in `{slug}.port` is LSP JSON-RPC (`Content-Length`). `isograph send` is an LSP client: `initialize` (no `processId`), `initialized`, one notification, drop. The notification is `isograph/event`. Its params are `IsographEvent`, the same JSON as `--file` today.

Requests other than `initialize` / `shutdown` get `MethodNotFound`. `shutdown` / `exit` end that connection. They do not `Quit` the daemon. SIGTERM still sends `IsographEvent::Quit` in-process.

`handle` is unchanged. The session deserializes the notification params and sends that event on the existing channel. Watcher (later) still posts `IsographEvent` in-process, never the wire.

Origin of bind, port file, and send: send-events.md. Origin of the empty enum: `lsp-types` 0.97 `notification::Initialized`. Origin of framing: `lsp-server` 0.7.8 `Message`. Delta: TCP instead of `freddie_event_socket` (`Connection` IO threads `unwrap`, so we use `Message::{read,write}`); params are `IsographEvent`; postfix constructors.

One shippable change. Existing CLI send tests stay green.

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
// from crates/isograph_cli/src/lsp_socket.rs
use lsp_types::notification::Notification;

#[derive(Debug)]
pub enum Event {}

impl Notification for Event {
    type Params = crate::event::IsographEvent;
    const METHOD: &'static str = "isograph/event";
}
```

`--file` JSON is `Event::Params`.

isograph does not have a session enum. It calls `connection.initialize(server_capabilities)` then loops on `connection.receiver`. `lsp_server::Connection::listen` accepts one connection and its IO threads `unwrap`. We accept N TCP streams ourselves. Each stream runs the same two functions isograph runs: handshake, then a recv loop.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn initialize_result() -> Result<serde_json::Value, serde_json::Error> {
    serde_json::to_value(lsp_types::InitializeResult {
        capabilities: lsp_types::ServerCapabilities::default(),
        server_info: lsp_types::ServerInfo {
            name: "isograph".to_owned(),
            version: None,
        }
        .wrap_some(),
    })
}

fn handshake(
    reader: &mut impl std::io::BufRead,
    writer: &mut impl std::io::Write,
) -> Result<(), std::io::Error> {
    loop {
        let Some(message) = lsp_server::Message::read(reader)? else {
            return std::io::Error::from(std::io::ErrorKind::UnexpectedEof).wrap_err();
        };
        match message {
            lsp_server::Message::Request(request)
                if request.method == lsp_types::request::Initialize::METHOD =>
            {
                let result = initialize_result().map_err(std::io::Error::other)?;
                lsp_server::Message::Response(lsp_server::Response {
                    id: request.id,
                    result: result.wrap_some(),
                    error: None,
                })
                .write(writer)?;
                return ().wrap_ok();
            }
            lsp_server::Message::Request(request) => {
                lsp_server::Message::Response(lsp_server::Response::new_err(
                    request.id,
                    lsp_server::ErrorCode::ServerNotInitialized as i32,
                    "expected initialize request".to_owned(),
                ))
                .write(writer)?;
            }
            lsp_server::Message::Notification(notification)
                if notification.method == lsp_types::notification::Exit::METHOD =>
            {
                return std::io::Error::from(std::io::ErrorKind::ConnectionAborted).wrap_err();
            }
            lsp_server::Message::Notification(_) | lsp_server::Message::Response(_) => {}
        }
    }
}
```

Origin: `lsp_server::Connection::initialize_start` / `initialize_finish`. Delta: `Message::read` / `write` on the stream; `Response::new_ok` unwraps, so we build `Response` with `to_value`.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
pub(crate) async fn accept_loop(
    listener: tokio::net::TcpListener,
    event_tx: tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
) {
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let event_tx = event_tx.clone();
                std::thread::spawn(move || match stream.into_std() {
                    Ok(std_stream) => {
                        if let Err(e) = std_stream.set_nonblocking(false) {
                            debug!(error = %e, %peer, "could not set the lsp stream blocking");
                            return;
                        }
                        session(std_stream, event_tx);
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
) {
    let writer = match stream.try_clone() {
        Ok(writer) => writer,
        Err(e) => {
            debug!(error = %e, "could not clone the lsp stream");
            return;
        }
    };
    let mut reader = std::io::BufReader::new(stream);
    let mut writer = writer;
    if let Err(e) = handshake(&mut reader, &mut writer) {
        debug!(error = %e, "lsp handshake");
        return;
    }
    loop {
        let message = match lsp_server::Message::read(&mut reader) {
            Ok(message) => {
                let Some(message) = message else {
                    break;
                };
                message
            }
            Err(e) => {
                debug!(error = %e, "lsp connection ended");
                break;
            }
        };
        match message {
            lsp_server::Message::Request(request)
                if request.method == lsp_types::request::Shutdown::METHOD =>
            {
                let _ = lsp_server::Message::Response(lsp_server::Response {
                    id: request.id,
                    result: serde_json::Value::Null.wrap_some(),
                    error: None,
                })
                .write(&mut writer);
                break;
            }
            lsp_server::Message::Request(request) => {
                let _ = lsp_server::Message::Response(lsp_server::Response::new_err(
                    request.id,
                    lsp_server::ErrorCode::MethodNotFound as i32,
                    format!("No handler registered for method '{}'", request.method),
                ))
                .write(&mut writer);
            }
            lsp_server::Message::Notification(notification)
                if notification.method == lsp_types::notification::Exit::METHOD =>
            {
                break;
            }
            lsp_server::Message::Notification(notification)
                if notification.method == Event::METHOD =>
            {
                match serde_json::from_value::<crate::event::IsographEvent>(notification.params) {
                    Ok(event) => {
                        let _ = event_tx.send(event);
                    }
                    Err(e) => warn!(error = %e, "isograph/event params"),
                }
            }
            lsp_server::Message::Notification(notification)
                if notification.method == lsp_types::notification::Initialized::METHOD => {}
            lsp_server::Message::Notification(notification) => {
                warn!(method = notification.method.as_str(), "unknown notification");
            }
            lsp_server::Message::Response(_) => {}
        }
    }
}
```

Origin of the loop: isograph `server.rs` `run` matching `Message::Request` / `Notification`. Origin of shutdown: `Connection::handle_shutdown`. Delta: we do not wait 30s for `exit` after `shutdown`; we break. `initialized` may arrive after handshake returns; ignore it. `into_std` streams are non-blocking; `set_nonblocking(false)` is required. `run_event_loop` still recvs `IsographEvent`.

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

The listen callback today is `freddie_event_socket`. Drop it. `event_tx` is cloned into `accept_loop` and `_hold_events` as today. Delete `external.rs`. `on_message` goes away; deserialize is `Event::Params`.

### `send`

```rust
// from crates/isograph_cli/src/send.rs
    let event: crate::event::IsographEvent =
        serde_json::from_str(frame.trim()).map_err(SendError::NotEvent)?;
    let stream = std::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).map_err(
        |source| SendError::Connect(Connect { port, source }),
    )?;
    handshake_and_notify(stream, event)
```

```rust
// from crates/isograph_cli/src/send.rs
fn handshake_and_notify(
    stream: std::net::TcpStream,
    event: crate::event::IsographEvent,
) -> Result<(), SendError> {
    let mut writer = stream.try_clone().map_err(SendError::Write)?;
    let mut reader = std::io::BufReader::new(stream);
    let id = lsp_server::RequestId::from(1);
    lsp_server::Message::Request(lsp_server::Request {
        id: id.clone(),
        method: lsp_types::request::Initialize::METHOD.to_owned(),
        params: serde_json::json!({ "capabilities": {} }),
    })
    .write(&mut writer)
    .map_err(SendError::Write)?;
    wait_for_ok(&mut reader, id.reference())?;
    lsp_server::Message::Notification(lsp_server::Notification {
        method: lsp_types::notification::Initialized::METHOD.to_owned(),
        params: serde_json::json!({}),
    })
    .write(&mut writer)
    .map_err(SendError::Write)?;
    let params = serde_json::to_value(&event).map_err(SendError::Encode)?;
    lsp_server::Message::Notification(lsp_server::Notification {
        method: crate::lsp_socket::Event::METHOD.to_owned(),
        params,
    })
    .write(&mut writer)
    .map_err(SendError::Write)?;
    ().wrap_ok()
}

fn wait_for_ok(
    reader: &mut impl std::io::BufRead,
    expected: &lsp_server::RequestId,
) -> Result<(), SendError> {
    loop {
        let message = lsp_server::Message::read(reader).map_err(SendError::Read)?;
        let Some(message) = message else {
            return SendError::Closed.wrap_err();
        };
        let lsp_server::Message::Response(response) = message else {
            continue;
        };
        if &response.id != expected {
            continue;
        }
        match response.error {
            None => return ().wrap_ok(),
            Some(error) => return SendError::Lsp(error.message).wrap_err(),
        }
    }
}
```

Initialize params omit `processId`. `Connect.source` is `io::Error`. Drop tungstenite. `SendError` gains `Encode`, `Write(io::Error)`, `Read(io::Error)`, `Closed`, `Lsp(String)`. Send does not wait after the notification.

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

### Design doc

`docs-website/docs/design-docs/event-model.md` Ports / Outer: the TCP port is LSP; send is an LSP client; the event on the wire is notification `isograph/event`. Drop `freddie_event_socket` and `{slug}.lsp`.

## Tests

`lsp_socket.rs`: bind `accept_loop`, 250ms settle.

- initialize then `isograph/event` `HelloWorld`: `event_rx` is `HelloWorld`
- initialize then `isograph/event` DiskChanged present, then absent
- `isograph/event` before initialize: not an event
- unknown notification after initialize: no event; then `isograph/event` HelloWorld arrives
- unknown request after initialize: `MethodNotFound`
- request before initialize: `ServerNotInitialized`
- `shutdown` ends the connection and does not `Quit`; a second connection can initialize + HelloWorld
- malformed payload closes the connection
- two connections both HelloWorld

`send.rs`: `parse_port` / `read_port` stay.

`cli.rs`: HelloWorld, DiskChanged present then absent, daemon stopped, not json, unknown kind, not in help stay.

## Call sites

- `serve` -> `event_tx` -> `accept_loop` / SIGTERM `Quit` / `run_event_loop`
- `Running` + `isograph/event` -> `event_tx.send` -> `handle`
- `isograph send` -> handshake -> `isograph/event`
- watcher (later) -> `event_tx.send` -> `handle`, never the wire
