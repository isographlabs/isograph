use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches, Parser};
use freddie_cli::{App, Instance, NoArgs};
use prelude::Postfix;

mod discover;

pub fn run() -> ExitCode {
    // First, so `--help` prints and a bad flag exits before the lock is taken.
    // The matches are kept beside the parse because `run_lifecycle_verb` reads what was written
    // from them, to forward to the daemon it spawns.
    let matches = Cli::command().get_matches();
    let cli = Cli::from_arg_matches(matches.reference())
        .expect("the derived type matches the command it derived");

    match cli.verb {
        Some(verb) => freddie_cli::run_lifecycle_verb::<Isograph>(verb, matches.reference()),
        None => freddie_cli::run_lifecycle_verb::<Isograph>(
            freddie_cli::verb_for_bare_invocation::<Isograph>(),
            matches.reference(),
        ),
    }
}

#[derive(Parser)]
#[command(name = "isograph", version, about = "The isograph compiler.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    verb: Option<freddie_cli::Verb<Isograph>>,
}

#[derive(clap::Args, Debug)]
struct ConfigFlag {
    /// Path to the isograph config. When absent, the nearest isograph.config.json, .js, or .ts
    /// at or above the current directory.
    #[arg(long)]
    pub config: Option<std::path::PathBuf>,
}

struct Isograph;

impl App for Isograph {
    type Id = ConfigFlag;
    type DaemonArgs = NoArgs;

    const NAME: &'static str = "isograph";

    fn instance(id: &ConfigFlag) -> Result<Instance, Box<dyn std::error::Error + Send + Sync>> {
        let (_, instance, _) = discover::config_and_instance(id.config.as_deref())?;
        instance.wrap_ok()
    }

    fn run_daemon(id: &ConfigFlag, _: &NoArgs) {
        match discover::config_and_instance(id.config.as_deref()) {
            Ok((path, _, _config)) => {
                tracing::info!(config = %path.display(), "isograph daemon up");
                loop {
                    std::thread::park();
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "the config went away between naming this daemon and starting it");
            }
        }
    }
}
