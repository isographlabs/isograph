# `isograph start` returns when the port accepts

Requires lsp-port.md (landed). Prefactor for lsp-proxy.md.

`isograph start` returns when the flock is held. The child then unlinks `{slug}.port`, loads the config, binds, writes the port file. A client that `start`s and immediately connects can hit a missing file or a leftover port.

After a successful `start` or `restart` (including the bare binary), wait until `TcpStream::connect` to that port succeeds, then return. Sleep between tries, 10s deadline. Same wait for nested `isograph start` from `isograph lsp`.

Origin of start: freddie `client::ensure_started` / `wait_until_held`. Origin of the port file: `send.rs` `parse_port` / `read_port`. Delta: isograph does not return from start until a connect works. Freddie still waits only for the lock.

One shippable change.

## What the user does

```
$ isograph start
```

When it exits 0, `isograph send` and `TcpStream::connect` to `{slug}.port` work. No extra wait.

`start` of `{}` (missing `source_files`) still exits 1. That fails in `instance()` before the daemon is spawned.

## Types

```rust
// from crates/isograph_cli/src/lib.rs
        Some(CliVerb::Lifecycle(verb)) => match verb {
            freddie_cli::Verb::Start(_) | freddie_cli::Verb::Restart(_) => {
                crate::start::run::<THostLanguage>(verb, matches.reference())
            }
            verb => {
                freddie_cli::run_lifecycle_verb::<Isograph<THostLanguage>>(verb, matches.reference())
            }
        },
        Some(CliVerb::Send(args)) => send::run(args.reference()),
        Some(CliVerb::ConfigPath(id)) => config_path::run(id.reference()),
        None => crate::start::run::<THostLanguage>(
            freddie_cli::verb_for_bare_invocation::<Isograph<THostLanguage>>(),
            matches.reference(),
        ),
```

`lib.rs`: `mod start;`.

```rust
// from crates/isograph_cli/src/start.rs
use std::fs;
use std::net::{Ipv4Addr, TcpStream};
use std::path::Path;
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

use clap::ArgMatches;
use freddie_cli::{App, Verb};
use prelude::Postfix;

use crate::Isograph;
use crate::discover;

const DEADLINE: Duration = Duration::from_secs(10);
const SLEEP: Duration = Duration::from_millis(50);

#[expect(clippy::print_stderr)]
pub fn run<THostLanguage: isograph_compiler::HostLanguage>(
    verb: Verb<Isograph<THostLanguage>>,
    matches: &ArgMatches,
) -> ExitCode {
    let instance = match Isograph::<THostLanguage>::instance(verb.id()) {
        Ok(instance) => instance,
        Err(e) => {
            clap::Error::raw(clap::error::ErrorKind::ValueValidation, format!("{e}\n")).exit()
        }
    };
    let port_path = discover::port_file(instance.lock_file());
    let code = freddie_cli::run_lifecycle_verb::<Isograph<THostLanguage>>(verb, matches);
    match freddie_single_instance::holder_at(instance.lock_file()) {
        Ok(freddie_single_instance::Held::Free) | Err(_) => return code,
        Ok(_) => {}
    }
    match wait_until_listening(port_path.reference()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum WaitError {
    #[error("the daemon has not recorded its port yet")]
    NoPort,
}

fn wait_until_listening(path: &Path) -> Result<(), WaitError> {
    let deadline = Instant::now() + DEADLINE;
    loop {
        if let Ok(text) = fs::read_to_string(path) {
            if let Some(port) = discover::parse_port(text.reference()) {
                if let Ok(_stream) = TcpStream::connect((Ipv4Addr::LOCALHOST, port)) {
                    return ().wrap_ok();
                }
            }
        }
        if Instant::now() >= deadline {
            return WaitError::NoPort.wrap_err();
        }
        thread::sleep(SLEEP);
    }
}
```

`#[expect(clippy::print_stderr)]` on `run`. `TcpStream::connect(...).is_ok()` is whether the port accepted. `instance` failing uses the same clap exit as `run_lifecycle_verb`.

### `parse_port` moves to `discover.rs`

Origin: `send.rs` `parse_port` and its tests. Delta: `pub(crate)`. `send.rs` `read_port` calls `crate::discover::parse_port`. The `parse_port_*` tests move with it. `read_port_of_a_missing_file_is_no_port` stays in `send.rs`.

```rust
// from crates/isograph_cli/src/discover.rs
use std::num::NonZeroU16;

pub(crate) fn parse_port(text: &str) -> Option<u16> {
    text.trim().parse::<NonZeroU16>().ok().map(NonZeroU16::get)
}
```

```rust
// from crates/isograph_cli/src/send.rs
        Ok(text) => crate::discover::parse_port(text.reference()).ok_or(SendError::BadPort),
```

Drop `parse_port` from `send.rs`. Drop `use std::num::NonZeroU16` if unused.

## Tests

`cli.rs`. `Daemon::start` is `isograph start --filesystem injected`. After it returns, `isograph send` of HelloWorld succeeds with no extra wait. Drop the `settle` / `poll` for `isograph daemon up` inside `Daemon::start`. The log may still contain `isograph daemon up`; do not wait for it before send.

`start_then_status_reports_running` unchanged.

`start_with_missing_source_files_exits_1` unchanged (`instance` loads the config).

`a_second_start_adopts_the_running_daemon`: second start still exits 0 and still connects (the port is already accepting).

## Call sites

- `isograph start` / `restart` / bare binary -> freddie start -> wait until connect -> return
- `isograph lsp` nested `isograph start` -> same wait, then one `TcpStream::connect`
