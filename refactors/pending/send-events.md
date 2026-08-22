# Send events to the daemon

Requires event-loop.md (landed) and config-discovery.md (landed).

The daemon listens on `freddie_event_socket` at `127.0.0.1:0`. The kernel assigns a port from its local/dynamic range. `isograph send` finds the config, reads the daemon pid from the lock, and discovers that process's loopback TCP listen port. Every event is `Serialize` + `Deserialize`. The wire is `serde_json`. `on_message` deserializes `IsographEvent` and sends it. There is no second event enum. There is no `--port` and no port file.

Origin of the socket and of `on_message`: figaro `src/daemon.rs` and `src/external.rs`. Origin of `isograph send`: `refactors/pending/filesystem-events.md` change 2. Delta: the wire type is `IsographEvent`, not a separate `IncomingEvent`; figaro keeps `IncomingEvent` so keys and quit are unrepresentable on the socket; isograph events are all serde JSON, including `Quit`; tungstenite 0.24, matching `freddie_event_socket`; `listen(0)` plus `EventSocket::local_addr()`; the client discovers the port from the pid, not from a file. Figaro binds a fixed default because it is one process per machine.

## What the user does

```
$ isograph start
/Users/x/app/isograph.config.json started (pid 12345)
$ isograph send <<'EOF'
{"kind":"HelloWorld"}
EOF
$ isograph logs
{"timestamp":"...","level":"INFO","fields":{"message":"isograph daemon up","config":"/Users/x/app/isograph.config.json","port":53124}}
{"timestamp":"...","level":"INFO","fields":{"message":"hello world"}}
```

`isograph send` does not start the daemon. Walk-up / `--config` is the same as every other verb. The port is the daemon process's loopback TCP listen.

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

pub fn on_message(text: &str, event_tx: &UnboundedSender<IsographEvent>) {
    match serde_json::from_str::<IsographEvent>(text) {
        Ok(event) => {
            let _ = event_tx.send(event);
        }
        Err(e) => warn!(error = %e, frame = text, "undeserializable frame"),
    }
}
```

A frame that is not a valid `IsographEvent` is logged and dropped. The connection stays up.

`App::DaemonArgs` stays `NoArgs`. `App::Id` stays `ConfigFlag`.

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
            Ok((path, _, _config)) => {
                crate::daemon::run(path);
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

pub use event::IsographEvent;
pub use external::on_message;
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
```

```rust
// from crates/isograph_cli/src/daemon.rs (after)
use std::path::PathBuf;

use crate::external::on_message;
use prelude::Postfix;

pub fn run(config_path: PathBuf) {
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
    runtime.block_on(serve(config_path));
}

async fn serve(config_path: PathBuf) {
    let (event_tx, event_rx) = unbounded_channel::<IsographEvent>();
    let (effect_tx, effect_rx) = unbounded_channel::<IsographEffect>();
    let socket = match freddie_event_socket::listen(0, {
        let event_tx = event_tx.clone();
        move |text| on_message(text, event_tx.reference())
    }) {
        Ok(socket) => socket,
        Err(e) => {
            tracing::error!(error = %e, "could not bind the event socket");
            return;
        }
    };
    let port = socket.local_addr().port();
    tracing::info!(config = %config_path.display(), port, "isograph daemon up");
```

The SIGTERM task, `_hold_events`, and `select!` stay. `_socket` is held across `select!` the way figaro holds the listener. `listen(0)` is the kernel's pick from its local/dynamic port range. `serve` logs the assigned port and does not write it to disk. `serve` does not send `HelloWorld`. That event arrives on the socket.

`EventSocket::local_addr` returns `SocketAddr`, not `io::Result`. Origin: freddie `refactors/past/event-socket-local-addr.md`.

### Tests

