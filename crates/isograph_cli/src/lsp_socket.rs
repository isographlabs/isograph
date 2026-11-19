use std::{
    io::{self, BufReader},
    net::TcpStream,
    thread,
};

use crossbeam::channel::{Receiver, Sender, bounded};
use lsp_server::Message;
use lsp_types::notification::{Exit, Notification};
use prelude::Postfix;
use tracing::debug;

#[derive(Debug)]
pub enum IsographEventNotification {}

impl Notification for IsographEventNotification {
    type Params = crate::event::InternalEvent;
    const METHOD: &'static str = "isograph/event";
}

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
            let is_exit = matches!(&msg, Message::Notification(n) if n.method == Exit::METHOD);
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

fn connection_from_stream(
    stream: std::net::TcpStream,
) -> io::Result<(lsp_server::Connection, IoThreads)> {
    let (sender, receiver, io_threads) = socket_transport(stream)?;
    (lsp_server::Connection { sender, receiver }, io_threads).wrap_ok()
}

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
    if let Some(client) = start_session(connection.reference(), event_tx.reference()) {
        forward_messages(connection.reference(), event_tx.reference(), client);
        let _ = event_tx.send(crate::event::ClientConnectionEvent::Gone(client).to());
    }
    drop(connection);
    let _ = io_threads.join();
}

/// Initializes, posts `Started`, and returns the `ClientId` `handle` assigned. `None` when
/// initialize fails or the daemon dropped the assignment.
fn start_session(
    connection: &lsp_server::Connection,
    event_tx: &tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
) -> Option<isograph_compiler::ClientId> {
    let capabilities = match serde_json::to_value(lsp_types::ServerCapabilities {
        semantic_tokens_provider:
            lsp_types::SemanticTokensServerCapabilities::SemanticTokensOptions(
                lsp_types::SemanticTokensOptions {
                    work_done_progress_options: lsp_types::WorkDoneProgressOptions::default(),
                    legend: isograph_lsp::semantic_token_legend(),
                    range: None,
                    full: lsp_types::SemanticTokensFullOptions::Bool(true).wrap_some(),
                },
            )
            .wrap_some(),
        document_formatting_provider: lsp_types::OneOf::Left(true).wrap_some(),
        hover_provider: lsp_types::HoverProviderCapability::Simple(true).wrap_some(),
        definition_provider: lsp_types::OneOf::Left(true).wrap_some(),
        document_highlight_provider: lsp_types::OneOf::Left(true).wrap_some(),
        completion_provider: lsp_types::CompletionOptions::default().wrap_some(),
        text_document_sync: lsp_types::TextDocumentSyncCapability::Options(
            lsp_types::TextDocumentSyncOptions {
                open_close: true.wrap_some(),
                change: lsp_types::TextDocumentSyncKind::FULL.wrap_some(),
                will_save_wait_until: true.wrap_some(),
                ..Default::default()
            },
        )
        .wrap_some(),
        ..Default::default()
    }) {
        Ok(value) => value,
        Err(e) => {
            debug!(error = %e, "server capabilities");
            return None;
        }
    };
    let params = match connection.initialize(capabilities) {
        Ok(params) => params,
        Err(e) => {
            debug!(error = %e, "lsp initialize");
            return None;
        }
    };
    let params = match serde_json::from_value::<lsp_types::InitializeParams>(params) {
        Ok(params) => params,
        Err(e) => {
            debug!(error = %e, "initialize params");
            return None;
        }
    };
    let (assign, assigned) = crossbeam::channel::bounded(1);
    event_tx
        .send(
            crate::event::ClientConnectionEvent::Started(crate::event::ClientStartedEvent {
                session: crate::event::LiveSession {
                    sender: connection.sender.clone(),
                    capabilities: params.capabilities,
                    published_diagnostics: crate::diagnostics::PublishedDiagnostics::default(),
                    outstanding: std::collections::HashSet::new(),
                },
                assign,
            })
            .to(),
        )
        .ok()?;
    assigned.recv().ok()
}

