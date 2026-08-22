# The daemon port speaks LSP

Requires send-events.md (landed). Independent of filesystem-watcher.md. Watcher later posts `IsographEvent::DiskChanged` in-process. Injected send uses this port.

The TCP port in `{slug}.port` is LSP JSON-RPC (`Content-Length`). Every inbound message is a request, a response, or a notification. There are no custom notifications. Ingest that used to be a websocket `IsographEvent` frame is a custom **request** (`impl lsp_types::request::Request`). The client waits for the result, so a following `semanticTokens/full` (lsp-tokens.md) cannot race the ingest.

`--file` is still `IsographEvent` JSON. `isograph send` is an ephemeral LSP client: `initialize` (no `processId`), `initialized`, one ingest request, wait for `null`, drop. It does not send `shutdown` / `exit`.

`handle` is unchanged. SIGTERM still sends `IsographEvent::Quit` in-process. The file watcher (later) does too. Those never go on the wire.

Origin of bind, port file, and send: send-events.md. Origin of extra methods: `lsp-types` 0.97 empty enums (`Shutdown`). Origin of framing: `lsp-server` 0.7.8 `Message`. Delta: TCP instead of `Connection` (its IO threads `unwrap`); ingest is custom requests whose params map to `IsographEvent`; the session speaks LSP; the worker only sees events. No request dispatcher: every ingest method is `handle` plus `null`. Dispatch waits until a method is not that (lsp-tokens.md).

One shippable change. Existing CLI send tests stay green.

## What the user does

```
$ printf '%s\n' '{"kind":"HelloWorld"}' > /tmp/hello.json
$ isograph send --file /tmp/hello.json
```

The log has `hello world`. The socket sees `initialize`, `initialized`, `isograph/helloWorld` (request id 2), then a `null` result. Send exits 0 after that result.

```
$ printf '%s\n' '{"kind":"DiskChanged","value":{"path":"/tmp/proj/src/a.ts","presence":{"Present":"export const a = 1;\n"}}}' > /tmp/disk.json
$ isograph send --file /tmp/disk.json
```

```
$ printf '%s\n' '{"kind":"Quit"}' > /tmp/quit.json
$ isograph send --file /tmp/quit.json
```

The result is `null`, then the daemon exits (`Kill`). Send is not in `--help`. The daemon not running is the same error as today.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/lsp_methods.rs
use lsp_types::request::Request;

#[derive(Debug)]
pub enum HelloWorld {}

impl Request for HelloWorld {
    type Params = ();
    type Result = ();
    const METHOD: &'static str = "isograph/helloWorld";
}

#[derive(Debug)]
pub enum DiskChanged {}

impl Request for DiskChanged {
    type Params = crate::event::DiskChanged;
    type Result = ();
    const METHOD: &'static str = "isograph/diskChanged";
}

#[derive(Debug)]
pub enum Quit {}

impl Request for Quit {
    type Params = ();
    type Result = ();
    const METHOD: &'static str = "isograph/quit";
}
```

Origin of the empty enum: `lsp_types::request::Shutdown`. `DiskChanged::Params` is `{path, presence}`, not `{kind, value}`.

```rust
// from crates/isograph_cli/src/daemon.rs
enum Work {
    Event(IsographEvent),
    Ingest(IsographEvent, tokio::sync::oneshot::Sender<()>),
}
```

`Event` is watcher and SIGTERM: no reply. `Ingest` is an LSP ingest request the session already decoded. The worker does not see `lsp_server::Request`. No `Serialize`.

The session maps `HelloWorld` / `DiskChanged` / `Quit` requests onto `IsographEvent`. Those three methods all run `handle`. There is no dispatcher and no per-method handler.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn event_from_ingest_request(
    request: &lsp_server::Request,
) -> Option<Result<crate::event::IsographEvent, serde_json::Error>> {
    if request.method == HelloWorld::METHOD {
        return serde_json::from_value::<()>(request.params.clone())
            .map(|()| crate::event::IsographEvent::HelloWorld)
            .wrap_some();
    }
    if request.method == DiskChanged::METHOD {
        return serde_json::from_value(request.params.clone())
            .map(crate::event::IsographEvent::DiskChanged)
            .wrap_some();
    }
    if request.method == Quit::METHOD {
        return serde_json::from_value::<()>(request.params.clone())
            .map(|()| crate::event::IsographEvent::Quit)
            .wrap_some();
    }
    None
}
```

`None` is not ingest (`MethodNotFound` until lsp-tokens.md). `Err` is `InvalidParams`. `Ok` is `Work::Ingest`.

