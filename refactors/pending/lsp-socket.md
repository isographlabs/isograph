# LSP as the daemon's outside protocol

Requires send-events.md (landed). Independent of filesystem-watcher.md, file-semantic-tokens.md, and e2e-semantic-tokens.md.

The daemon's TCP port is an LSP server. Every message is a request, a response, or a notification from `lsp_server::Message`. `IsographEvent` is not on the wire. `--file` is still `IsographEvent` JSON; `isograph send` is an LSP client that encodes it as a notification. `handle` is unchanged. Same `127.0.0.1:0` port and `{slug}.port` file.

Custom methods are empty enums that implement `lsp_types::notification::Notification`, the same shape as `lsp_types` (`Initialized`, `Exit`) and as isograph's `on_notification_sync::<DidOpenTextDocument>`. Standard methods come from `lsp_types`: `request::Initialize`, `request::Shutdown`, `notification::Initialized`, `notification::Exit`. Do not string-match `"initialize"` or `"isograph/helloWorld"`.

`lsp_server::Connection` IO threads `unwrap`. We never use `Connection`. We use `Message::{read,write}` and `Request::extract` / `Notification::extract`. We do not call `Notification::new`, `Request::new`, or `Response::new_ok` (`unwrap` on `to_value`). `Response::new_err` is fine.

Whether an LSP client should report filesystem facts is later. `DiskChanged` as a notification exists because send still interns a `DiskFile` that way.

Origin of bind, port file, and send: send-events.md. Origin of extra methods: `lsp-types` 0.97 `notification.rs` (`pub enum Initialized {}` + `impl Notification`). Origin of typed dispatch: isograph `crates/isograph_lsp/src/lsp_notification_dispatch.rs` `on_notification_sync`. Origin of framing: `lsp-server` 0.7.8 `Message`. Delta: replace the WebSocket of `IsographEvent` JSON; extract returns `Result`, no `expect`; `Quit` is a notification, not SIGTERM.

One shippable change.

## What the user does

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
```

The daemon exits. That is `isograph/quit`, the same path as `isograph stop` / SIGTERM: `IsographEvent::Quit` then `Kill`. Send is not in `--help`. The daemon not running is the same error as today.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
use lsp_types::notification::Notification;
use lsp_types::request::Request;

#[derive(Debug)]
pub enum HelloWorld {}

impl Notification for HelloWorld {
    type Params = ();
    const METHOD: &'static str = "isograph/helloWorld";
}

#[derive(Debug)]
pub enum DiskChanged {}

impl Notification for DiskChanged {
    type Params = crate::event::DiskChanged;
    const METHOD: &'static str = "isograph/diskChanged";
}

#[derive(Debug)]
pub enum Quit {}

impl Notification for Quit {
    type Params = ();
    const METHOD: &'static str = "isograph/quit";
}
```

Origin of the empty enum: `lsp_types::notification::Initialized`. Delta: method names under `isograph/`. `HelloWorld` and `Quit` have no params. `DiskChanged::Params` is the existing event payload (`path`, `presence`), not the tagged `{kind, value}` envelope.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
#[derive(Copy, Clone)]
enum Session {
    ExpectInitialize,
    Running,
    ExpectExit,
}

