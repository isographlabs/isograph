use std::io::{self, Write, copy};
use std::net::TcpStream;
use std::process::{Command, ExitCode, ExitStatus, Stdio};
use std::thread;

use prelude::Postfix;

use crate::LspArgs;
use crate::discover::DiscoverError;

#[derive(Debug, thiserror::Error)]
enum LspError {
    #[error("{0}")]
    Discover(#[from] DiscoverError),
    #[error("could not find this executable: {0}")]
    CurrentExe(io::Error),
    #[error("could not spawn isograph start: {0}")]
    Spawn(io::Error),
    #[error("{0}")]
    Port(#[from] crate::discover::PortError),
    #[error("could not clone the stream: {0}")]
    Clone(io::Error),
}

struct FlushStdout(io::Stdout);

impl Write for FlushStdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.0.write(buf)?;
        self.0.flush()?;
        n.wrap_ok()
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

#[expect(clippy::print_stderr)]
pub fn run(args: &LspArgs) -> ExitCode {
    match start_daemon(args) {
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
        Ok(status) if !status.success() => exit_from_status(status),
        Ok(_) => match connect_pair(args) {
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
            Ok((to_daemon, from_daemon)) => copy_stdio(to_daemon, from_daemon),
        },
    }
}

fn start_daemon(args: &LspArgs) -> Result<ExitStatus, LspError> {
    let _ = crate::discover::instance_for_config_path(args.id.config.as_deref())?;
    let exe = std::env::current_exe().map_err(LspError::CurrentExe)?;
    let mut command = Command::new(exe);
    command
        .arg("start")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    if let Some(config) = args.id.config.as_deref() {
        command.arg("--config").arg(config);
    }
    command.status().map_err(LspError::Spawn)
}

fn connect_pair(args: &LspArgs) -> Result<(TcpStream, TcpStream), LspError> {
    let (_, instance) = crate::discover::instance_for_config_path(args.id.config.as_deref())?;
    let stream =
        crate::discover::connect_to_daemon(&crate::discover::port_file(instance.lock_file()))?;
    let to_daemon = stream.try_clone().map_err(LspError::Clone)?;
    (to_daemon, stream).wrap_ok()
}

fn exit_from_status(status: ExitStatus) -> ExitCode {
    match status.code() {
        Some(code) => u8::try_from(code)
            .map(ExitCode::from)
            .unwrap_or(ExitCode::FAILURE),
        None => ExitCode::FAILURE,
    }
}

fn copy_stdio(mut to_daemon: TcpStream, mut from_daemon: TcpStream) -> ! {
    thread::spawn(move || {
        let _ = copy(&mut io::stdin(), &mut to_daemon);
        std::process::exit(0);
    });
    let _ = copy(&mut from_daemon, &mut FlushStdout(io::stdout()));
    std::process::exit(0);
}
