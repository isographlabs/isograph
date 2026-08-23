# Windows `stop` without `--force` asks the app to quit

Requires start-returns-when-listening.md (landed).

Freddie `stop` / `restart` without `--force` is SIGTERM. On Windows, `signal_pid(Terminate)` returns `graceful stop is not yet available on Windows; use --force`. `stop` and `restart` are freddie verbs: find the pid, subscribe to lock release, ask, wait. An isograph intercept of those verbs duplicates that and fights `init_client_logging` on restart.

The app-specific piece is how to ask. Unix stays SIGTERM. Windows calls `App::ask_to_quit`. Isograph's impl is `send::notify` of `Internal::Quit` on the port next to the lock, the same frame `isograph send --file '{"kind":"Quit"}'` already posts. No temp file. No subprocess.

`--force` stays SIGKILL / `taskkill /F`. `Daemon` drop stays `stop --force`. Mercury keeps the default `None`.

Two shippable changes. Freddie first, then isograph bumps the rev. Same git rev for `freddie_cli` and `freddie_single_instance`, as today.

Do not add a bool. `--force` is clap's existing flag.

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
C:\Users\x\app\isograph.config.json stopped (pid 4100)
C:\Users\x\app\isograph.config.json started (pid 4101)
```

Same two lines as unix. The old process is gone. The new port accepts.

A daemon that is not running: `stop` exits 0 and prints `is not running`, same as unix.

A daemon that does not answer the port: `stop` without `--force` exits 1, `could not signal pid N: ...`. `--force` still destroys it.

Unix `stop` / `restart` without `--force` is unchanged (SIGTERM). `isograph` still implements `ask_to_quit`; freddie does not call it on unix.

Mercury on Windows without `--force` still prints today's error: the default is `None`, and freddie maps that to the same string `signal_pid(Terminate)` returns today.

## Change 1: `App::ask_to_quit` in freddie

Lands in `github.com/freddiehg/freddie`. Origin: `crates/freddie_cli/src/lib.rs` `App`, `crates/freddie_cli/src/client.rs` `stop_daemon` / `signal_pid` / `stop` / `restart`. Delta: Windows `Terminate` calls the app instead of returning that error from `signal_pid`.

### Trait

```rust
// from crates/freddie_cli/src/lib.rs (freddie)
    fn run_daemon(id: &Self::Id, args: &Self::DaemonArgs);

    /// Ask the running daemon to quit, on platforms that have no SIGTERM.
    ///
    /// [`stop`](Verb::Stop) without `--force` is SIGTERM on unix. Windows has none: `taskkill`
    /// without `/F` posts `WM_CLOSE`, which a daemon with no window never sees. This is that
    /// ask. Freddie still finds the pid, still waits on the lock, still prints `stopped`.
    ///
    /// Takes [`Instance`], not [`Id`](Self::Id). `stop` has already resolved the daemon; a
    /// second [`instance`](Self::instance) would rediscover it.
    ///
    /// `None` if this app has no ask. `Some` is the attempt: `Ok` posted it, `Err` tried
    /// and could not. The default is `None`. Unix never calls this.
    fn ask_to_quit(
        _instance: &Instance,
    ) -> Option<Result<(), Box<dyn std::error::Error + Send + Sync>>> {
        None
    }
