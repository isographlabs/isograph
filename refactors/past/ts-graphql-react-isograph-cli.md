# `ts_graphql_react_isograph_cli` is the `isograph` binary

`isograph_cli` is a library: freddie lifecycle verbs around the parked daemon. `ts_graphql_react_isograph_cli` is the consumer. It exports the `isograph` binary and calls `isograph_cli::run`. Behavior is unchanged: start, status, logs, stop, same `App::NAME`.

The crate is named for the profile it will compose (TypeScript host, GraphQL protocol, React artifacts). This change does not pass those type params. `run` takes none.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/lib.rs
use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches, Parser};
use freddie_cli::{App, Instance, NoArgs};
use prelude::Postfix;

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

/// The flags the daemon takes: none yet.
///
/// Not [`NoArgs`], because `start` flattens [`App::Id`] and [`App::DaemonArgs`] into one clap
/// command, and clap requires the two derived argument groups to have distinct names.
#[derive(clap::Args, Debug)]
struct IsographArgs;

/// isograph, to the verbs that manage it.
struct Isograph;

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
```

Before: this file is `crates/isograph_cli/src/main.rs`. The clap type is `IsographCli`. `Isograph` and `IsographArgs` are `pub`. `fn main` is the body of `run`. After: `lib.rs`; clap type is `Cli`; `Isograph` and `IsographArgs` are crate-private; `run` is the public entry.

The comment on `IsographArgs` (not `NoArgs`, because `start` flattens `App::Id` and `App::DaemonArgs` and clap requires distinct group names) stays on `IsographArgs`.

The comment on `main` (parse first so `--help` prints and a bad flag exits before the lock; matches kept beside the parse because `run_lifecycle_verb` reads them) stays on `run`.

```rust
// from crates/ts_graphql_react_isograph_cli/src/main.rs
//! The isograph binary: TypeScript host, GraphQL protocol, React artifacts.

use std::process::ExitCode;

fn main() -> ExitCode {
    isograph_cli::run()
}
```

## Change 1: library and binary crate

Both crates are root workspace members via `./crates/*`. No exclude. No second lockfile.

```toml
# from crates/ts_graphql_react_isograph_cli/Cargo.toml
[package]
name = "ts_graphql_react_isograph_cli"
version = { workspace = true }
edition = { workspace = true }
license = { workspace = true }

[[bin]]
name = "isograph"
path = "src/main.rs"

[dependencies]
isograph_cli = { path = "../isograph_cli" }

[dev-dependencies]
prelude = { path = "../prelude" }
tempfile = "3"

[lints]
workspace = true
```

```toml
# from crates/isograph_cli/Cargo.toml
[package]
name = "isograph_cli"
version = { workspace = true }
edition = { workspace = true }
license = { workspace = true }

[dependencies]
clap = { workspace = true }
freddie_cli = { git = "https://github.com/freddiehg/freddie", rev = "c81e3782d53ca1775cbc761dab322f296d9b25a4" }
prelude = { path = "../prelude" }
tracing = { workspace = true }

[lints]
workspace = true
```

Before: `[[bin]] name = "isograph"` and `[dev-dependencies]` tempfile on `isograph_cli`. After: `isograph_cli` is a `[lib]` only; tempfile moves with the tests.

`crates/isograph_cli/src/main.rs` is deleted. `crates/isograph_cli/tests/cli.rs` moves to `crates/ts_graphql_react_isograph_cli/tests/cli.rs`. Contents of the test file are unchanged: `CARGO_BIN_EXE_isograph` is set for the package that owns the bin.

```
# from AGENTS.md
The new work is `crates/isograph_parser` (the parser), `crates/tests` (its tests), `crates/isograph_cli` (freddie_cli's lifecycle verbs around the daemon), and `crates/ts_graphql_react_isograph_cli` (the `isograph` binary).
```

Before: the sentence ends at `crates/isograph_cli` (freddie_cli's lifecycle verbs around the daemon). After: the binary crate is named.

cli-ci-build.md's cargo bin is `isograph` from `ts_graphql_react_isograph_cli`. The artifact file name stays `isograph_cli`.

Later CLI docs that name `crates/isograph_cli/src/main.rs` mean `crates/isograph_cli/src/lib.rs`. Tests that drive `CARGO_BIN_EXE_isograph` live in `crates/ts_graphql_react_isograph_cli/tests`.

## Order

1. Change 1; library, binary crate, tests move, AGENTS.md.
