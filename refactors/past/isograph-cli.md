# No-op isograph CLI

The `isograph` binary is figaro's CLI: `freddie_cli`'s lifecycle verbs around an `App`. The daemon logs that it is up and parks. It compiles nothing.

`freddie_cli` is the same crate figaro uses, same rev.

## What the user does

```
$ isograph
$ isograph status
isograph: running, pid 12345
$ isograph logs
{"timestamp":"...","level":"INFO","fields":{"message":"hello from isograph"}}
$ isograph stop
```

Bare `isograph` is `start`. `logs` follows the daemon's tracing file. `stop` sends SIGTERM, which ends a parked process.

## Change 1: the binary is the lifecycle verbs around a parked daemon

`crates/isograph_cli` is already this crate: excluded from the workspace (freddie_cli's serde vs swc), binary name `isograph`, `freddie_cli` + `clap` + `prelude` + `tracing`.

```rust
// from crates/isograph_cli/src/main.rs
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

pub struct Isograph;

impl App for Isograph {
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
    let matches = IsographCli::command().get_matches();
    let cli = IsographCli::from_arg_matches(matches.reference())
        .expect("the derived type matches the command it derived");

    match cli.verb {
        Some(verb) => freddie_cli::run_lifecycle_verb::<Isograph>(verb, matches.reference()),
        None => freddie_cli::run_lifecycle_verb::<Isograph>(
            freddie_cli::verb_for_bare_invocation::<Isograph>(),
            matches.reference(),
        ),
    }
}
```

One daemon per machine. Config identity is the next doc.

`expect` on `from_arg_matches` is the same line as figaro: the derived type matches the command it derived.

## Tests

Drive the built binary (`CARGO_BIN_EXE_isograph`, so the tests live in this crate, not `crates/tests`). Point `HOME` (and `XDG_STATE_HOME` / `LOCALAPPDATA`) at a temp directory so locks and logs are private. `tempfile` is a dev-dependency.

- `start` then `status` reports running.
- `logs` contains `hello from isograph`.
- `stop` then `status` reports not running.
- A second `start` while one is running is not an error (freddie_cli adopts it).
