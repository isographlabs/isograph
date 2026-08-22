# LSP as the daemon's outside protocol

Requires send-events.md (landed). The daemon's TCP port speaks LSP JSON-RPC (`Content-Length` framing). `isograph send` is an LSP client: `initialize`, `initialized`, one domain notification, then it drops the connection. Domain notifications this slice handles are `isograph/helloWorld` and `isograph/diskChanged`. `shutdown` / `exit` end that session. They do not produce `IsographEvent::Quit`. `Quit` stays SIGTERM / `isograph stop`.

Origin of the bind, port file, and `isograph send` as a hidden client: send-events.md. Origin of message types and framing: `lsp-server` 0.7.8 `Message`, `Request`, `Response`, `Notification`, `Message::read`, `Message::write`. Origin of session vs process: `docs-website/docs/design-docs/event-model.md` (several editors share one process). Delta: replace `freddie_event_socket` WebSocket text frames of `IsographEvent` JSON; the same `127.0.0.1:0` port and `{slug}.port` file; `--file` is still `IsographEvent` JSON and send encodes it; `handle` is unchanged.

One shippable change.

`lsp_server::Connection` IO threads `unwrap`. We do not use `Connection`. We use `Message::{read,write}` and construct `Request` / `Response` / `Notification` as structs. `Response::new_ok` and `Notification::new` / `Request::new` also `unwrap` `serde_json::to_value`; we do not call them. `Response::new_err` does not.

Later LSP methods (editor `didOpen`, `semanticTokens/full`, `isograph lsp` stdio) use this same listener and the same session machine. They are not this slice. Do not add a second query port.

## What the user does

The daemon is already up. `--file` is still `IsographEvent` JSON. Send unlinks it after the attempt.

```
$ isograph logs
{"timestamp":"...","level":"INFO","fields":{"message":"isograph daemon up","config":"/Users/x/app/isograph.config.json","port":53124}}
$ printf '%s\n' '{"kind":"HelloWorld"}' > /tmp/hello.json
$ isograph send --file /tmp/hello.json
$ isograph logs
{"timestamp":"...","level":"INFO","fields":{"message":"hello world"}}
```

```
$ printf '%s\n' '{"kind":"DiskChanged","value":{"path":"/tmp/proj/src/a.ts","presence":{"Present":"export const a = 1;\n"}}}' > /tmp/disk.json
$ isograph send --file /tmp/disk.json
```

```
$ printf '%s\n' '{"kind":"Quit"}' > /tmp/quit.json
$ isograph send --file /tmp/quit.json
Quit is not sent on this socket
```

Exit 1. `isograph stop` still kills the daemon. A client that sends LSP `shutdown` then `exit` on a connection leaves the daemon up.

```
$ isograph send --file /tmp/hello.json
the daemon is not running
```

Same errors as today when the lock is free or the port file is missing. Send does not wait. Send is not in `--help`.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
use std::io::{self, BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::thread;

use lsp_server::{ErrorCode, Message, Notification, Request, RequestId, Response};
use prelude::Postfix;
use tokio::net::TcpListener;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, warn};

use crate::event::{DiskChanged, IsographEvent};

pub(crate) const HELLO_WORLD: &str = "isograph/helloWorld";
pub(crate) const DISK_CHANGED: &str = "isograph/diskChanged";

const INITIALIZE: &str = "initialize";
const INITIALIZED: &str = "initialized";
const SHUTDOWN: &str = "shutdown";
const EXIT: &str = "exit";

#[derive(Copy, Clone)]
enum Session {
    ExpectInitialize,
    Running,
    ExpectExit,
}
```

`Session` is the connection, not the process. `ExpectInitialize`: only `initialize` is answered; other requests get `ServerNotInitialized`; notifications other than `exit` are dropped. `exit` in any state ends the session thread and does not send `IsographEvent::Quit`. `Running`: domain notifications become events; `shutdown` is answered with `null` and the session becomes `ExpectExit`; unknown notifications are logged and dropped; unknown requests get `MethodNotFound`. `ExpectExit`: only `exit` is valid; other messages get `InvalidRequest`.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
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
        EXIT => None,
        method => {
            warn!(method, "unknown notification");
            None
        }
    }
}
```

`HELLO_WORLD` ignores params. `INITIALIZED` is not an event. `EXIT` is handled in `session` before this function. Unknown methods are not events. A `DiskChanged` whose params are not `DiskChanged` JSON is dropped; the connection stays up.