```rust
// from crates/isograph_cli/src/event.rs
#[cfg(test)]
mod serde_tests {
    use super::IsographEvent;

    #[test]
    fn hello_world_round_trips() {
        let json = r#"{"kind":"HelloWorld"}"#;
        let event: IsographEvent =
            serde_json::from_str(json).expect("a HelloWorld frame deserializes");
        assert!(matches!(event, IsographEvent::HelloWorld));
        assert_eq!(
            serde_json::to_string(&event).expect("HelloWorld serializes"),
            json
        );
    }

    #[test]
    fn quit_round_trips() {
        let json = r#"{"kind":"Quit"}"#;
        let event: IsographEvent = serde_json::from_str(json).expect("a Quit frame deserializes");
        assert!(matches!(event, IsographEvent::Quit));
        assert_eq!(serde_json::to_string(&event).expect("Quit serializes"), json);
    }

    #[test]
    fn garbage_does_not_deserialize() {
        for frame in [
            r#"{"kind":"IncomingEvent.HelloWorld"}"#,
            r#"{"kind":"Nope"}"#,
            "{}",
            "not json at all",
        ] {
            assert!(
                serde_json::from_str::<IsographEvent>(frame).is_err(),
                "{frame} should not deserialize"
            );
        }
    }
}
```

```rust
// from crates/isograph_cli/tests/socket.rs
use std::time::Duration;

use futures_util::SinkExt;
use isograph_cli::{IsographEvent, on_message};
use tokio::sync::mpsc::unbounded_channel;
use tokio_tungstenite::tungstenite::Message;

const SETTLE: Duration = Duration::from_millis(250);

#[tokio::test]
async fn a_hello_world_frame_arrives_as_an_event() {
    let (event_tx, mut event_rx) = unbounded_channel();
    let socket = freddie_event_socket::listen(0, move |text| {
        on_message(text, &event_tx);
    })
    .expect("binding port 0");
    let port = socket.local_addr().port();
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
        .await
        .expect("connecting");
    ws.send(Message::Text(
        r#"{"kind":"HelloWorld"}"#.to_owned(),
    ))
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
    let (event_tx, mut event_rx) = unbounded_channel();
    let socket = freddie_event_socket::listen(0, move |text| {
        on_message(text, &event_tx);
    })
    .expect("binding port 0");
    let port = socket.local_addr().port();
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
    ws.send(Message::Text(
        r#"{"kind":"HelloWorld"}"#.to_owned(),
    ))
    .await
    .expect("the connection survived two bad frames");
    tokio::time::sleep(SETTLE).await;
    assert!(
        event_rx.try_recv().is_ok(),
        "the next good frame still arrived"
    );
}
```

Origin of the socket tests: figaro `tests/external.rs`. Delta: `listen(0)` plus `local_addr().port()` instead of a probe bind; `HelloWorld` instead of `Tab`.

```toml
# from crates/isograph_cli/Cargo.toml (dev-dependencies, after)
tempfile = "3"
tokio-tungstenite = "0.24"
futures-util = { version = "0.3", default-features = false, features = ["sink"] }
```

`0.24` matches `freddie_event_socket`. `Message::Text` takes `String`.

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
        (log.contains("isograph daemon up") && log.contains(path_in_log.reference())).then_some(())
