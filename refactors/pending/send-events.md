# Send events to the daemon

Requires event-loop.md (landed) and config-discovery.md (landed).

The daemon listens on `freddie_event_socket` at `127.0.0.1:0`. The kernel assigns a port from its local/dynamic range. After bind, the daemon writes `local_addr().port()` next to its lock (`{slug}.lock` → `{slug}.port`). `isograph send` finds the config path, keys the lock, and reads that file. Every event is `Serialize` + `Deserialize`. The wire is `serde_json`. `on_message` deserializes `IsographEvent` and sends it. There is no second event enum. There is no `--port`. There is no stdin path.

Origin of the socket and of `on_message`: figaro `src/daemon.rs` and `src/external.rs`. Origin of `isograph send`: `refactors/pending/filesystem-events.md` change 2. Origin of writing the assigned port next to the lock: freddie `refactors/past/event-socket-local-addr.md`. Delta: the wire type is `IsographEvent`, not a separate `IncomingEvent`; figaro keeps `IncomingEvent` so keys and quit are unrepresentable on the socket; isograph events are all serde JSON, including `Quit`; tungstenite 0.24, matching `freddie_event_socket`; `listen(0)` plus `EventSocket::local_addr()` written next to the lock. Figaro binds a fixed default because it is one process per machine.

Send fails unless the port file is already there. Send does not poll. Waiting is the caller's problem. `isograph start` returning means the lock is held, not that listen has run. `listen(0)` is inside `serve`, after the runtime is built.

`send` is a hidden verb, the same `#[command(hide = true)]` as `freddie_cli::Verb::Daemon`. It is not in `--help`. Tests and CI type it.

## What the user does

The daemon is already up. The log has `isograph daemon up`, which is written after the port file. `--file` is a temp file; send unlinks it after the attempt, success or failure.

```
$ isograph logs
{"timestamp":"...","level":"INFO","fields":{"message":"isograph daemon up","config":"/Users/x/app/isograph.config.json","port":53124}}
$ printf '%s\n' '{"kind":"HelloWorld"}' > /tmp/hello.json
$ isograph send --file /tmp/hello.json
$ isograph logs
{"timestamp":"...","level":"INFO","fields":{"message":"isograph daemon up","config":"/Users/x/app/isograph.config.json","port":53124}}
{"timestamp":"...","level":"INFO","fields":{"message":"hello world"}}
```

`isograph send` does not start the daemon. Walk-up / `--config` is the same as every other verb. The port is the decimal in the port file.

```
$ isograph --help
```

The help lists start, restart, status, logs, stop. It does not list send.

```
$ isograph send --file /tmp/hello.json
the daemon is not running
```

That process exits 1. `Held::Free` on the lock is `SendError::NotRunning`, `run` prints it on stderr and returns `ExitCode::FAILURE`. `--file` is still unlinked.

```
$ isograph send --file /tmp/hello.json
the daemon has not recorded its port yet
```

That process exits 1. The lock is held and the port file is absent: `serve` has not written it yet. Send does not wait.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum IsographEvent {
    HelloWorld,
    Quit,
}
```

Origin: landed `event.rs`. Delta: `Serialize` + `Deserialize`, adjacent tagging. Unit variant JSON is `{"kind":"HelloWorld"}` with no `value` field. `Quit` is on the wire. A send of `Quit` is `handle(Quit)` and `Kill`.

```rust
// from crates/isograph_cli/src/external.rs
use tokio::sync::mpsc::UnboundedSender;
use tracing::warn;

use crate::event::IsographEvent;

