# LSP as the daemon's outside protocol

Requires send-events.md (landed). Independent of filesystem-watcher.md, file-semantic-tokens.md, and e2e-semantic-tokens.md.

`--file` stays `IsographEvent` JSON. `isograph send` encodes that as an LSP notification and writes it to the daemon. Domain methods: `isograph/helloWorld`, `isograph/diskChanged`. `handle` is unchanged. Same `127.0.0.1:0` port and `{slug}.port` file.

`lsp_server::Connection` IO threads `unwrap`. We never use `Connection`. We use `Message::{read,write}` and construct `Request` / `Response` / `Notification` as structs. `Response::new_ok`, `Notification::new`, and `Request::new` `unwrap` `serde_json::to_value`; we do not call them. `Response::new_err` does not.

Later editor methods (`didOpen`, `semanticTokens/full`, `isograph lsp` stdio) use this listener. They are not these changes. Do not add a second query port.

Origin of bind, port file, and send: send-events.md. Origin of framing: `lsp-server` 0.7.8 `Message`. Delta from send-events: the wire is LSP, not a WebSocket text frame of `IsographEvent`.

Five independently shippable changes. Each leaves `isograph send` and the existing CLI tests working.

## What the user does after change 2

The daemon is already up. `--file` is still `IsographEvent` JSON. Send unlinks it after the attempt.

```
$ printf '%s\n' '{"kind":"HelloWorld"}' > /tmp/hello.json
$ isograph send --file /tmp/hello.json
```

The log has `hello world`. The bytes on the socket are an `isograph/helloWorld` notification, not `{"kind":"HelloWorld"}`.

```
$ printf '%s\n' '{"kind":"DiskChanged","value":{"path":"/tmp/proj/src/a.ts","presence":{"Present":"export const a = 1;\n"}}}' > /tmp/disk.json
$ isograph send --file /tmp/disk.json
```

```
$ printf '%s\n' '{"kind":"Quit"}' > /tmp/quit.json
$ isograph send --file /tmp/quit.json
Quit is not sent on this socket
```

Exit 1. `isograph stop` still kills the daemon.

Send is not in `--help`. The daemon not running is the same error as today.

Changes 3–5 do not change those commands. 3 changes the transport to `Content-Length` TCP. 4 makes send handshake `initialize` first. 5 adds `shutdown` / `exit` as session close.

## Change 1: the daemon accepts an LSP notification on the existing socket

Origin: `crates/isograph_cli/src/external.rs` `on_message`. Delta: a frame that is not `IsographEvent` is parsed as `lsp_server::Notification`; known methods become the same events. WebSocket, send, and `IsographEvent` frames stay.

`lsp-server` is added to `isograph_cli`. Not `lsp-types`.

```rust
// from crates/isograph_cli/src/external.rs
use lsp_server::Notification;
use tracing::warn;

use crate::event::{DiskChanged, IsographEvent};

pub(crate) const HELLO_WORLD: &str = "isograph/helloWorld";
pub(crate) const DISK_CHANGED: &str = "isograph/diskChanged";

pub(crate) fn on_message(text: &str, emit: impl FnOnce(IsographEvent)) {
    if let Ok(event) = serde_json::from_str::<IsographEvent>(text) {
        emit(event);
        return;
    }
    match serde_json::from_str::<Notification>(text) {
        Ok(notification) => {
            if let Some(event) = event_from_notification(notification) {
                emit(event);
            }
        }
        Err(e) => warn!(error = %e, frame = text, "undeserializable frame"),
    }
}

fn event_from_notification(notification: Notification) -> Option<IsographEvent> {
    match notification.method.as_str() {
        HELLO_WORLD => IsographEvent::HelloWorld.wrap_some(),
        DISK_CHANGED => match serde_json::from_value::<DiskChanged>(notification.params) {
            Ok(change) => IsographEvent::DiskChanged(change).wrap_some(),
            Err(e) => {
                warn!(error = %e, "isograph/diskChanged params");
                None
            }
        },
        method => {
            warn!(method, "unknown notification");
            None
        }
    }
}
```

`HELLO_WORLD` ignores params. Unknown method: no event, connection stays up. Bad `DiskChanged` params: no event, connection stays up. `Quit` as `{"kind":"Quit"}` is still an `IsographEvent` frame.

`event_from_notification` is called from `on_message`. Do not add `notification_for_event` here; send does not call it yet.

Existing `external.rs` websocket tests stay. Add:

- `an_isograph_hello_world_notification_arrives_as_an_event`: WS text `{"method":"isograph/helloWorld"}`. `HelloWorld`.
- `an_isograph_disk_changed_notification_arrives_as_an_event`: WS text `{"method":"isograph/diskChanged","params":{"path":"/tmp/proj/src/a.ts","presence":{"Present":"export const a = 1;\n"}}}`. Same `DiskChanged` as the existing present-frame test.
- `an_unknown_method_notification_is_dropped`: method `nope`, no event, then a `HelloWorld` `IsographEvent` frame still arrives.
- `a_disk_changed_notification_with_bad_params_is_dropped`: params `[]`, no event.

