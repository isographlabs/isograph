# Windows `stop` without `--force` sends `Quit` on the socket

Requires start-returns-when-listening.md (landed).

Freddie `stop` / `restart` without `--force` is SIGTERM. `signal_pid` on Windows returns `graceful stop is not yet available on Windows; use --force`. The daemon already turns `Internal::Quit` into `Kill` from the socket. `isograph send` already posts that frame. Unix stays SIGTERM. Windows graceful stop is that same `Quit` frame, in-process, not a subprocess and not a temp `--file`.

`--force` stays freddie: `taskkill /F` on Windows, SIGKILL on unix. `Daemon` drop stays `stop --force`.

Do not bump freddie. Do not add a Windows SIGTERM analog. Do not add a bool. `--force` is clap's existing flag on `StopArgs` / `RestartArgs`; this slice matches it.

One shippable change.

## What the user does

```
$ isograph start
C:\Users\x\app\isograph.config.json started (pid 4100)
$ isograph stop
C:\Users\x\app\isograph.config.json stopped (pid 4100)
$ isograph status
C:\Users\x\app\isograph.config.json is not running
```

Windows, no `--force`. The log has `kill: exiting`. It does not have `SIGTERM: quitting`.

```
$ isograph restart --filesystem injected
C:\Users\x\app\isograph.config.json started (pid 4101)
```

Exits 0. The old process is gone. The new port accepts. Unix still prints `stopped` then `started`; Windows restart prints `started` only. The daemon log of the old process has `kill: exiting`.

A daemon that is not running: `stop` exits 0 and prints `is not running`, same as unix.

A daemon that does not answer the port: `stop` without `--force` exits 1 and says to use `--force`. `--force` still destroys it.

Unix `stop` / `restart` without `--force` is unchanged (SIGTERM).

## Types

Most important first.

```rust
// from crates/isograph_cli/src/stop.rs
use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use clap::error::ErrorKind;
use freddie_cli::{App, Instance};
use freddie_single_instance::{Held, LockError, Pid};
use isograph_compiler::HostLanguage;
use prelude::Postfix;
use thiserror::Error;

use crate::Isograph;
use crate::event::Internal;
use crate::send::{SendError, notify};

const STOP_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) enum GracefulStop {
    Stopped(Pid),
    NotRunning,
}

#[derive(Debug, Error)]
pub(crate) enum StopError {
    #[error("{0}")]
    Lock(#[from] LockError),
    #[error(
        "something holds the lock but recorded no pid; --force destroys it"
    )]
    Unnamed,
    #[error("{0}; --force destroys it")]
    Port(#[from] crate::discover::PortError),
    #[error("{0}; --force destroys it")]
    Notify(SendError),
    #[error("pid {0} still holds the lock; --force destroys it")]
    Ignored(Pid),
}
```

`GracefulStop` is whether there was a process. Not a bool. `Stopped` is `Held::By`. `NotRunning` is `Held::Free`. `Unnamed` is an error, same as freddie: a graceful stop needs a running daemon that can take `Quit`.

`StopError` does not `From<SendError>`: `notify` is the only `SendError` source, and `SendError` also has `NotRunning` / `ReadFile` / `NotEvent` which this function does not produce. Map with `StopError::Notify`.

`STOP_TIMEOUT` is freddie `client.rs` `STOP_TIMEOUT`.

### `quit_and_await_free`

```rust
// from crates/isograph_cli/src/stop.rs
pub(crate) fn quit_and_await_free(instance: &Instance) -> Result<GracefulStop, StopError> {
    let pid = match freddie_single_instance::holder_at(instance.lock_file())? {
        Held::Free => return GracefulStop::NotRunning.wrap_ok(),
        Held::Unnamed => return StopError::Unnamed.wrap_err(),
        Held::By(pid) => pid,
    };
    let lock = instance.lock_file().to_owned();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(freddie_single_instance::await_free_at(&lock));
    });
    let stream =
        crate::discover::connect_to_daemon(&crate::discover::port_file(instance.lock_file()))?;
    notify(stream, Internal::Quit).map_err(StopError::Notify)?;
    match rx.recv_timeout(STOP_TIMEOUT) {
        Ok(Ok(())) => GracefulStop::Stopped(pid).wrap_ok(),
        Ok(Err(e)) => e.to(),
        Err(_) => StopError::Ignored(pid).wrap_err(),
    }
}
```

Subscribe (`await_free_at` on a thread) then produce (`notify` `Quit`). A subscribe after `notify` can miss the free. `await_free_at` parks on flock. The timeout is `recv_timeout` on that thread, the same shape as freddie `watch_for_free`. Abandoning the thread on connect/`notify` failure is what freddie does when `signal_pid` fails.