pub(crate) fn on_message(text: &str, event_tx: &UnboundedSender<IsographEvent>) {
    match serde_json::from_str::<IsographEvent>(text) {
        Ok(event) => {
            let _ = event_tx.send(event);
        }
        Err(e) => warn!(error = %e, frame = text, "undeserializable frame"),
    }
}
```

A frame that is not a valid `IsographEvent` is logged and dropped. The connection stays up. `on_message` is `pub(crate)`. Daemon and the tests in this file call it. Nothing outside the crate does.

A frame larger than 64 KiB closes that connection (`freddie_event_socket` `MAX_FRAME_BYTES`). HelloWorld is under. filesystem-events.md does not send production file contents over the socket.

`App::DaemonArgs` stays `NoArgs`. `App::Id` stays `ConfigFlag`.

```rust
// from crates/isograph_cli/src/discover.rs
pub fn port_file(lock: &Path) -> PathBuf {
    lock.with_extension("port")
}
```

`Instance::named` keys the lock to `{slug}.lock`. The port file is the sibling `{slug}.port`, the same relation `freddie_single_instance` uses for `{slug}.pid`.

```rust
// from crates/isograph_cli/src/discover.rs
pub fn instance_for_config_path(
    flag: Option<&Path>,
) -> Result<(PathBuf, Instance), DiscoverError> {
    let config_path = config_path(flag)?;
    let instance = Instance::named(
        "isograph",
        slug(config_path.reference()),
        config_path.display().to_string(),
    )?;
    (config_path, instance).wrap_ok()
}
```

Canonical path plus `Instance::named`. It does not call `load_config`. A `.ts` / `.js` that would fail to evaluate still names the lock.

## Change 1: the daemon listens

```rust
// from crates/isograph_cli/src/event.rs (before)
#[derive(Debug)]
pub enum IsographEvent {
    HelloWorld,
    Quit,
}
```

The after is the `Serialize` + `Deserialize` enum in Types.

`run_daemon` today logs `isograph daemon up` and calls `daemon::run()` with no instance.

`DaemonArgs` stays `NoArgs`.

```rust
// from crates/isograph_cli/src/lib.rs (before)
    fn run_daemon(id: &ConfigFlag, _: &NoArgs) {
        match discover::config_and_instance(id.config.as_deref()) {
            Ok((path, _, _config)) => {
                tracing::info!(config = %path.display(), "isograph daemon up");
                crate::daemon::run();
            }
            Err(e) => {
                tracing::error!(error = %e, "the config went away between naming this daemon and starting it");
            }
        }
    }
```

```rust
// from crates/isograph_cli/src/lib.rs (after)
    fn run_daemon(id: &ConfigFlag, _: &NoArgs) {
        match discover::config_and_instance(id.config.as_deref()) {
            Ok((path, instance, _config)) => {
                crate::daemon::run(path, discover::port_file(instance.lock_file()));
            }
            Err(e) => {
                tracing::error!(error = %e, "the config went away between naming this daemon and starting it");
            }
        }
    }
```

```toml
# from crates/isograph_cli/Cargo.toml (before)
freddie_cli = { git = "https://github.com/freddiehg/freddie", rev = "af6b57df9732f42cde9bafc109bcf1b6a32f28e0" }
```

```toml
# from crates/isograph_cli/Cargo.toml (after)
freddie_cli = { git = "https://github.com/freddiehg/freddie", rev = "2cf07439ce57fd55079b89128b2f22603ef8d1a8" }
freddie_event_socket = { git = "https://github.com/freddiehg/freddie", rev = "2cf07439ce57fd55079b89128b2f22603ef8d1a8" }
```

`2cf07439ce57fd55079b89128b2f22603ef8d1a8` is `EventSocket reports the address it bound` (freddie `refactors/past/event-socket-local-addr.md`). `af6b57d` is an ancestor. Same rev for both crates, one checkout. On `origin/master`.

```rust
// from crates/isograph_cli/src/lib.rs (before)
mod daemon;
mod discover;
mod effect;
mod event;
mod state;
```

```rust
// from crates/isograph_cli/src/lib.rs (after)
mod daemon;
mod discover;
mod effect;
mod event;
mod external;
mod state;
```

No `pub use`. `on_message` is `pub(crate)` in `external.rs`. `IsographEvent` stays `pub` on the enum, as today.

```rust
// from crates/isograph_cli/src/discover.rs (after, next to slug)
pub fn port_file(lock: &Path) -> PathBuf {
    lock.with_extension("port")
}
```

```rust
// from crates/isograph_cli/src/daemon.rs (before)
pub fn run() {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            tracing::error!(error = %e, "could not start the tokio runtime");
            return;
        }
    };
    runtime.block_on(serve());
}