`--file` is still `IsographEvent`. Send calls `notification_for_event` after deserialize. The wire JSON for `isograph/diskChanged` params is the `DiskChanged` struct (`path`, `presence`), not the tagged `{kind, value}` envelope.

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
```

Empty `ServerCapabilities`. Domain requests are not this slice; `MethodNotFound` is the answer until a later doc.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
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

enum Step {
    Continue,
    Reply(Response),
    End,
}
```

`into_std` streams are non-blocking. `Message::read` is blocking `BufRead`. `set_nonblocking(false)` is required. The session lives on a `std::thread` so a blocked read does not stall the current-thread tokio runtime. `accept_loop` is a `select!` arm in `serve`; `Kill` ends the effect loop, `select!` drops `accept_loop`, the listener closes. Session threads then see a read error. `event_tx.send` uses `let _ =`.

A malformed LSP header or payload is `Message::read` `Err`. That connection closes. Unknown notifications after `initialize` do not close it.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn step(
    session: &mut Session,
    message: Message,
    event_tx: &UnboundedSender<IsographEvent>,
) -> Step {
    match (*session, message) {
        (_, Message::Notification(notification)) if notification.method == EXIT => Step::End,
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
        (Session::Running, Message::Request(request)) if request.method == SHUTDOWN => {
            *session = Session::ExpectExit;
            Step::Reply(Response {
                id: request.id,
                result: serde_json::Value::Null.wrap_some(),
                error: None,
            })
        }
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
        (Session::ExpectExit, Message::Request(request)) => Step::Reply(Response::new_err(
            request.id,
            ErrorCode::InvalidRequest as i32,
            "shutdown already received".to_owned(),
        )),
        (Session::ExpectExit, Message::Notification(notification)) => {
            warn!(method = notification.method.as_str(), "notification after shutdown");
            Step::Continue
        }
        (Session::ExpectExit, Message::Response(_)) => Step::Continue,
    }
}
```

`EXIT` is matched first so it ends the session in every state. A second `initialize` after `Running` is `MethodNotFound`. `step` does not write; `session` writes `Reply`. `RequestId` is not `Copy`; clone it for `initialize_response` so the encode-failure arm still has `id`. `InitializeResult` encode failing is a `lsp_types` serde bug; the error response is still required so the client is not left hanging.

### `serve`

Origin: `crates/isograph_cli/src/daemon.rs` `serve`. Delta: `tokio::net::TcpListener` instead of `freddie_event_socket::listen`; `accept_loop` as a third `select!` arm; port from `listener.local_addr()`.

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

`_hold_events` still holds a sender so the event loop does not end on its own. `accept_loop` clones per connection. Drop `freddie_event_socket` from this file. `run_event_loop`, `handle`, SIGTERM sending `Quit`, and the port file path are unchanged.

### `send`

Origin: `crates/isograph_cli/src/send.rs`. Delta: `TcpStream` plus `Message` instead of tungstenite; `notification_for_event`; wait for the `initialize` response before the domain notification.

```rust
// from crates/isograph_cli/src/send.rs
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::net::{Ipv4Addr, TcpStream};
use std::num::NonZeroU16;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use lsp_server::{Message, Notification, Request, RequestId, Response};
use prelude::Postfix;

use crate::SendArgs;
use crate::discover::DiscoverError;
use crate::event::IsographEvent;
use crate::lsp_socket::{self, NotificationError};

#[derive(Debug)]
struct Connect {
    pub port: u16,
    pub source: io::Error,
}