fn forward_messages(
    connection: &lsp_server::Connection,
    event_tx: &tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
    client: isograph_compiler::ClientId,
) {
    for message in &connection.receiver {
        let event = match message {
            lsp_server::Message::Request(request) => {
                crate::event::LspEvent::Request(crate::event::LspRequest { client, request })
            }
            lsp_server::Message::Notification(notification) => {
                crate::event::LspEvent::Notification(crate::event::LspNotification {
                    client,
                    notification,
                })
            }
            lsp_server::Message::Response(response) => {
                crate::event::LspEvent::Response(crate::event::LspResponse { client, response })
            }
        };
        let _ = event_tx.send(event.to());
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::{Ipv4Addr, TcpStream};
    use std::path::PathBuf;
    use std::time::Duration;

    use lsp_server::{ErrorCode, Message, Request, RequestId};
    use lsp_types::notification::{Initialized, Notification};
    use lsp_types::request::{
        Formatting, Initialize, Request as LspRequest, SemanticTokensFullRequest, Shutdown,
        WillSaveWaitUntil,
    };
    use prelude::Postfix;
    use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

    use super::{IsographEventNotification, accept_loop};
    use crate::daemon::{run_effect_loop, run_event_loop};
    use crate::event::{
        ClientConnectionEvent, ClientStartedEvent, DiskChanged, DiskFileChanged, InternalEvent,
        IsographEvent, Presence,
    };
    use crate::send::notify;
    use crate::state::IsographState;
    use crate::test_profile::TestProfile;
    use isograph_compiler::ClientId;

    const SETTLE: Duration = Duration::from_millis(250);

    // Tests use a multi-thread runtime: TcpStream::connect and Message::read are blocking.
    // On current-thread they would stall accept_loop.
    async fn listen_for_events() -> (u16, UnboundedReceiver<IsographEvent>) {
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("binding port 0");
        let port = listener
            .local_addr()
            .expect("the listener has an address")
            .port();
        let (event_tx, event_rx) = unbounded_channel();
        tokio::spawn(accept_loop(listener, event_tx));
        tokio::time::sleep(SETTLE).await;
        (port, event_rx)
    }

    async fn listen_and_reply() -> (u16, UnboundedReceiver<IsographEvent>) {
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("binding port 0");
        let port = listener
            .local_addr()
            .expect("the listener has an address")
            .port();
        let (session_tx, mut session_rx) = unbounded_channel();
        let (test_tx, test_rx) = unbounded_channel();
        let (loop_tx, loop_rx) = unbounded_channel();
        let effect_event_tx = loop_tx.clone();
        let (effect_tx, effect_rx) = unbounded_channel();
        tokio::spawn(accept_loop(listener, session_tx));
        tokio::spawn(async move {
            while let Some(event) = session_rx.recv().await {
                if !matches!(
                    event,
                    IsographEvent::Lsp(crate::event::LspEvent::Request(_))
                ) {
                    let _ = test_tx.send(event.clone());
                }
                let _ = loop_tx.send(event);
            }
        });
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("a test can start a tokio runtime");
            let mut state = IsographState::<TestProfile>::new(
                crate::watch::ProjectGlobs::parse(&["src/**".to_owned()], &[], None)
                    .expect("the test glob is valid"),
                None,
            );
            crate::state::intern_config_directory(
                &mut state.db,
                std::path::Path::new("/tmp/proj/isograph.config.json"),
            );
            runtime.block_on(async {
                tokio::select! {
                    () = run_event_loop(state, loop_rx, effect_tx) => {}
                    () = run_effect_loop(effect_rx, effect_event_tx) => {}
                }
            });
        });
        tokio::time::sleep(SETTLE).await;
        (port, test_rx)
    }

    fn connect(port: u16) -> TcpStream {
        TcpStream::connect((Ipv4Addr::LOCALHOST, port)).expect("connecting")
    }

    fn split(stream: TcpStream) -> (TcpStream, BufReader<TcpStream>) {
        let writer = stream.try_clone().expect("cloning the stream");
        (writer, BufReader::new(stream))
    }

    fn write_message(writer: &mut impl Write, message: Message) {
        message.write(writer).expect("writing an lsp message");
    }

    fn read_message(reader: &mut impl BufRead) -> Message {
        Message::read(reader)
            .expect("reading an lsp message")
            .expect("the connection stayed open")
    }

    fn initialize(writer: &mut impl Write, reader: &mut impl BufRead) -> serde_json::Value {
        initialize_with(writer, reader, serde_json::json!({ "capabilities": {} }))
    }

    fn initialize_with(
        writer: &mut impl Write,
        reader: &mut impl BufRead,
        params: serde_json::Value,
    ) -> serde_json::Value {
        let id = RequestId::from(1);
        write_message(
            writer,
            Message::Request(Request {
                id: id.clone(),
                method: Initialize::METHOD.to_owned(),
                params,
            }),
        );
        let Message::Response(response) = read_message(reader) else {
            panic!("initialize must be answered with a response");
        };
        assert_eq!(&response.id, id.reference());
        assert!(response.error.is_none(), "{response:?}");
        write_message(
            writer,
            Message::Notification(lsp_server::Notification {
                method: Initialized::METHOD.to_owned(),
                params: serde_json::json!({}),
            }),
        );
        response.result.expect("initialize result")
    }

    fn request_response(
        writer: &mut impl Write,
        reader: &mut impl BufRead,
        id: i32,
        method: &str,
        params: serde_json::Value,
    ) -> lsp_server::Response {
        let id = RequestId::from(id);
        write_message(
            writer,
            Message::Request(Request {
                id: id.clone(),
                method: method.to_owned(),
                params,
            }),
        );
        let Message::Response(response) = read_message(reader) else {
            panic!("a request is answered with a response");
        };
        assert_eq!(response.id, id);
        response
    }

    fn semantic_tokens_full(
        writer: &mut impl Write,
        reader: &mut impl BufRead,
        id: i32,
        params: serde_json::Value,
    ) -> lsp_server::Response {
        request_response(
            writer,
            reader,
            id,
            SemanticTokensFullRequest::METHOD,
            params,
        )
    }

    fn formatting(
        writer: &mut impl Write,
        reader: &mut impl BufRead,
        id: i32,
        params: serde_json::Value,
    ) -> lsp_server::Response {
        request_response(writer, reader, id, Formatting::METHOD, params)
    }

    fn will_save_wait_until(
        writer: &mut impl Write,
        reader: &mut impl BufRead,
        id: i32,
        params: serde_json::Value,
    ) -> lsp_server::Response {
        request_response(writer, reader, id, WillSaveWaitUntil::METHOD, params)
    }

    fn formatting_params(uri: &str) -> serde_json::Value {
        serde_json::json!({
            "textDocument": { "uri": uri },
            "options": { "tabSize": 2, "insertSpaces": true }
        })
    }

    fn will_save_wait_until_params(uri: &str) -> serde_json::Value {
        serde_json::json!({
            "textDocument": { "uri": uri },
            "reason": 1
        })
    }

    async fn intern_iso_field(port: u16) {
        notify(
            connect(port),
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/x.ts"),
                presence: Presence::Present("iso(`field Query.x{id}`)".to_owned()),
            })),
        )
        .expect("notify DiskChanged");
        tokio::time::sleep(SETTLE).await;
    }

    fn hello_world() -> lsp_server::Notification {
        lsp_server::Notification {
            method: IsographEventNotification::METHOD.to_owned(),
            params: serde_json::to_value(InternalEvent::HelloWorld).expect("HelloWorld serializes"),
        }
    }

    fn posted_internal(event: IsographEvent) -> InternalEvent {
        let IsographEvent::Lsp(crate::event::LspEvent::Notification(
            crate::event::LspNotification { notification, .. },
        )) = event
        else {
            panic!("session posts Lsp::Notification");
        };
        assert_eq!(notification.method, IsographEventNotification::METHOD);
        serde_json::from_value(notification.params).expect("Internal params")
    }

    fn client_started(event: IsographEvent) -> ClientStartedEvent {
        match event {
            IsographEvent::ClientConnection(ClientConnectionEvent::Started(started)) => started,
            other => panic!("expected ClientConnectionEvent::Started, got {other:?}"),
        }
    }

    fn client_gone(event: IsographEvent) -> ClientId {
        match event {
            IsographEvent::ClientConnection(ClientConnectionEvent::Gone(client)) => client,
            other => panic!("expected ClientConnectionEvent::Gone, got {other:?}"),
        }
    }

    async fn take_started(event_rx: &mut UnboundedReceiver<IsographEvent>) -> ClientStartedEvent {
        client_started(event_rx.recv().await.expect("Started arrived"))
    }

    /// Awaits the next `Started`, assigns it `client`, and returns `client`.
    async fn take_started_and_assign(
        event_rx: &mut UnboundedReceiver<IsographEvent>,
        client: ClientId,
    ) -> ClientId {
        let started = client_started(event_rx.recv().await.expect("Started arrived"));
        started
            .assign
            .send(client)
            .expect("the session waits for its id");
        client
    }

    async fn take_gone(event_rx: &mut UnboundedReceiver<IsographEvent>) -> ClientId {
        client_gone(event_rx.recv().await.expect("Gone arrived"))
    }

    async fn take_request(
        event_rx: &mut UnboundedReceiver<IsographEvent>,
    ) -> crate::event::LspRequest {
        match event_rx.recv().await.expect("Request arrived") {
            IsographEvent::Lsp(crate::event::LspEvent::Request(incoming)) => incoming,
            other => panic!("expected LspEvent::Request, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn notify_hello_world_arrives_as_an_event() {
        let (port, mut event_rx) = listen_for_events().await;
        notify(connect(port), InternalEvent::HelloWorld).expect("notify returns");
        tokio::time::sleep(SETTLE).await;
        let started = take_started_and_assign(&mut event_rx, ClientId(1)).await;
        assert!(matches!(
            posted_internal(event_rx.recv().await.expect("HelloWorld arrived")),
            InternalEvent::HelloWorld
        ));
        assert_eq!(take_gone(&mut event_rx).await, started);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn notify_disk_changed_present_then_absent() {
        let (port, mut event_rx) = listen_for_events().await;
        notify(
            connect(port),
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("export const a = 1;\n".to_owned()),
            })),
        )
        .expect("notify present returns");
        tokio::time::sleep(SETTLE).await;
        let started = take_started_and_assign(&mut event_rx, ClientId(1)).await;
        let InternalEvent::DiskChanged(present) =
            posted_internal(event_rx.recv().await.expect("present arrived"))
        else {
            panic!("present is DiskChanged");
        };
        assert_eq!(
            present,
            DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("export const a = 1;\n".to_owned()),
            })
        );
        assert_eq!(take_gone(&mut event_rx).await, started);
        notify(
            connect(port),
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Absent,
            })),
        )
        .expect("notify absent returns");
        tokio::time::sleep(SETTLE).await;
        let started = take_started_and_assign(&mut event_rx, ClientId(2)).await;
        let InternalEvent::DiskChanged(absent) =
            posted_internal(event_rx.recv().await.expect("absent arrived"))
        else {
            panic!("absent is DiskChanged");
        };
        assert_eq!(
            absent,
            DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Absent,
            })
        );
        assert_eq!(take_gone(&mut event_rx).await, started);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn event_before_initialize_is_not_an_event() {
        let (port, mut event_rx) = listen_for_events().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        write_message(&mut writer, Message::Notification(hello_world()));
        tokio::time::sleep(SETTLE).await;
        assert!(event_rx.try_recv().is_err(), "nothing was dispatched");
        initialize(&mut writer, &mut reader);
        write_message(&mut writer, Message::Notification(hello_world()));
        tokio::time::sleep(SETTLE).await;
        let _started = take_started_and_assign(&mut event_rx, ClientId(1)).await;
        assert!(matches!(
            posted_internal(event_rx.recv().await.expect("the later HelloWorld arrived")),
            InternalEvent::HelloWorld
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn request_before_initialize_is_server_not_initialized() {
        let (port, mut event_rx) = listen_for_events().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        write_message(
            &mut writer,
            Message::Request(Request {
                id: RequestId::from(1),
                method: "textDocument/hover".to_owned(),
                params: serde_json::json!({}),
            }),
        );
        let Message::Response(response) = read_message(&mut reader) else {
            panic!("a request is answered with a response");
        };
        assert_eq!(
            response.error.expect("an error").code,
            ErrorCode::ServerNotInitialized as i32
        );
        assert!(event_rx.try_recv().is_err(), "nothing was dispatched");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unknown_request_after_initialize_is_method_not_found_then_hello_world() {
        let (port, mut event_rx) = listen_and_reply().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        write_message(
            &mut writer,
            Message::Request(Request {
                id: RequestId::from(2),
                method: "textDocument/references".to_owned(),
                params: serde_json::json!({}),
            }),
        );
        let Message::Response(response) = read_message(&mut reader) else {
            panic!("a request is answered with a response");
        };
        assert_eq!(
            response.error.expect("an error").code,
            ErrorCode::MethodNotFound as i32
        );
        write_message(&mut writer, Message::Notification(hello_world()));
        tokio::time::sleep(SETTLE).await;
        let _started = take_started(&mut event_rx).await;
        assert!(matches!(
            posted_internal(event_rx.recv().await.expect("HelloWorld arrived")),
            InternalEvent::HelloWorld
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn shutdown_is_method_not_found_and_does_not_quit() {
        let (port, mut event_rx) = listen_and_reply().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        write_message(
            &mut writer,
            Message::Request(Request {
                id: RequestId::from(2),
                method: Shutdown::METHOD.to_owned(),
                params: serde_json::json!(null),
            }),
        );
        let Message::Response(response) = read_message(&mut reader) else {
            panic!("a request is answered with a response");
        };
        assert_eq!(
            response.error.expect("an error").code,
            ErrorCode::MethodNotFound as i32
        );
        write_message(&mut writer, Message::Notification(hello_world()));
        tokio::time::sleep(SETTLE).await;
        let _started = take_started(&mut event_rx).await;
        assert!(matches!(
            posted_internal(event_rx.recv().await.expect("HelloWorld arrived")),
            InternalEvent::HelloWorld
        ));
        assert!(event_rx.try_recv().is_err(), "shutdown is not Quit");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn truncated_body_ends_that_connection() {
        let (port, mut event_rx) = listen_for_events().await;
        {
            let mut stream = connect(port);
            stream
                .write_all(b"Content-Length: 100\r\n\r\n{")
                .expect("writing a truncated body");
        }
        hello_world_on_a_new_connection(port, &mut event_rx, ClientId(1)).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn initialize_then_event_without_initialized_is_not_an_event() {
        let (port, mut event_rx) = listen_for_events().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        write_message(
            &mut writer,
            Message::Request(Request {
                id: RequestId::from(1),
                method: Initialize::METHOD.to_owned(),
                params: serde_json::json!({ "capabilities": {} }),
            }),
        );
        let _ = read_message(&mut reader);
        write_message(&mut writer, Message::Notification(hello_world()));
        drop(writer);
        drop(reader);
        let event = first_event_after_a_second_connection(port, &mut event_rx).await;
        assert!(
            matches!(
                event,
                IsographEvent::ClientConnection(ClientConnectionEvent::Started(_))
            ),
            "a failed handshake posts nothing, got {event:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unknown_notification_is_not_an_event() {
        let (port, mut event_rx) = listen_for_events().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        write_message(
            &mut writer,
            Message::Notification(lsp_server::Notification {
                method: "window/logMessage".to_owned(),
                params: serde_json::json!({}),
            }),
        );
        tokio::time::sleep(SETTLE).await;
        let _started = take_started_and_assign(&mut event_rx, ClientId(1)).await;
        let IsographEvent::Lsp(crate::event::LspEvent::Notification(
            crate::event::LspNotification {
                notification: unknown,
                ..
            },
        )) = event_rx.recv().await.expect("logMessage arrived")
        else {
            panic!("session posts Lsp::Notification");
        };
        assert_eq!(unknown.method, "window/logMessage");
        write_message(&mut writer, Message::Notification(hello_world()));
        tokio::time::sleep(SETTLE).await;
        assert!(matches!(
            posted_internal(event_rx.recv().await.expect("HelloWorld arrived")),
            InternalEvent::HelloWorld
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bad_event_params_are_not_an_event() {
        let (port, mut event_rx) = listen_for_events().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        write_message(
            &mut writer,
            Message::Notification(lsp_server::Notification {
                method: IsographEventNotification::METHOD.to_owned(),
                params: serde_json::json!({"kind":"Nope"}),
            }),
        );
        tokio::time::sleep(SETTLE).await;
        let _started = take_started_and_assign(&mut event_rx, ClientId(1)).await;
        let IsographEvent::Lsp(crate::event::LspEvent::Notification(
            crate::event::LspNotification {
                notification: bad, ..
            },
        )) = event_rx.recv().await.expect("Nope arrived")
        else {
            panic!("session posts Lsp::Notification");
        };
        assert_eq!(bad.method, IsographEventNotification::METHOD);
        assert_eq!(bad.params, serde_json::json!({"kind":"Nope"}));
        write_message(&mut writer, Message::Notification(hello_world()));
        tokio::time::sleep(SETTLE).await;
        assert!(matches!(
            posted_internal(event_rx.recv().await.expect("HelloWorld arrived")),
            InternalEvent::HelloWorld
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_second_connection_can_hello_world() {
        let (port, mut event_rx) = listen_for_events().await;
        notify(connect(port), InternalEvent::HelloWorld).expect("first notify");
        tokio::time::sleep(SETTLE).await;
        let first = take_started_and_assign(&mut event_rx, ClientId(1)).await;
        assert!(matches!(
            posted_internal(event_rx.recv().await.expect("first HelloWorld")),
            InternalEvent::HelloWorld
        ));
        assert_eq!(take_gone(&mut event_rx).await, first);
        notify(connect(port), InternalEvent::HelloWorld).expect("second notify");
        tokio::time::sleep(SETTLE).await;
        let second = take_started_and_assign(&mut event_rx, ClientId(2)).await;
        assert!(matches!(
            posted_internal(event_rx.recv().await.expect("second HelloWorld")),
            InternalEvent::HelloWorld
        ));
        assert_eq!(take_gone(&mut event_rx).await, second);
    }

    /// `notify` of `HelloWorld` on a new connection: assigns it `client`, then awaits its
    /// `HelloWorld` and its `Gone`.
    async fn hello_world_on_a_new_connection(
        port: u16,
        event_rx: &mut UnboundedReceiver<IsographEvent>,
        client: ClientId,
    ) {
        notify(connect(port), InternalEvent::HelloWorld).expect("notify returns");
        take_started_and_assign(event_rx, client).await;
        assert!(matches!(
            posted_internal(event_rx.recv().await.expect("HelloWorld arrived")),
            InternalEvent::HelloWorld
        ));
        assert_eq!(take_gone(event_rx).await, client);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn two_connections_both_hello_world() {
        let (port, mut event_rx) = listen_for_events().await;
        hello_world_on_a_new_connection(port, &mut event_rx, ClientId(1)).await;
        hello_world_on_a_new_connection(port, &mut event_rx, ClientId(2)).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn two_sequential_notifys_then_a_third() {
        let (port, mut event_rx) = listen_for_events().await;
        hello_world_on_a_new_connection(port, &mut event_rx, ClientId(1)).await;
        hello_world_on_a_new_connection(port, &mut event_rx, ClientId(2)).await;
        hello_world_on_a_new_connection(port, &mut event_rx, ClientId(3)).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn initialize_legend_token_type_15_is_keyword() {
        let (port, _event_rx) = listen_for_events().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        let result = initialize(&mut writer, &mut reader);
        assert_eq!(
            result["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"][15],
            "keyword"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn semantic_tokens_full_of_an_interned_file_starts_with_entrypoint() {
        let (port, _event_rx) = listen_and_reply().await;
        notify(
            connect(port),
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/Home.ts"),
                presence: Presence::Present(
                    "export const Home = iso(`entrypoint Query.HomeRoute`)".to_owned(),
                ),
            })),
        )
        .expect("notify DiskChanged");
        tokio::time::sleep(SETTLE).await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        let response = semantic_tokens_full(
            &mut writer,
            &mut reader,
            2,
            serde_json::json!({
                "textDocument": { "uri": "file:///tmp/proj/src/Home.ts" }
            }),
        );
        assert!(response.error.is_none(), "{response:?}");
        let tokens: lsp_types::SemanticTokensResult =
            serde_json::from_value(response.result.expect("tokens result"))
                .expect("SemanticTokensResult");
        let lsp_types::SemanticTokensResult::Tokens(tokens) = tokens else {
            panic!("full is Tokens");
        };
        assert_eq!(tokens.data[0].token_type, 15);
        assert_eq!(tokens.data[0].length, 10);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn semantic_tokens_full_of_a_never_interned_uri_is_null() {
        let (port, _event_rx) = listen_and_reply().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        let response = semantic_tokens_full(
            &mut writer,
            &mut reader,
            2,
            serde_json::json!({
                "textDocument": { "uri": "file:///tmp/proj/src/never.ts" }
            }),
        );
        assert!(response.error.is_none(), "{response:?}");
        assert!(
            response.result.is_none()
                || response.result.as_ref() == serde_json::Value::Null.reference().wrap_some(),
            "{response:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn semantic_tokens_full_of_a_non_file_uri_is_null() {
        let (port, _event_rx) = listen_and_reply().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        let response = semantic_tokens_full(
            &mut writer,
            &mut reader,
            2,
            serde_json::json!({
                "textDocument": { "uri": "https://example.net/foo.ts" }
            }),
        );
        assert!(response.error.is_none(), "{response:?}");
        assert!(
            response.result.is_none()
                || response.result.as_ref() == serde_json::Value::Null.reference().wrap_some(),
            "{response:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn semantic_tokens_full_with_empty_params_keeps_the_request_id() {
        let (port, _event_rx) = listen_and_reply().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        let id = RequestId::from(2);
        write_message(
            &mut writer,
            Message::Request(Request {
                id: id.clone(),
                method: SemanticTokensFullRequest::METHOD.to_owned(),
                params: serde_json::json!({}),
            }),
        );
        let Message::Response(response) = read_message(&mut reader) else {
            panic!("a request is answered with a response");
        };
        assert_eq!(response.id, id);
        assert_ne!(response.id, RequestId::from("default-lsp-id".to_owned()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn initialize_advertises_formatting_will_save_wait_until_hover_highlight_definition_and_completion()
     {
        let (port, _event_rx) = listen_for_events().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        let result = initialize(&mut writer, &mut reader);
        assert_eq!(
            result["capabilities"]["documentFormattingProvider"],
            serde_json::json!(true)
        );
        assert_eq!(
            result["capabilities"]["hoverProvider"],
            serde_json::json!(true)
        );
        assert_eq!(
            result["capabilities"]["definitionProvider"],
            serde_json::json!(true)
        );
        assert_eq!(
            result["capabilities"]["documentHighlightProvider"],
            serde_json::json!(true)
        );
        assert_eq!(
            result["capabilities"]["textDocumentSync"]["willSaveWaitUntil"],
            serde_json::json!(true)
        );
        assert_eq!(
            result["capabilities"]["textDocumentSync"]["openClose"],
            serde_json::json!(true)
        );
        assert_eq!(
            result["capabilities"]["textDocumentSync"]["change"],
            serde_json::json!(1)
        );
        assert_eq!(
            result["capabilities"]["completionProvider"],
            serde_json::json!({})
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn formatting_of_an_interned_iso_literal_edits_the_interior() {
        let (port, _event_rx) = listen_and_reply().await;
        intern_iso_field(port).await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        let response = formatting(
            &mut writer,
            &mut reader,
            2,
            formatting_params("file:///tmp/proj/src/x.ts"),
        );
        assert!(response.error.is_none(), "{response:?}");
        let edits: Vec<lsp_types::TextEdit> =
            serde_json::from_value(response.result.expect("edits")).expect("Vec<TextEdit>");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "\n  field Query.x {\n    id,\n  }\n");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn formatting_skips_a_failed_literal_and_edits_the_good_one() {
        let (port, _event_rx) = listen_and_reply().await;
        notify(
            connect(port),
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/x.ts"),
                presence: Presence::Present(
                    "iso(`field Query.x{id}`) iso(`not a declaration`)".to_owned(),
                ),
            })),
        )
        .expect("notify DiskChanged");
        tokio::time::sleep(SETTLE).await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        let response = formatting(
            &mut writer,
            &mut reader,
            2,
            formatting_params("file:///tmp/proj/src/x.ts"),
        );
        assert!(response.error.is_none(), "{response:?}");
        let edits: Vec<lsp_types::TextEdit> =
            serde_json::from_value(response.result.expect("edits")).expect("Vec<TextEdit>");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "\n  field Query.x {\n    id,\n  }\n");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn formatting_of_a_never_interned_uri_is_null() {
        let (port, _event_rx) = listen_and_reply().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        let response = formatting(
            &mut writer,
            &mut reader,
            2,
            formatting_params("file:///tmp/proj/src/never.ts"),
        );
        assert!(response.error.is_none(), "{response:?}");
        assert!(
            response.result.is_none()
                || response.result.as_ref() == serde_json::Value::Null.reference().wrap_some(),
            "{response:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn will_save_wait_until_matches_formatting() {
        let (port, _event_rx) = listen_and_reply().await;
        intern_iso_field(port).await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        let formatted = formatting(
            &mut writer,
            &mut reader,
            2,
            formatting_params("file:///tmp/proj/src/x.ts"),
        );
        let will_save = will_save_wait_until(
            &mut writer,
            &mut reader,
            3,
            will_save_wait_until_params("file:///tmp/proj/src/x.ts"),
        );
        assert_eq!(formatted.result, will_save.result);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn semantic_tokens_of_an_interned_field_literal_encodes_field_as_keyword() {
        let (port, _event_rx) = listen_and_reply().await;
        intern_iso_field(port).await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        let response = semantic_tokens_full(
            &mut writer,
            &mut reader,
            2,
            serde_json::json!({
                "textDocument": { "uri": "file:///tmp/proj/src/x.ts" }
            }),
        );
        assert!(response.error.is_none(), "{response:?}");
        let tokens: lsp_types::SemanticTokensResult =
            serde_json::from_value(response.result.expect("tokens result"))
                .expect("SemanticTokensResult");
        let lsp_types::SemanticTokensResult::Tokens(tokens) = tokens else {
            panic!("full is Tokens");
        };
        assert_eq!(tokens.data[0].token_type, 15);
        assert_eq!(tokens.data[0].length, 5);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn semantic_tokens_after_did_open_read_the_buffer_over_disk() {
        let (port, _event_rx) = listen_and_reply().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        let uri = "file:///tmp/proj/src/Home.ts";
        write_message(
            &mut writer,
            Message::Notification(lsp_server::Notification {
                method: IsographEventNotification::METHOD.to_owned(),
                params: serde_json::to_value(InternalEvent::DiskChanged(DiskChanged::File(
                    DiskFileChanged {
                        path: PathBuf::from("/tmp/proj/src/Home.ts"),
                        presence: Presence::Present(
                            "export const Home = iso(`entrypoint Query.HomeRoute`)".to_owned(),
                        ),
                    },
                )))
                .expect("DiskChanged serializes"),
            }),
        );
        write_message(
            &mut writer,
            Message::Notification(lsp_server::Notification {
                method: lsp_types::notification::DidOpenTextDocument::METHOD.to_owned(),
                params: serde_json::json!({
                    "textDocument": {
                        "uri": uri,
                        "languageId": "typescript",
                        "version": 1,
                        "text": "iso(`field Query.x{id}`)"
                    }
                }),
            }),
        );
        let response = semantic_tokens_full(
            &mut writer,
            &mut reader,
            2,
            serde_json::json!({ "textDocument": { "uri": uri } }),
        );
        assert!(response.error.is_none(), "{response:?}");
        let tokens: lsp_types::SemanticTokensResult =
            serde_json::from_value(response.result.expect("tokens result"))
                .expect("SemanticTokensResult");
        let lsp_types::SemanticTokensResult::Tokens(tokens) = tokens else {
            panic!("full is Tokens");
        };
        assert_eq!(tokens.data[0].token_type, 15);
        assert_eq!(tokens.data[0].length, 5);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unknown_request_is_still_method_not_found() {
        let (port, _event_rx) = listen_and_reply().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        write_message(
            &mut writer,
            Message::Request(Request {
                id: RequestId::from(2),
                method: "textDocument/references".to_owned(),
                params: serde_json::json!({}),
            }),
        );
        let Message::Response(response) = read_message(&mut reader) else {
            panic!("a request is answered with a response");
        };
        assert_eq!(
            response.error.expect("an error").code,
            ErrorCode::MethodNotFound as i32
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn initialize_posts_started_with_default_capabilities_then_gone_on_drop() {
        let (port, mut event_rx) = listen_for_events().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        let started = take_started(&mut event_rx).await;
        assert_eq!(
            started.session.capabilities,
            lsp_types::ClientCapabilities::default()
        );
        started
            .assign
            .send(ClientId(1))
            .expect("the session waits for its id");
        drop(writer);
        drop(reader);
        assert_eq!(take_gone(&mut event_rx).await, ClientId(1));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn initialize_stores_publish_diagnostics_and_hover_capabilities() {
        let (port, mut event_rx) = listen_for_events().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        let params = serde_json::json!({
            "capabilities": {
                "textDocument": {
                    "publishDiagnostics": {},
                    "hover": {}
                }
            }
        });
        let expected: lsp_types::InitializeParams =
            serde_json::from_value(params.clone()).expect("InitializeParams");
        initialize_with(&mut writer, &mut reader, params);
        let started = take_started(&mut event_rx).await;
        assert_eq!(started.session.capabilities, expected.capabilities);
        let text_document = started
            .session
            .capabilities
            .text_document
            .as_ref()
            .expect("textDocument");
        assert!(text_document.publish_diagnostics.is_some());
        assert!(text_document.hover.is_some());
        drop(writer);
        drop(reader);
    }

    /// The next event after a second connection initializes: its `Started`, unless an earlier
    /// connection posted something first.
    async fn first_event_after_a_second_connection(
        port: u16,
        event_rx: &mut UnboundedReceiver<IsographEvent>,
    ) -> IsographEvent {
        let (mut writer, mut reader) = split(connect(port));
        initialize(&mut writer, &mut reader);
        event_rx.recv().await.expect("an event arrived")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn initialize_params_that_are_not_initialize_params_post_nothing() {
        let (port, mut event_rx) = listen_for_events().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize_with(&mut writer, &mut reader, serde_json::json!(null));
        drop(writer);
        drop(reader);
        let event = first_event_after_a_second_connection(port, &mut event_rx).await;
        assert!(
            matches!(
                event,
                IsographEvent::ClientConnection(ClientConnectionEvent::Started(_))
            ),
            "bad InitializeParams post nothing, got {event:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn drop_before_initialize_posts_nothing() {
        let (port, mut event_rx) = listen_for_events().await;
        drop(connect(port));
        let event = first_event_after_a_second_connection(port, &mut event_rx).await;
        assert!(
            matches!(
                event,
                IsographEvent::ClientConnection(ClientConnectionEvent::Started(_))
            ),
            "drop before initialize posts nothing, got {event:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_session_whose_assignment_is_dropped_posts_no_gone() {
        let (port, mut event_rx) = listen_for_events().await;
        let (mut writer, mut reader) = split(connect(port));
        initialize(&mut writer, &mut reader);
        let started = take_started(&mut event_rx).await;
        drop(started);
        drop(writer);
        drop(reader);
        let event = first_event_after_a_second_connection(port, &mut event_rx).await;
        assert!(
            matches!(
                event,
                IsographEvent::ClientConnection(ClientConnectionEvent::Started(_))
            ),
            "a dropped assignment posts no Gone, got {event:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_session_forwards_nothing_until_assigned() {
        let (port, mut event_rx) = listen_for_events().await;
        let (mut writer, mut reader) = split(connect(port));
        initialize(&mut writer, &mut reader);
        write_message(
            &mut writer,
            Message::Request(Request {
                id: RequestId::from(2),
                method: lsp_types::request::HoverRequest::METHOD.to_owned(),
                params: serde_json::json!({}),
            }),
        );
        let started = take_started(&mut event_rx).await;
        write_message(&mut writer, Message::Notification(hello_world()));
        started
            .assign
            .send(ClientId(7))
            .expect("the session waits for its id");
        let request = take_request(&mut event_rx).await;
        assert_eq!(request.client, ClientId(7));
        assert_eq!(request.request.id, RequestId::from(2));
        let IsographEvent::Lsp(crate::event::LspEvent::Notification(notification)) =
            event_rx.recv().await.expect("the notification arrived")
        else {
            panic!("the second event is the notification");
        };
        assert_eq!(notification.client, ClientId(7));
        drop(writer);
        drop(reader);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn two_connections_get_different_ids_and_dropping_one_posts_one_gone() {
        let (port, mut event_rx) = listen_for_events().await;
        let stream1 = connect(port);
        let (mut writer1, mut reader1) = split(stream1);
        initialize(&mut writer1, &mut reader1);
        let started1 = take_started_and_assign(&mut event_rx, ClientId(1)).await;
        let stream2 = connect(port);
        let (mut writer2, mut reader2) = split(stream2);
        initialize(&mut writer2, &mut reader2);
        let started2 = take_started_and_assign(&mut event_rx, ClientId(2)).await;
        drop(writer1);
        drop(reader1);
        assert_eq!(take_gone(&mut event_rx).await, started1);
        drop(writer2);
        drop(reader2);
        assert_eq!(take_gone(&mut event_rx).await, started2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn hover_request_carries_the_started_client() {
        let (port, mut event_rx) = listen_for_events().await;
        let stream = connect(port);
        let (mut writer, mut reader) = split(stream);
        initialize(&mut writer, &mut reader);
        write_message(
            &mut writer,
            Message::Request(Request {
                id: RequestId::from(2),
                method: "textDocument/hover".to_owned(),
                params: serde_json::json!({}),
            }),
        );
        tokio::time::sleep(SETTLE).await;
        let started = take_started_and_assign(&mut event_rx, ClientId(1)).await;
        let incoming = take_request(&mut event_rx).await;
        assert_eq!(incoming.client, started);
        assert_eq!(incoming.request.method, "textDocument/hover");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn two_connections_hover_requests_carry_their_started_clients() {
        let (port, mut event_rx) = listen_for_events().await;
        let stream1 = connect(port);
        let (mut writer1, mut reader1) = split(stream1);
        initialize(&mut writer1, &mut reader1);
        write_message(
            &mut writer1,
            Message::Request(Request {
                id: RequestId::from(2),
                method: "textDocument/hover".to_owned(),
                params: serde_json::json!({}),
            }),
        );
        tokio::time::sleep(SETTLE).await;
        let started1 = take_started_and_assign(&mut event_rx, ClientId(1)).await;
        let incoming1 = take_request(&mut event_rx).await;
        assert_eq!(incoming1.client, started1);

        let stream2 = connect(port);
        let (mut writer2, mut reader2) = split(stream2);
        initialize(&mut writer2, &mut reader2);
        write_message(
            &mut writer2,
            Message::Request(Request {
                id: RequestId::from(2),
                method: "textDocument/hover".to_owned(),
                params: serde_json::json!({}),
            }),
        );
        tokio::time::sleep(SETTLE).await;
        let started2 = take_started_and_assign(&mut event_rx, ClientId(2)).await;
        let incoming2 = take_request(&mut event_rx).await;
        assert_eq!(incoming2.client, started2);
        assert_ne!(started1, started2);

        drop(writer1);
        drop(reader1);
        drop(writer2);
        drop(reader2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dropping_one_connection_leaves_the_other_answered() {
        let (port, _event_rx) = listen_and_reply().await;
        let stream1 = connect(port);
        let (mut writer1, mut reader1) = split(stream1);
        initialize(&mut writer1, &mut reader1);
        let stream2 = connect(port);
        let (mut writer2, mut reader2) = split(stream2);
        initialize(&mut writer2, &mut reader2);
        drop(writer1);
        drop(reader1);
        tokio::time::sleep(SETTLE).await;
        write_message(
            &mut writer2,
            Message::Request(Request {
                id: RequestId::from(2),
                method: "textDocument/references".to_owned(),
                params: serde_json::json!({}),
            }),
        );
        let Message::Response(response) = read_message(&mut reader2) else {
            panic!("a request is answered with a response");
        };
        assert_eq!(
            response.error.expect("an error").code,
            ErrorCode::MethodNotFound as i32
        );
    }
}
