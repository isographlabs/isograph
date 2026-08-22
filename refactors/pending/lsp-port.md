# The daemon port speaks LSP

Requires send-events.md (landed).

The TCP port in `{slug}.port` is LSP JSON-RPC (`Content-Length`). Same crate isograph uses: `lsp-server` 0.7.8 `Connection`, `lsp-types` 0.97. Pin the workspace to `0.7.8` (today `0.7.2`; isograph's lock is `0.7.8`).

isograph: `Connection::stdio()`, `server::initialize` (`connection.initialize(server_capabilities)`), loop on messages, unhandled requests `MethodNotFound`, then `drop(connection)` then `io_handles.join()`. That is the server. We do the same join order.

`isograph send` is an LSP client of that port: owned `TcpStream`, `Message::{read,write}`, initialize request, `initialized`, one notification, drop the stream. Not `Connection::connect`. `Connection::connect` builds `IoThreads` on a `try_clone`d fd; `join` waits for the writer until `sender` is dropped, and dropping `Connection` does not FIN while the reader thread still holds its clone. isograph is a long-lived stdio server, not a one-shot TCP client. Origin of the send path: `refactors/past/lsp-socket.md` `handshake_and_notify`.

The notification is `isograph/event`. Its params are the same JSON as `--file` today (`HelloWorld` / `Quit` / `DiskChanged`).

Each accepted TCP connection is one LSP client. `accept_loop` accepts forever. Each client is one `session`. Each `isograph send` is one client: connect, handshake, one notification, drop.

`session` is the per-client LSP: `Connection` from the stream, `connection.initialize`, then a pump. After initialize, every `lsp_server::Message` becomes `IsographEvent::Lsp` on `event_tx` (the message plus that client's id). Requests also get `MethodNotFound` on that connection. This slice: `session` is the only writer. When dispatch grows, that `MethodNotFound` send moves with it into `handle` (writer keyed by `LspClientId`). lsp-tokens.md cannot land until the reply path is one place; two writers is two responses.

Not every `IsographEvent` is LSP. SIGTERM still posts `Quit` in-process. Tests and the later watcher still post `HelloWorld` / `DiskChanged` / `Quit` in-process. The wire is always `Lsp`.

`std::thread::spawn` session threads keep the process alive after main returns. After `Kill` ends `select!` and the port file is unlinked, `std::process::exit(0)`. Do not return from `serve` into a process that waits on those threads. Flock is released on process exit. There is no session registry this slice.

One shippable change. Existing CLI send tests stay green: send owns the `TcpStream` and returns when `Message::write` of the notification returns.

## What is not isograph

isograph is one process per editor on stdio. i2 is one daemon per config. Send and later editors share the TCP port that send-events.md already binds. Everything below is that, or a later slice.

- `Connection::stdio` / `Connection::listen` are one client. We accept N streams on one bind, then `Connection { sender, receiver }` from each `TcpStream`. `socket_transport` is `pub(crate)`. Copy `lsp-server` 0.7.8 `src/socket.rs` `socket_transport` / `make_reader` / `make_write` and `src/stdio.rs` `make_io_threads` / `IoThreads` into `lsp_socket.rs`. Delta: `use crate::Message` becomes `use lsp_server::Message`; both copies live in this module, so there is no `crate::stdio`. Do not change `unwrap`s. They panic a session thread, not `handle`.
- Session is a std thread per stream. `Connection::initialize` blocks. N of those cannot sit on the daemon's current-thread runtime.
- `tokio::net::TcpListener`, `into_std` + `set_nonblocking(false)` on the runtime thread, then spawn `session`. Serve is already tokio. `into_std` streams are non-blocking; `Message::read` is not.
- After `Kill`, `process::exit(0)`. isograph has no leftover TCP session threads: editor disconnect ends that process.
- Code lives in `isograph_cli`, not `isograph_lsp`. The daemon is already `isograph_cli`. i2 `isograph_lsp` is encoding.
- Empty `ServerCapabilities`. isograph's `initialize` advertises tokens, hover, and the rest. Those land with lsp-tokens.md / later docs.
- No `LspState`, no `LSPNotificationDispatch` / `LSPRequestDispatch` in this slice. isograph's dispatch chain (`on_notification_sync` / `on_request_sync`) lands later inside `handle`'s `IsographEvent::Lsp` match, not in `session`. This slice is a `match` on the message.
- No `bridge_crossbeam_to_tokio`. isograph bridges so one `select!` can mix LSP, watcher, and debounce. We already have `run_event_loop`. The session thread iterates `connection.receiver`.
- `isograph/event`. isograph has no CLI event socket. send-events.md already sends `HelloWorld` / `DiskChanged` / `Quit`; the port is now LSP, so that JSON is a notification's params. The channel event is `Lsp`, not those variants.
- Send uses `Message::{read,write}` on an owned `TcpStream`. `Connection::initialize` is server-only. There is no client initialize in `lsp-server`. `tokio-lsp` / `lsp-client-rs` / `async-lsp-client` are other stacks. A second client crate is not isograph. Do not add one.
- No 64 KiB frame cap. `freddie_event_socket` had one. Localhost; send fixtures stay small; the watcher is later.

`shutdown` is `MethodNotFound`, same as isograph (no `handle_shutdown`). Dropping the TCP connection ends that session after `drop(connection)` then `join`. `exit` ends it because copied `make_reader` stops on `exit`. Neither `Quit`s the daemon. SIGTERM still sends `IsographEvent::Quit` in-process.

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
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LspClientId(u64);

#[derive(Debug)]
pub(crate) struct Lsp {
    pub(crate) client: LspClientId,
    pub(crate) message: lsp_server::Message,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, derive_more::From)]
#[serde(tag = "kind", content = "value")]
pub enum IsographEvent {
    HelloWorld,
    Quit,
    DiskChanged(DiskChanged),
    #[serde(skip)]
    #[from]
    Lsp(Lsp),
}
```

`--file` JSON is `HelloWorld` / `Quit` / `DiskChanged`. `Lsp` is not on the wire as a `kind`. `LspClientId` is monotonic per daemon process, never reused. The field exists so `IsographEvent::Lsp` is the shape later slices write back with; lsp-sessions.md is the first production reader. Tests comparing ids are not why it exists. Origin of `From`: `derive_more` 2. `#[from]` only on `Lsp` so there is no `From<DiskChanged>`. strum is already in the workspace; it is `Display` / `FromStr` / `FromRepr`, not `From<payload>`. `enum_derive` is unit variants only. Drop `PartialEq` / `Eq` on `IsographEvent`: `lsp_server::Message` does not implement them. Tests use `matches!`.

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
                std::thread::spawn(move || session(std_stream, event_tx, client));
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
    run_session(&connection, event_tx, client);
    drop(connection);
    let _ = io_threads.join();
}