#[derive(Debug, thiserror::Error)]
enum SendError {
    #[error("{0}")]
    Discover(#[from] DiscoverError),
    #[error("could not read {}: {}", .0.path.display(), .0.source)]
    ReadFile(ReadFile),
    #[error("the frame is not IsographEvent JSON: {0}")]
    NotEvent(serde_json::Error),
    #[error("{0}")]
    Notification(#[from] NotificationError),
    #[error("the daemon is not running")]
    NotRunning,
    #[error("the daemon has not recorded its pid yet")]
    Unnamed,
    #[error("{0}")]
    Lock(#[from] freddie_single_instance::LockError),
    #[error("the daemon has not recorded its port yet")]
    NoPort,
    #[error("could not read {}: {}", .0.path.display(), .0.source)]
    ReadPort(ReadPort),
    #[error("the daemon's port file is not a port")]
    BadPort,
    #[error("could not connect to 127.0.0.1:{}: {}", .0.port, .0.source)]
    Connect(Connect),
    #[error("could not write an lsp message: {0}")]
    Write(io::Error),
    #[error("could not read an lsp message: {0}")]
    Read(io::Error),
    #[error("the daemon closed the lsp connection before initialize completed")]
    InitializeClosed,
    #[error("initialize failed: {0}")]
    Initialize(String),
}

fn run_inner(args: &SendArgs) -> Result<(), SendError> {
    let (_, instance) = crate::discover::instance_for_config_path(args.id.config.as_deref())?;
    require_running(instance.lock_file())?;
    let port = read_port(&crate::discover::port_file(instance.lock_file()))?;
    let frame = fs::read_to_string(args.file.reference()).map_err(|source| {
        SendError::ReadFile(ReadFile {
            path: args.file.clone(),
            source,
        })
    })?;
    let event: IsographEvent =
        serde_json::from_str(frame.trim()).map_err(SendError::NotEvent)?;
    let notification = lsp_socket::notification_for_event(event)?;
    let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).map_err(|source| {
        SendError::Connect(Connect {
            port,
            source,
        })
    })?;
    handshake_and_notify(stream, notification)
}

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
            Some(Message::Response(_)) | Some(Message::Notification(_)) => {}
            Some(Message::Request(_)) => {}
        }
    }
}
```

`require_running`, `read_port`, `parse_port`, `ReadFile`, `ReadPort`, `run` (unlink `--file`, print stderr) stay. `RequestId` is cloned once for the initialize write and the wait. Send does not send `shutdown` / `exit`; it drops the socket. The server sees EOF and ends the session thread.

`SendArgs` doc comment: `JSON IsographEvent to encode as an LSP notification.` `CliVerb::Send` doc comment: `Encode one IsographEvent as an LSP notification to the running daemon. Not for typing: tests and CI.`

### Cargo

```toml
# from crates/isograph_cli/Cargo.toml
# drop freddie_event_socket, tungstenite
# drop dev-dependencies tokio-tungstenite, futures-util
lsp-server = { workspace = true }
lsp-types = { workspace = true }
tokio = { workspace = true, features = ["rt", "macros", "signal", "sync", "time", "net"] }
```

`lib.rs` `mod lsp_socket;`. Delete `external.rs`.

### Design doc

Origin: `docs-website/docs/design-docs/event-model.md`. Delta: the TCP port is LSP; send is an LSP client; `Quit` is not on the socket; `shutdown` / `exit` are session-only; no `{slug}.lsp` listener in this slice.

```markdown
# from docs-website/docs/design-docs/event-model.md
- Watcher: OS notifications become `DiskChanged` (path plus contents or absent). It may read the disk to fill `Present.contents`. `handle` does not. The watcher posts in-process on the event channel. It does not run the CLI and it does not write to the LSP socket.
- LSP socket: LSP JSON-RPC on `127.0.0.1` with `Content-Length` framing. Each accepted TCP connection is one session. Notifications the server knows become `IsographEvent` and go through `handle`. `initialize` / `shutdown` / `exit` are session lifecycle and do not go through `handle`. Domain requests (hover, `semanticTokens/full`, …) are request/response in the session: they read state, they do not run `handle`. Effects from `handle` (`ReportDiagnostics`) become LSP notifications (`publishDiagnostics`) on sessions that are still up.
- Effect loop: performs `WriteArtifacts`, `ReportDiagnostics`, `StartAsyncWork`, `Kill`.

`isograph lsp` is a stdio proxy onto this socket. Walk-up / `--config` is the same as every other verb. It starts the daemon if needed, dials the port, and copies stdin/stdout. Dropping the editor drops the proxy and that connection. The daemon stays up. Several editors share one process. The vscode-extension already spawns `isograph lsp` on stdio. The proxy is not this slice.

`isograph send` is a hidden LSP client of the same socket. It does not start the daemon. It is not in `--help`. `--file` is `IsographEvent` JSON. Send encodes `HelloWorld` and `DiskChanged` as notifications `isograph/helloWorld` and `isograph/diskChanged`. It does not encode `Quit`.
```

```markdown
# from docs-website/docs/design-docs/event-model.md
## Ports

Figaro is one process per machine, so a default port is enough. Isograph is one process per config. Two configs cannot share a port.

