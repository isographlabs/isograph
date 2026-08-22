# Print the resolved config path

Requires config-discovery.md (landed).

`isograph config-path` prints the canonical path of the isograph config for this invocation: `--config` when given, otherwise the nearest `isograph.config.json`, `.js`, or `.ts` at or above the current directory. It does not start the daemon, take the lock, or parse the file as JSON. Exit 0 with the path on stdout. Exit 1 when there is no config.

Origin of the walk-up: `crates/isograph_cli/src/discover.rs` `config_path`. Origin of extra verbs beside `freddie_cli::Verb`: figaro `src/cli/mod.rs`. Delta: one verb, `config-path`. This file does not depend on send-events.md. If `CliVerb` already exists, add `ConfigPath` to it. If not, the snippets introduce `CliVerb`.

## What the user does

```
$ cd /Users/x/app/src
$ isograph config-path
/Users/x/app/isograph.config.json
$ isograph config-path --config /Users/x/other/isograph.config.json
/Users/x/other/isograph.config.json
$ cd /tmp
$ isograph config-path
no isograph.config.json, isograph.config.js, or isograph.config.ts at or above /tmp; create one, or pass one using the --config flag
```

The last process exits 1. stdout is empty. The message is on stderr.

## Types

Most important first. `config_path` already exists. This change adds a verb that calls it.

```rust
// from crates/isograph_cli/src/lib.rs
#[derive(clap::Subcommand)]
enum CliVerb {
    /// start, restart, status, logs, stop, and the hidden daemon.
    #[command(flatten)]
    Lifecycle(freddie_cli::Verb<Isograph>),

    /// Print the canonical isograph config path.
    ConfigPath(ConfigFlag),
}
```

`ConfigFlag` is the same `--config` every other verb takes.

## Change 1: `isograph config-path`

```rust
// from crates/isograph_cli/src/lib.rs (before)
#[derive(Parser)]
#[command(name = "isograph", version, about = "The isograph compiler.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    verb: Option<freddie_cli::Verb<Isograph>>,
}
```

```rust
// from crates/isograph_cli/src/lib.rs (after)
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

    /// Print the canonical isograph config path.
    ConfigPath(ConfigFlag),
}
```

```rust
// from crates/isograph_cli/src/lib.rs (before)
    match cli.verb {
        Some(verb) => freddie_cli::run_lifecycle_verb::<Isograph>(verb, matches.reference()),
        None => freddie_cli::run_lifecycle_verb::<Isograph>(
            freddie_cli::verb_for_bare_invocation::<Isograph>(),
            matches.reference(),
        ),
    }
```

```rust
// from crates/isograph_cli/src/lib.rs (after)
    match cli.verb {
        Some(CliVerb::Lifecycle(verb)) => {
            freddie_cli::run_lifecycle_verb::<Isograph>(verb, matches.reference())
        }
        Some(CliVerb::ConfigPath(id)) => config_path::run(id.reference()),
        None => freddie_cli::run_lifecycle_verb::<Isograph>(
            freddie_cli::verb_for_bare_invocation::<Isograph>(),
            matches.reference(),
        ),
    }
```

```rust
// from crates/isograph_cli/src/lib.rs (after, modules)
mod config_path;
```

```rust
// from crates/isograph_cli/src/config_path.rs
use std::process::ExitCode;

use crate::ConfigFlag;
use crate::discover;
use prelude::Postfix;

#[expect(clippy::print_stdout, clippy::print_stderr)]
pub fn run(id: &ConfigFlag) -> ExitCode {
    match discover::config_path(id.config.as_deref()) {
        Ok(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
```

Workspace clippy denies `print_stdout` and `print_stderr` in library crates. `config_path::run` is the process entry for this verb. `#[expect(clippy::print_stdout, clippy::print_stderr)]` on `run`. The function is not a library API. Do not add a tracing subscriber to avoid the expect: the path is for scripts to read from stdout, and a missing config is a command-line error on stderr, the same as `send`.

`config_path` does not call `load_config`. A file that exists and can be canonicalized is enough.

### Tests

`discover::config_path` already covers walk-up, `--config`, missing file, and no config. This change adds e2e on the verb.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
#[test]
fn config_path_prints_the_canonical_path() {
    let dir = tempfile::tempdir().expect("a test can create a temp directory");
    let config = dir.path().join("isograph.config.json");
    std::fs::write(config.reference(), "{}\n").expect("a test can write a config file");
    let nested = dir.path().join("src");
    std::fs::create_dir_all(nested.reference()).expect("a test can create a nested directory");
    let output = Command::new(isograph_bin())
        .args(["config-path"].reference())
        .current_dir(nested.reference())
        .output()
        .expect("the isograph binary runs");
    assert!(
        output.status.success(),
        "stderr: {}",
        stderr(output.reference())
    );
    let expected = config.canonicalize().expect("the fixture exists");
    assert_eq!(stdout(output.reference()).trim(), expected.to_str().expect("utf-8"));
}

#[test]
fn config_path_with_flag_prints_that_file() {
    let dir = tempfile::tempdir().expect("a test can create a temp directory");
    let config = dir.path().join("isograph.config.json");
    std::fs::write(config.reference(), "{}\n").expect("a test can write a config file");
    let output = Command::new(isograph_bin())
        .args(
            [
                "config-path",
                "--config",
                config.to_str().expect("utf-8"),
            ]
            .reference(),
        )
        .current_dir(dir.path())
        .output()
        .expect("the isograph binary runs");
    assert!(output.status.success());
    let expected = config.canonicalize().expect("the fixture exists");
    assert_eq!(stdout(output.reference()).trim(), expected.to_str().expect("utf-8"));
}

#[test]
fn config_path_with_no_config_exits_1() {
    let dir = tempfile::tempdir().expect("a test can create a temp directory");
    let output = Command::new(isograph_bin())
        .args(["config-path"].reference())
        .current_dir(dir.path())
        .output()
        .expect("the isograph binary runs");
    assert!(!output.status.success());
    assert!(stdout(output.reference()).is_empty());
    let err = stderr(output.reference());
    assert!(err.contains("no isograph.config.json"), "{err}");
}
```

These tests do not set `HOME`. Walk-up uses the current directory. `Daemon` is for lifecycle verbs.