```

The record also has `port`. Change 2 sends `HelloWorld` through `isograph send` and asserts `hello world`.

## Change 2: `isograph send`

A client verb. It does not start the daemon. It reads one JSON `IsographEvent` from stdin, or from `--file`, and writes it as one websocket text frame.

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

    /// Write one IsographEvent JSON frame to the running daemon.
    Send(SendArgs),
}

#[derive(clap::Args, Debug)]
struct SendArgs {
    #[command(flatten)]
    pub id: ConfigFlag,

    /// File containing the JSON frame. When absent, stdin.
    #[arg(long)]
    pub file: Option<std::path::PathBuf>,
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
// from crates/isograph_cli/src/send.rs
use std::fs;
use std::io::{self, Read};
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
    #[error("could not read stdin: {0}")]
    ReadStdin(io::Error),
    #[error("the frame is not IsographEvent JSON: {0}")]
    NotEvent(serde_json::Error),
    #[error("the daemon is not running")]
    NotRunning,
    #[error("the daemon has not recorded its pid yet")]
    Unnamed,
    #[error("{0}")]
    Lock(#[from] freddie_single_instance::LockError),
    #[error("could not list listen ports: {0}")]
    ListPorts(io::Error),
    #[error("pid {0} has no loopback TCP listen")]
    NoListen(u32),
    #[error("pid {0} has more than one loopback TCP listen")]
    AmbiguousListen(u32),
    #[error("could not connect to 127.0.0.1:{}: {}", .0.port, .0.source)]
    Connect(Connect),
    #[error("could not write the frame: {0}")]
    Write(tungstenite::Error),
}

#[expect(clippy::print_stderr)]
pub fn run(args: &SendArgs) -> ExitCode {
    match run_inner(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn run_inner(args: &SendArgs) -> Result<(), SendError> {
    let (_, instance, _) = crate::discover::config_and_instance(args.id.config.as_deref())?;
    let port = listen_port(daemon_pid(instance.lock_file())?)?;
    let frame = match args.file.as_deref() {
        Some(path) => fs::read_to_string(path).map_err(|source| {
            SendError::ReadFile(ReadFile {
                path: path.to_owned(),
                source,
            })
        })?,
        None => {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .map_err(SendError::ReadStdin)?;
            buf
        }
    };
    let frame = frame.trim();
    let _: IsographEvent = serde_json::from_str(frame).map_err(SendError::NotEvent)?;
    let (mut ws, _) = connect(format!("ws://127.0.0.1:{port}"))
        .map_err(|source| SendError::Connect(Connect { port, source }))?;
    ws.send(Message::Text(frame.to_owned()))
        .map_err(SendError::Write)?;
    ().wrap_ok()
}

fn daemon_pid(lock: &Path) -> Result<freddie_single_instance::Pid, SendError> {
    match freddie_single_instance::holder_at(lock)? {
        freddie_single_instance::Held::By(pid) => pid.wrap_ok(),
        freddie_single_instance::Held::Free => SendError::NotRunning.wrap_err(),
        freddie_single_instance::Held::Unnamed => SendError::Unnamed.wrap_err(),
    }
}

fn listen_port(pid: freddie_single_instance::Pid) -> Result<u16, SendError> {
    let ports = loopback_listens(pid)?;
    match ports.as_slice() {
        [port] => (*port).wrap_ok(),
        [] => SendError::NoListen(pid.0).wrap_err(),
        _ => SendError::AmbiguousListen(pid.0).wrap_err(),
    }
}

#[cfg(unix)]
fn loopback_listens(pid: freddie_single_instance::Pid) -> Result<Vec<u16>, SendError> {
    let output = std::process::Command::new("lsof")
        .args(["-nP", "-iTCP@127.0.0.1", "-sTCP:LISTEN", "-a", "-p"])
        .arg(pid.to_string())
        .output()
        .map_err(SendError::ListPorts)?;
    parse_lsof(&String::from_utf8_lossy(output.stdout.reference())).wrap_ok()
}

fn parse_lsof(stdout: &str) -> Vec<u16> {
    stdout
        .lines()
        .filter_map(|line| {
            let name = line.split_whitespace().last()?;
            let addr = name.strip_suffix(" (LISTEN)")?;
            let port = addr.rsplit_once(':')?.1;
            port.parse().ok()
        })
        .collect()
}

#[cfg(windows)]
fn loopback_listens(pid: freddie_single_instance::Pid) -> Result<Vec<u16>, SendError> {
    let output = std::process::Command::new("netstat")
        .args(["-ano", "-p", "TCP"])
        .output()
        .map_err(SendError::ListPorts)?;
    parse_netstat(&String::from_utf8_lossy(output.stdout.reference()), pid.0).wrap_ok()
}

fn parse_netstat(stdout: &str, pid: u32) -> Vec<u16> {
    stdout
        .lines()
        .filter_map(|line| {
            let cols: Vec<&str> = line.split_whitespace().collect();
            let addr = cols.get(1)?;
            let state = cols.get(3)?;
            let owner = cols.get(4)?;
            if *state != "LISTENING" {
                return None;
            }
            if owner.parse::<u32>().ok()? != pid {
                return None;
            }
            let port = addr.strip_prefix("127.0.0.1:")?;
            port.parse().ok()
        })
        .collect()
}
```

Origin of a subprocess instead of an unsafe bind: `freddie_cli` `signal_pid` uses `/bin/kill`. The event socket is this process's only loopback TCP listen.

Validate then send. A frame the daemon would drop is rejected at the client with a non-zero exit. The daemon still drops undeserializable frames from any other client.

