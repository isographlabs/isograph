# LSP as the daemon's outside protocol

Requires send-events.md (landed). Independent of filesystem-watcher.md and e2e-semantic-tokens.md.

The daemon's TCP port is an LSP server. Every message is a `lsp_server::Message`. `IsographEvent` is not on the wire. `--file` is still `IsographEvent` JSON; `isograph send` encodes it as a notification. `handle` is unchanged. Same `127.0.0.1:0` port and `{slug}.port` file.

Notifications are dispatched the way isograph does: `LSPNotificationDispatch::new(notification, state).on_notification_sync::<T>(handler)?`. Custom methods are empty enums that implement `lsp_types::notification::Notification`, same as `Initialized` / `Exit`. Standard methods come from `lsp_types`: `request::Initialize`, `request::Shutdown`, `notification::Initialized`, `notification::Exit`.

`lsp_server::Connection` IO threads `unwrap`. We never use `Connection`. We use `Message::{read,write}`. We do not call `Notification::new`, `Request::new`, or `Response::new_ok`. `Response::new_err` is fine. `Notification::extract` / `Request::extract` return `Result`; we match it. isograph `expect`s those; that is the delta.

Whether an LSP client should report filesystem facts is later. `DiskChanged` as a notification exists because send still interns a `DiskFile` that way.

Origin of bind, port file, and send: send-events.md. Origin of extra methods: `lsp-types` 0.97 `notification.rs`. Origin of dispatch: isograph `crates/isograph_lsp/src/lsp_notification_dispatch.rs` and `server.rs` `dispatch_notification`. Origin of framing: `lsp-server` 0.7.8 `Message`. Delta: TCP instead of `Connection::stdio`; extract is `Result`; postfix constructors; `Quit` is a notification.

One shippable change.

## What the user does

The daemon is already up. `--file` is still `IsographEvent` JSON. Send unlinks it after the attempt.

```
$ printf '%s\n' '{"kind":"HelloWorld"}' > /tmp/hello.json
$ isograph send --file /tmp/hello.json
```

The log has `hello world`. The socket sees `initialize`, `initialized`, then `isograph/helloWorld`.

```
$ printf '%s\n' '{"kind":"DiskChanged","value":{"path":"/tmp/proj/src/a.ts","presence":{"Present":"export const a = 1;\n"}}}' > /tmp/disk.json
$ isograph send --file /tmp/disk.json
```

```
$ printf '%s\n' '{"kind":"Quit"}' > /tmp/quit.json
$ isograph send --file /tmp/quit.json
```

The daemon exits (`isograph/quit` → `IsographEvent::Quit` → `Kill`). Send is not in `--help`.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
use lsp_types::notification::Notification;

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

Origin of the empty enum: `lsp_types::notification::Initialized`. Delta: methods under `isograph/`. `DiskChanged::Params` is the event payload (`path`, `presence`), not `{kind, value}`.

```rust
// from crates/isograph_cli/src/lsp_dispatch.rs
use std::ops::ControlFlow;

use lsp_types::notification::Notification;
use prelude::Postfix;
use tracing::warn;

pub struct LspNotificationDispatch<'state, TState> {
    notification: lsp_server::Notification,
    state: &'state mut TState,
}

impl<'state, TState> LspNotificationDispatch<'state, TState> {
    pub fn new(notification: lsp_server::Notification, state: &'state mut TState) -> Self {
        Self {
            notification,
            state,
        }
    }

    pub fn on_notification_sync<TNotification: Notification>(
        self,
        handler: fn(&mut TState, TNotification::Params),
    ) -> ControlFlow<(), Self> {
        if self.notification.method != TNotification::METHOD {
            return ControlFlow::Continue(self);
        }
        match self.notification.extract(TNotification::METHOD) {
            Ok(params) => {
                handler(self.state, params);
                ControlFlow::Break(())
            }
            Err(lsp_server::ExtractError::MethodMismatch(notification)) => {
                ControlFlow::Continue(Self {
                    notification,
                    state: self.state,
                })
            }
            Err(lsp_server::ExtractError::JsonError { method, error }) => {
                warn!(method = method.as_str(), error = %error, "notification params");
                ControlFlow::Break(())
            }
        }
    }

    pub fn notification(self) -> lsp_server::Notification {
        self.notification
    }
}
```