enum Step {
    Continue,
    Reply(lsp_server::Response),
    End,
}
```

`ExpectInitialize`: answer `Initialize`, drop other notifications, `ServerNotInitialized` for other requests. `Running`: domain notifications become events; `Shutdown` replies `null` and becomes `ExpectExit`; unknown requests `MethodNotFound`. `ExpectExit`: only `Exit` ends the thread. `Exit` in any state ends the thread and does not by itself send `IsographEvent::Quit`. `Quit` the notification does.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn notification_from_event(
    event: crate::event::IsographEvent,
) -> Result<lsp_server::Notification, serde_json::Error> {
    match event {
        crate::event::IsographEvent::HelloWorld => notify::<HelloWorld>(()),
        crate::event::IsographEvent::DiskChanged(change) => notify::<DiskChanged>(change),
        crate::event::IsographEvent::Quit => notify::<Quit>(()),
    }
}

fn notify<N: Notification>(params: N::Params) -> Result<lsp_server::Notification, serde_json::Error> {
    serde_json::to_value(params).map(|params| lsp_server::Notification {
        method: N::METHOD.to_owned(),
        params,
    })
}

fn extract<N: Notification>(
    notification: &lsp_server::Notification,
) -> Option<Result<N::Params, serde_json::Error>> {
    if notification.method != N::METHOD {
        return None;
    }
    serde_json::from_value(notification.params.clone()).wrap_some()
}

fn event_from_notification(
    notification: &lsp_server::Notification,
) -> Option<crate::event::IsographEvent> {
    if let Some(params) = extract::<HelloWorld>(notification) {
        return match params {
            Ok(()) => crate::event::IsographEvent::HelloWorld.wrap_some(),
            Err(e) => {
                warn!(error = %e, "isograph/helloWorld params");
                None
            }
        };
    }
    if let Some(params) = extract::<DiskChanged>(notification) {
        return match params {
            Ok(change) => crate::event::IsographEvent::DiskChanged(change).wrap_some(),
            Err(e) => {
                warn!(error = %e, "isograph/diskChanged params");
                None
            }
        };
    }
    if let Some(params) = extract::<Quit>(notification) {
        return match params {
            Ok(()) => crate::event::IsographEvent::Quit.wrap_some(),
            Err(e) => {
                warn!(error = %e, "isograph/quit params");
                None
            }
        };
    }
    if notification.method == lsp_types::notification::Initialized::METHOD {
        return None;
    }
    warn!(method = notification.method.as_str(), "unknown notification");
    None
}
```

`extract` compares `N::METHOD`. `params.clone()` is a `serde_json::Value`. `Initialized` is not an event. `Exit` is handled in `step` before this function. Bad params of a known method are not an event; the connection stays up.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn initialize_response(id: lsp_server::RequestId) -> Result<lsp_server::Response, serde_json::Error> {
    serde_json::to_value(lsp_types::InitializeResult {
        capabilities: lsp_types::ServerCapabilities::default(),
        server_info: lsp_types::ServerInfo {
            name: "isograph".to_owned(),
            version: None,
        }
        .wrap_some(),
    })
    .map(|result| lsp_server::Response {
        id,
        result: result.wrap_some(),
        error: None,
    })
}

fn request_is<R: Request>(request: &lsp_server::Request) -> bool {
    request.method == R::METHOD
}
```

Empty `ServerCapabilities`. Domain requests are later docs; they are `MethodNotFound` here.

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
    let mut session = Session::ExpectInitialize;
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
        match step(&mut session, message, event_tx.reference()) {
            Step::Continue => {}
            Step::Reply(response) => {
                if let Err(e) = lsp_server::Message::Response(response).write(&mut writer) {
                    debug!(error = %e, "could not write the lsp response");
                    break;
                }
            }
            Step::End => break,
        }
    }
}
```