Do not write a frame file. Do not call `send::run`. Do not spawn `isograph send`. `notify` is the handshake plus `isograph/event` with params `{"kind":"Quit"}`.

`Internal::Quit` already reaches `handle` from the socket. `handle` returns `Kill`. `perform` logs `kill: exiting`. `process::exit(0)` unlinks the port file and drops the lock. No new daemon event.

### `run` is the `stop` verb on Windows without `--force`

```rust
// from crates/isograph_cli/src/stop.rs
pub(crate) fn run<THostLanguage: HostLanguage>(
    args: &freddie_cli::StopArgs<crate::ConfigFlag>,
) -> ExitCode {
    let instance = match Isograph::<THostLanguage>::instance(&args.id) {
        Ok(instance) => instance,
        Err(e) => clap::Error::raw(ErrorKind::ValueValidation, format!("{e}\n")).exit(),
    };
    freddie_cli::init_client_logging(&instance);
    match quit_and_await_free(&instance) {
        Ok(GracefulStop::Stopped(pid)) => {
            tracing::info!("{} stopped (pid {pid})", instance.display_name());
            ExitCode::SUCCESS
        }
        Ok(GracefulStop::NotRunning) => {
            tracing::info!("{} is not running", instance.display_name());
            ExitCode::SUCCESS
        }
        Err(e) => {
            tracing::warn!("{}: {e}", instance.display_name());
            ExitCode::FAILURE
        }
    }
}
```

`init_client_logging` is the public hook freddie documents for app verbs. `info!` is stdout, `warn!` is stderr, same as freddie `stop`. Do not `init_client_logging` from `start.rs`: a second `run_lifecycle_verb` would panic on a second subscriber.

`ExitCode` is not `Eq`. This function constructs `SUCCESS` / `FAILURE`. Callers of `quit_and_await_free` match `Result`.

`ConfigFlag` stays in `lib.rs`. `StopArgs` fields are `pub`. `stop.rs` is `#[cfg(windows)]` only.

## Dispatch

```rust
// from crates/isograph_cli/src/lib.rs
mod send;
mod start;
#[cfg(windows)]
mod stop;
mod state;
mod watch;

pub fn run<THostLanguage: HostLanguage>() -> ExitCode {
    // First, so `--help` prints and a bad flag exits before the lock is taken.
    // The matches are kept beside the parse because `run_lifecycle_verb` reads what was written
    // from them, to forward to the daemon it spawns.
    let matches = Cli::<THostLanguage>::command().get_matches();
    let cli = Cli::<THostLanguage>::from_arg_matches(matches.reference())
        .expect("the derived type matches the command it derived");

    match cli.verb {
        Some(CliVerb::Lifecycle(verb)) => match verb {
            freddie_cli::Verb::Start(_) | freddie_cli::Verb::Restart(_) => {
                crate::start::run::<THostLanguage>(verb, matches.reference())
            }
            #[cfg(windows)]
            freddie_cli::Verb::Stop(ref args) if !args.force => {
                crate::stop::run::<THostLanguage>(args)
            }
            verb => freddie_cli::run_lifecycle_verb::<Isograph<THostLanguage>>(
                verb,
                matches.reference(),
            ),
        },
        Some(CliVerb::Send(args)) => send::run(args.reference()),
        Some(CliVerb::ConfigPath(id)) => config_path::run(id.reference()),
        Some(CliVerb::Lsp(args)) => lsp_stdio::run(args.reference()),
        None => crate::start::run::<THostLanguage>(
            freddie_cli::verb_for_bare_invocation::<Isograph<THostLanguage>>(),
            matches.reference(),
        ),
    }
}
```

`args.force` is freddie's clap flag, not a new field. Without `--force` is this slice. With `--force` falls through to freddie (`taskkill /F`). Unix has no `Stop` arm here.

### Restart without `--force` on Windows becomes Start after `Quit`

`TypedArgs::of` reads `matches.subcommand()`, not the `Verb` variant. `isograph restart --filesystem injected` still forwards `--filesystem` if the verb we pass to `run_lifecycle_verb` is `Start`. Passing `Verb::Restart` after `Quit` would SIGTERM the new nothing and fail. Convert to `Start`.