CLI tests unchanged. Send still writes `IsographEvent` JSON.

## Change 2: send writes the notification

Origin: `crates/isograph_cli/src/send.rs` `run_inner`. Delta: after parsing `--file` as `IsographEvent`, encode a `Notification` and write that JSON as the websocket text frame. `--file` format is unchanged.

```rust
// from crates/isograph_cli/src/external.rs
#[derive(Debug, thiserror::Error)]
pub(crate) enum NotificationError {
    #[error("Quit is not sent on this socket")]
    QuitOnWire,
    #[error("could not encode notification params: {0}")]
    Params(serde_json::Error),
}

pub(crate) fn notification_for_event(
    event: IsographEvent,
) -> Result<Notification, NotificationError> {
    match event {
        IsographEvent::HelloWorld => Notification {
            method: HELLO_WORLD.to_owned(),
            params: serde_json::Value::Null,
        }
        .wrap_ok(),
        IsographEvent::DiskChanged(change) => serde_json::to_value(change)
            .map_err(NotificationError::Params)
            .map(|params| Notification {
                method: DISK_CHANGED.to_owned(),
                params,
            }),
        IsographEvent::Quit => NotificationError::QuitOnWire.wrap_err(),
    }
}
```

`DiskChanged` params are `{path, presence}`, not the tagged `{kind, value}` envelope.

```rust
// from crates/isograph_cli/src/send.rs
    let event: IsographEvent =
        serde_json::from_str(frame.trim()).map_err(SendError::NotEvent)?;
    let notification = crate::external::notification_for_event(event)?;
    let frame = serde_json::to_string(&notification).map_err(SendError::Encode)?;
    let (mut ws, _) = connect(format!("ws://127.0.0.1:{port}")).map_err(|source| {
        SendError::Connect(Connect {
            port,
            source: source.boxed(),
        })
    })?;
    ws.send(Message::Text(frame))
        .map_err(|e| SendError::Write(e.boxed()))?;
```

`SendError` gains `Notification(NotificationError)` with `#[from]` and `Encode(serde_json::Error)` with `#[error("could not encode the notification: {0}")]`. Discover / not-running / port errors stay.

`CliVerb::Send` doc comment: `Encode one IsographEvent as an LSP notification to the running daemon. Not for typing: tests and CI.` `SendArgs.file` doc comment: `JSON IsographEvent to encode as an LSP notification.`

Unit tests next to `notification_for_event`: HelloWorld method is `isograph/helloWorld`; DiskChanged present params contain the path; Quit is `QuitOnWire`.

CLI: existing send tests stay (`--file` is still `IsographEvent`). Add `send_of_quit_fails`: `--file` `{"kind":"Quit"}`, exit 1, stderr contains `Quit is not sent on this socket`, `--file` unlinked, daemon log has no `kill: exiting`.

A raw websocket client can still send `{"kind":"HelloWorld"}` or `{"kind":"Quit"}`. Send does not.

## Change 3: the port is LSP `Content-Length` TCP

Origin: `daemon.rs` `freddie_event_socket::listen`; `send.rs` tungstenite `connect`. Delta: `tokio::net::TcpListener` on `127.0.0.1:0`, one session thread per connection, `Message::read` / `Message::write`. Send writes `Message::Notification` on a `TcpStream`. Drop `freddie_event_socket`, tungstenite, `tokio-tungstenite`, `futures-util`. Add tokio feature `net`. Move `HELLO_WORLD`, `DISK_CHANGED`, `event_from_notification`, `notification_for_event` to `crates/isograph_cli/src/lsp_socket.rs`. Delete `external.rs`.

No `initialize` yet. A domain notification is enough. A request gets `MethodNotFound`. Malformed LSP closes that connection. `IsographEvent` JSON is no longer on the wire; change 2 already stopped send from writing it.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
use std::io::{BufReader, Write};
use std::net::TcpStream;
use std::thread;

use lsp_server::{ErrorCode, Message, Notification, Response};
use prelude::Postfix;
use tokio::net::TcpListener;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, warn};

use crate::event::IsographEvent;