Blocking `tungstenite`, not tokio, on the client. The daemon already has a runtime; the client is a one-shot.

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
    use super::parse_lsof;

    #[test]
    fn parse_lsof_reads_the_loopback_listen_port() {
        let stdout = "\
COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME
isograph 12345 user    8u  IPv4 0x0      0t0  TCP 127.0.0.1:53124 (LISTEN)
";
        assert_eq!(parse_lsof(stdout), [53124]);
    }

    #[test]
    fn parse_lsof_of_a_header_only_is_empty() {
        let stdout = "COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME\n";
        assert!(parse_lsof(stdout).is_empty());
    }
}
```

E2E in `crates/ts_graphql_react_isograph_cli/tests/cli.rs`.

`Daemon::isograph` today has no stdin. Add a second function, not a flag.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
use std::io::Write;
use std::process::Stdio;

impl Daemon {
    fn isograph_stdin(&self, args: &[&str], stdin: &str) -> Output {
        let home = self.dir.path().join("home");
        std::fs::create_dir_all(home.reference()).expect("a test can create its private HOME");
        let mut child = Command::new(isograph_bin())
            .args(args)
            .current_dir(self.dir.path())
            .env("HOME", home.reference())
            .env("XDG_STATE_HOME", home.join("state"))
            .env("LOCALAPPDATA", home.join("appdata"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the isograph binary runs");
        child
            .stdin
            .as_mut()
            .expect("piped stdin")
            .write_all(stdin.as_bytes())
            .expect("the test writes the frame");
        child
            .wait_with_output()
            .expect("the isograph binary exits")
    }
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
        (log.contains("isograph daemon up") && log.contains(path_in_log.reference())).then_some(())
    });
    let sent = daemon.isograph_stdin(
        ["send"].reference(),
        "{\"kind\":\"HelloWorld\"}\n",
    );
    assert!(
        sent.status.success(),
        "stdout: {} stderr: {}",
        stdout(sent.reference()),
        stderr(sent.reference())
    );
    poll(|| daemon.log_text().contains("hello world").then_some(()));
}

#[test]
fn send_with_the_daemon_stopped_fails() {
    let dir = tempfile::tempdir().expect("a test can create a temp directory");
    let config = dir.path().join("isograph.config.json");
    std::fs::write(config.reference(), "{}\n").expect("a test can write a config file");
    let home = dir.path().join("home");
    std::fs::create_dir_all(home.reference()).expect("a test can create its private HOME");
    let output = Command::new(isograph_bin())
        .args(["send"].reference())
        .current_dir(dir.path())
        .env("HOME", home.reference())
        .env("XDG_STATE_HOME", home.join("state"))
        .env("LOCALAPPDATA", home.join("appdata"))
        .output()
        .expect("the isograph binary runs");
    assert!(!output.status.success());
    let err = stderr(output.reference());
    assert!(
        err.contains("not running"),
        "{err}"
    );
}

#[test]
fn send_of_not_json_fails() {
    let daemon = Daemon::start();
    let sent = daemon.isograph_stdin(["send"].reference(), "not json\n");
    assert!(!sent.status.success());
    let err = stderr(sent.reference());
    assert!(err.contains("IsographEvent"), "{err}");
}

#[test]
fn send_file_logs_hello_world() {
    let daemon = Daemon::start();
    let frame = daemon.dir.path().join("frame.json");
    std::fs::write(
        frame.reference(),
        "{\"kind\":\"HelloWorld\"}\n",
    )
    .expect("a test can write a frame");
    let sent = daemon.isograph(["send", "--file", frame.to_str().expect("utf-8")].reference());
    assert!(
        sent.status.success(),
        "stdout: {} stderr: {}",
        stdout(sent.reference()),
        stderr(sent.reference())
    );
    poll(|| daemon.log_text().contains("hello world").then_some(()));
}
```

Change 2 replaces `the_log_contains_the_config_path` with the version above: start, then `isograph send` of `HelloWorld`, then the log has `hello world`. One e2e covers daemon up, the config path, and the CLI.

`isograph send` must run with the same `HOME` / cwd as the daemon so walk-up finds the same config and the same lock. The harness already does that.

`send --file` uses `Daemon::isograph`, not stdin.