Origin: isograph `lsp_notification_dispatch.rs` `LSPNotificationDispatch` verbatim except: handler returns `()` not `LSPRuntimeResult`; extract is `match` not `expect`; `JsonError` logs and counts as handled; postfix. The `?` chain is the same: first matching method runs, then `Break`.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn dispatch_notification(
    notification: lsp_server::Notification,
    event_tx: &mut tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
) {
    let dispatch = || {
        crate::lsp_dispatch::LspNotificationDispatch::new(notification, event_tx)
            .on_notification_sync::<HelloWorld>(on_hello_world)?
            .on_notification_sync::<DiskChanged>(on_disk_changed)?
            .on_notification_sync::<Quit>(on_quit)?
            .on_notification_sync::<lsp_types::notification::Initialized>(on_initialized)?
            .notification();
        ControlFlow::Continue(())
    };
    match dispatch() {
        ControlFlow::Break(()) => {}
        ControlFlow::Continue(_) => {
            warn!("unknown notification");
        }
    }
}

fn on_hello_world(
    event_tx: &mut tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
    (): (),
) {
    let _ = event_tx.send(crate::event::IsographEvent::HelloWorld);
}

fn on_disk_changed(
    event_tx: &mut tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
    change: crate::event::DiskChanged,
) {
    let _ = event_tx.send(crate::event::IsographEvent::DiskChanged(change));
}

fn on_quit(
    event_tx: &mut tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
    (): (),
) {
    let _ = event_tx.send(crate::event::IsographEvent::Quit);
}

fn on_initialized(
    _event_tx: &mut tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
    _params: lsp_types::InitializedParams,
) {
}
```

Origin: isograph `server.rs` `dispatch_notification`. Delta: `TState` is the event sender; handlers push `IsographEvent`. `Initialized` is a no-op so it is not "unknown". `Exit` is not in this chain.

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

fn notify<N: Notification>(
    params: N::Params,
) -> Result<lsp_server::Notification, serde_json::Error> {
    serde_json::to_value(params).map(|params| lsp_server::Notification {
        method: N::METHOD.to_owned(),
        params,
    })
}
```

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