pub(crate) async fn accept_loop(
    listener: TcpListener,
    event_tx: UnboundedSender<IsographEvent>,
) {
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let event_tx = event_tx.clone();
                thread::spawn(move || match stream.into_std() {
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

fn session(stream: TcpStream, event_tx: UnboundedSender<IsographEvent>) {
    let writer = match stream.try_clone() {
        Ok(writer) => writer,
        Err(e) => {
            debug!(error = %e, "could not clone the lsp stream");
            return;
        }
    };
    let mut reader = BufReader::new(stream);
    let mut writer = writer;
    loop {
        let message = match Message::read(&mut reader) {
            Ok(Some(message)) => message,
            Ok(None) => break,
            Err(e) => {
                debug!(error = %e, "lsp connection ended");
                break;
            }
        };
        match message {
            Message::Notification(notification) => {
                if let Some(event) = event_from_notification(notification) {
                    let _ = event_tx.send(event);
                }
            }
            Message::Request(request) => {
                let response = Response::new_err(
                    request.id,
                    ErrorCode::MethodNotFound as i32,
                    format!("{} is not a request this server answers", request.method),
                );
                if let Err(e) = Message::Response(response).write(&mut writer) {
                    debug!(error = %e, "could not write the lsp response");
                    break;
                }
            }
            Message::Response(_) => {}
        }
    }
}
```

`into_std` streams are non-blocking. `Message::read` is blocking `BufRead`. `set_nonblocking(false)` is required. The session is a `std::thread` so a blocked read does not stall the current-thread tokio runtime.

```rust
// from crates/isograph_cli/src/daemon.rs
use std::net::{Ipv4Addr, SocketAddr};

use tokio::net::TcpListener;

async fn serve<THostLanguage: HostLanguage>(config_path: PathBuf, port_path: PathBuf) {
    let (event_tx, event_rx) = unbounded_channel::<IsographEvent>();
    let (effect_tx, effect_rx) = unbounded_channel::<IsographEffect>();
    let listener = match TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await {
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
    if let Err(e) = std::fs::write(port_path.reference(), format!("{port}\n")) {
        tracing::error!(
            error = %e,
            path = %port_path.display(),
            "could not write the event socket port"
        );
        return;
    }
    tracing::info!(config = %config_path.display(), port, "isograph daemon up");

    // SIGTERM task as today

    let _hold_events = event_tx.clone();
    let state = IsographState::<THostLanguage>::default();
    tokio::select! {
        () = run_event_loop(state, event_rx, effect_tx) => {}
        () = run_effect_loop(effect_rx) => {}
        () = crate::lsp_socket::accept_loop(listener, event_tx) => {}
    }
    let _ = std::fs::remove_file(port_path.reference());
}
```

`_hold_events` still holds a sender. `Kill` ends the effect loop, `select!` drops `accept_loop`, the listener closes. Session threads then see a read error.

```rust
// from crates/isograph_cli/src/send.rs
    let notification = crate::lsp_socket::notification_for_event(event)?;
    let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).map_err(|source| {
        SendError::Connect(Connect { port, source })
    })?;
    Message::Notification(notification)
        .write(&mut stream)
        .map_err(SendError::Write)?;
```

`Connect.source` is `io::Error`, not tungstenite. Drop the `Encode` arm if `Message::write` serializes the notification; `notification_for_event` already encoded params. Keep `Write(io::Error)`.

`lib.rs`: `mod lsp_socket;`. Drop `mod external`.

Tests that spoke websocket (`external.rs`) move to `lsp_socket.rs` and write `Message`s on TCP. Settle 250ms as today.

- `hello_world_notification_is_an_event`
- `disk_changed_present_notification_is_an_event`
- `disk_changed_absent_notification_is_an_event`
- `unknown_notification_is_dropped` then a following hello world on the same connection arrives
- `a_request_is_method_not_found`: `error.code` is `-32601`, then a hello world on the same connection arrives
- `malformed_payload_closes_the_connection`
- `two_connections_both_hello_world`

CLI send tests stay. They now go through Content-Length.

Update `docs-website/docs/design-docs/event-model.md` Outer / Ports: the TCP port speaks LSP; send is an LSP client; drop `freddie_event_socket` and the `{slug}.lsp` second listener. `Quit` is still sendable only by a client that writes `{"kind":"Quit"}`, which this transport no longer accepts. Leave the `Quit` sentence for change 5 except: a `Quit` frame is gone with the old websocket. SIGTERM / `isograph stop` still `Quit`. Send of Quit already failed in change 2.

Landing sequence `refactors/pending/event-model.md` item 2 notes the wire is this file. Item 13 is further methods on this listener.

## Change 4: `initialize` before domain notifications

Origin: change 3 `session`. Delta: `Session::{ExpectInitialize, Running}`. Add `lsp-types`. Send writes `initialize`, waits for the result, writes `initialized`, then the domain notification.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
const INITIALIZE: &str = "initialize";
const INITIALIZED: &str = "initialized";

#[derive(Copy, Clone)]
enum Session {
    ExpectInitialize,
    Running,
}

enum Step {
    Continue,
    Reply(Response),
    End,
}

fn initialize_response(id: RequestId) -> Result<Response, serde_json::Error> {
    serde_json::to_value(lsp_types::InitializeResult {
        capabilities: lsp_types::ServerCapabilities::default(),
        server_info: lsp_types::ServerInfo {
            name: "isograph".to_owned(),
            version: None,
        }
        .wrap_some(),
    })
    .map(|result| Response {
        id,
        result: result.wrap_some(),
        error: None,
    })
}
```

Empty `ServerCapabilities`. Domain requests stay `MethodNotFound`.

`event_from_notification`: `INITIALIZED` returns `None` (not an event, not a warning).

`session` starts `ExpectInitialize`. `step`:

- `ExpectInitialize` + `initialize` request: reply `initialize_response`, set `Running`. Encode failure: `Response::new_err` with `InternalError`, stay `ExpectInitialize`.
- `ExpectInitialize` + other request: `ServerNotInitialized` (`-32002`).
- `ExpectInitialize` + notification: log, `Continue`.
- `Running` + notification: `event_from_notification` as today.
- `Running` + `initialize`: `MethodNotFound`.
- `Running` + other request: `MethodNotFound`.
- Response in either state: `Continue`.
- `Message::read` `None` or `Err`: `End` (drop the connection).

`RequestId` is not `Copy`. Clone it for `initialize_response` so the encode-failure arm still has `id`.

```rust
// from crates/isograph_cli/src/send.rs
fn handshake_and_notify(stream: TcpStream, notification: Notification) -> Result<(), SendError> {
    let mut writer = stream.try_clone().map_err(SendError::Write)?;
    let mut reader = BufReader::new(stream);
    let id = RequestId::from(1);
    Message::Request(Request {
        id: id.clone(),
        method: "initialize".to_owned(),
        params: serde_json::json!({ "capabilities": {} }),
    })
    .write(&mut writer)
    .map_err(SendError::Write)?;
    wait_for_initialize_result(&mut reader, id.reference())?;
    Message::Notification(Notification {
        method: "initialized".to_owned(),
        params: serde_json::json!({}),
    })
    .write(&mut writer)
    .map_err(SendError::Write)?;
    Message::Notification(notification)
        .write(&mut writer)
        .map_err(SendError::Write)?;
    ().wrap_ok()
}

fn wait_for_initialize_result(
    reader: &mut impl BufRead,
    expected: &RequestId,
) -> Result<(), SendError> {
    loop {
        match Message::read(reader).map_err(SendError::Read)? {
            None => return SendError::InitializeClosed.wrap_err(),
            Some(Message::Response(Response {
                id,
                result: Some(_),
                error: None,
            })) if &id == expected => return ().wrap_ok(),
            Some(Message::Response(Response {
                id,
                error: Some(error),
                ..
            })) if &id == expected => return SendError::Initialize(error.message).wrap_err(),
            Some(Message::Response(_)) | Some(Message::Notification(_)) | Some(Message::Request(_)) => {}
        }
    }
}
```

`SendError` gains `Read(io::Error)`, `InitializeClosed`, `Initialize(String)`. Send does not send `shutdown` / `exit`. It drops the socket.

Tests: existing notification tests handshake first. Add:

- `hello_world_before_initialize_is_not_an_event`; then `initialize` + hello world on the same connection works
- `request_before_initialize_is_server_not_initialized`
- `initialize_then_hello_world_response_has_empty_capabilities`

CLI send tests stay.

## Change 5: `shutdown` / `exit` close the session

Origin: change 4 `Session`. Delta: `ExpectExit`. `exit` in any state ends the thread and does not send `IsographEvent::Quit`.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
const SHUTDOWN: &str = "shutdown";
const EXIT: &str = "exit";

#[derive(Copy, Clone)]
enum Session {
    ExpectInitialize,
    Running,
    ExpectExit,
}
```

`step` matches `exit` first: `Step::End`. `Running` + `shutdown`: reply `result: Null`, set `ExpectExit`. `ExpectExit` + request: `InvalidRequest`. `ExpectExit` + notification other than `exit`: log, `Continue`. `event_from_notification`: `EXIT` is not reached.

Send still drops without `shutdown` / `exit`. EOF still ends the session.

Tests:

- `shutdown_then_exit_does_not_emit_quit`; a second TCP connection can still initialize + hello world
- `exit_without_shutdown_ends_the_session`; accept still works for a new connection

Design doc: `Quit` is `isograph stop` and SIGTERM. `shutdown` and `exit` close the session that sent them. They do not produce `Quit`.

## Call sites after all five

- `serve` -> bind -> port file -> `select!` `accept_loop`
- `accept_loop` -> `session` -> `step` -> `event_tx.send` / `Message::write`
- `isograph send` -> `notification_for_event` -> `handshake_and_notify`
- SIGTERM / `isograph stop` -> `IsographEvent::Quit` -> `handle` -> `Kill`. Not the socket.