```

Not `windows_soft_quit`. Freddie already says "asking it to quit" on `StopArgs`. The call site is Windows; the method is the ask. Mercury can implement it later without the name being wrong.

`None` is no implementation, not a successful no-op. `Result<Option<()>, _>` would make `Ok(None)` look like the ask succeeded at doing nothing. Freddie maps `None` to `graceful stop is not yet available on Windows; use --force`. `Some(Err)` is an app that has an ask and failed it (`could not signal pid N: ...`). Mercury and `TestApp` keep the default.

### `stop_daemon` is generic and Windows `Terminate` calls the app

```rust
// from crates/freddie_cli/src/client.rs (freddie)
fn stop_daemon<TApp: App>(instance: &Instance, signal: Signal) -> Result<Option<Pid>, Failure> {
    let pid = match find_daemon(instance) {
        Ok(Target::Running(pid)) => pid,
        Ok(Target::NotRunning) => return None.wrap_ok(),
        Ok(Target::Anonymous) => {
            debug!("the lock is held by a holder that recorded no pid");
            return Failure::Anonymous(instance.lock_file().to_owned()).wrap_err();
        }
        Err(error) => {
            debug!(%error, "could not read the lock");
            return Failure::Unreadable(error).wrap_err();
        }
    };
    // Before the ask, so the wait cannot miss a daemon that exits between the two.
    let freed = watch_for_free(instance);
    debug!(daemon = %pid, ?signal, "signalling the daemon");
    if let Err(error) = deliver::<TApp>(instance, pid, signal) {
        debug!(daemon = %pid, %error, "could not signal the daemon");
        return Failure::Unsignalable(SignalFailure { pid, error }).wrap_err();
    }
    if matches!(freed.recv_timeout(STOP_TIMEOUT), Ok(Ok(()))) {
        debug!(daemon = %pid, "the daemon released the lock");
        pid.wrap_some().wrap_ok()
    } else {
        debug!(daemon = %pid, ?signal, timeout = ?STOP_TIMEOUT, "the daemon still holds the lock");
        Failure::Ignored(SignalIgnored { pid, signal }).wrap_err()
    }
}