`ExpectInitialize` is `Connection::initialize` in isograph: answer `Initialize`, drop other notifications, `ServerNotInitialized` for other requests. `Running` is the isograph server loop. `ExpectExit`: only `Exit` ends the thread. `Exit` in any state ends the thread and is not `IsographEvent::Quit`. `Quit` the notification is.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn initialize_response(
    id: lsp_server::RequestId,
) -> Result<lsp_server::Response, serde_json::Error> {
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
```

Empty `ServerCapabilities`. Domain requests are lsp-request.md.

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
    mut event_tx: tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
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
        match step(&mut session, message, &mut event_tx) {
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

fn step(
    session: &mut Session,
    message: lsp_server::Message,
    event_tx: &mut tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
) -> Step {
    match (*session, message) {
        (_, lsp_server::Message::Notification(notification))
            if notification.method == lsp_types::notification::Exit::METHOD =>
        {
            Step::End
        }
        (Session::ExpectInitialize, lsp_server::Message::Request(request))
            if request.method == lsp_types::request::Initialize::METHOD =>
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
        (Session::ExpectInitialize, lsp_server::Message::Request(request)) => Step::Reply(
            lsp_server::Response::new_err(
                request.id,
                lsp_server::ErrorCode::ServerNotInitialized as i32,
                "server not initialized".to_owned(),
            ),
        ),
        (Session::ExpectInitialize, lsp_server::Message::Notification(notification)) => {
            warn!(method = notification.method.as_str(), "notification before initialize");
            Step::Continue
        }
        (Session::ExpectInitialize, lsp_server::Message::Response(_)) => Step::Continue,
        (Session::Running, lsp_server::Message::Request(request))
            if request.method == lsp_types::request::Shutdown::METHOD =>
        {
            *session = Session::ExpectExit;
            Step::Reply(lsp_server::Response {
                id: request.id,
                result: serde_json::Value::Null.wrap_some(),
                error: None,
            })
        }
        (Session::Running, lsp_server::Message::Request(request)) => Step::Reply(
            lsp_server::Response::new_err(
                request.id,
                lsp_server::ErrorCode::MethodNotFound as i32,
                format!("No handler registered for method '{}'", request.method),
            ),
        ),
        (Session::Running, lsp_server::Message::Notification(notification)) => {
            dispatch_notification(notification, event_tx);
            Step::Continue
        }
        (Session::Running, lsp_server::Message::Response(_)) => Step::Continue,
        (Session::ExpectExit, lsp_server::Message::Request(request)) => Step::Reply(
            lsp_server::Response::new_err(
                request.id,
                lsp_server::ErrorCode::InvalidRequest as i32,
                "shutdown already received".to_owned(),
            ),
        ),
        (Session::ExpectExit, lsp_server::Message::Notification(notification)) => {
            warn!(method = notification.method.as_str(), "notification after shutdown");
            Step::Continue
        }
        (Session::ExpectExit, lsp_server::Message::Response(_)) => Step::Continue,
    }
}
```

`into_std` streams are non-blocking. `Message::read` is blocking. `set_nonblocking(false)` is required. The session is a `std::thread` so a blocked read does not stall the current-thread tokio runtime. Method strings for `Initialize` / `Shutdown` / `Exit` are `T::METHOD`, not literals. Domain request `MethodNotFound` message is isograph's `No handler registered for method '{}'`.

### `serve`

Origin: `crates/isograph_cli/src/daemon.rs` `serve`. Delta: `tokio::net::TcpListener`; `accept_loop` as a third `select!` arm. Keep `intern_config_directory`.

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

Drop `freddie_event_socket`.

### `send`

Origin: `crates/isograph_cli/src/send.rs`. Delta: `TcpStream` plus `Message`; `notification_from_event`; handshake with `Initialize::METHOD` / `Initialized::METHOD`.

```rust
// from crates/isograph_cli/src/send.rs
    let event: crate::event::IsographEvent =
        serde_json::from_str(frame.trim()).map_err(SendError::NotEvent)?;
    let notification =
        crate::lsp_socket::notification_from_event(event).map_err(SendError::Encode)?;
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

`Connect.source` is `io::Error`. `SendError` keeps Discover / NotEvent / NotRunning / port errors, drops tungstenite, gains `Encode(serde_json::Error)`, `Write(io::Error)`, `Read(io::Error)`, `InitializeClosed`, `Initialize(String)`. Send does not send `Shutdown` / `Exit`.

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

`lib.rs`: `mod lsp_dispatch; mod lsp_socket;`. Delete `external.rs`.

### Design doc

`docs-website/docs/design-docs/event-model.md` Outer / Ports: the TCP port is the LSP server; send is an LSP client; drop `freddie_event_socket` and `{slug}.lsp`. `isograph/quit` is `IsographEvent::Quit`. `shutdown` / `exit` close the session.

Landing sequence `refactors/pending/event-model.md` item 2 notes this file. Item 13 is lsp-request.md: `LspRequestDispatch` on the worker.

## Tests

`lsp_dispatch.rs`: origin isograph `calls_first_matching_notification_handler`. Two handlers, first method does not match, second does, state is 2. `expect` names the fixture JSON. No `unwrap` on `to_value`; `expect` with reason.

`lsp_socket.rs`: bind `accept_loop`, write `Message`s, drain `event_rx` after 250ms.

- `HelloWorld::METHOD` is `isograph/helloWorld`
- initialize then `isograph/helloWorld` is `IsographEvent::HelloWorld`
- initialize then disk changed present / absent
- initialize then `isograph/quit` is `IsographEvent::Quit`
- hello world before initialize is not an event
- unknown notification after initialize is dropped; then hello world arrives
- unknown request after initialize is `MethodNotFound`
- request before initialize is `ServerNotInitialized`
- `shutdown` then `exit` does not emit `Quit`; a second connection can initialize + hello world
- `exit` without `shutdown` ends the session
- malformed payload closes the connection
- two connections both hello world

`send.rs`: existing `parse_port` / `read_port` stay.

`cli.rs`: HelloWorld, DiskChanged present then absent, daemon stopped, not json, unknown kind, not in help stay. Add `send_of_quit_stops_the_daemon`: `--file` `{"kind":"Quit"}`, exit 0, log contains `kill: exiting`.

## Call sites

- `serve` -> bind -> port file -> `select!` `accept_loop`
- `Running` + notification -> `dispatch_notification` -> `on_hello_world` / `on_disk_changed` / `on_quit` -> `event_tx`
- `isograph send` -> `notification_from_event` -> `handshake_and_notify`
- SIGTERM / `isograph stop` / `isograph/quit` -> `IsographEvent::Quit` -> `handle` -> `Kill`
