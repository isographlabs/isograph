use std::fs;
use std::net::{Ipv4Addr, TcpStream};
use std::path::Path;
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

use clap::ArgMatches;
use clap::error::ErrorKind;
use freddie_cli::{App, Verb};
use isograph_compiler::HostLanguage;
use prelude::Postfix;

use crate::Isograph;
use crate::discover;

const DEADLINE: Duration = Duration::from_secs(5);
const SLEEP: Duration = Duration::from_millis(10);

#[expect(clippy::print_stderr)]
pub fn run<THostLanguage: HostLanguage>(
    verb: Verb<Isograph<THostLanguage>>,
    matches: &ArgMatches,
) -> ExitCode {
    let id = match &verb {
        Verb::Start(args) => &args.id,
        Verb::Restart(args) => &args.id,
        _ => {
            return freddie_cli::run_lifecycle_verb::<Isograph<THostLanguage>>(verb, matches);
        }
    };
    let instance = match Isograph::<THostLanguage>::instance(id) {
        Ok(instance) => instance,
        Err(e) => clap::Error::raw(ErrorKind::ValueValidation, format!("{e}\n")).exit(),
    };
    let port_path = discover::port_file(instance.lock_file());
    let code = freddie_cli::run_lifecycle_verb::<Isograph<THostLanguage>>(verb, matches);
    match freddie_single_instance::holder_at(instance.lock_file()) {
        Ok(freddie_single_instance::Held::By(_)) | Ok(freddie_single_instance::Held::Unnamed) => {}
        Ok(freddie_single_instance::Held::Free) | Err(_) => return ExitCode::FAILURE,
    }
    match wait_until_listening(port_path.reference(), instance.lock_file()) {
        Ok(()) => code,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum WaitError {
    #[error("the daemon did not open the lsp port")]
    NotListening,
}

fn wait_until_listening(port_path: &Path, lock: &Path) -> Result<(), WaitError> {
    let deadline = Instant::now() + DEADLINE;
    loop {
        match freddie_single_instance::holder_at(lock) {
            Ok(freddie_single_instance::Held::Free) | Err(_) => {
                return WaitError::NotListening.wrap_err();
            }
            Ok(_) => {}
        }
        if let Ok(text) = fs::read_to_string(port_path)
            && let Some(port) = discover::parse_port(text.reference())
            && let Ok(_stream) = TcpStream::connect((Ipv4Addr::LOCALHOST, port))
        {
            return ().wrap_ok();
        }
        if Instant::now() >= deadline {
            return WaitError::NotListening.wrap_err();
        }
        thread::sleep(SLEEP);
    }
}
