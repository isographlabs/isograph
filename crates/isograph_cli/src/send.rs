use std::fs;
use std::io;
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
    #[error("{0}")]
    Port(#[from] crate::discover::PortError),
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
    let stream =
        crate::discover::connect_to_daemon(&crate::discover::port_file(instance.lock_file()))?;
    let frame = fs::read_to_string(args.file.reference()).map_err(|source| {
        SendError::ReadFile(ReadFile {
            path: args.file.clone(),
            source,
        })
    })?;
    let internal: Internal = serde_json::from_str(frame.trim()).map_err(SendError::NotEvent)?;
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

#[cfg(test)]
mod tests {
    use prelude::Postfix;

    #[test]
    fn read_port_of_a_missing_file_is_no_port() {
        let dir = tempfile::tempdir().expect("a test can create a temp directory");
        let path = dir.path().join("gone.port");
        let err = crate::discover::read_port(path.reference())
            .map_err(super::SendError::from)
            .expect_err("the file is missing");
        assert!(matches!(
            err,
            super::SendError::Port(crate::discover::PortError::NoPort)
        ));
    }
}
