# The daemon port speaks LSP

Requires send-events.md (landed).

The TCP port in `{slug}.port` is LSP JSON-RPC (`Content-Length`). Same crate isograph uses: `lsp-server` 0.7.8 `Connection`, `lsp-types` 0.97. Pin the workspace to `0.7.8` (today `0.7.2`; isograph's lock is `0.7.8`).

isograph: `Connection::stdio()`, `server::initialize` (`connection.initialize(server_capabilities)`), loop on messages, unhandled requests `MethodNotFound`, then `drop(connection)` then `io_handles.join()`. That is the server. We do the same join order.

`handle` is unchanged. The session deserializes `isograph/event` params the way `on_message` deserializes a frame today, and sends `HelloWorld` / `Quit` / `DiskChanged` on `event_tx`. There is no `IsographEvent::Lsp`. Later docs that put dispatch in `handle` are stale; that is those docs' problem.

`isograph send` is an LSP client of that port: owned `TcpStream`, `Message::{read,write}`, initialize request, `initialized`, one notification, drop the stream. Not `Connection::connect`. `Connection::connect` builds `IoThreads` on a `try_clone`d fd; `join` waits for the writer until `sender` is dropped, and dropping `Connection` does not FIN while the reader thread still holds its clone. isograph is a long-lived stdio server, not a one-shot TCP client. Origin of the send path: `refactors/past/lsp-socket.md` `handshake_and_notify`.

The notification is `isograph/event`. Its params are the same JSON as `--file` today.

Each accepted TCP connection is one LSP client. `accept_loop` accepts forever. Each client is one `session`. Each `isograph send` is one client: connect, handshake, one notification, drop.

`session` is the per-client LSP: `Connection` from the stream, `connection.initialize`, then a pump. Requests get `MethodNotFound` on that connection. This slice: `session` is the only writer. `handle` is not request/response.

Not every `IsographEvent` is from the wire. SIGTERM still posts `Quit` in-process. Tests and the later watcher still post `HelloWorld` / `DiskChanged` / `Quit` in-process.

`std::thread::spawn` session threads keep the process alive after main returns. After `Kill` ends `select!` and the port file is unlinked, `std::process::exit(0)`. Do not return from `serve` into a process that waits on those threads. Flock is released on process exit. There is no session registry this slice.

The daemon installs `freddie_cli`'s panic hook (`log_panics`) before `run_daemon`. That hook `process::abort()`s. A panic on a session or IO thread kills every client of that config. Copied `make_reader` / `make_write` must not `unwrap`. `Message::read` / `send` / `write` `Err` ends that session. `#[cfg(test)]` does not install the hook; a unit test that "survives" a bad frame is not proof the daemon survives. The production assertion is `cli.rs`.

`Connection::initialize` still `unwrap`s `sender.send` inside `lsp-server`. A client that drops during handshake can still abort. Send does the handshake in order and does not hit that. This slice does not reimplement `initialize`.

One shippable change. Existing CLI send tests stay green: send owns the `TcpStream` and returns when `Message::write` of the notification returns. `Message::write` flushes.

## What is not isograph

isograph is one process per editor on stdio. i2 is one daemon per config. Send and later editors share the TCP port that send-events.md already binds. Everything below is that, or a later slice.

- `Connection::stdio` / `Connection::listen` are one client. We accept N streams on one bind, then `Connection { sender, receiver }` from each `TcpStream`. `socket_transport` is `pub(crate)`. Copy lives in `lsp_socket.rs` (bodies below). `lsp-server`'s `IoThreads` fields are private; constructing `lsp_server::IoThreads` from outside is impossible.
- Session is a std thread per stream. `Connection::initialize` blocks. N of those cannot sit on the daemon's current-thread runtime.
- `tokio::net::TcpListener`, `into_std` + `set_nonblocking(false)` on the runtime thread, then spawn `session`. Serve is already tokio. `into_std` streams are non-blocking; `Message::read` is not.
- After `Kill`, `process::exit(0)`. isograph has no leftover TCP session threads: editor disconnect ends that process.
- Code lives in `isograph_cli`, not `isograph_lsp`. The daemon is already `isograph_cli`. i2 `isograph_lsp` is encoding.
- Empty `ServerCapabilities`. isograph's `initialize` advertises tokens, hover, and the rest. Those land with later docs.
- No `LspState`, no `LSPNotificationDispatch` / `LSPRequestDispatch` in this slice. This slice is a `match` on the message in `session`.
- No `bridge_crossbeam_to_tokio`. isograph bridges so one `select!` can mix LSP, watcher, and debounce. We already have `run_event_loop`. The session thread iterates `connection.receiver`.
- `isograph/event`. isograph has no CLI event socket. send-events.md already sends `HelloWorld` / `DiskChanged` / `Quit`; the port is now LSP, so that JSON is a notification's params. Deserialize is in `session`, same as `on_message` today.
- Send uses `Message::{read,write}` on an owned `TcpStream`. `Connection::initialize` is server-only. There is no client initialize in `lsp-server`. `tokio-lsp` / `lsp-client-rs` / `async-lsp-client` are other stacks. A second client crate is not isograph. Do not add one.
- No 64 KiB frame cap. `freddie_event_socket` had one. Localhost; send fixtures stay small; the watcher is later.
- Copied IO does not `unwrap`. isograph can, because one process per editor.

`shutdown` is `MethodNotFound`, same as isograph (no `handle_shutdown`). Dropping the TCP connection ends that session after `drop(connection)` then `join`. `exit` ends it because `make_reader` stops on `exit`. Neither `Quit`s the daemon. SIGTERM still sends `IsographEvent::Quit` in-process.

## What the user does

```
$ printf '%s\n' '{"kind":"HelloWorld"}' > /tmp/hello.json
$ isograph send --file /tmp/hello.json
```

The log has `hello world`. The socket sees `initialize`, `initialized`, then `isograph/event` with params `{"kind":"HelloWorld"}`. Send does not wait for a result of that notification. Send's process exits.

```
$ printf '%s\n' '{"kind":"DiskChanged","value":{"path":"/tmp/proj/src/a.ts","presence":{"Present":"export const a = 1;\n"}}}' > /tmp/disk.json
$ isograph send --file /tmp/disk.json
```

Send is not in `--help`. The daemon not running is the same error as today.

```
$ isograph stop
```

After a send, `stop` without `--force` is SIGTERM. The log has `kill: exiting`. `status` is not running.

## Types

Most important first.

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

`--file` JSON is `IsographEventNotification::Params` (`HelloWorld` / `Quit` / `DiskChanged`). Origin of the empty enum: `lsp-types` 0.97 `notification::Initialized`. `IsographEvent` is unchanged (`PartialEq`, `Eq`, no `Lsp` variant).

```rust
// from crates/isograph_cli/src/lsp_socket.rs
use std::{
    io::{self, BufReader},
    net::TcpStream,
    thread,
};

use crossbeam::channel::{bounded, Receiver, Sender};
use lsp_server::Message;

fn socket_transport(
    stream: TcpStream,
) -> io::Result<(Sender<Message>, Receiver<Message>, IoThreads)> {
    let (reader_receiver, reader) = make_reader(stream.try_clone()?);
    let (writer_sender, writer) = make_write(stream);
    let io_threads = make_io_threads(reader, writer);
    (writer_sender, reader_receiver, io_threads).wrap_ok()
}

fn make_reader(stream: TcpStream) -> (Receiver<Message>, thread::JoinHandle<io::Result<()>>) {
    let (reader_sender, reader_receiver) = bounded::<Message>(0);
    let reader = thread::spawn(move || {
        let mut buf_read = BufReader::new(stream);
        while let Some(msg) = Message::read(&mut buf_read)? {
            let is_exit = matches!(&msg, Message::Notification(n) if n.is_exit());
            if reader_sender.send(msg).is_err() {
                break;
            }
            if is_exit {
                break;
            }
        }
        ().wrap_ok()
    });
    (reader_receiver, reader)
}

fn make_write(mut stream: TcpStream) -> (Sender<Message>, thread::JoinHandle<io::Result<()>>) {
    let (writer_sender, writer_receiver) = bounded::<Message>(0);
    let writer = thread::spawn(move || {
        writer_receiver
            .into_iter()
            .try_for_each(|it| it.write(&mut stream))
    });
    (writer_sender, writer)
}

fn make_io_threads(
    reader: thread::JoinHandle<io::Result<()>>,
    writer: thread::JoinHandle<io::Result<()>>,
) -> IoThreads {
    IoThreads { reader, writer }
}

struct IoThreads {
    reader: thread::JoinHandle<io::Result<()>>,
    writer: thread::JoinHandle<io::Result<()>>,
}

impl IoThreads {
    fn join(self) -> io::Result<()> {
        match self.reader.join() {
            Ok(r) => r?,
            Err(err) => std::panic::panic_any(err),
        }
        match self.writer.join() {
            Ok(r) => r,
            Err(err) => {
                std::panic::panic_any(err);
            }
        }
    }
}
```

Origin: `lsp-server` 0.7.8 `src/socket.rs` `socket_transport` / `make_reader` / `make_write` and `src/stdio.rs` `make_io_threads` / `IoThreads`. Delta: `use lsp_server::Message` not `crate::Message`; `use crossbeam::channel` not `crossbeam_channel` (workspace `crossbeam`); `try_clone` is `?`; `make_reader` / `make_write` return `io::Result` on the thread (`Message::read` `?`, `send` `is_err` breaks, writer `try_for_each` with no `unwrap`); `socket_transport` returns `io::Result`. `IoThreads::join` still `panic_any` if a thread panicked. Do not panic on a bad frame.

`bounded(0)` is a rendezvous. `MethodNotFound` send blocks until the writer thread takes it. Fine while the client is alive. `Connection::initialize_finish` errors if the next message is not `initialized`. Then `drop(connection)` then `join` waits on a reader still blocked on the fd until the client disconnects. Send does the handshake in order and does not hit this. A raw client that sends `initialize` then `isograph/event` with no `initialized` hangs that session until it drops the stream.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn connection_from_stream(
    stream: std::net::TcpStream,
) -> io::Result<(lsp_server::Connection, IoThreads)> {
    let (sender, receiver, io_threads) = socket_transport(stream)?;
    (lsp_server::Connection { sender, receiver }, io_threads).wrap_ok()
}
```

Origin: `lsp-server` 0.7.8 `Connection::listen` after `accept`. Delta: the `TcpStream` is already accepted; `socket_transport` can fail.

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
                let std_stream = match stream.into_std() {
                    Ok(std_stream) => std_stream,
                    Err(e) => {
                        debug!(error = %e, %peer, "could not take the lsp stream");
                        continue;
                    }
                };
                if let Err(e) = std_stream.set_nonblocking(false) {
                    debug!(error = %e, %peer, "could not set the lsp stream blocking");
                    continue;
                }
                std::thread::spawn(move || session(std_stream, event_tx));
            }
            Err(e) => debug!(error = %e, "accept failed"),
        }
    }
}

fn session(
    stream: std::net::TcpStream,
    event_tx: tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
) {
    let (connection, io_threads) = match connection_from_stream(stream) {
        Ok(pair) => pair,
        Err(e) => {
            debug!(error = %e, "lsp transport");
            return;
        }
    };
    run_session(&connection, event_tx);
    drop(connection);
    let _ = io_threads.join();
}

fn run_session(
    connection: &lsp_server::Connection,
    event_tx: tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
) {
    let capabilities = match serde_json::to_value(lsp_types::ServerCapabilities::default()) {
        Ok(value) => value,
        Err(e) => {
            debug!(error = %e, "server capabilities");
            return;
        }
    };
    if let Err(e) = connection.initialize(capabilities) {
        debug!(error = %e, "lsp initialize");
        return;
    }
    for message in &connection.receiver {
        match message {
            lsp_server::Message::Request(request) => {
                let _ = connection.sender.send(lsp_server::Message::Response(
                    lsp_server::Response {
                        id: request.id,
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

Origin of the loop: isograph `server.rs` `run` matching `Message`. Origin of the `MethodNotFound` body: isograph's unhandled arm. Origin of join order: isograph `lib.rs`. Origin of deserialize: `external.rs` `on_message`. `ProtocolError::new` is `pub(crate)`; do not call it. `ServerCapabilities::default` failing to serialize is logged and the session returns. `initialized` is consumed by `Connection::initialize`. `run_event_loop` still recvs `IsographEvent`. `handle` is not called from `session`. Accept is one task; no client id this slice.

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
    if let Err(e) = std::fs::write(port_path.reference(), format!("{port}\n")) {
        tracing::error!(
            error = %e,
            path = %port_path.display(),
            "could not write the event socket port"
        );
        return;
    }
    tracing::info!(config = %config_path.display(), port, "isograph daemon up");
    // SIGTERM as today
    let _hold_events = event_tx.clone();
    let mut state = IsographState::<THostLanguage>::default();
    intern_config_directory(&mut state, config_path.reference());
    tokio::select! {
        () = run_event_loop(state, event_rx, effect_tx) => {}
        () = run_effect_loop(effect_rx) => {}
        () = crate::lsp_socket::accept_loop(listener, event_tx) => {}
    }
    let _ = std::fs::remove_file(port_path.reference());
    std::process::exit(0);
```

The listen callback today is `freddie_event_socket`. Drop it. Delete `external.rs`. `on_message` goes away; deserialize is in `run_session`.

`_hold_events` is a clone. `accept_loop` takes the other sender. Today's code moves `event_tx` into `_hold_events` because there is no third `select!` arm.

`process::exit(0)` is after the port file is unlinked. Session threads and IO threads must not keep the process alive. Flock is released by process exit.

### `send`

```rust
// from crates/isograph_cli/src/send.rs
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
    #[error("could not encode the event: {0}")]
    Encode(serde_json::Error),
    #[error("could not clone the stream: {0}")]
    Clone(io::Error),
    #[error("could not write the frame: {0}")]
    Write(io::Error),
    #[error("could not read the frame: {0}")]
    Read(io::Error),
    #[error("the lsp connection closed")]
    Closed,
    #[error("{0}")]
    Lsp(String),
}
```

Keep `ReadFile` / `ReadPort` as they are today. Drop tungstenite. `Connect.source` is `io::Error`.

```rust
// from crates/isograph_cli/src/send.rs
    let event: crate::event::IsographEvent =
        serde_json::from_str(frame.trim()).map_err(SendError::NotEvent)?;
    let stream = std::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).map_err(
        |source| SendError::Connect(Connect { port, source }),
    )?;
    notify(stream, event)
```

```rust
// from crates/isograph_cli/src/send.rs
pub(crate) fn notify(
    stream: std::net::TcpStream,
    event: crate::event::IsographEvent,
) -> Result<(), SendError> {
    let mut writer = stream.try_clone().map_err(SendError::Clone)?;
    let mut reader = std::io::BufReader::new(stream);
    // JSON-RPC ids are per connection. This client has one outstanding request. A second send is another connection.
    let id = lsp_server::RequestId::from(1);
    lsp_server::Message::Request(lsp_server::Request {
        id: id.clone(),
        method: lsp_types::request::Initialize::METHOD.to_owned(),
        params: serde_json::json!({ "capabilities": {} }),
    })
    .write(&mut writer)
    .map_err(SendError::Write)?;
    wait_for_initialize_result(&mut reader, id.reference())?;
    lsp_server::Message::Notification(lsp_server::Notification {
        method: lsp_types::notification::Initialized::METHOD.to_owned(),
        params: serde_json::json!({}),
    })
    .write(&mut writer)
    .map_err(SendError::Write)?;
    let params = serde_json::to_value(&event).map_err(SendError::Encode)?;
    lsp_server::Message::Notification(lsp_server::Notification {
        method: crate::lsp_socket::IsographEventNotification::METHOD.to_owned(),
        params,
    })
    .write(&mut writer)
    .map_err(SendError::Write)?;
    ().wrap_ok()
}

fn wait_for_initialize_result(
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

Initialize params omit `processId`. Origin: `refactors/past/lsp-socket.md` `handshake_and_notify`. Dropping `writer` / `reader` closes the fd; that is the FIN. No `IoThreads`. Send does not wait after the notification and does not send `shutdown`.

The loop skips non-responses (`window/logMessage`, `$/` notifications). Send has one outstanding request, id `1`. A response with another id is a protocol bug, not a case we wait through.

`CliVerb::Send` doc comment: `Encode one IsographEvent as an LSP notification to the running daemon. Not for typing: tests and CI.` `SendArgs.file` stays `JSON frame to send.`

### Cargo

```toml
# from Cargo.toml
lsp-server = "0.7.8"
```

```toml
# from crates/isograph_cli/Cargo.toml
# drop freddie_event_socket, tungstenite, tokio-tungstenite, futures-util
crossbeam = { workspace = true }
lsp-server = { workspace = true }
lsp-types = { workspace = true }
tokio = { workspace = true, features = ["rt", "macros", "signal", "sync", "time", "net"] }
```

`lib.rs`: `mod lsp_socket;`

Do not add `derive_more`. Do not add `crossbeam-channel`.

### Design doc

Replace Outer and Ports in `docs-website/docs/design-docs/event-model.md` with the text below. Inner is unchanged: `handle` has no LSP and no socket. `handle` is not request/response.

```
## Outer

The binary is the outer. Each source is outside `handle` and feeds it. One process, one channel, one worker that owns `IsographState`. Sources do not read state. Performers do not mutate it.

- Watcher: OS notifications become `DiskChanged` (path plus contents or absent). It may read the disk to fill `Present.contents`. `handle` does not. The watcher posts in-process on the event channel. It does not run the CLI and it does not write to the LSP port.
- LSP port: LSP JSON-RPC on `{slug}.port`. `isograph send` is a client: initialize, `initialized`, notification `isograph/event` whose params are `HelloWorld` / `Quit` / `DiskChanged`. The session deserializes those params and posts that `IsographEvent`. An LSP request this slice is `MethodNotFound` in the session. Later, `textDocument/didOpen` / `didChange` / `didClose` become `EditorChanged`; hover and `semanticTokens/full` are request/response in the adapter. `handle` is not request/response. Effects from `handle` (`ReportDiagnostics`) become LSP notifications (`publishDiagnostics`).
- Effect loop: performs `WriteArtifacts`, `ReportDiagnostics`, `StartAsyncWork`, `Kill`.

`isograph lsp` is a stdio proxy onto the port. Walk-up / `--config` is the same as every other verb. It starts the daemon if needed, dials the port, and copies stdin/stdout. Dropping the editor drops the proxy and that connection. The daemon stays up. Several editors share one process. The vscode-extension already spawns `isograph lsp` on stdio.

`isograph send` is a hidden LSP client of that port. It does not start the daemon. It is not in `--help`.

The sources are theoretically separate daemons. They are one process because they share `Database` and because a socket hop on every save is the wrong latency.

## Ports

Figaro is one process per machine, so a default port is enough. Isograph is one process per config. Two configs cannot share a port.

The LSP port binds `127.0.0.1:0`. The kernel assigns a port from its local/dynamic range. There is no `--port`. After the lock, `run_daemon` unlinks the leftover `{slug}.port` before loading the config, then `serve` binds and writes the port to a sibling of its lock (`{slug}.lock` → `{slug}.port`). On quit, after `Kill` ends the loops, `serve` unlinks the file, then `process::exit(0)`. Flock is released when the holder dies. `isograph send` reads the lock, then that file. `Held::Free` is not running and the file is not consulted. Lock held and the file absent means the daemon has taken the lock and has not bound yet, or is on the way out; send fails. Send does not wait.

There is one listener. There is no `{slug}.lsp` and no `freddie_event_socket`. Until the watcher exists, `isograph send` is the only source of `DiskChanged` and carries `Present.contents` on the wire.
```

## Tests

`lsp_socket.rs`: bind `accept_loop`, 250ms settle. Socket tests assert `event_rx` only. They do not call `handle`. `handle_disk_changed` panics without `CurrentWorkingDirectory`; that intern is `state.rs`.

Happy path client is `TcpStream::connect` then `crate::send::notify`. Do not `try_recv` immediately after `notify` returns; `notify` does not wait for ingest. Sleep the settle, then `try_recv`.

These cases cannot use `notify` (`notify` always sends `initialize` first). They write `Message` on the stream:

- `isograph/event` before initialize: not an event; then initialize (and `initialized`) then a second HelloWorld arrives
- request before initialize: `ServerNotInitialized` (`Connection::initialize_start`, not `handle`)
- unknown request after initialize: `MethodNotFound` on the socket; then `isograph/event` HelloWorld on the same connection arrives as an event
- `shutdown` after initialize is `MethodNotFound` and does not `Quit`; then `isograph/event` HelloWorld on the same connection arrives
- truncated `Content-Length` body, or bytes that are not a JSON-RPC object: that connection ends (`Message::read` `Err`, reader thread returns, session `join`s); drop the stream so `join` is not waiting on a live fd; a second connection can initialize + HelloWorld
- `initialize` then `isograph/event` with no `initialized`: `initialize_finish` errors; that session stays in `join` until the test drops the stream; no event

`notify` cases (settle, then `event_rx`):

- initialize then `isograph/event` `HelloWorld`: `event_rx` is `HelloWorld`
- initialize then `isograph/event` DiskChanged present, then absent: `event_rx` is those two `DiskChanged` values (do not call `handle`)
- unknown notification after initialize: no event; then `isograph/event` HelloWorld arrives
- valid JSON-RPC `isograph/event` with params that are not `HelloWorld` / `Quit` / `DiskChanged`: connection stays; no event; a following HelloWorld arrives
- a second connection can initialize + HelloWorld
- two connections both HelloWorld
- `notify` returns after `isograph/event` (does not hang)
- two sequential `notify`s on the same listener, then a third still works

`state.rs`: existing `HelloWorld` / `Quit` / `DiskChanged` in-process tests stay. No `Lsp` tests.

`send.rs`: `parse_port` / `read_port` stay.

`cli.rs`: HelloWorld, DiskChanged present then absent, daemon stopped, not json, unknown kind, not in help stay. Add: after a successful send, `stop` without `--force` (unix `STOP`); log has `kill: exiting`; `status` is not running. Add: two sends, then that `stop`. Add: a raw TCP client writes a truncated body (or non-JSON-RPC bytes) to the daemon's port; a following `isograph send` HelloWorld still exits 0 and the log has `hello world`. That is the panic-hook assertion. The `lsp_socket` unit test is not it.

## Call sites

- `serve` -> `event_tx` -> `accept_loop` / SIGTERM `Quit` / `run_event_loop`
- `isograph/event` in `session` -> `event_tx.send` -> `handle`
- `isograph send` -> `TcpStream::connect` -> `notify` -> `isograph/event`
- watcher (later) -> `event_tx.send(DiskChanged)` -> `handle`, never the wire
- `Kill` -> unlink port file -> `process::exit(0)`