fn run_session(
    connection: &lsp_server::Connection,
    event_tx: tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
    client: crate::event::LspClientId,
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
}
```

Origin of the loop: isograph `server.rs` `run` matching `Message`. Origin of the `MethodNotFound` body: isograph's unhandled arm. Origin of join order: isograph `lib.rs` `drop` of `connection` by moving it into `run`, then `io_handles.join()`. `ProtocolError::new` is `pub(crate)`; do not call it. `ServerCapabilities::default` failing to serialize is logged and the session returns. `initialized` is consumed by `Connection::initialize`. `run_event_loop` still recvs `IsographEvent`.

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

`handle` already matches `HelloWorld` / `Quit` / `DiskChanged`. The `Lsp` arm of `isograph/event` deserializes those and calls `handle` with them. Other LSP messages are no-ops this slice. Recursion does not see `Lsp`: params cannot deserialize to it.

Later, this `match message` is where isograph's `LSPNotificationDispatch` / `LSPRequestDispatch` go. Not now. Not in `session`.

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

The listen callback today is `freddie_event_socket`. Drop it. Delete `external.rs`. `on_message` goes away.

`_hold_events` is a clone. `accept_loop` takes the other sender. Today's code moves `event_tx` into `_hold_events` because there is no third `select!` arm.

`process::exit(0)` is after the port file is unlinked. Session threads and copied IO threads must not keep the process alive. Flock is released by process exit.

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
    let mut writer = stream.try_clone().map_err(SendError::Write)?;
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

`CliVerb::Send` doc comment: `Encode one IsographEvent as an LSP notification to the running daemon. Not for typing: tests and CI.`

### Cargo

```toml
# from Cargo.toml
lsp-server = "0.7.8"
```

```toml
# from crates/isograph_cli/Cargo.toml
# drop freddie_event_socket, tungstenite, tokio-tungstenite, futures-util
derive_more = { version = "2", features = ["from"] }
lsp-server = { workspace = true }
lsp-types = { workspace = true }
tokio = { workspace = true, features = ["rt", "macros", "signal", "sync", "time", "net"] }
```

`lib.rs`: `mod lsp_socket;`

Do not add `crossbeam-channel`. Send does not name `crossbeam_channel::Receiver`.

### Design doc

`docs-website/docs/design-docs/event-model.md`:

- Inner: `handle` matches `IsographEvent::Lsp`.
- Outer: drop the event-socket bullet. The TCP port is the LSP adapter. Send is an LSP client of that port. `isograph/event` params are `HelloWorld` / `Quit` / `DiskChanged`. Channel event is `Lsp`. SIGTERM, tests, and the later watcher still post non-`Lsp` variants in-process.
- Ports: one listener, `{slug}.port`, LSP JSON-RPC. Delete `{slug}.lsp` as a second listener. Delete `freddie_event_socket` and the 64 KiB cap. On quit, unlink the port file, then `process::exit(0)`.

## Tests

`lsp_socket.rs`: bind `accept_loop`, 250ms settle. The test client is `TcpStream::connect`, then `crate::send::notify`.

- initialize then `isograph/event` `HelloWorld`: `event_rx` is `Lsp` whose notification params deserialize to `HelloWorld`; `handle` of that event is `LogHelloWorld`
- initialize then `isograph/event` DiskChanged present, then absent
- `isograph/event` before initialize: not an event; initialize then a second HelloWorld arrives
- unknown notification after initialize: `event_rx` is `Lsp`; `handle` of it is no effects; then `isograph/event` HelloWorld arrives
- unknown request after initialize: `MethodNotFound` on the socket; `event_rx` is `Lsp`; then `isograph/event` HelloWorld arrives
- request before initialize: `ServerNotInitialized`
- `shutdown` is `MethodNotFound` and does not `Quit`; a following `isograph/event` HelloWorld arrives
- a second connection can initialize + HelloWorld; the two `Lsp` events have different `LspClientId`s
- two connections both HelloWorld
- `notify` returns after `isograph/event` (does not hang)
- two sequential `notify`s on the same daemon, then the accept loop is still alive for a third
- valid JSON-RPC `isograph/event` with params that are not `HelloWorld` / `Quit` / `DiskChanged`: connection stays; `handle` warns; a following HelloWorld arrives
- bytes that are not a JSON-RPC object, or a truncated `Content-Length` body: that connection ends (copied `make_reader` `unwrap`s `Message::read`, `IoThreads::join` is `panic_any`); a second connection can initialize + HelloWorld

`state.rs`: existing `HelloWorld` / `Quit` / `DiskChanged` in-process tests stay. Add `handle` of `Lsp` wrapping `isograph/event` HelloWorld is `LogHelloWorld`. Add `handle` of `Lsp` wrapping an unknown notification is no effects.

`send.rs`: `parse_port` / `read_port` stay.

`cli.rs`: HelloWorld, DiskChanged present then absent, daemon stopped, not json, unknown kind, not in help stay. Add: after a successful send, `stop` without `--force` (unix `STOP`); log has `kill: exiting`; `status` is not running. Add: two sends, then that `stop`.

## Call sites

- `serve` -> `event_tx` -> `accept_loop` / SIGTERM `Quit` / `run_event_loop`
- pump -> `event_tx.send(Lsp { client, message }.to())` -> `handle`
- `isograph/event` inside `handle` -> `HelloWorld` / `DiskChanged` / `Quit`
- `isograph send` -> `TcpStream::connect` -> `notify` -> `isograph/event`
- watcher (later) -> `event_tx.send(DiskChanged)` -> `handle`, never the wire
- `Kill` -> unlink port file -> `process::exit(0)`
