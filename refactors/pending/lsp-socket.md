# LSP as the daemon's outside protocol

Requires send-events.md (landed). Independent of filesystem-watcher.md, file-semantic-tokens.md, and e2e-semantic-tokens.md.

The daemon's TCP port speaks LSP JSON-RPC (`Content-Length`). Every message is a request, a response, or a notification. `IsographEvent` is not on the wire. `--file` is still `IsographEvent` JSON; `isograph send` encodes it as a notification. `handle` is unchanged. Same `127.0.0.1:0` port and `{slug}.port` file.

`isograph/helloWorld` and `isograph/diskChanged` are notifications this slice understands. Whether an LSP client should report filesystem facts is later. `isograph/diskChanged` exists because send still interns a `DiskFile` that way.

`initialize` is a request. `initialized` is a notification and not an event. `shutdown` is a request. `exit` is a notification. They do not produce `IsographEvent::Quit`. `Quit` is SIGTERM / `isograph stop`.

`lsp_server::Connection` IO threads `unwrap`. We never use `Connection`. We use `Message::{read,write}` and construct `Request` / `Response` / `Notification` as structs. `Response::new_ok`, `Notification::new`, and `Request::new` `unwrap` `serde_json::to_value`; we do not call them. `Response::new_err` does not.

Later editor methods (`didOpen`, `semanticTokens/full`, `isograph lsp` stdio) use this listener. They are not these changes. Do not add a second query port.

Origin of bind, port file, and send: send-events.md. Origin of framing: `lsp-server` 0.7.8 `Message`. Delta: the wire is LSP, not a WebSocket text frame of `IsographEvent`.

Two independently shippable changes. Each leaves `isograph send` and the existing CLI tests working.

## What the user does after change 1

The daemon is already up. `--file` is still `IsographEvent` JSON. Send unlinks it after the attempt.

```
$ printf '%s\n' '{"kind":"HelloWorld"}' > /tmp/hello.json
$ isograph send --file /tmp/hello.json
```

The log has `hello world`. The socket sees `initialize`, `initialized`, then `isograph/helloWorld`. It does not see `{"kind":"HelloWorld"}`.

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

Change 2 does not change those commands. A client that sends `shutdown` then `exit` leaves the daemon up.

## Change 1: the port is LSP

Replace `freddie_event_socket` with a `tokio::net::TcpListener`. Each connection is an LSP session. Send is an LSP client: `initialize`, `initialized`, one domain notification, drop. Drop tungstenite, `tokio-tungstenite`, `futures-util`. Add `lsp-server`, `lsp-types`, tokio feature `net`. Delete `external.rs`. `lib.rs` gains `mod lsp_socket`.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
use std::io::{BufReader, Write};
use std::net::TcpStream;
use std::thread;

use lsp_server::{ErrorCode, Message, Notification, RequestId, Response};
use prelude::Postfix;
use tokio::net::TcpListener;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, warn};

use crate::event::{DiskChanged, IsographEvent};

pub(crate) const HELLO_WORLD: &str = "isograph/helloWorld";
pub(crate) const DISK_CHANGED: &str = "isograph/diskChanged";

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
        INITIALIZED => None,
        method => {
            warn!(method, "unknown notification");
            None
        }
    }
}
```

`HELLO_WORLD` ignores params. `INITIALIZED` is not an event. Unknown method: no event, connection stays up. Bad `DiskChanged` params: no event, connection stays up. `DiskChanged` params are `{path, presence}`, not `{kind, value}`.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
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
    let mut session = Session::ExpectInitialize;
    loop {
        let message = match Message::read(&mut reader) {
            Ok(Some(message)) => message,
            Ok(None) => break,
            Err(e) => {
                debug!(error = %e, "lsp connection ended");
                break;
            }
        };
        match step(&mut session, message, event_tx.reference()) {
            Step::Continue => {}
            Step::Reply(response) => {
                if let Err(e) = Message::Response(response).write(&mut writer) {
                    debug!(error = %e, "could not write the lsp response");
                    break;
                }
            }
            Step::End => break,
        }
    }
}

fn step(
    session: &mut Session,
    message: Message,
    event_tx: &UnboundedSender<IsographEvent>,
) -> Step {
    match (*session, message) {
        (Session::ExpectInitialize, Message::Request(request)) if request.method == INITIALIZE => {
            let id = request.id;
            match initialize_response(id.clone()) {
                Ok(response) => {
                    *session = Session::Running;
                    Step::Reply(response)
                }
                Err(e) => {
                    warn!(error = %e, "could not encode initialize result");
                    Step::Reply(Response::new_err(
                        id,
                        ErrorCode::InternalError as i32,
                        "could not encode initialize result".to_owned(),
                    ))
                }
            }
        }
        (Session::ExpectInitialize, Message::Request(request)) => Step::Reply(Response::new_err(
            request.id,
            ErrorCode::ServerNotInitialized as i32,
            "server not initialized".to_owned(),
        )),
        (Session::ExpectInitialize, Message::Notification(notification)) => {
            warn!(method = notification.method.as_str(), "notification before initialize");
            Step::Continue
        }
        (Session::ExpectInitialize, Message::Response(_)) => Step::Continue,
        (Session::Running, Message::Request(request)) => Step::Reply(Response::new_err(
            request.id,
            ErrorCode::MethodNotFound as i32,
            format!("{} is not a request this server answers", request.method),
        )),
        (Session::Running, Message::Notification(notification)) => {
            if let Some(event) = event_from_notification(notification) {
                let _ = event_tx.send(event);
            }
            Step::Continue
        }
        (Session::Running, Message::Response(_)) => Step::Continue,
    }
}
```