```rust
// from crates/isograph_cli/src/daemon.rs
pub(crate) async fn run_event_loop<THostLanguage: HostLanguage>(
    mut state: IsographState<THostLanguage>,
    mut work_rx: tokio::sync::mpsc::UnboundedReceiver<Work>,
    effect_tx: tokio::sync::mpsc::UnboundedSender<IsographEffect>,
) {
    while let Some(work) = work_rx.recv().await {
        match work {
            Work::Event(event) => {
                for effect in handle(&mut state, event) {
                    let _ = effect_tx.send(effect);
                }
            }
            Work::Ingest(event, reply) => {
                let effects = handle(&mut state, event);
                let _ = reply.send(());
                for effect in effects {
                    let _ = effect_tx.send(effect);
                }
            }
        }
    }
}
```

Ack first, then effects. `Quit` → `Kill` must not drop the socket before send reads `null`.

Existing loop tests send `Work::Event(...)`. Add: `Work::Ingest(HelloWorld, oneshot)` replies `()` and then one `LogHelloWorld` effect.

### Session

`Initialize` / `Shutdown` / `Exit` stay on the session, like isograph `Connection::initialize` plus shutdown outside the domain loop. `initialized` is ignored. Unknown notifications are logged.

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

fn ingest_request(
    work_tx: &tokio::sync::mpsc::UnboundedSender<crate::daemon::Work>,
    request: lsp_server::Request,
) -> Step {
    let id = request.id.clone();
    match event_from_ingest_request(request.reference()) {
        None => Step::Reply(lsp_server::Response::new_err(
            id,
            lsp_server::ErrorCode::MethodNotFound as i32,
            format!("No handler registered for method '{}'", request.method),
        )),
        Some(Err(e)) => {
            warn!(error = %e, "ingest params");
            Step::Reply(lsp_server::Response::new_err(
                id,
                lsp_server::ErrorCode::InvalidParams as i32,
                "invalid request params".to_owned(),
            ))
        }
        Some(Ok(event)) => {
            let (reply, rx) = tokio::sync::oneshot::channel();
            if work_tx
                .send(crate::daemon::Work::Ingest(event, reply))
                .is_err()
            {
                return Step::End;
            }
            match rx.blocking_recv() {
                Ok(()) => match serde_json::to_value(()) {
                    Ok(result) => Step::Reply(lsp_server::Response {
                        id,
                        result: result.wrap_some(),
                        error: None,
                    }),
                    Err(e) => {
                        warn!(error = %e, "could not encode null result");
                        Step::Reply(lsp_server::Response::new_err(
                            id,
                            lsp_server::ErrorCode::InternalError as i32,
                            "could not encode null result".to_owned(),
                        ))
                    }
                },
                Err(_) => Step::End,
            }
        }
    }
}
```

Empty `ServerCapabilities`. lsp-tokens.md adds the legend. The session writes the LSP `Response`. The worker only acks `()`.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
pub(crate) async fn accept_loop(
    listener: tokio::net::TcpListener,
    work_tx: tokio::sync::mpsc::UnboundedSender<crate::daemon::Work>,
) {
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let work_tx = work_tx.clone();
                std::thread::spawn(move || match stream.into_std() {
                    Ok(std_stream) => {
                        if let Err(e) = std_stream.set_nonblocking(false) {
                            debug!(error = %e, %peer, "could not set the lsp stream blocking");
                            return;
                        }
                        session(std_stream, work_tx);
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
    work_tx: tokio::sync::mpsc::UnboundedSender<crate::daemon::Work>,
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
        match step(&mut session, message, work_tx.reference()) {
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
    work_tx: &tokio::sync::mpsc::UnboundedSender<crate::daemon::Work>,
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
        (Session::Running, lsp_server::Message::Request(request)) => {
            ingest_request(work_tx, request)
        }
        (Session::Running, lsp_server::Message::Notification(notification)) => {
            if notification.method != lsp_types::notification::Initialized::METHOD {
                warn!(method = notification.method.as_str(), "unknown notification");
            }
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

`into_std` streams are non-blocking. `set_nonblocking(false)` is required. The session is a `std::thread`. One in-flight domain request per connection.

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
    // write port file, SIGTERM as Work::Event(Quit), intern_config_directory as today
    let (work_tx, work_rx) = unbounded_channel::<Work>();
    let _hold_work = work_tx.clone();
    tokio::select! {
        () = run_event_loop(state, work_rx, effect_tx) => {}
        () = run_effect_loop(effect_rx) => {}
        () = crate::lsp_socket::accept_loop(listener, work_tx) => {}
    }
```

Drop `freddie_event_socket`. Delete `external.rs`.

### `send`