async fn serve() {
    let (event_tx, event_rx) = unbounded_channel::<IsographEvent>();
    let (effect_tx, effect_rx) = unbounded_channel::<IsographEffect>();
    let _ = event_tx.send(IsographEvent::HelloWorld);

    // `isograph stop` sends SIGTERM. Route it into the event channel as Quit, so the
    // model turns it into Kill, the effect loop breaks, and serve returns.
    //
    // A spawned task rather than a third `select!` arm, because an arm that completed
    // would drop the other two futures and skip the graceful path this exists to run.
    #[cfg(unix)]
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
        Ok(mut term) => {
            let event_tx = event_tx.clone();
            tokio::spawn(async move {
                if term.recv().await.is_some() {
                    tracing::info!("SIGTERM: quitting");
                    let _ = event_tx.send(IsographEvent::Quit);
                }
            });
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "no SIGTERM handler; a terminated isograph will not run Kill"
            );
        }
    }

    // `select!` rather than `join!`: the effect loop ends on `Kill`, and the event
    // loop never does, because `_hold_events` holds a sender for as long as serve runs.
    let _hold_events = event_tx;
    let state = IsographState;
    tokio::select! {
        () = run_event_loop(state, event_rx, effect_tx) => {}
        () = run_effect_loop(effect_rx) => {}
    }
}
```

```rust
// from crates/isograph_cli/src/daemon.rs (after)
use std::path::PathBuf;

use crate::external::on_message;
use prelude::Postfix;

pub fn run(config_path: PathBuf, port_path: PathBuf) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            tracing::error!(error = %e, "could not start the tokio runtime");
            return;
        }
    };
    runtime.block_on(serve(config_path, port_path));
}

async fn serve(config_path: PathBuf, port_path: PathBuf) {
    let (event_tx, event_rx) = unbounded_channel::<IsographEvent>();
    let (effect_tx, effect_rx) = unbounded_channel::<IsographEffect>();
    let _ = std::fs::remove_file(port_path.reference());
    let _socket = match freddie_event_socket::listen(0, {
        let event_tx = event_tx.clone();
        move |text| on_message(text, event_tx.reference())
    }) {
        Ok(socket) => socket,
        Err(e) => {
            tracing::error!(error = %e, "could not bind the event socket");
            return;
        }
    };
    let port = _socket.local_addr().port();
    if let Err(e) = std::fs::write(port_path.reference(), format!("{port}\n")) {
        tracing::error!(
            error = %e,
            path = %port_path.display(),
            "could not write the event socket port"
        );
        return;
    }
    tracing::info!(config = %config_path.display(), port, "isograph daemon up");

    // `isograph stop` sends SIGTERM. Route it into the event channel as Quit, so the
    // model turns it into Kill, the effect loop breaks, and serve returns.
    //
    // A spawned task rather than a third `select!` arm, because an arm that completed
    // would drop the other two futures and skip the graceful path this exists to run.
    #[cfg(unix)]
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
        Ok(mut term) => {
            let event_tx = event_tx.clone();
            tokio::spawn(async move {
                if term.recv().await.is_some() {
                    tracing::info!("SIGTERM: quitting");
                    let _ = event_tx.send(IsographEvent::Quit);
                }
            });
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "no SIGTERM handler; a terminated isograph will not run Kill"
            );
        }
    }

    // `select!` rather than `join!`: the effect loop ends on `Kill`, and the event
    // loop never does, because `_hold_events` holds a sender for as long as serve runs.
    let _hold_events = event_tx;
    let state = IsographState;
    tokio::select! {
        () = run_event_loop(state, event_rx, effect_tx) => {}
        () = run_effect_loop(effect_rx) => {}
    }
}
```

`_socket` is the one binding. It is in scope across `select!`. Dropping `serve` drops the listener. A write failure returns, which drops `_socket` and then `run_daemon` returns, which drops the lock. Do not bind `listen` in a block that ends before `select!`.

`listen(0)` is the kernel's pick from its local/dynamic port range. `serve` unlinks the port file, then binds, then writes the assigned port, then logs it. NotFound on the unlink is the first boot; `let _ =` is not fatal. After a crash, the leftover file is gone before send can see `Held::By`. Lock held and the file absent is `NoPort`. `serve` does not send `HelloWorld`. That event arrives on the socket.

`EventSocket::local_addr` returns `SocketAddr`, not `io::Result`. Origin: freddie `refactors/past/event-socket-local-addr.md`. The file contents are the decimal port and a newline, `"{port}\n"`.

### Tests

Socket tests live in `external.rs` under `#[cfg(test)]`, next to `on_message`. They compile against the crate's `[dependencies]` tokio (`rt`, `macros`, `signal`, `sync`, `time`).

