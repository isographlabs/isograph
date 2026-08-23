# Windows `stop` without `--force` asks the app to quit

Requires start-returns-when-listening.md (landed).

Freddie `stop` / `restart` without `--force` is SIGTERM. On Windows, `signal_pid(Terminate)` returns `graceful stop is not yet available on Windows; use --force`. `stop` and `restart` are freddie verbs: find the pid, subscribe to lock release, ask, wait. An isograph intercept of those verbs duplicates that.

The app-specific piece is how to ask. Unix stays SIGTERM. `App::ask_to_quit` is `#[cfg(windows)]`. Isograph posts `InternalEvent::Quit` on the port next to the lock. No temp file. No subprocess. No `notify` wait without a socket timeout.

`--force` stays SIGKILL / `taskkill /F`. `Daemon` drop stays `stop --force`. Mercury keeps the default `Err`.

Two shippable changes. Freddie first, then isograph bumps the rev. Same git rev for `freddie_cli` and `freddie_single_instance`, as today.

Do not add a bool. `--force` is clap's existing flag.

Freddie has no postfix prelude, `unused = deny`, clippy pedantic deny. Freddie snippets below are `text` and use `Ok` / `Err` / `Some` / `.into()` as that crate does. Isograph snippets are `rust` and use postfix.

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

Same two lines as unix.

A daemon that is not running: `stop` exits 0 and prints `is not running`, same as unix.

A daemon whose port file is missing or not a port, or whose port refuses connect, or whose LSP initialize does not complete before the ask timeout: `stop` without `--force` exits 1, `could not signal pid N: ...`. `--force` still destroys it.

Lock held and `{slug}.port` absent is `PortError::NoPort`. Unix `stop` without `--force` still SIGTERM that pid: the kernel delivers it even if `serve` has not reached `select!` and has not installed the tokio handler. Windows cannot: there is no socket yet. That is exit 1, then `--force`.

Unix `stop` / `restart` without `--force` is unchanged (SIGTERM). The method does not exist on unix.

Mercury on Windows without `--force` still prints `graceful stop is not yet available on Windows; use --force`. Isograph never hits the default.

## Change 1: `App::ask_to_quit` in freddie

Lands in `github.com/freddiehg/freddie`. Origin: `crates/freddie_cli/src/lib.rs` `App`, `crates/freddie_cli/src/client.rs` `stop_daemon` / `signal_pid` / `stop` / `restart`. Delta: Windows `Terminate` calls the app. The default is today's error string. Do not extract a `deliver` that is generic over `TApp` on unix.

### Trait

```text
// from crates/freddie_cli/src/lib.rs (freddie)
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

Not `ask_to_quit_if_no_sigterm`. The cfg is that `if`. Not `windows_soft_quit`: Freddie already says "asking it to quit" on `StopArgs`. Not `try_to_quit`: `try_` plus a `Result` is two "did it fail" layers.

Not `Option<Result<(), _>>`. A default `Err(...)` already prints the string. Mercury writes nothing either way. Both "no ask" and "ask failed" are `Failure::Unsignalable`. Keep today's string. Isograph never hits it.

### `stop_daemon` stays one function. Do not extract `deliver`.

Origin `signal_pid(pid, signal)` becomes a match. Origin `debug!(daemon = %pid, ?signal, ...)` also records `app = TApp::NAME`, so `TApp` is used on unix (`unused_variables` / `clippy::extra_unused_type_parameters` are deny). The rest of the function is origin, including `Ok` / `Err`.

```text
// from crates/freddie_cli/src/client.rs (freddie)
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
    // Before the ask, so the wait cannot miss a daemon that exits between the two.
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

Unix `signal_pid` is unchanged. Windows `signal_pid` is deleted. Windows Kill is `kill_pid`, which does not take `Signal`. No `Terminate` arm, no `unreachable!`.

```text
// from crates/freddie_cli/src/client.rs (freddie)
#[cfg(windows)]
fn kill_pid(pid: Pid) -> io::Result<()> {
    // `taskkill /F` terminates the process, which is SIGKILL. `taskkill` without it posts
    // WM_CLOSE, which a daemon with no window never sees, so there is no gentle taskkill.
    ran(Command::new("taskkill")
        .args(["/F", "/PID"])
        .arg(pid.to_string()))
}
```

`stop` is generic. `restart` already is.

```text
// from crates/freddie_cli/src/client.rs (freddie)
pub(crate) fn stop<TApp: App>(instance: &Instance, force: bool) -> ExitCode {
```

```text
// from crates/freddie_cli/src/client.rs (freddie)
    match stop_daemon::<TApp>(instance, signal) {
```

in both `stop` and `restart`.

```text
// from crates/freddie_cli/src/lib.rs (freddie)
        Verb::Stop(args) => client::stop::<TApp>(instance, args.force),
```

Subscribe then ask is unchanged: `watch_for_free` before the match.

### Change 1 tests

Mercury keeps the default. Do not name a type from `refactors/pending/freddie-cli.md`.

`#[cfg(test)]` in `client.rs`. Three apps, each `NoArgs` / `Instance::global`, empty `run_daemon`. Windows-only tests: `ask_to_quit` is cfg.

