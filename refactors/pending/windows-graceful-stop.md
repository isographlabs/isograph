# Windows `stop` without `--force` asks the app to quit

Requires start-returns-when-listening.md (landed).

`isograph stop` without `--force` on Windows calls `App::ask_to_quit`. Isograph posts `InternalEvent::Quit` on the daemon port, with a 5s read and write timeout on the stream, then freddie waits for the lock. Unix `stop` without `--force` is SIGTERM. `--force` is SIGKILL / `taskkill /F`.

Two shippable changes. Freddie first, then isograph bumps both freddie git deps to that rev.

## What the user does

```
$ isograph start
C:\Users\x\app\isograph.config.json started (pid 4100)
$ isograph stop
C:\Users\x\app\isograph.config.json stopped (pid 4100)
$ isograph status
C:\Users\x\app\isograph.config.json is not running
```

The log has `kill: exiting`.

```
$ isograph restart --filesystem injected
C:\Users\x\app\isograph.config.json stopped (pid 4100)
C:\Users\x\app\isograph.config.json started (pid 4101)
```

`stop` of a daemon that is not running exits 0, `is not running`.

`stop` without `--force` exits 1, `could not signal pid N: ...`, when the port file is missing, is not a port, connect fails, or initialize does not complete in 5s. Lock held and `{slug}.port` absent is that case: unix SIGTERM still reaches the pid; Windows has no socket yet. `--force` then exits 0 and status is not running.

Mercury on Windows without `--force` prints `graceful stop is not yet available on Windows; use --force`.

## Change 1: `App::ask_to_quit`

Origin: `crates/freddie_cli/src/lib.rs` `App`, `crates/freddie_cli/src/client.rs` `stop_daemon` / `signal_pid` / `stop` / `restart`. Delta: `#[cfg(windows)] App::ask_to_quit`; Windows `Terminate` calls it; Windows Kill is `kill_pid`; `stop_daemon` is generic and the existing `debug!` records `TApp::NAME`; `stop` is generic.

```rust
// external freddie/crates/freddie_cli/src/lib.rs
    fn run_daemon(id: &Self::Id, args: &Self::DaemonArgs);

    /// Ask the running daemon to quit. Windows only.
    ///
    /// [`stop`](Verb::Stop) without `--force` is SIGTERM on unix. Windows has none: `taskkill`
    /// without `/F` posts `WM_CLOSE`, which a daemon with no window never sees. This is that
    /// ask. Freddie still finds the pid, still waits on the lock, still prints `stopped`.
    ///
    /// Takes [`Instance`], not [`Id`](Self::Id). `stop` has already resolved the daemon; a
    /// second [`instance`](Self::instance) would rediscover it.
    ///
    /// The default is today's Windows error. An app that can reach its daemon overrides.
    #[cfg(windows)]
    fn ask_to_quit(
        _instance: &Instance,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err("graceful stop is not yet available on Windows; use --force".into())
    }
```

```rust
// external freddie/crates/freddie_cli/src/client.rs
fn stop_daemon<TApp: App>(instance: &Instance, signal: Signal) -> Result<Option<Pid>, Failure> {
    let pid = match find_daemon(instance) {
        Ok(Target::Running(pid)) => pid,
        Ok(Target::NotRunning) => return Ok(None),
        Ok(Target::Anonymous) => {
            debug!("the lock is held by a holder that recorded no pid");
            return Err(Failure::Anonymous(instance.lock_file().to_owned()));
        }
        Err(error) => {
            debug!(%error, "could not read the lock");
            return Err(Failure::Unreadable(error));
        }
    };
    let freed = watch_for_free(instance);
    debug!(app = TApp::NAME, daemon = %pid, ?signal, "signalling the daemon");
    if let Err(error) = match signal {
        Signal::Kill => {
            #[cfg(unix)]
            {
                signal_pid(pid, Signal::Kill)
            }
            #[cfg(windows)]
            {
                kill_pid(pid)
            }
        }
        Signal::Terminate => {
            #[cfg(unix)]
            {
                signal_pid(pid, Signal::Terminate)
            }
            #[cfg(windows)]
            {
                TApp::ask_to_quit(instance).map_err(io::Error::other)
            }
        }
    } {
        debug!(daemon = %pid, %error, "could not signal the daemon");
        return Err(Failure::Unsignalable(SignalFailure { pid, error }));
    }
    if matches!(freed.recv_timeout(STOP_TIMEOUT), Ok(Ok(()))) {
        debug!(daemon = %pid, "the daemon released the lock");
        Ok(Some(pid))
    } else {
        debug!(daemon = %pid, ?signal, timeout = ?STOP_TIMEOUT, "the daemon still holds the lock");
        Err(Failure::Ignored(SignalIgnored { pid, signal }))
    }
}
```

