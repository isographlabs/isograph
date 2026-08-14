//! The isograph binary: freddie's lifecycle verbs around a daemon that, for now, only says hello.

use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches, Parser};
use freddie_cli::{App, Instance, NoArgs};
use prelude::Postfix;

#[derive(Parser)]
#[command(name = "isograph", version, about = "The isograph compiler.", long_about = None)]
struct IsographCli {
    #[command(subcommand)]
    verb: Option<freddie_cli::Verb<Isograph>>,
}

/// The flags the daemon takes: none yet.
///
/// Not [`NoArgs`], because `start` flattens [`App::Id`] and [`App::DaemonArgs`] into one clap
/// command, and clap requires the two derived argument groups to have distinct names.
#[derive(clap::Args, Debug)]
pub struct IsographArgs;

/// isograph, to the verbs that manage it.
pub struct Isograph;

impl App for Isograph {
    // One isograph daemon to a machine, so no flag names which.
    type Id = NoArgs;
    type DaemonArgs = IsographArgs;

    const NAME: &'static str = "isograph";

    fn instance(_: &NoArgs) -> Result<Instance, Box<dyn std::error::Error + Send + Sync>> {
        Instance::global(Self::NAME)?.wrap_ok()
    }

    fn run_daemon(_: &NoArgs, _: &IsographArgs) {
        tracing::info!("hello from isograph");
        loop {
            std::thread::park();
        }
    }
}

fn main() -> ExitCode {
    // First, so `--help` prints and a bad flag exits before the lock is taken.
    // The matches are kept beside the parse because `run_lifecycle_verb` reads what was written
    // from them, to forward to the daemon it spawns.
    let matches = IsographCli::command().get_matches();
    let cli = IsographCli::from_arg_matches(&matches)
        .expect("the derived type matches the command it derived");

    match cli.verb {
        Some(verb) => freddie_cli::run_lifecycle_verb::<Isograph>(verb, &matches),
        None => freddie_cli::run_lifecycle_verb::<Isograph>(
            freddie_cli::verb_for_bare_invocation::<Isograph>(),
            &matches,
        ),
    }
}