```rust
// from crates/isograph_cli/src/external.rs
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures_util::SinkExt;
    use prelude::Postfix;
    use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
    use tokio_tungstenite::tungstenite::Message;

    use super::on_message;
    use crate::event::IsographEvent;

    const SETTLE: Duration = Duration::from_millis(250);

    fn listen_for_events() -> (
        freddie_event_socket::EventSocket,
        u16,
        UnboundedReceiver<IsographEvent>,
    ) {
        let (event_tx, event_rx) = unbounded_channel();
        let socket = freddie_event_socket::listen(0, move |text| {
            on_message(text, event_tx.reference());
        })
        .expect("binding port 0");
        let port = socket.local_addr().port();
        (socket, port, event_rx)
    }

    #[tokio::test]
    async fn a_hello_world_frame_arrives_as_an_event() {
        let (_socket, port, mut event_rx) = listen_for_events();
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
            .await
            .expect("connecting");
        ws.send(Message::Text(r#"{"kind":"HelloWorld"}"#.to_owned()))
            .await
            .expect("sending");
        tokio::time::sleep(SETTLE).await;
        assert!(matches!(
            event_rx.try_recv().expect("an event arrived"),
            IsographEvent::HelloWorld
        ));
    }

    #[tokio::test]
    async fn an_unknown_frame_is_dropped_without_disturbing_the_connection() {
        let (_socket, port, mut event_rx) = listen_for_events();
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
            .await
            .expect("connecting");
        for frame in [r#"{"kind":"Nope"}"#, "not json at all"] {
            ws.send(Message::Text(frame.to_owned()))
                .await
                .expect("sending");
        }
        tokio::time::sleep(SETTLE).await;
        assert!(event_rx.try_recv().is_err(), "nothing was dispatched");
        ws.send(Message::Text(r#"{"kind":"HelloWorld"}"#.to_owned()))
            .await
            .expect("the connection survived two bad frames");
        tokio::time::sleep(SETTLE).await;
        assert!(
            event_rx.try_recv().is_ok(),
            "the next good frame still arrived"
        );
    }
}
```

Origin of the socket tests: figaro / mercury `tests/external.rs`. Delta: `listen(0)` plus `local_addr().port()` instead of a probe bind; `HelloWorld` instead of `Tab`; the tests sit next to `on_message` so it stays `pub(crate)`.

```toml
# from crates/isograph_cli/Cargo.toml (dev-dependencies, after)
tokio-tungstenite = "0.24"
futures-util = { version = "0.3", default-features = false, features = ["sink"] }
```

`tempfile = "3"` is already in `[dev-dependencies]`. `0.24` matches `freddie_event_socket`. `Message::Text` takes `String`.

```rust
// from crates/isograph_cli/src/discover.rs (tests)
    #[test]
    fn port_file_is_the_lock_with_a_port_extension() {
        assert_eq!(
            super::port_file(Path::new("/tmp/isograph-abcd.lock")),
            Path::new("/tmp/isograph-abcd.port")
        );
    }
```

`the_log_contains_the_config_path` today polls for `hello world` because `serve` sends `HelloWorld` at boot. After this change it does not:

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs (before)
        (log.contains("isograph daemon up")
            && log.contains(path_in_log.reference())
            && log.contains("hello world"))
            .then_some(())
```

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs (after)
        (log.contains("isograph daemon up")
            && log.contains(path_in_log.reference())
            && log.contains("\"port\":"))
            .then_some(())
```

The record has `port`. `"port":` is the JSON field `tracing` writes for the numeric `port` in `isograph daemon up`. Change 2 sends `HelloWorld` through `isograph send` and asserts `hello world`.

## Change 2: `isograph send`

A hidden client verb for tests and CI. It does not start the daemon. `--file` is required. It reads that file as one JSON `IsographEvent` and writes it as one websocket text frame. Then it deletes `--file`, success or failure. There is no stdin path. filesystem-events.md sends `DiskChanged` the same way: a temp file and `--file`.

It does not call `load_config`. A daemon that is up stays reachable if the `.ts` / `.js` config has since broken.

freddie_cli `Verb` is closed. Extra verbs sit beside it, the way figaro's launch-agent verbs do.

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

    /// Write one IsographEvent JSON frame to the running daemon. Not for typing: tests and CI.
    #[command(hide = true)]
    Send(SendArgs),
}

#[derive(clap::Args, Debug)]
struct SendArgs {
    #[command(flatten)]
    pub id: ConfigFlag,