```rust
// external freddie/crates/freddie_cli/src/client.rs
#[cfg(windows)]
fn kill_pid(pid: Pid) -> io::Result<()> {
    // `taskkill /F` terminates the process, which is SIGKILL. `taskkill` without it posts
    // WM_CLOSE, which a daemon with no window never sees, so there is no gentle taskkill.
    ran(Command::new("taskkill")
        .args(["/F", "/PID"])
        .arg(pid.to_string()))
}
```

Unix `signal_pid` is unchanged.

```rust
// external freddie/crates/freddie_cli/src/client.rs
pub(crate) fn stop<TApp: App>(instance: &Instance, force: bool) -> ExitCode {
```

```rust
// external freddie/crates/freddie_cli/src/client.rs
    match stop_daemon::<TApp>(instance, signal) {
```

in `stop` and `restart`.

```rust
// external freddie/crates/freddie_cli/src/lib.rs
        Verb::Stop(args) => client::stop::<TApp>(instance, args.force),
```

### Tests

`#[cfg(test)]` in `client.rs`. Three apps: `NoArgs`, `Instance::global`, empty `run_daemon`. `ask_to_quit` overrides are `#[cfg(windows)]`.

```rust
// external freddie/crates/freddie_cli/src/client.rs
    struct Silent;

    impl App for Silent {
        type Id = crate::NoArgs;
        type DaemonArgs = crate::NoArgs;
        const NAME: &'static str = "silent";
        fn instance(_: &crate::NoArgs) -> Result<Instance, Box<dyn std::error::Error + Send + Sync>> {
            Ok(Instance::global(Self::NAME)?)
        }
        fn run_daemon(_: &crate::NoArgs, _: &crate::NoArgs) {}
    }

    struct Posts;

    impl App for Posts {
        type Id = crate::NoArgs;
        type DaemonArgs = crate::NoArgs;
        const NAME: &'static str = "posts";
        fn instance(_: &crate::NoArgs) -> Result<Instance, Box<dyn std::error::Error + Send + Sync>> {
            Ok(Instance::global(Self::NAME)?)
        }
        fn run_daemon(_: &crate::NoArgs, _: &crate::NoArgs) {}
        #[cfg(windows)]
        fn ask_to_quit(_: &Instance) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }
    }

    struct Fails;

    impl App for Fails {
        type Id = crate::NoArgs;
        type DaemonArgs = crate::NoArgs;
        const NAME: &'static str = "fails";
        fn instance(_: &crate::NoArgs) -> Result<Instance, Box<dyn std::error::Error + Send + Sync>> {
            Ok(Instance::global(Self::NAME)?)
        }
        fn run_daemon(_: &crate::NoArgs, _: &crate::NoArgs) {}
        #[cfg(windows)]
        fn ask_to_quit(_: &Instance) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Err("no port".into())
        }
    }

    #[cfg(windows)]
    #[test]
    fn default_ask_to_quit_is_the_windows_error() {
        let instance = Silent::instance(&crate::NoArgs).expect("a test can name an instance");
        let err = Silent::ask_to_quit(&instance).expect_err("the default cannot ask");
        assert!(
            err.to_string()
                .contains("graceful stop is not yet available on Windows; use --force"),
            "{err}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn an_ok_ask_to_quit_is_ok() {
        let instance = Posts::instance(&crate::NoArgs).expect("a test can name an instance");
        Posts::ask_to_quit(&instance).expect("posted");
    }

    #[cfg(windows)]
    #[test]
    fn a_failed_ask_to_quit_is_the_app_error() {
        let instance = Fails::instance(&crate::NoArgs).expect("a test can name an instance");
        let err = Fails::ask_to_quit(&instance).expect_err("the app failed the ask");
        assert!(err.to_string().contains("no port"), "{err}");
        let mapped = io::Error::other(err);
        assert!(mapped.to_string().contains("no port"), "{mapped}");
    }
```

