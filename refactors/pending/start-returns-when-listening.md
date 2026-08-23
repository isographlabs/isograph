# `isograph start` returns when the port accepts

Requires lsp-port.md (landed). Prefactor for lsp-proxy.md.

Freddie returns when the lock is held. The child then unlinks `{slug}.port`, loads the config, binds, walks the tree, writes the port. `send` does not wait. A client that `start`s and immediately connects can hit a missing file, leftover garbage, or a closed leftover port.

This slice: write the port file immediately after bind, before the watcher. Wrap `start`, `restart`, and the bare binary. After freddie returns, if the lock is held, sleep-retry `TcpStream::connect` until it succeeds, the lock is `Free`, or 5s. Return freddie's `ExitCode` when the wait succeeds. `send` stays one connect. Nested `isograph start` from `isograph lsp` gets the wait for free.

Origin of start: freddie `client::ensure_started` / `wait_until_held` (5s, 10ms). Origin of the port file: `send.rs` `parse_port` / `read_port`. Origin of bind-then-write: `daemon.rs` `serve`, today after `start_if_watching`. Delta: the write moves to after `local_addr`; start does not return until a connect works; a failed restart still returns failure.

One shippable change.

## What the user does

```
$ isograph start
```

When it exits 0, `isograph send` and `TcpStream::connect` to `{slug}.port` work. No extra wait. Intern of boot files is not part of that contract.

`start` of `{}` (missing `source_files`) still exits 1. That fails in `instance()` before the daemon is spawned.

A `restart` that cannot stop still exits nonzero. It does not report success because the old port still accepts.

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

`lib.rs`: `mod start;`. Not `daemon`, not `stop`.

### Port file after bind

```rust
// from crates/isograph_cli/src/daemon.rs
    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(e) => {
            tracing::error!(error = %e, "could not read the lsp socket address");
            return;
        }
    };
    if let Err(e) = std::fs::write(port_path.reference(), format!("{port}\n")) {
        tracing::error!(
            error = %e,
            path = %port_path.display(),
            "could not write the event socket port"
        );
        return;
    }
    tracing::info!(config = %config_path.display(), port, "isograph daemon up");

    let mut state = IsographState::<THostLanguage>::default();
    intern_config_directory(&mut state, config_path.reference());
    let config_directory = config_path
        .parent()
        .expect("a config file path has a parent directory")
        .to_owned();
    let source_files = config.source_files.clone();
    let _hold_watch = watch_tx.clone();
    let _watcher = match crate::watch::start_if_watching::<THostLanguage>(
        filesystem,
        watch_tx,
        config_path.reference(),
        config.source_files.as_slice(),
    ) {
```

Today the write and the log sit after `start_if_watching`. After: bind, write, log, then intern and the watcher. Watcher failure is a daemon that dies after start 0.

### Start wrapper

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