`into_std` streams are non-blocking. `Message::read` is blocking. `set_nonblocking(false)` is required. The session is a `std::thread` so a blocked read does not stall the current-thread tokio runtime.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn step(
    session: &mut Session,
    message: lsp_server::Message,
    event_tx: &tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
) -> Step {
    match (*session, message) {
        (_, lsp_server::Message::Notification(notification))
            if notification.method == lsp_types::notification::Exit::METHOD =>
        {
            Step::End
        }
        (Session::ExpectInitialize, lsp_server::Message::Request(request))
            if request_is::<lsp_types::request::Initialize>(&request) =>
        {
            let id = request.id;
            match initialize_response(id.clone()) {
                Ok(response) => {
                    *session = Session::Running;
                    Step::Reply(response)
                }
                Err(e) => {
                    warn!(error = %e, "could not encode initialize result");
                    Step::Reply(lsp_server::Response::new_err(
                        id,
                        lsp_server::ErrorCode::InternalError as i32,
                        "could not encode initialize result".to_owned(),
                    ))
                }
            }
        }
        (Session::ExpectInitialize, lsp_server::Message::Request(request)) => {
            Step::Reply(lsp_server::Response::new_err(
                request.id,
                lsp_server::ErrorCode::ServerNotInitialized as i32,
                "server not initialized".to_owned(),
            ))
        }
        (Session::ExpectInitialize, lsp_server::Message::Notification(notification)) => {
            warn!(method = notification.method.as_str(), "notification before initialize");
            Step::Continue
        }
        (Session::ExpectInitialize, lsp_server::Message::Response(_)) => Step::Continue,
        (Session::Running, lsp_server::Message::Request(request))
            if request_is::<lsp_types::request::Shutdown>(&request) =>
        {
            *session = Session::ExpectExit;
            Step::Reply(lsp_server::Response {
                id: request.id,
                result: serde_json::Value::Null.wrap_some(),
                error: None,
            })
        }
        (Session::Running, lsp_server::Message::Request(request)) => {
            Step::Reply(lsp_server::Response::new_err(
                request.id,
                lsp_server::ErrorCode::MethodNotFound as i32,
                format!("{} is not a request this server answers", request.method),
            ))
        }
        (Session::Running, lsp_server::Message::Notification(notification)) => {
            if let Some(event) = event_from_notification(notification.reference()) {
                let _ = event_tx.send(event);
            }
            Step::Continue
        }
        (Session::Running, lsp_server::Message::Response(_)) => Step::Continue,
        (Session::ExpectExit, lsp_server::Message::Request(request)) => {
            Step::Reply(lsp_server::Response::new_err(
                request.id,
                lsp_server::ErrorCode::InvalidRequest as i32,
                "shutdown already received".to_owned(),
            ))
        }
        (Session::ExpectExit, lsp_server::Message::Notification(notification)) => {
            warn!(method = notification.method.as_str(), "notification after shutdown");
            Step::Continue
        }
        (Session::ExpectExit, lsp_server::Message::Response(_)) => Step::Continue,
    }
}
```

`Exit` is first so it ends the session in every state. A second `Initialize` after `Running` is `MethodNotFound`. `RequestId` is not `Copy`; clone it for `initialize_response`.

### `serve`

Origin: `crates/isograph_cli/src/daemon.rs` `serve`. Delta: `tokio::net::TcpListener` instead of `freddie_event_socket::listen`; `accept_loop` as a third `select!` arm. Keep `intern_config_directory` as today.

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
    // write port file, SIGTERM task, intern_config_directory as today
    let _hold_events = event_tx.clone();
    tokio::select! {
        () = run_event_loop(state, event_rx, effect_tx) => {}
        () = run_effect_loop(effect_rx) => {}
        () = crate::lsp_socket::accept_loop(listener, event_tx) => {}
    }
```

Drop `freddie_event_socket` from this file. `Kill` ends the effect loop, `select!` drops `accept_loop`.

### `send`

Origin: `crates/isograph_cli/src/send.rs`. Delta: `TcpStream` plus `Message`; `notification_from_event`; handshake with `Initialize` / `Initialized`.

```rust
// from crates/isograph_cli/src/send.rs
    let event: crate::event::IsographEvent =
        serde_json::from_str(frame.trim()).map_err(SendError::NotEvent)?;
    let notification = crate::lsp_socket::notification_from_event(event).map_err(SendError::Encode)?;
    let stream = std::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).map_err(
        |source| SendError::Connect(Connect { port, source }),
    )?;
    handshake_and_notify(stream, notification)
```