```text
// from crates/freddie_cli/src/client.rs (freddie)
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

`Posts` and `Fails` name `ask_to_quit` only under `#[cfg(windows)]` on those methods, same cfg as the trait method. Put `#[cfg(windows)]` on those two methods, not on `Silent`.

Unix `cargo test -p freddie_cli` compiles: `TApp::NAME` is used, `ask_to_quit` is not named, `kill_pid` is not compiled.

## Change 2: isograph implements `ask_to_quit`

Bump both freddie git deps to the change-1 rev.

`send::notify` writes initialize, then `wait_for_initialize_result` (`Message::read` with no timeout), then `initialized` and the event. Unix `/bin/kill` returns when the syscall is done. `STOP_TIMEOUT` then bounds `watch_for_free`.

The daemon runtime is `new_current_thread()`. `handle` is sync on that thread. `accept_loop` is a `select!` arm, so it is not polled during `handle`. `TcpStream::connect` can still succeed (listen backlog). Nobody accepts, nobody answers initialize, `notify` blocks.

Same window at startup: `serve` writes the port file, then interns and starts the watcher, then `select!`. The port is advertised. `accept_loop` is not running. Unix in that window: the kernel delivers SIGTERM even if the tokio handler is not installed. Windows without a bound ask: hang.

A fully-up daemon stuck in `handle` is the same split. Unix: `kill` returns, 5s, `Failure::Ignored`, `--force`. Windows `notify` with no timeout never reaches that match.

Do not skip the initialize wait. A live session that writes the initialize result will block on that write if the client never reads it, and then never read `Quit`. Set `set_read_timeout` and `set_write_timeout` on the stream, then reuse `notify`. The wait that means the daemon left is still `watch_for_free`. The ask returns the way `/bin/kill` does, or it fails in one `STOP_TIMEOUT`.

`ask_to_quit` does not call `require_running`. Freddie already has the pid. Correct.

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

Named function, not an IIFE. `PortError`, `io::Error`, and `SendError` each implement `std::error::Error + Send + Sync`. `?` into the box is real.

5s is freddie `STOP_TIMEOUT`. A handshake that never completes is `SendError::Read` / `Write`, then `Failure::Unsignalable`, exit 1. A handshake that completes posts `Quit` and returns; `stop_daemon` then waits on the lock.

Do not add `stop.rs`. `start.rs` already intercepts `Verb::Start` / `Verb::Restart` for wait-until-listening. Do not add a second intercept that performs the ask.

### Daemon comment

```rust
// from crates/isograph_cli/src/daemon.rs
    // Unix `isograph stop` without `--force` sends SIGTERM. Route it into the event
    // channel as Quit, so the model turns it into Kill, the effect loop breaks, and
    // serve returns. Windows `stop` without `--force` is App::ask_to_quit, which posts
    // InternalEvent::Quit on the socket; this task does not exist there.
    //
    // A spawned task rather than a third `select!` arm, because an arm that completed
    // would drop the other two futures and skip the graceful path this exists to run.
    #[cfg(unix)]
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
```

### Event model

```text
// from docs-website/docs/design-docs/event-model.md
`Quit` is SIGTERM, and a `Quit` frame on the socket (`isograph send` and Windows `stop`). `handle` returns `Kill`.
```

## Tests

`cli.rs`. Drop the Windows `--force` argv. `STOP` is `stop` on every OS. Drop `RESTART`; the one restart test types the same argv as unix.

`Daemon` drop stays `["stop", "--force"]`. That is teardown, not the `--force` assertion.

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

The three tests that wait for `SIGTERM: quitting` wait for `kill: exiting` first. Absence of `SIGTERM: quitting` is not a poll predicate. `kill: exiting` is the later event. Wait for that, then assert the SIGTERM line is absent on Windows.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
    poll(|| (!daemon.isograph(["status"].reference()).status.success()).then_some(()));
    poll(|| daemon.log_text().contains("kill: exiting").then_some(()));
    #[cfg(windows)]
    assert!(!daemon.log_text().contains("SIGTERM: quitting"));
    #[cfg(not(windows))]
    assert!(daemon.log_text().contains("SIGTERM: quitting"));
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

`notify` of `Quit` through the hidden verb, all platforms. No SIGTERM on this path, including unix.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
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

Windows without a port cannot ask. Unix SIGTERM still stops that pid. `--force` is the destroy path, not `Daemon` drop.

Keep `poll` until no-poll-in-tests.md.

## Call graph

- Unix `stop` / `restart` without `--force` -> freddie `signal_pid(Terminate)` -> SIGTERM -> daemon `Quit` -> `Kill`
- Windows `stop` / `restart` without `--force` -> freddie `App::ask_to_quit` -> isograph `post_quit` -> `notify(Quit)` with socket timeouts -> `Kill` -> wait on lock -> `stopped` / then start
- `--force` -> SIGKILL / `kill_pid` (`taskkill /F`) on every OS
- `isograph send --file` of `{"kind":"Quit"}` -> `send::run` -> `notify` (already; no timeout change)