The LSP socket binds `127.0.0.1:0`. The kernel assigns a port from its local/dynamic range. There is no `--port`. After the lock, `run_daemon` unlinks the leftover `{slug}.port` before loading the config, then `serve` binds and writes the assigned port to a sibling of its lock (`{slug}.lock` → `{slug}.port`). On quit, after `Kill` ends the loops, `serve` unlinks the file, then returns, then the lock drops. `isograph send` reads the lock, then that file. `Held::Free` is not running and the file is not consulted. Lock held and the file absent means the daemon has taken the lock and has not bound yet, or is on the way out; send fails. Send does not wait.

A watcher (later) reads the file and posts `DiskChanged` in-process, so production contents do not go over the socket. Until that watcher exists, `isograph send` is the only source of `DiskChanged` and carries `Present.contents` on the wire.
```

```markdown
# from docs-website/docs/design-docs/event-model.md
Ingested events come from outside `handle`: `DiskChanged`, `EditorChanged`. The CLI can submit `HelloWorld` and `DiskChanged` through send. The watcher submits `DiskChanged`. The editor session (later) submits `EditorChanged`.

The wire is LSP. A domain notification the server does not know is logged and dropped. The connection stays up. A malformed LSP message closes that connection.

`Quit` is `isograph stop` and SIGTERM. `handle` returns `Kill`. `shutdown` and `exit` close the LSP session that sent them. They do not produce `Quit`.
```

Landing sequence `refactors/pending/event-model.md` item 2 notes the wire is lsp-socket.md. Item 13 is further methods on this listener, not a second protocol.

## Tests

### `lsp_socket.rs`

Helpers in the test module: bind `accept_loop` on a tokio test runtime, `TcpStream::connect`, write `Message`s, `event_rx.try_recv` after a short settle (250ms, same as today's `external.rs`).

- `initialize_then_hello_world_is_an_event`: `initialize` request, read one `Response` with empty `capabilities`, `initialized`, `isograph/helloWorld`, `event_rx` is `HelloWorld`.
- `initialize_then_disk_changed_present_is_an_event`: params `{path, presence: {Present: "export const a = 1;\n"}}`. Event equals today's `DiskChanged` present fixture.
- `initialize_then_disk_changed_absent_is_an_event`.
- `hello_world_before_initialize_is_not_an_event`. A following `initialize` + `helloWorld` still works on the same connection.
- `unknown_notification_after_initialize_is_dropped`: method `nope`, no event, then `isograph/helloWorld` still arrives.
- `unknown_request_after_initialize_is_method_not_found`: read the `Response`, `error.code` is `-32601`, connection still takes `isograph/helloWorld`.
- `request_before_initialize_is_server_not_initialized`: code `-32002`.
- `shutdown_then_exit_does_not_emit_quit`: `event_rx` empty of `Quit`. A second TCP connection can still `initialize` + `helloWorld`.
- `exit_without_shutdown_ends_the_session`: further writes fail or reads return `None`. Daemon accept still works for a new connection.
- `malformed_payload_closes_the_connection`: write `Content-Length` of `{`. Next `Message::read` on the client is `Err` or `None`.
- `notification_for_event_quit_is_quit_on_wire`.
- `notification_for_event_hello_world_method`.
- `two_connections_both_hello_world`: two events.

Do not add a production function only the tests call. Drain `event_rx`. `expect` names the fixture the test wrote.

### `send.rs`

Existing `parse_port` / `read_port` tests stay.

### `cli.rs`

`the_log_contains_the_config_path` (HelloWorld), `send_of_disk_changed_present_then_absent_exits_0`, `send_with_the_daemon_stopped_fails`, `send_of_not_json_fails`, `send_of_unknown_kind_fails`, `send_is_not_in_help` stay. `--file` JSON is unchanged except:

- `send_of_quit_fails`: `--file` `{"kind":"Quit"}`, exit 1, stderr contains `Quit is not sent on this socket`, `--file` unlinked, daemon log has no `kill: exiting`.

## Call sites

- `serve` -> bind -> port file -> `select!` `accept_loop`.
- `accept_loop` -> `session` -> `step` -> `event_tx.send` / `Message::write`.
- `isograph send` -> `notification_for_event` -> `handshake_and_notify`.
- SIGTERM / `isograph stop` -> `IsographEvent::Quit` -> `handle` -> `Kill`. Not the socket.