```rust
// from crates/isograph_cli/src/send.rs
fn handshake_and_notify(
    stream: std::net::TcpStream,
    notification: lsp_server::Notification,
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
    wait_for_initialize_result(&mut reader, id.reference())?;
    lsp_server::Message::Notification(lsp_server::Notification {
        method: lsp_types::notification::Initialized::METHOD.to_owned(),
        params: serde_json::json!({}),
    })
    .write(&mut writer)
    .map_err(SendError::Write)?;
    lsp_server::Message::Notification(notification)
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
            return SendError::InitializeClosed.wrap_err();
        };
        let lsp_server::Message::Response(response) = message else {
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

`Connect.source` is `io::Error`. `SendError` keeps Discover / NotEvent / NotRunning / port errors, drops tungstenite, gains `Encode(serde_json::Error)`, `Write(io::Error)`, `Read(io::Error)`, `InitializeClosed`, `Initialize(String)`. `require_running`, `read_port`, `run` (unlink `--file`) stay. Send does not send `Shutdown` / `Exit`. Dropping the socket ends the session.

`CliVerb::Send` doc comment: `Encode one IsographEvent as an LSP notification to the running daemon. Not for typing: tests and CI.` `SendArgs.file`: `JSON IsographEvent to encode as an LSP notification.`

### Cargo

```toml
# from crates/isograph_cli/Cargo.toml
# drop freddie_event_socket, tungstenite
# drop dev-dependencies tokio-tungstenite, futures-util
lsp-server = { workspace = true }
lsp-types = { workspace = true }
tokio = { workspace = true, features = ["rt", "macros", "signal", "sync", "time", "net"] }
```

`lib.rs`: `mod lsp_socket;`. Delete `external.rs`.

### Design doc

`docs-website/docs/design-docs/event-model.md` Outer / Ports: the TCP port is the LSP server; send is an LSP client of it; drop `freddie_event_socket` and the `{slug}.lsp` second listener. `isograph/quit` is `IsographEvent::Quit`. `shutdown` / `exit` close the session.

Landing sequence `refactors/pending/event-model.md` item 2 notes the wire is this file. Item 13 is further methods on this listener (`DidOpenTextDocument`, `SemanticTokensFullRequest`), dispatched the same way: `impl Notification` / `impl Request` already in `lsp_types`.

## Tests

`lsp_socket.rs`. Bind `accept_loop` on a tokio test runtime. `TcpStream::connect`. Write `Message`s. Drain `event_rx` after 250ms (same settle as today's `external.rs`). `expect` names the fixture.

- `HelloWorld::METHOD` is `isograph/helloWorld`; `notification_from_event(HelloWorld)` uses that method
- `DiskChanged` present params contain the path
- `Quit` encodes as `isograph/quit`
- initialize then `isograph/helloWorld` is `IsographEvent::HelloWorld`; initialize result has empty `capabilities`
- initialize then disk changed present / absent
- initialize then `isograph/quit` is `IsographEvent::Quit`
- hello world before initialize is not an event; then initialize + hello world on the same connection works
- unknown notification after initialize is dropped; then hello world arrives
- unknown request after initialize is `MethodNotFound` (`-32601`)
- request before initialize is `ServerNotInitialized` (`-32002`)
- `shutdown` then `exit` does not emit `Quit`; a second connection can still initialize + hello world
- `exit` without `shutdown` ends the session; accept still works for a new connection
- malformed payload closes the connection
- two connections both hello world

`send.rs`: existing `parse_port` / `read_port` tests stay.

`cli.rs`: HelloWorld, DiskChanged present then absent, daemon stopped, not json, unknown kind, not in help stay. `--file` JSON is unchanged. Add `send_of_quit_stops_the_daemon`: `--file` `{"kind":"Quit"}`, exit 0, log contains `kill: exiting`.

## Call sites

- `serve` -> bind -> port file -> `select!` `accept_loop`
- `accept_loop` -> `session` -> `step` -> `event_from_notification` / `Message::write`
- `isograph send` -> `notification_from_event` -> `handshake_and_notify`
- SIGTERM / `isograph stop` / `isograph/quit` -> `IsographEvent::Quit` -> `handle` -> `Kill`