`into_std` streams are non-blocking. `Message::read` is blocking `BufRead`. `set_nonblocking(false)` is required. The session is a `std::thread` so a blocked read does not stall the current-thread tokio runtime. Empty `ServerCapabilities`. A second `initialize` after `Running` is `MethodNotFound`. `RequestId` is not `Copy`; clone it for `initialize_response`. Malformed LSP closes that connection. Unknown notifications after `initialize` do not.

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

`_hold_events` still holds a sender. `Kill` ends the effect loop, `select!` drops `accept_loop`, the listener closes. Session threads then see a read error. `run_event_loop`, `handle`, SIGTERM sending `Quit`, and the port file path are unchanged.

```rust
// from crates/isograph_cli/src/send.rs
    let event: IsographEvent =
        serde_json::from_str(frame.trim()).map_err(SendError::NotEvent)?;
    let notification = lsp_socket::notification_for_event(event)?;
    let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).map_err(|source| {
        SendError::Connect(Connect { port, source })
    })?;
    handshake_and_notify(stream, notification)
```

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
        let message = Message::read(reader).map_err(SendError::Read)?;
        let Some(message) = message else {
            return SendError::InitializeClosed.wrap_err();
        };
        let Message::Response(response) = message else {
            continue;
        };
        if &response.id != expected {
            continue;
        }
        match response.error {
            None => return ().wrap_ok(),
            Some(error) => return SendError::Initialize(error.message).wrap_err(),
        }
    }
}
```

`Connect.source` is `io::Error`. `SendError` keeps Discover / NotEvent / NotRunning / port errors, drops tungstenite, gains `Notification(NotificationError)` with `#[from]`, `Write(io::Error)`, `Read(io::Error)`, `InitializeClosed`, `Initialize(String)`. `require_running`, `read_port`, `parse_port`, `run` (unlink `--file`) stay. Send does not send `shutdown` / `exit`.

`CliVerb::Send` doc comment: `Encode one IsographEvent as an LSP notification to the running daemon. Not for typing: tests and CI.` `SendArgs.file`: `JSON IsographEvent to encode as an LSP notification.`

Update `docs-website/docs/design-docs/event-model.md` Outer / Ports: the TCP port speaks LSP; send is an LSP client of it; drop `freddie_event_socket` and the `{slug}.lsp` second listener. A `Quit` JSON frame is gone. SIGTERM / `isograph stop` still `Quit`.

Landing sequence `refactors/pending/event-model.md` item 2 notes the wire is this file. Item 13 is further methods on this listener.

### Tests

`lsp_socket.rs`: bind `accept_loop` on a tokio test runtime, `TcpStream::connect`, write `Message`s, drain `event_rx` after 250ms (same settle as today's `external.rs`). `expect` names the fixture.

- `notification_for_event` HelloWorld method; DiskChanged present params contain the path; Quit is `QuitOnWire`
- `initialize_then_hello_world_is_an_event`; initialize result has empty `capabilities`
- `initialize_then_disk_changed_present_is_an_event`
- `initialize_then_disk_changed_absent_is_an_event`
- `hello_world_before_initialize_is_not_an_event`; then initialize + hello world on the same connection works
- `unknown_notification_after_initialize_is_dropped`; then hello world on the same connection arrives
- `unknown_request_after_initialize_is_method_not_found` (`-32601`); then hello world arrives
- `request_before_initialize_is_server_not_initialized` (`-32002`)
- `malformed_payload_closes_the_connection`
- `two_connections_both_hello_world`

`send.rs`: existing `parse_port` / `read_port` tests stay.

`cli.rs`: HelloWorld, DiskChanged present then absent, daemon stopped, not json, unknown kind, not in help stay. `--file` JSON is unchanged. Add `send_of_quit_fails`: exit 1, stderr contains `Quit is not sent on this socket`, `--file` unlinked, log has no `kill: exiting`.

## Change 2: `shutdown` / `exit` close the session

Origin: change 1 `Session`. Delta: `ExpectExit`. `exit` in any state ends the thread and does not send `IsographEvent::Quit`.

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

`step` matches `exit` first: `Step::End`. `Running` + `shutdown`: reply `result: Null`, set `ExpectExit`. `ExpectExit` + request: `InvalidRequest`. `ExpectExit` + notification other than `exit`: log, `Continue`. `event_from_notification` does not see `EXIT`.

Send still drops without `shutdown` / `exit`. EOF still ends the session.

Tests:

- `shutdown_then_exit_does_not_emit_quit`; a second TCP connection can still initialize + hello world
- `exit_without_shutdown_ends_the_session`; accept still works for a new connection

Design doc: `Quit` is `isograph stop` and SIGTERM. `shutdown` and `exit` close the session that sent them. They do not produce `Quit`.

## Call sites after both

- `serve` -> bind -> port file -> `select!` `accept_loop`
- `accept_loop` -> `session` -> `step` -> `event_tx.send` / `Message::write`
- `isograph send` -> `notification_for_event` -> `handshake_and_notify`
- SIGTERM / `isograph stop` -> `IsographEvent::Quit` -> `handle` -> `Kill`. Not the socket.