    /// JSON frame to send.
    #[arg(long)]
    pub file: std::path::PathBuf,
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
        Some(CliVerb::Send(args)) => send::run(args.reference()),
        None => freddie_cli::run_lifecycle_verb::<Isograph>(
            freddie_cli::verb_for_bare_invocation::<Isograph>(),
            matches.reference(),
        ),
    }
```

`expect` on `from_arg_matches` is the same line as today.

```rust
// from crates/isograph_cli/src/lib.rs (after, modules)
mod send;
```

```rust
// from crates/isograph_cli/src/discover.rs (before)
pub fn config_and_instance(
    flag: Option<&Path>,
) -> Result<(PathBuf, Instance, IsographConfig), DiscoverError> {
    let config_path = config_path(flag)?;
    let config = load_config(config_path.reference()).map_err(DiscoverError::Load)?;
    let instance = Instance::named(
        "isograph",
        slug(config_path.reference()),
        config_path.display().to_string(),
    )?;
    (config_path, instance, config).wrap_ok()
}
```

```rust
// from crates/isograph_cli/src/discover.rs (after)
pub fn instance_for_config_path(
    flag: Option<&Path>,
) -> Result<(PathBuf, Instance), DiscoverError> {
    let config_path = config_path(flag)?;
    let instance = Instance::named(
        "isograph",
        slug(config_path.reference()),
        config_path.display().to_string(),
    )?;
    (config_path, instance).wrap_ok()
}

pub fn config_and_instance(
    flag: Option<&Path>,
) -> Result<(PathBuf, Instance, IsographConfig), DiscoverError> {
    let (config_path, instance) = instance_for_config_path(flag)?;
    let config = load_config(config_path.reference()).map_err(DiscoverError::Load)?;
    (config_path, instance, config).wrap_ok()
}
```

`App::instance` and `run_daemon` still call `config_and_instance`. Send calls `instance_for_config_path`.

```rust
// from crates/isograph_cli/src/send.rs
use std::fs;
use std::io;
use std::num::NonZeroU16;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use prelude::Postfix;
use tungstenite::Message;
use tungstenite::client::connect;

use crate::SendArgs;
use crate::discover::DiscoverError;
use crate::event::IsographEvent;

#[derive(Debug)]
struct ReadFile {
    pub path: PathBuf,
    pub source: io::Error,
}

#[derive(Debug)]
struct ReadPort {
    pub path: PathBuf,
    pub source: io::Error,
}

#[derive(Debug)]
struct Connect {
    pub port: u16,
    pub source: tungstenite::Error,
}