const DEADLINE: Duration = Duration::from_secs(5);
const SLEEP: Duration = Duration::from_millis(10);

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
        Ok(freddie_single_instance::Held::By(_))
        | Ok(freddie_single_instance::Held::Unnamed) => {}
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
        if let Ok(text) = fs::read_to_string(port_path) {
            if let Some(port) = discover::parse_port(text.reference()) {
                if let Ok(_stream) = TcpStream::connect((Ipv4Addr::LOCALHOST, port)) {
                    return ().wrap_ok();
                }
            }
        }
        if Instant::now() >= deadline {
            return WaitError::NotListening.wrap_err();
        }
        thread::sleep(SLEEP);
    }
}
```

Same 5s / 10ms as freddie `wait_until_held`. `ExitCode` is not `Eq`; do not compare it to `SUCCESS`. Held after a failed restart is the old daemon: wait connects, then return `code` (failure). Freddie SUCCESS then child dead is `Free`: return `FAILURE`, do not sit 10s. During the wait, `Free` fails immediately.

`TcpStream::connect` then drop is one `accept_loop` session that hits EOF before initialize and returns. Do not send initialize. Do not treat "file exists" as ready. `if let Ok(_stream)` is whether the port accepted.

`instance` failing uses the same clap `.exit()` as `run_lifecycle_verb`. The extra `instance()` is for `port_path`.

### `parse_port` moves to `discover.rs`

Origin: `send.rs` `parse_port` and its tests. Delta: `pub(crate)`. `send.rs` `read_port` calls `crate::discover::parse_port`. The `parse_port_*` tests move with it. `read_port_of_a_missing_file_is_no_port` stays in `send.rs`. Do not add `connect_to_daemon`; lsp-proxy.md is that helper.

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

`cli.rs`. `Daemon::start` is `isograph start --filesystem injected`. Drop `settle` / `poll` for `isograph daemon up` inside `Daemon::start` and `start_watch`. Connect can succeed after the port write and before that log line. Do not wait for the line before send. Tests that still assert the line exists follow the log, they do not call `log_text()` immediately.

`start_then_status_reports_running` unchanged (status is the lock).

`start_with_missing_source_files_exits_1` unchanged.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
#[test]
fn send_after_start_needs_no_extra_wait() {
    let daemon = Daemon::start();
    let frame = write_frame(daemon.dir.path(), "{\"kind\":\"HelloWorld\"}\n");
    let sent = daemon.isograph(["send", "--file", frame.to_str().expect("utf-8")].reference());
    assert!(
        sent.status.success(),
        "stdout: {} stderr: {}",
        stdout(sent.reference()),
        stderr(sent.reference())
    );
}

#[test]
fn send_after_restart_needs_no_extra_wait() {
    let daemon = Daemon::start();
    let restarted = daemon.isograph(["restart", "--filesystem", "injected"].reference());
    assert!(
        restarted.status.success(),
        "stdout: {} stderr: {}",
        stdout(restarted.reference()),
        stderr(restarted.reference())
    );
    let frame = write_frame(daemon.dir.path(), "{\"kind\":\"HelloWorld\"}\n");
    let sent = daemon.isograph(["send", "--file", frame.to_str().expect("utf-8")].reference());
    assert!(
        sent.status.success(),
        "stdout: {} stderr: {}",
        stdout(sent.reference()),
        stderr(sent.reference())
    );
}

#[test]
fn start_with_a_garbage_leftover_port_file_then_send() {
    let daemon = Daemon::start();
    let path = port_file_path(daemon.reference());
    let _ = daemon.isograph(STOP);
    std::fs::write(path.reference(), "not-a-port\n").expect("a test can write a leftover port file");
    let started = daemon.isograph(["start", "--filesystem", "injected"].reference());
    assert!(
        started.status.success(),
        "stdout: {} stderr: {}",
        stdout(started.reference()),
        stderr(started.reference())
    );
    let frame = write_frame(daemon.dir.path(), "{\"kind\":\"HelloWorld\"}\n");
    let sent = daemon.isograph(["send", "--file", frame.to_str().expect("utf-8")].reference());
    assert!(
        sent.status.success(),
        "stdout: {} stderr: {}",
        stdout(sent.reference()),
        stderr(sent.reference())
    );
}

#[test]
fn start_with_a_closed_leftover_port_then_send() {
    let daemon = Daemon::start();
    let path = port_file_path(daemon.reference());
    let _ = daemon.isograph(STOP);
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .expect("binding an ephemeral port");
    let port = listener
        .local_addr()
        .expect("the leftover listener has an address")
        .port();
    drop(listener);
    std::fs::write(path.reference(), format!("{port}\n"))
        .expect("a test can write a leftover port file");
    let started = daemon.isograph(["start", "--filesystem", "injected"].reference());
    assert!(
        started.status.success(),
        "stdout: {} stderr: {}",
        stdout(started.reference()),
        stderr(started.reference())
    );
    let frame = write_frame(daemon.dir.path(), "{\"kind\":\"HelloWorld\"}\n");
    let sent = daemon.isograph(["send", "--file", frame.to_str().expect("utf-8")].reference());
    assert!(
        sent.status.success(),
        "stdout: {} stderr: {}",
        stdout(sent.reference()),
        stderr(sent.reference())
    );
}
```

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
fn port_file_path(daemon: &Daemon) -> PathBuf {
    let home = daemon.dir.path().join("home");
    let mut stack = home.wrap_vec();
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(dir.reference()) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "port") {
                return path;
            }
        }
    }
    panic!("the daemon wrote a port file");
}

fn daemon_port(daemon: &Daemon) -> u16 {
    let text = std::fs::read_to_string(port_file_path(daemon).reference()).expect("the port file");
    text.trim().parse().expect("the port file is a port")
}
```

`a_second_start_adopts_the_running_daemon` still exits 0; add send of HelloWorld with no extra wait on that second start.

Watch tests that need `scan finished` still wait for that line. Start returning is listen, not the boot walk.

## Call sites

- `isograph start` / `restart` / bare binary -> freddie start -> wait until connect -> return freddie's code if the lock is held
- `isograph send` -> one connect, no wait
- `isograph lsp` nested `isograph start` -> same wait, then one connect
