use std::fs;
use std::io;
use std::num::NonZeroU16;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use prelude::Postfix;
use tungstenite::Message;
use tungstenite::client::connect;

use crate::SendArgs;
use crate::discover::DiscoverError;
use crate::event::IsographEvent;

#[derive(Debug)]
struct ReadFile {
    pub path: PathBuf,
    pub source: io::Error,
}

#[derive(Debug)]
struct ReadPort {
    pub path: PathBuf,
    pub source: io::Error,
}

#[derive(Debug)]
struct Connect {
    pub port: u16,
    pub source: Box<tungstenite::Error>,
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
    #[error("could not write the frame: {0}")]
    Write(Box<tungstenite::Error>),
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
    let frame = frame.trim();
    let _: IsographEvent = serde_json::from_str(frame).map_err(SendError::NotEvent)?;
    let (mut ws, _) = connect(format!("ws://127.0.0.1:{port}")).map_err(|source| {
        SendError::Connect(Connect {
            port,
            source: source.boxed(),
        })
    })?;
    ws.send(Message::Text(frame.to_owned()))
        .map_err(|e| SendError::Write(e.boxed()))?;
    ().wrap_ok()
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