#[derive(Debug, thiserror::Error)]
enum SendError {
    #[error("{0}")]
    Discover(#[from] DiscoverError),
    #[error("could not read {}: {}", .0.path.display(), .0.source)]
    ReadFile(ReadFile),
    #[error("the frame is not IsographEvent JSON: {0}")]
    NotEvent(serde_json::Error),
    #[error("the daemon is not running")]
    NotRunning,
    #[error("the daemon has not recorded its pid yet")]
    Unnamed,
    #[error("{0}")]
    Lock(#[from] freddie_single_instance::LockError),
    #[error("the daemon has not recorded its port yet")]
    NoPort,
    #[error("could not read {}: {}", .0.path.display(), .0.source)]
    ReadPort(ReadPort),
    #[error("the daemon's port file is not a port")]
    BadPort,
    #[error("could not connect to 127.0.0.1:{}: {}", .0.port, .0.source)]
    Connect(Connect),
    #[error("could not write the frame: {0}")]
    Write(tungstenite::Error),
}

#[expect(clippy::print_stderr)]
pub fn run(args: &SendArgs) -> ExitCode {
    let result = run_inner(args);
    let _ = fs::remove_file(args.file.reference());
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn run_inner(args: &SendArgs) -> Result<(), SendError> {
    let (_, instance) =
        crate::discover::instance_for_config_path(args.id.config.as_deref())?;
    require_running(instance.lock_file())?;
    let port = read_port(&crate::discover::port_file(instance.lock_file()))?;
    let frame = fs::read_to_string(args.file.reference()).map_err(|source| {
        SendError::ReadFile(ReadFile {
            path: args.file.clone(),
            source,
        })
    })?;
    let frame = frame.trim();
    let _: IsographEvent = serde_json::from_str(frame).map_err(SendError::NotEvent)?;
    let (mut ws, _) = connect(format!("ws://127.0.0.1:{port}"))
        .map_err(|source| SendError::Connect(Connect { port, source }))?;
    ws.send(Message::Text(frame.to_owned()))
        .map_err(SendError::Write)?;
    ().wrap_ok()
}

fn require_running(lock: &Path) -> Result<(), SendError> {
    match freddie_single_instance::holder_at(lock)? {
        freddie_single_instance::Held::By(_) => ().wrap_ok(),
        freddie_single_instance::Held::Free => SendError::NotRunning.wrap_err(),
        freddie_single_instance::Held::Unnamed => SendError::Unnamed.wrap_err(),
    }
}

fn read_port(path: &Path) -> Result<u16, SendError> {
    match fs::read_to_string(path) {
        Ok(text) => parse_port(text.reference()).ok_or(SendError::BadPort),
        Err(source) if source.kind() == io::ErrorKind::NotFound => SendError::NoPort.wrap_err(),
        Err(source) => SendError::ReadPort(ReadPort {
            path: path.to_owned(),
            source,
        })
        .wrap_err(),
    }
}

fn parse_port(text: &str) -> Option<u16> {
    text.trim()
        .parse::<NonZeroU16>()
        .ok()
        .map(NonZeroU16::get)
}
```

Send reads the lock first. `Held::Free` is `NotRunning` and the port file is not consulted. Process death releases the lock. `serve` unlinks the port file before listen, then writes it after bind. Lock held and the file absent is `NoPort`.

`Held::Unnamed` and a missing port file are immediate errors. Send does not poll.

`remove_file` runs after `run_inner`, success or failure, so a CI temp file does not remain. A failed remove does not change the exit code.

Validate then send. A frame the daemon would drop is rejected at the client with a non-zero exit. The daemon still drops undeserializable frames from any other client.

`tungstenite` 0.24 `WebSocket::send` writes then flushes. Blocking client, not tokio. The daemon already has a runtime; the client is a one-shot.

```toml
# from crates/isograph_cli/Cargo.toml (dependencies, after)
freddie_single_instance = { git = "https://github.com/freddiehg/freddie", rev = "2cf07439ce57fd55079b89128b2f22603ef8d1a8" }
tungstenite = { version = "0.24", default-features = false, features = ["handshake"] }
```

`0.24` matches `freddie_event_socket`. `Message::Text` takes `String`.

Workspace clippy denies `print_stderr` in library crates. `send::run` is the process entry for this verb and lives in `isograph_cli`. `#[expect(clippy::print_stderr)]` on `send::run`. The function is not a library API. Do not add a tracing subscriber to avoid the expect, and do not add a freddie_cli export for this.

`Ok(())` in `run` is a match pattern on `run_inner`. `ExitCode::SUCCESS` stays.

### Tests

```rust
// from crates/isograph_cli/src/send.rs
#[cfg(test)]
mod tests {
    use prelude::Postfix;

    use super::parse_port;

    #[test]
    fn parse_port_reads_a_decimal_line() {
        assert_eq!(parse_port("53124\n"), 53124.wrap_some());
    }

    #[test]
    fn parse_port_reads_digits_without_a_newline() {
        assert_eq!(parse_port("53124"), 53124.wrap_some());
    }

    #[test]
    fn parse_port_of_empty_is_none() {
        assert_eq!(parse_port(""), None);
        assert_eq!(parse_port("\n"), None);
    }

    #[test]
    fn parse_port_of_zero_is_none() {
        assert_eq!(parse_port("0"), None);
        assert_eq!(parse_port("0\n"), None);
    }

    #[test]
    fn parse_port_of_garbage_is_none() {
        assert_eq!(parse_port("abc"), None);
        assert_eq!(parse_port("65536"), None);
        assert_eq!(parse_port("127.0.0.1:53124"), None);
    }

    #[test]
    fn read_port_of_a_missing_file_is_no_port() {
        let dir = tempfile::tempdir().expect("a test can create a temp directory");
        let path = dir.path().join("gone.port");
        let err = super::read_port(path.reference()).expect_err("the file is missing");
        assert!(matches!(err, super::SendError::NoPort));
    }
}
```

```rust
// from crates/isograph_cli/src/discover.rs (tests)
    #[test]
    fn instance_for_config_path_does_not_parse_json() {
        let dir = temp();
        let path = dir.path().join("isograph.config.json");
        write_file(path.reference(), "{");
        let (got, _) = super::instance_for_config_path(path.as_path().wrap_some())
            .expect("the file exists");
        let canonical = path.canonicalize().expect("the fixture file exists");
        assert_eq!(got, canonical);
        super::load_config(path.reference()).expect_err("truncated json is unparseable");
    }
```

E2E in `crates/ts_graphql_react_isograph_cli/tests/cli.rs`. Tests write a temp JSON file and pass `--file`. `Daemon::isograph` is enough.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
fn write_frame(dir: &std::path::Path, contents: &str) -> std::path::PathBuf {
    let path = dir.join("frame.json");
    std::fs::write(path.reference(), contents).expect("a test can write a frame");
    path
}
```

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs (after)
#[test]
fn the_log_contains_the_config_path() {
    let daemon = Daemon::start();
    let path = daemon
        .dir
        .path()
        .join("isograph.config.json")
        .canonicalize()
        .expect("the fixture exists")
        .display()
        .to_string();
    let path_in_log = path_in_json_log(path.reference());
    poll(|| {
        let log = daemon.log_text();
        (log.contains("isograph daemon up")
            && log.contains(path_in_log.reference())
            && log.contains("\"port\":"))
        .then_some(())
    });
    let frame = write_frame(daemon.dir.path(), "{\"kind\":\"HelloWorld\"}\n");
    let sent = daemon.isograph(["send", "--file", frame.to_str().expect("utf-8")].reference());
    assert!(
        sent.status.success(),
        "stdout: {} stderr: {}",
        stdout(sent.reference()),
        stderr(sent.reference())
    );
    poll(|| daemon.log_text().contains("hello world").then_some(()));
    assert!(!frame.exists(), "send deletes --file");
}

#[test]
fn send_with_the_daemon_stopped_fails() {
    let dir = tempfile::tempdir().expect("a test can create a temp directory");
    let config = dir.path().join("isograph.config.json");
    std::fs::write(config.reference(), "{}\n").expect("a test can write a config file");
    let frame = write_frame(dir.path(), "{\"kind\":\"HelloWorld\"}\n");
    let home = dir.path().join("home");
    std::fs::create_dir_all(home.reference()).expect("a test can create its private HOME");
    let output = Command::new(isograph_bin())
        .args(["send", "--file", frame.to_str().expect("utf-8")].reference())
        .current_dir(dir.path())
        .env("HOME", home.reference())
        .env("XDG_STATE_HOME", home.join("state"))
        .env("LOCALAPPDATA", home.join("appdata"))
        .output()
        .expect("the isograph binary runs");
    assert!(!output.status.success());
    let err = stderr(output.reference());
    assert!(err.contains("not running"), "{err}");
    assert!(!frame.exists(), "send deletes --file");
}

#[test]
fn send_of_not_json_fails() {
    let daemon = Daemon::start();
    poll(|| daemon.log_text().contains("isograph daemon up").then_some(()));
    let frame = write_frame(daemon.dir.path(), "not json\n");
    let sent = daemon.isograph(["send", "--file", frame.to_str().expect("utf-8")].reference());
    assert!(!sent.status.success());
    let err = stderr(sent.reference());
    assert!(err.contains("IsographEvent"), "{err}");
    assert!(!frame.exists(), "send deletes --file");
}

#[test]
fn send_of_unknown_kind_fails() {
    let daemon = Daemon::start();
    poll(|| daemon.log_text().contains("isograph daemon up").then_some(()));
    let frame = write_frame(daemon.dir.path(), "{\"kind\":\"Nope\"}\n");
    let sent = daemon.isograph(["send", "--file", frame.to_str().expect("utf-8")].reference());
    assert!(!sent.status.success());
    let err = stderr(sent.reference());
    assert!(err.contains("IsographEvent"), "{err}");
    assert!(!frame.exists(), "send deletes --file");
}

#[test]
fn send_is_not_in_help() {
    let output = Command::new(isograph_bin())
        .arg("--help")
        .output()
        .expect("the isograph binary runs");
    assert!(output.status.success());
    let text = stdout(output.reference());
    assert!(text.contains("start"), "{text}");
    assert!(!text.contains("send"), "{text}");
}
```

Change 2 replaces `the_log_contains_the_config_path` with the version above: start, wait until `isograph daemon up` (the port file exists), write a frame file, `isograph send --file`, then the log has `hello world`. The wait is the test, not send. Send does not wait.

`isograph send` must run with the same `HOME` / cwd as the daemon so walk-up finds the same config and the same lock. The harness already does that.