## Change 2: isograph `ask_to_quit`

Origin: `send::notify`, `STOP_TIMEOUT` 5s. Delta: Windows `ask_to_quit` connects, sets 5s read and write timeouts, `notify`s `InternalEvent::Quit`. `watch_for_free` is still the wait for the lock.

```rust
// from crates/isograph_cli/src/lib.rs
    #[cfg(windows)]
    fn ask_to_quit(
        instance: &Instance,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        post_quit(instance)
    }
```

```rust
// from crates/isograph_cli/src/lib.rs
#[cfg(windows)]
fn post_quit(instance: &Instance) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let stream =
        crate::discover::connect_to_daemon(&crate::discover::port_file(instance.lock_file()))?;
    stream.set_read_timeout(std::time::Duration::from_secs(5).wrap_some())?;
    stream.set_write_timeout(std::time::Duration::from_secs(5).wrap_some())?;
    crate::send::notify(stream, crate::event::InternalEvent::Quit)?;
    ().wrap_ok()
}
```

A handshake that never completes is `SendError::Read` or `Write`, then `Failure::Unsignalable`, exit 1. A handshake that completes posts `Quit`; `stop_daemon` then waits on the lock.

```rust
// from crates/isograph_cli/src/daemon.rs
    // `isograph stop` without `--force` is SIGTERM on unix, App::ask_to_quit on Windows
    // (InternalEvent::Quit on the socket). Route SIGTERM into the event channel as Quit,
    // so the model turns it into Kill, the effect loop breaks, and serve returns.
    //
    // A spawned task rather than a third `select!` arm, because an arm that completed
    // would drop the other two futures and skip the graceful path this exists to run.
    #[cfg(unix)]
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
```

```
// from docs-website/docs/design-docs/event-model.md
Quit is SIGTERM, and a Quit frame on the socket (isograph send and Windows stop). handle returns Kill.
```

## Tests

Origin: `cli.rs` `STOP` / `RESTART`. Delta: `STOP` is `["stop"]` on every OS. The restart test types `["restart", "--filesystem", "injected"]`. After `kill: exiting`, Windows asserts no `SIGTERM: quitting`; unix asserts that line. `send` of `Quit`, `stop` when not running, and `stop` with the port file removed then `--force` are below.

`Daemon` drop stays `["stop", "--force"]`.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
const STOP: &[&str] = &["stop"];
```

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

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
    poll(|| (!daemon.isograph(["status"].reference()).status.success()).then_some(()));
    poll(|| daemon.log_text().contains("kill: exiting").then_some(()));
    #[cfg(windows)]
    assert!(!daemon.log_text().contains("SIGTERM: quitting"));
    #[cfg(not(windows))]
    assert!(daemon.log_text().contains("SIGTERM: quitting"));
```

in `stop_after_send_exits_without_force`, `two_sends_then_stop_exits_without_force`, and `stop_then_status_reports_not_running`.

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

#[test]
fn stop_when_not_running_exits_0() {
    let daemon = Daemon::start();
    assert!(daemon.isograph(STOP).status.success());
    poll(|| (!daemon.isograph(["status"].reference()).status.success()).then_some(()));
    let again = daemon.isograph(STOP);
    assert!(
        again.status.success(),
        "stdout: {} stderr: {}",
        stdout(again.reference()),
        stderr(again.reference())
    );
}

#[test]
fn stop_with_no_port_exits_1_and_force_destroys() {
    let daemon = Daemon::start();
    std::fs::remove_file(port_file_path(daemon.reference())).expect("the port file exists");
    let stopped = daemon.isograph(STOP);
    #[cfg(windows)]
    assert!(
        !stopped.status.success(),
        "stdout: {} stderr: {}",
        stdout(stopped.reference()),
        stderr(stopped.reference())
    );
    #[cfg(not(windows))]
    assert!(
        stopped.status.success(),
        "stdout: {} stderr: {}",
        stdout(stopped.reference()),
        stderr(stopped.reference())
    );
    let forced = daemon.isograph(["stop", "--force"].reference());
    assert!(
        forced.status.success(),
        "stdout: {} stderr: {}",
        stdout(forced.reference()),
        stderr(forced.reference())
    );
    poll(|| (!daemon.isograph(["status"].reference()).status.success()).then_some(()));
}
```
