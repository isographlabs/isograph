use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches, Parser};
use freddie_cli::{App, Instance, NoArgs};
use prelude::Postfix;

mod config_path;
mod daemon;
mod discover;
mod effect;
mod event;
mod external;
mod send;
mod state;

pub fn run() -> ExitCode {
    // First, so `--help` prints and a bad flag exits before the lock is taken.
    // The matches are kept beside the parse because `run_lifecycle_verb` reads what was written
    // from them, to forward to the daemon it spawns.
    let matches = Cli::command().get_matches();
    let cli = Cli::from_arg_matches(matches.reference())
        .expect("the derived type matches the command it derived");

    match cli.verb {
        Some(CliVerb::Lifecycle(verb)) => {
            freddie_cli::run_lifecycle_verb::<Isograph>(verb, matches.reference())
        }
        Some(CliVerb::Send(args)) => send::run(args.reference()),
        Some(CliVerb::ConfigPath(id)) => config_path::run(id.reference()),
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
    verb: Option<CliVerb>,
}

#[derive(clap::Subcommand)]
enum CliVerb {
    /// start, restart, status, logs, stop, and the hidden daemon.
    #[command(flatten)]
    Lifecycle(freddie_cli::Verb<Isograph>),

    /// Write one IsographEvent JSON frame to the running daemon. Not for typing: tests and CI.
    #[command(hide = true)]
    Send(SendArgs),

    /// Print the canonical isograph config path.
    ConfigPath(ConfigFlag),
}

#[derive(clap::Args, Debug)]
struct SendArgs {
    #[command(flatten)]
    pub id: ConfigFlag,

    /// JSON frame to send.
    #[arg(long)]
    pub file: std::path::PathBuf,
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
        let (path, instance) = match discover::instance_for_config_path(id.config.as_deref()) {
            Ok(pair) => pair,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "the config went away between naming this daemon and starting it"
                );
                return;
            }
        };
        let port_path = discover::port_file(instance.lock_file());
        let _ = std::fs::remove_file(port_path.reference());
        if let Err(e) = discover::load_config(path.reference()) {
            tracing::error!(error = %e, "could not load the config");
            return;
        }
        crate::daemon::run(path, port_path);
    }
}
