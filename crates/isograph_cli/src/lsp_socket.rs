use std::{
    io::{self, BufReader},
    net::TcpStream,
    thread,
};

use crossbeam::channel::{Receiver, Sender, bounded};
use lsp_server::Message;
use lsp_types::notification::{Exit, Notification};
use prelude::Postfix;
use tracing::{debug, warn};

#[derive(Debug)]
pub enum IsographEventNotification {}

impl Notification for IsographEventNotification {
    type Params = crate::event::IsographEvent;
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
                let _ = event_tx.send(
                    crate::event::LspRequest {
                        request,
                        reply: connection.sender.clone(),
                    }
                    .to(),
                );
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

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::{Ipv4Addr, TcpStream};
    use std::path::PathBuf;
    use std::time::Duration;

    use lsp_server::{ErrorCode, Message, Request, RequestId};
    use lsp_types::notification::{Initialized, Notification};
    use lsp_types::request::{Initialize, Request as LspRequest, Shutdown};
    use prelude::Postfix;
    use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

    use super::{IsographEventNotification, accept_loop};
    use crate::daemon::{run_effect_loop, run_event_loop};
    use crate::event::{DiskChanged, DiskFileChanged, IsographEvent, Presence};
    use crate::send::notify;
    use crate::state::IsographState;
    use isograph_extract_typescript::TypeScriptHostLanguage;

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
        let (effect_tx, effect_rx) = unbounded_channel();
        tokio::spawn(accept_loop(listener, session_tx));
        tokio::spawn(async move {
            while let Some(event) = session_rx.recv().await {
                if !matches!(event, IsographEvent::LspRequest(_)) {
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
            runtime.block_on(async {
                tokio::select! {
                    () = run_event_loop(
                        IsographState::<TypeScriptHostLanguage>::default(),
                        loop_rx,
                        effect_tx,
                    ) => {}
                    () = run_effect_loop(effect_rx) => {}
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

    fn initialize(writer: &mut impl Write, reader: &mut impl BufRead) {
        let id = RequestId::from(1);
        write_message(
            writer,
            Message::Request(Request {
                id: id.clone(),
                method: Initialize::METHOD.to_owned(),
                params: serde_json::json!({ "capabilities": {} }),
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
    }

    fn hello_world() -> lsp_server::Notification {
        lsp_server::Notification {
            method: IsographEventNotification::METHOD.to_owned(),
            params: serde_json::to_value(IsographEvent::HelloWorld).expect("HelloWorld serializes"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn notify_hello_world_arrives_as_an_event() {
        let (port, mut event_rx) = listen_for_events().await;
        notify(connect(port), IsographEvent::HelloWorld).expect("notify returns");
        tokio::time::sleep(SETTLE).await;
        assert!(matches!(
            event_rx.try_recv().expect("an event arrived"),
            IsographEvent::HelloWorld
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn notify_disk_changed_present_then_absent() {
        let (port, mut event_rx) = listen_for_events().await;
        notify(
            connect(port),
            IsographEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("export const a = 1;\n".to_owned()),
            })),
        )
        .expect("notify present returns");
        tokio::time::sleep(SETTLE).await;
        let IsographEvent::DiskChanged(present) = event_rx.try_recv().expect("present arrived")
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
        notify(
            connect(port),
            IsographEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Absent,
            })),
        )
        .expect("notify absent returns");
        tokio::time::sleep(SETTLE).await;
        let IsographEvent::DiskChanged(absent) = event_rx.try_recv().expect("absent arrived")
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
        assert!(matches!(
            event_rx.try_recv().expect("the later HelloWorld arrived"),
            IsographEvent::HelloWorld
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
                method: "textDocument/hover".to_owned(),
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
        assert!(matches!(
            event_rx.try_recv().expect("HelloWorld arrived"),
            IsographEvent::HelloWorld
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
        assert!(matches!(
            event_rx.try_recv().expect("HelloWorld arrived"),
            IsographEvent::HelloWorld
        ));
        assert!(
            !matches!(event_rx.try_recv(), Ok(IsographEvent::Quit)),
            "shutdown is not Quit"
        );
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
        tokio::time::sleep(SETTLE).await;
        notify(connect(port), IsographEvent::HelloWorld).expect("a second connection works");
        tokio::time::sleep(SETTLE).await;
        assert!(matches!(
            event_rx.try_recv().expect("HelloWorld arrived"),
            IsographEvent::HelloWorld
        ));
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
        tokio::time::sleep(SETTLE).await;
        assert!(event_rx.try_recv().is_err(), "nothing was dispatched");
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
        assert!(event_rx.try_recv().is_err(), "nothing was dispatched");
        write_message(&mut writer, Message::Notification(hello_world()));
        tokio::time::sleep(SETTLE).await;
        assert!(matches!(
            event_rx.try_recv().expect("HelloWorld arrived"),
            IsographEvent::HelloWorld
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
        assert!(event_rx.try_recv().is_err(), "nothing was dispatched");
        write_message(&mut writer, Message::Notification(hello_world()));
        tokio::time::sleep(SETTLE).await;
        assert!(matches!(
            event_rx.try_recv().expect("HelloWorld arrived"),
            IsographEvent::HelloWorld
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_second_connection_can_hello_world() {
        let (port, mut event_rx) = listen_for_events().await;
        notify(connect(port), IsographEvent::HelloWorld).expect("first notify");
        tokio::time::sleep(SETTLE).await;
        assert!(matches!(
            event_rx.try_recv().expect("first HelloWorld"),
            IsographEvent::HelloWorld
        ));
        notify(connect(port), IsographEvent::HelloWorld).expect("second notify");
        tokio::time::sleep(SETTLE).await;
        assert!(matches!(
            event_rx.try_recv().expect("second HelloWorld"),
            IsographEvent::HelloWorld
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn two_connections_both_hello_world() {
        let (port, mut event_rx) = listen_for_events().await;
        notify(connect(port), IsographEvent::HelloWorld).expect("first notify");
        notify(connect(port), IsographEvent::HelloWorld).expect("second notify");
        tokio::time::sleep(SETTLE).await;
        let first = event_rx.try_recv().expect("one HelloWorld");
        let second = event_rx.try_recv().expect("the other HelloWorld");
        assert!(matches!(first, IsographEvent::HelloWorld));
        assert!(matches!(second, IsographEvent::HelloWorld));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn two_sequential_notifys_then_a_third() {
        let (port, mut event_rx) = listen_for_events().await;
        notify(connect(port), IsographEvent::HelloWorld).expect("first");
        notify(connect(port), IsographEvent::HelloWorld).expect("second");
        notify(connect(port), IsographEvent::HelloWorld).expect("third");
        tokio::time::sleep(SETTLE).await;
        for i in 1..=3 {
            assert!(
                matches!(
                    event_rx.try_recv().unwrap_or_else(|_| panic!("event {i}")),
                    IsographEvent::HelloWorld
                ),
                "event {i}"
            );
        }
    }
}