fn deliver<TApp: App>(instance: &Instance, pid: Pid, signal: Signal) -> io::Result<()> {
    match signal {
        Signal::Kill => signal_pid(pid, Signal::Kill),
        Signal::Terminate => {
            #[cfg(unix)]
            {
                signal_pid(pid, Signal::Terminate)
            }
            #[cfg(windows)]
            {
                match TApp::ask_to_quit(instance) {
                    None => io::Error::other(
                        "graceful stop is not yet available on Windows; use --force",
                    )
                    .wrap_err(),
                    Some(Ok(())) => ().wrap_ok(),
                    Some(Err(e)) => io::Error::other(e).wrap_err(),
                }
            }
        }
    }
}
```

`None` and `Some(Err)` both become `Failure::Unsignalable`. `None` uses today's string. `Some(Err)` uses the app's error. `Failure::Unsignalable` still prints `could not signal pid {pid}: {error}`.

`stop` and `restart` already choose `Signal` from `force`. They pass `TApp` through. `stop` is not generic today; `restart` already is.

```rust
// from crates/freddie_cli/src/client.rs (freddie)
pub(crate) fn stop<TApp: App>(instance: &Instance, force: bool) -> ExitCode {
```

```rust
// from crates/freddie_cli/src/client.rs (freddie)
    match stop_daemon::<TApp>(instance, signal) {
```

in both `stop` and `restart`.

```rust
// from crates/freddie_cli/src/lib.rs (freddie)
        Verb::Stop(args) => client::stop::<TApp>(instance, args.force),
```

### Windows `signal_pid` is Kill only

```rust
// from crates/freddie_cli/src/client.rs (freddie)
#[cfg(windows)]
fn signal_pid(pid: Pid, signal: Signal) -> io::Result<()> {
    match signal {
        // `taskkill /F` terminates the process, which is SIGKILL. `taskkill` without it posts
        // WM_CLOSE, which a daemon with no window never sees, so there is no gentle taskkill.
        Signal::Kill => ran(Command::new("taskkill")
            .args(["/F", "/PID"])
            .arg(pid.to_string())),
        Signal::Terminate => {
            unreachable!("Windows Terminate is App::ask_to_quit, not signal_pid")
        }
    }
}
```

`deliver` never passes `Terminate` to `signal_pid` on Windows. The arm is the type of `Signal` remaining exhaustive. `unreachable!` is the invariant `deliver` already established: `Terminate` on Windows does not enter `signal_pid`. Name that in the string.

Unix `signal_pid` is unchanged.

Subscribe then ask is unchanged: `watch_for_free` before `deliver`.

## Change 2: isograph implements `ask_to_quit`

Bump both freddie git deps to the change-1 rev.

```rust
// from crates/isograph_cli/src/lib.rs
impl<THostLanguage: HostLanguage> App for Isograph<THostLanguage> {
    type Id = ConfigFlag;
    type DaemonArgs = IsographArgs;

    const NAME: &'static str = "isograph";

    fn instance(id: &ConfigFlag) -> Result<Instance, Box<dyn std::error::Error + Send + Sync>> {
        let (_, instance, _) = discover::config_and_instance(id.config.as_deref())?;
        instance.wrap_ok()
    }

    fn run_daemon(id: &ConfigFlag, args: &IsographArgs) {
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
        let config = match discover::load_config(path.reference()) {
            Ok(config) => config,
            Err(e) => {
                tracing::error!(error = %e, "could not load the config");
                return;
            }
        };
        crate::daemon::run::<THostLanguage>(path, port_path, args.filesystem, config);
    }

    fn ask_to_quit(
        instance: &Instance,
    ) -> Option<Result<(), Box<dyn std::error::Error + Send + Sync>>> {
        (|| {
            let stream = crate::discover::connect_to_daemon(&crate::discover::port_file(
                instance.lock_file(),
            ))?;
            crate::send::notify(stream, crate::event::Internal::Quit)?;
            ().wrap_ok()
        })()
        .wrap_some()
    }
}
```

No `#[cfg(windows)]` on the impl. `notify` of `Quit` is the same on every OS. Freddie only calls this on Windows. Always `Some`: isograph has an ask. Connect/`notify` failure is `Some(Err)`, not `None`.

`PortError` and `SendError` already implement `std::error::Error`. `?` into the box.

Do not add `stop.rs`. Do not intercept `Verb::Stop` / `Verb::Restart` in `lib.rs` or `start.rs`. `start.rs` wait-until-listening is unchanged.

### Daemon comment

```rust
// from crates/isograph_cli/src/daemon.rs
    // Unix `isograph stop` without `--force` sends SIGTERM. Route it into the event
    // channel as Quit, so the model turns it into Kill, the effect loop breaks, and
    // serve returns. Windows `stop` without `--force` is App::ask_to_quit, which posts
    // Internal::Quit on the socket; this task does not exist there.
    //
    // A spawned task rather than a third `select!` arm, because an arm that completed
    // would drop the other two futures and skip the graceful path this exists to run.
    #[cfg(unix)]
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
```

### Event model

```text
// from docs-website/docs/design-docs/event-model.md
`Quit` is `isograph stop` without `--force` (SIGTERM on unix; a `Quit` frame on the socket on Windows), SIGTERM, and a `Quit` frame on the socket. `handle` returns `Kill`.
```

## Tests

`cli.rs`. Drop the Windows `--force` argv. `STOP` is `stop` on every OS. Drop `RESTART`; the one restart test types the same argv as unix.

`Daemon` drop stays `["stop", "--force"]`.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
// Unix stop without --force is SIGTERM. Windows stop without --force is App::ask_to_quit
// (a Quit frame on the socket). Both run Kill.
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

Same replacement in `stop_after_send_exits_without_force`, `two_sends_then_stop_exits_without_force`, and `stop_then_status_reports_not_running`.

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

`notify` of `Quit` through the hidden verb, all platforms. Windows `stop` is that same `notify` via `ask_to_quit`. No SIGTERM on this path, including unix.

Keep `poll` until no-poll-in-tests.md.

## Call graph

- Unix `stop` / `restart` without `--force` -> freddie `signal_pid(Terminate)` -> SIGTERM -> daemon `Quit` -> `Kill`
- Windows `stop` / `restart` without `--force` -> freddie `App::ask_to_quit` -> isograph `notify(Quit)` -> `Kill` -> wait on lock -> `stopped` / then start
- `--force` -> SIGKILL / `taskkill /F` on every OS
- `isograph send --file` of `{"kind":"Quit"}` -> `send::run` -> `notify` (already)