```rust
// from crates/isograph_cli/src/send.rs
    let event: crate::event::IsographEvent =
        serde_json::from_str(frame.trim()).map_err(SendError::NotEvent)?;
    let request = crate::lsp_socket::request_from_event(event, lsp_server::RequestId::from(2))
        .map_err(SendError::Encode)?;
    let stream = std::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).map_err(
        |source| SendError::Connect(Connect { port, source }),
    )?;
    handshake_and_request(stream, request)
```

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn request_from_event(
    event: crate::event::IsographEvent,
    id: lsp_server::RequestId,
) -> Result<lsp_server::Request, serde_json::Error> {
    match event {
        crate::event::IsographEvent::HelloWorld => request::<HelloWorld>(id, ()),
        crate::event::IsographEvent::DiskChanged(change) => request::<DiskChanged>(id, change),
        crate::event::IsographEvent::Quit => request::<Quit>(id, ()),
    }
}

fn request<R: lsp_types::request::Request>(
    id: lsp_server::RequestId,
    params: R::Params,
) -> Result<lsp_server::Request, serde_json::Error> {
    serde_json::to_value(params).map(|params| lsp_server::Request {
        id,
        method: R::METHOD.to_owned(),
        params,
    })
}
```

```rust
// from crates/isograph_cli/src/send.rs
fn handshake_and_request(
    stream: std::net::TcpStream,
    request: lsp_server::Request,
) -> Result<(), SendError> {
    let mut writer = stream.try_clone().map_err(SendError::Write)?;
    let mut reader = std::io::BufReader::new(stream);
    let init_id = lsp_server::RequestId::from(1);
    lsp_server::Message::Request(lsp_server::Request {
        id: init_id.clone(),
        method: lsp_types::request::Initialize::METHOD.to_owned(),
        params: serde_json::json!({ "capabilities": {} }),
    })
    .write(&mut writer)
    .map_err(SendError::Write)?;
    wait_for_ok(&mut reader, init_id.reference())?;
    lsp_server::Message::Notification(lsp_server::Notification {
        method: lsp_types::notification::Initialized::METHOD.to_owned(),
        params: serde_json::json!({}),
    })
    .write(&mut writer)
    .map_err(SendError::Write)?;
    let id = request.id.clone();
    lsp_server::Message::Request(request)
        .write(&mut writer)
        .map_err(SendError::Write)?;
    wait_for_ok(&mut reader, id.reference())
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

Initialize params omit `processId`. `Connect.source` is `io::Error`. Drop tungstenite. `SendError` gains `Encode`, `Write(io::Error)`, `Read(io::Error)`, `Closed`, `Lsp(String)`.

`CliVerb::Send` doc comment: `Encode one IsographEvent as an LSP request to the running daemon. Not for typing: tests and CI.`

### Cargo

```toml
# from crates/isograph_cli/Cargo.toml
# drop freddie_event_socket, tungstenite, tokio-tungstenite, futures-util
lsp-server = { workspace = true }
lsp-types = { workspace = true }
tokio = { workspace = true, features = ["rt", "macros", "signal", "sync", "time", "net"] }
```

`lib.rs`: `mod lsp_methods; mod lsp_socket;`

### Design doc

`docs-website/docs/design-docs/event-model.md` Ports / Outer: the TCP port is LSP; send is an LSP client; ingest on the wire is custom requests; watcher and SIGTERM stay in-process events. Drop `freddie_event_socket` and `{slug}.lsp`.

## Tests

`lsp_socket.rs`: bind `accept_loop`, 250ms settle.

- initialize then `isograph/helloWorld` request: result `null`, log/effect `hello world`
- initialize then `isograph/diskChanged` present, then absent
- initialize then `isograph/quit`: result `null`, then `Kill`
- hello world request before initialize: `ServerNotInitialized`
- unknown request after initialize: `MethodNotFound`
- `shutdown` then `exit` does not `Quit`; a second connection can initialize + hello world
- malformed payload closes the connection
- two connections both hello world

`send.rs`: `parse_port` / `read_port` stay.

`cli.rs`: HelloWorld, DiskChanged present then absent, daemon stopped, not json, unknown kind, not in help stay. Add `send_of_quit_stops_the_daemon`: `--file` `{"kind":"Quit"}`, send exits 0, log contains `kill: exiting`.

## Call sites

- `serve` -> `Work` channel -> `accept_loop` / SIGTERM `Work::Event(Quit)` / `run_event_loop`
- `Running` + ingest request -> `event_from_ingest_request` -> `Work::Ingest` -> `handle` -> ack -> session writes `null` -> effects
- `isograph send` -> `request_from_event` -> `handshake_and_request`
- watcher (later) -> `Work::Event(DiskChanged)` -> `handle`, never the wire
