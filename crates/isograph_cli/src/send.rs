use std::fs;
use std::io;
use std::net::Ipv4Addr;
use std::num::NonZeroU16;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use lsp_types::notification::{Initialized, Notification};
use lsp_types::request::{Initialize, Request};
use prelude::Postfix;

use crate::SendArgs;
use crate::discover::DiscoverError;
use crate::event::Internal;
use crate::lsp_socket::IsographEventNotification;

#[derive(Debug)]
pub(crate) struct ReadFile {
    pub path: PathBuf,
    pub source: io::Error,
}

#[derive(Debug)]
pub(crate) struct ReadPort {
    pub path: PathBuf,
    pub source: io::Error,
}

#[derive(Debug)]
pub(crate) struct Connect {
    pub port: u16,
    pub source: io::Error,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum SendError {
    #[error("{0}")]
    Discover(#[from] DiscoverError),
    #[error("could not read {}: {}", .0.path.display(), .0.source)]
    ReadFile(ReadFile),
    #[error("the frame is not Internal JSON: {0}")]
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

#[expect(clippy::print_stderr)]
pub fn run(args: &SendArgs) -> ExitCode {
    let result = run_inner(args);
    let _ = fs::remove_file(args.file.reference());
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
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
    let internal: Internal = serde_json::from_str(frame.trim()).map_err(SendError::NotEvent)?;
    let stream = std::net::TcpStream::connect((Ipv4Addr::LOCALHOST, port))
        .map_err(|source| SendError::Connect(Connect { port, source }))?;
    notify(stream, internal)
}

pub(crate) fn notify(stream: std::net::TcpStream, event: Internal) -> Result<(), SendError> {
    let mut writer = stream.try_clone().map_err(SendError::Clone)?;
    let mut reader = std::io::BufReader::new(stream);
    // JSON-RPC ids are per connection. This client has one outstanding request. A second send is another connection.
    let id = lsp_server::RequestId::from(1);
    lsp_server::Message::Request(lsp_server::Request {
        id: id.clone(),
        method: Initialize::METHOD.to_owned(),
        params: serde_json::json!({ "capabilities": {} }),
    })
    .write(&mut writer)
    .map_err(SendError::Write)?;
    wait_for_initialize_result(&mut reader, id.reference())?;
    lsp_server::Message::Notification(lsp_server::Notification {
        method: Initialized::METHOD.to_owned(),
        params: serde_json::json!({}),
    })
    .write(&mut writer)
    .map_err(SendError::Write)?;
    let params = serde_json::to_value(&event).map_err(SendError::Encode)?;
    lsp_server::Message::Notification(lsp_server::Notification {
        method: IsographEventNotification::METHOD.to_owned(),
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

fn require_running(lock: &Path) -> Result<(), SendError> {
    match freddie_single_instance::holder_at(lock)? {
        freddie_single_instance::Held::By(_) => ().wrap_ok(),
        freddie_single_instance::Held::Free => SendError::NotRunning.wrap_err(),
        freddie_single_instance::Held::Unnamed => SendError::Unnamed.wrap_err(),
    }
}

fn read_port(path: &Path) -> Result<u16, SendError> {
    match fs::read_to_string(path) {
        Ok(text) => parse_port(text.reference()).ok_or(SendError::BadPort),
        Err(source) if source.kind() == io::ErrorKind::NotFound => SendError::NoPort.wrap_err(),
        Err(source) => SendError::ReadPort(ReadPort {
            path: path.to_owned(),
            source,
        })
        .wrap_err(),
    }
}

fn parse_port(text: &str) -> Option<u16> {
    text.trim().parse::<NonZeroU16>().ok().map(NonZeroU16::get)
}

#[cfg(test)]
mod tests {
    use prelude::Postfix;

    use super::parse_port;

    #[test]
    fn parse_port_reads_a_decimal_line() {
        assert_eq!(parse_port("53124\n"), 53124.wrap_some());
    }

    #[test]
    fn parse_port_reads_digits_without_a_newline() {
        assert_eq!(parse_port("53124"), 53124.wrap_some());
    }

    #[test]
    fn parse_port_of_empty_is_none() {
        assert_eq!(parse_port(""), None);
        assert_eq!(parse_port("\n"), None);
    }

    #[test]
    fn parse_port_of_zero_is_none() {
        assert_eq!(parse_port("0"), None);
        assert_eq!(parse_port("0\n"), None);
    }

    #[test]
    fn parse_port_of_garbage_is_none() {
        assert_eq!(parse_port("abc"), None);
        assert_eq!(parse_port("65536"), None);
        assert_eq!(parse_port("127.0.0.1:53124"), None);
    }

    #[test]
    fn read_port_of_a_missing_file_is_no_port() {
        let dir = tempfile::tempdir().expect("a test can create a temp directory");
        let path = dir.path().join("gone.port");
        let err = super::read_port(path.reference()).expect_err("the file is missing");
        assert!(matches!(err, super::SendError::NoPort));
    }
}