```rust
// from crates/isograph_cli/src/start.rs
use clap::ArgMatches;
use clap::error::ErrorKind;
use freddie_cli::{App, StartArgs, Verb};
use isograph_compiler::HostLanguage;
use prelude::Postfix;

use crate::Isograph;
use crate::discover;

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
    let verb = match verb {
        #[cfg(windows)]
        Verb::Restart(args) if !args.force => match crate::stop::quit_and_await_free(&instance) {
            Ok(_) => Verb::Start(StartArgs {
                id: args.id,
                app: args.app,
            }),
            Err(e) => {
                eprintln!("{}: not restarting: {e}", instance.display_name());
                return ExitCode::FAILURE;
            }
        },
        verb => verb,
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
```

Do not `init_client_logging` here. `run_lifecycle_verb` of `Start` inits once and prints `started`. Restart's quit half does not print `stopped` on Windows. A quit error uses `eprintln` with the `not restarting:` prefix freddie uses, under the `print_stderr` expect already on `run`.

`--force` restart is still `Verb::Restart` into freddie, then wait until listening.

`wait_until_listening` is unchanged.

## Daemon comment

Unix SIGTERM task stays. Windows has none; this slice is the client.

```rust
// from crates/isograph_cli/src/daemon.rs
    // Unix `isograph stop` without `--force` sends SIGTERM. Route it into the event
    // channel as Quit, so the model turns it into Kill, the effect loop breaks, and
    // serve returns. Windows `stop` without `--force` posts Internal::Quit on the
    // socket instead (stop.rs); this task does not exist there.
    //
    // A spawned task rather than a third `select!` arm, because an arm that completed
    // would drop the other two futures and skip the graceful path this exists to run.
    #[cfg(unix)]
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
```

## Event model

```text
// from docs-website/docs/design-docs/event-model.md
`Quit` is `isograph stop` without `--force` (SIGTERM on unix; a `Quit` frame on the socket on Windows), SIGTERM, and a `Quit` frame on the socket. `handle` returns `Kill`.
```

## Tests

`cli.rs`. Drop the Windows `--force` argv. `STOP` is `stop` on every OS. Drop `RESTART`; the one restart test types the same argv as unix.

`Daemon` drop stays `["stop", "--force"]`.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
// freddie_cli's stop without --force is SIGTERM on unix. Windows stop without
// --force posts Quit on the socket. Both run Kill.
const STOP: &[&str] = &["stop"];
```

Delete the `#[cfg(windows)]` / `#[cfg(not(windows))]` `STOP` and `RESTART` consts.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
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
```

The three tests that `poll` for `SIGTERM: quitting` && `kill: exiting` wait for `kill: exiting` on every OS. Unix also waits for `SIGTERM: quitting`. Windows asserts that line is absent.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
    poll(|| (!daemon.isograph(["status"].reference()).status.success()).then_some(()));
    poll(|| {
        let log = daemon.log_text();
        let killed = log.contains("kill: exiting");
        #[cfg(windows)]
        let ok = killed && !log.contains("SIGTERM: quitting");
        #[cfg(not(windows))]
        let ok = killed && log.contains("SIGTERM: quitting");
        ok.then_some(())
    });
```

Same replacement in `stop_after_send_exits_without_force`, `two_sends_then_stop_exits_without_force`, and `stop_then_status_reports_not_running`. Those names are true on Windows.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
#[test]
fn send_of_quit_then_status_reports_not_running() {
    let daemon = Daemon::start();
    let frame = write_frame(daemon.dir.path(), "{\"kind\":\"Quit\"}\n");
    let sent = daemon.isograph(["send", "--file", frame.to_str().expect("utf-8")].reference());
    assert!(
        sent.status.success(),
        "stdout: {} stderr: {}",
        stdout(sent.reference()),
        stderr(sent.reference())
    );
    poll(|| (!daemon.isograph(["status"].reference()).status.success()).then_some(()));
    poll(|| daemon.log_text().contains("kill: exiting").then_some(()));
    assert!(!daemon.log_text().contains("SIGTERM: quitting"));
}
```

This is `notify` of `Quit` through the hidden verb, all platforms. Windows `stop` is that same `notify` without the file. No SIGTERM on this path, including unix.

Keep `poll` until no-poll-in-tests.md. Do not add `await_containing` here.

## Call graph

- Windows `isograph stop` without `--force` -> `lib.rs` -> `stop::run` -> `init_client_logging` -> `quit_and_await_free` -> `notify(Quit)` -> stdout `stopped` / `is not running`
- Windows `isograph restart` without `--force` -> `start::run` -> `quit_and_await_free` -> `Verb::Start` -> freddie start -> `wait_until_listening`
- Windows `stop --force` / `restart --force` -> freddie
- Unix stop / restart -> freddie
- `isograph send --file` of `{"kind":"Quit"}` -> `send::run` -> `notify` (already)
