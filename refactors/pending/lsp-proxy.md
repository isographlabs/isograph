# `isograph lsp` stdio proxy

Requires lsp-port.md (landed). Independent of lsp-tokens.md, lsp-sessions.md, lsp-diagnostics.md, no-poll-in-tests.md. The daemon port is already LSP JSON-RPC. This verb is a byte copy, not a second handshake.

VS Code and Zed spawn a process on stdio. They do not dial `{slug}.port`. `isograph lsp` is that process: the same walk-up / `--config` as every verb, start the daemon if needed, dial the LSP port, copy stdin/stdout. Dropping the editor drops the proxy. The daemon stays up.

Origin: `docs-website/docs/design-docs/event-model.md` (`isograph lsp` as stdio proxy). Origin of spawn args: `vscode-extension/src/languageClient.ts` (verbatim below). Origin of start: `freddie_cli` `client::start` via a nested `isograph start` (not an in-process call). Origin of the port file: `crates/isograph_cli/src/send.rs` `parse_port` / `read_port`. Origin of `process::exit(0)` when the daemon side is gone and stdin is still open: `crates/isograph_cli/src/daemon.rs` after `Kill`. Delta: the binary actually has the verb; the daemon is already the LSP server; the proxy does not parse LSP; `parse_port` moves to `discover.rs` so send and the proxy share it; connect retries until the live port accepts, because `start` returning is lock-held not listen-done.

```ts
// from vscode-extension/src/languageClient.ts
  const args = ['lsp'];

  if (config.pathToConfig != null) {
    args.push('--config');
    args.push(config.pathToConfig);
  }
```

isograph's `lsp` is `Connection::stdio()` in that process. Ours cannot be: the server is already the per-config daemon on TCP. Do not move `session` onto stdio. Do not send `initialize` from this process. Send remains a separate client.

One shippable change.

## What the user does

```
$ isograph lsp
```

With a config at or above cwd, the daemon is running (or this process starts it the way `isograph start` does), and stdio is LSP. The process stays until stdin EOF (then it drains daemon-to-stdout and returns 0) or the TCP connection ends (then `process::exit(0)`). `isograph status` is still running. `isograph stop` ends the daemon.

```
$ isograph --help
```

The help lists start, restart, status, logs, stop, config-path, lsp. It does not list send.

```
$ isograph lsp --help
```

`--config` is `ConfigFlag`, same as send and config-path. There is no `--filesystem`. Nested start uses start's default (`Watch`). An already-running daemon, including one started with `--filesystem injected`, is adopted.

```
$ isograph lsp
no isograph.config.json, isograph.config.js, or isograph.config.ts at or above /tmp; create one, or pass one using the --config flag
```

That process exits 1. stdout is empty. The message is on stderr. Same `DiscoverError` as config-path.

VS Code with `isograph.pathToIsograph` pointed at the cargo binary, and Zed after `semantic_tokens` is `combined` or `full`, start highlighting once lsp-tokens.md has landed. This slice: the editor's `initialize` gets a response whose result has `capabilities`. Other methods are whatever the daemon already answers (`MethodNotFound` until those docs).

Do not bring up VS Code.

## Nested start, not in-process `freddie_cli`

`freddie_cli::client::start` is `pub(crate)` and `info!`s "started" / "already running" on the client terminal, which is stdout. Stdout of this process is LSP. Calling start in-process would mix those lines into the editor's JSON-RPC.

`isograph start` from this process is a nested `Command::new(current_exe())`. stdin `Stdio::null()` so it does not take the editor's stdin. stdout `Stdio::null()` so "started (pid N)" does not go to the editor. stderr inherit so a start failure is already on the LSP output channel. The child inherits cwd, `HOME`, `XDG_STATE_HOME`, `LOCALAPPDATA`. `--config` is forwarded when the flag was set. `--filesystem` is not. `current_exe` is this binary, not PATH.

Always invoke start. Start adopts if the lock is held (`a_second_start_adopts_the_running_daemon`). Do not re-read `Held` in the proxy.

`isograph start` returning means the lock is held, not that listen has run. The lock is taken at the start of `run_in_foreground`. `run_daemon` unlinks `{slug}.port` after that, then loads the config, binds, starts the watcher, and writes the file. A kill -9 leftover port file is still there when start returns. Send never hits this: it checks `Held` first and `Free` is `NotRunning`. The proxy started the daemon, so it must not treat a parseable port file as ready.

One loop until `PORT_DEADLINE` (10s, same as today's `cli.rs` wait): read the port file, `TcpStream::connect((Ipv4Addr::LOCALHOST, port))`. Connection refused / reset / timed out: sleep `POLL` and retry. First successful connect wins. Never a parseable file: `NoPort`. Retries exhausted after at least one connect error: that `Connect`. Permission errors on the file fail now. Send's `read_port` stays fail-fast.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/lib.rs
    /// Speak LSP on stdio with the daemon for this config.
    Lsp(ConfigFlag),
```

Not hidden. In `--help`. `ConfigFlag` is already the `--config` every other verb takes. Do not add `IsographArgs`.

```rust
// from crates/isograph_cli/src/lib.rs (before)
#[derive(clap::Subcommand)]
enum CliVerb<THostLanguage: HostLanguage> {
    /// start, restart, status, logs, stop, and the hidden daemon.
    #[command(flatten)]
    Lifecycle(freddie_cli::Verb<Isograph<THostLanguage>>),

    /// Encode one IsographEvent as an LSP notification to the running daemon. Not for typing: tests and CI.
    #[command(hide = true)]
    Send(SendArgs),

    /// Print the canonical isograph config path.
    ConfigPath(ConfigFlag),
}
```

```rust
// from crates/isograph_cli/src/lib.rs (after)
#[derive(clap::Subcommand)]
enum CliVerb<THostLanguage: HostLanguage> {
    /// start, restart, status, logs, stop, and the hidden daemon.
    #[command(flatten)]
    Lifecycle(freddie_cli::Verb<Isograph<THostLanguage>>),

    /// Encode one IsographEvent as an LSP notification to the running daemon. Not for typing: tests and CI.
    #[command(hide = true)]
    Send(SendArgs),

    /// Print the canonical isograph config path.
    ConfigPath(ConfigFlag),

    /// Speak LSP on stdio with the daemon for this config.
    Lsp(ConfigFlag),
}
```

```rust
// from crates/isograph_cli/src/lib.rs (before)
        Some(CliVerb::Lifecycle(verb)) => {
            freddie_cli::run_lifecycle_verb::<Isograph<THostLanguage>>(verb, matches.reference())
        }
        Some(CliVerb::Send(args)) => send::run(args.reference()),
        Some(CliVerb::ConfigPath(id)) => config_path::run(id.reference()),
        None => freddie_cli::run_lifecycle_verb::<Isograph<THostLanguage>>(
            freddie_cli::verb_for_bare_invocation::<Isograph<THostLanguage>>(),
            matches.reference(),
        ),
```

```rust
// from crates/isograph_cli/src/lib.rs (after)
        Some(CliVerb::Lifecycle(verb)) => {
            freddie_cli::run_lifecycle_verb::<Isograph<THostLanguage>>(verb, matches.reference())
        }
        Some(CliVerb::Send(args)) => send::run(args.reference()),
        Some(CliVerb::ConfigPath(id)) => config_path::run(id.reference()),
        Some(CliVerb::Lsp(id)) => lsp_stdio::run(id.reference()),
        None => freddie_cli::run_lifecycle_verb::<Isograph<THostLanguage>>(
            freddie_cli::verb_for_bare_invocation::<Isograph<THostLanguage>>(),
            matches.reference(),
        ),
```

`lib.rs`: `mod lsp_stdio;`. Do not call `init_client_logging`. Tracing to the client terminal is stdout.

```rust
// from crates/isograph_cli/src/lsp_stdio.rs
use std::fs;
use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, Shutdown, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use prelude::Postfix;

use crate::ConfigFlag;
use crate::discover::DiscoverError;

const PORT_DEADLINE: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(50);

#[derive(Debug)]
struct ReadPort {
    pub path: PathBuf,
    pub source: io::Error,
}

#[derive(Debug)]
struct Connect {
    pub port: u16,
    pub source: io::Error,
}

enum ConnectRetry {
    Retry,
    Fail,
}

enum CopyEnd {
    Stdin,
    Socket,
}

#[derive(Debug, thiserror::Error)]
enum LspError {
    #[error("{0}")]
    Discover(#[from] DiscoverError),
    #[error("could not find this executable: {0}")]
    CurrentExe(io::Error),
    #[error("could not spawn isograph start: {0}")]
    Spawn(io::Error),
    #[error("could not start the daemon")]
    Start,
    #[error("the daemon has not recorded its port yet")]
    NoPort,
    #[error("could not read {}: {}", .0.path.display(), .0.source)]
    ReadPort(ReadPort),
    #[error("could not clone the stream: {0}")]
    Clone(io::Error),
    #[error("could not connect to 127.0.0.1:{}: {}", .0.port, .0.source)]
    Connect(Connect),
}
```

`ReadPort` / `Connect` / `NoPort` wording matches send. `Start` is a nested start that exited nonzero; that child's stderr is already inherited. Still print `LspError` on stderr, same `run` shape as send.

`ConnectRetry` is whether a failed `connect` is "the port file is leftover / the listener is not up yet" (`ConnectionRefused`, `ConnectionReset`, `TimedOut`) or a hard failure. `CopyEnd` is which copy direction finished: stdin EOF vs the TCP connection.

```rust
// from crates/isograph_cli/src/lsp_stdio.rs
#[expect(clippy::print_stderr)]
pub fn run(id: &ConfigFlag) -> ExitCode {
    match run_inner(id) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn run_inner(id: &ConfigFlag) -> Result<(), LspError> {
    let (_, instance) = crate::discover::instance_for_config_path(id.config.as_deref())?;
    start_daemon(id)?;
    let stream = connect_to_daemon(&crate::discover::port_file(instance.lock_file()))?;
    copy_stdio(stream)
}

fn start_daemon(id: &ConfigFlag) -> Result<(), LspError> {
    let exe = std::env::current_exe().map_err(LspError::CurrentExe)?;
    let mut command = Command::new(exe);
    command
        .arg("start")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    if let Some(config) = id.config.as_deref() {
        command.arg("--config").arg(config);
    }
    let status = command.status().map_err(LspError::Spawn)?;
    if status.success() {
        ().wrap_ok()
    } else {
        LspError::Start.wrap_err()
    }
}

fn connect_retry(source: &io::Error) -> ConnectRetry {
    match source.kind() {
        io::ErrorKind::ConnectionRefused
        | io::ErrorKind::ConnectionReset
        | io::ErrorKind::TimedOut => ConnectRetry::Retry,
        _ => ConnectRetry::Fail,
    }
}

fn connect_to_daemon(path: &Path) -> Result<TcpStream, LspError> {
    let start = Instant::now();
    let mut last_connect = None;
    loop {
        match fs::read_to_string(path) {
            Ok(text) => {
                if let Some(port) = crate::discover::parse_port(text.reference()) {
                    match TcpStream::connect((Ipv4Addr::LOCALHOST, port)) {
                        Ok(stream) => return stream.wrap_ok(),
                        Err(source) => match connect_retry(source.reference()) {
                            ConnectRetry::Retry => {
                                last_connect = Connect { port, source }.wrap_some();
                            }
                            ConnectRetry::Fail => {
                                return LspError::Connect(Connect { port, source }).wrap_err();
                            }
                        },
                    }
                }
            }
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return LspError::ReadPort(ReadPort {
                    path: path.to_owned(),
                    source,
                })
                .wrap_err();
            }
        }
        if start.elapsed() >= PORT_DEADLINE {
            return match last_connect {
                Some(connect) => LspError::Connect(connect).wrap_err(),
                None => LspError::NoPort.wrap_err(),
            };
        }
        thread::sleep(POLL);
    }
}
```

`instance_for_config_path` does not parse the config as JSON. Nested start does. `--config` is the path the user typed, not the canonical one. Absent flag: nested start walk-up from the same cwd.

The connect loop is a state machine (deadline + leftover file + not-yet-listening). Send's `read_port` is still fail-fast (`BadPort` / `NoPort` with no loop).

### Byte copy

Do not parse LSP. No `Message::read`. No `initialize`. `processId` in the editor's `initialize` is the editor, not the proxy. We still do not watch `processId` on the daemon (lsp-sessions.md).

Stdout of this process is often a pipe (VS Code `LanguageClient`, the test). Rust block-buffers pipe stdout. `std::io::copy` can hold an `initialize` result until 8KiB more arrives. Flush after every write both directions. `set_nodelay(true)` is best-effort; ignore its error. Drop the original `TcpStream` after the two clones.

The two copy directions are not symmetric.

Stdin EOF is the editor finishing (`shutdown` / `exit` / close). vscode-languageclient sends `shutdown`, waits for the response, sends `exit`, then closes stdin. `Shutdown::Write` (half-close). Join the socket-to-stdout thread so the last daemon writes, including that `shutdown` result, reach the editor. Then return 0. `run`'s `Ok` arm is this path.

Socket EOF is the daemon gone. The stdin-to-socket thread is blocked on stdin, not on the socket, so joining it hangs until the editor closes stdin. `process::exit(0)`. Origin: `daemon.rs` after `Kill`. After a successful connect, copy errors (broken pipe) are still exit 0.

```rust
// from crates/isograph_cli/src/lsp_stdio.rs
fn copy_stdio(stream: TcpStream) -> Result<(), LspError> {
    let _ = stream.set_nodelay(true);
    let mut to_daemon = stream.try_clone().map_err(LspError::Clone)?;
    let mut from_daemon = stream.try_clone().map_err(LspError::Clone)?;
    drop(stream);
    let (done_tx, done_rx) = mpsc::channel();
    let stdin_done = done_tx.clone();
    thread::spawn(move || {
        let mut stdin = io::stdin();
        let _ = copy_flush(&mut stdin, &mut to_daemon);
        let _ = to_daemon.shutdown(Shutdown::Write);
        let _ = stdin_done.send(CopyEnd::Stdin);
    });
    let outgoing = thread::spawn(move || {
        let mut stdout = io::stdout();
        let result = copy_flush(&mut from_daemon, &mut stdout);
        let _ = done_tx.send(CopyEnd::Socket);
        result
    });
    match done_rx.recv() {
        Ok(CopyEnd::Stdin) => {
            let _ = outgoing.join();
            ().wrap_ok()
        }
        Ok(CopyEnd::Socket) | Err(_) => std::process::exit(0),
    }
}

fn copy_flush(reader: &mut impl Read, writer: &mut impl Write) -> io::Result<u64> {
    let mut buf = [0u8; 8192];
    let mut total = 0u64;
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => return total.wrap_ok(),
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return e.wrap_err(),
        };
        writer.write_all(&buf[..n])?;
        writer.flush()?;
        total += n as u64;
    }
}
```

No tokio. No `Connection`. No `init_client_logging`.

### `parse_port` moves to `discover.rs`

Origin: `crates/isograph_cli/src/send.rs` `parse_port` and its tests. Delta: `pub(crate)`. `send.rs` `read_port` calls `crate::discover::parse_port`. The `parse_port_*` tests move with it. `read_port_of_a_missing_file_is_no_port` stays in `send.rs`.

```rust
// from crates/isograph_cli/src/discover.rs
use std::num::NonZeroU16;

pub(crate) fn parse_port(text: &str) -> Option<u16> {
    text.trim().parse::<NonZeroU16>().ok().map(NonZeroU16::get)
}
```

Body unchanged. `port_file` is already here.

```rust
// from crates/isograph_cli/src/send.rs
fn read_port(path: &Path) -> Result<u16, SendError> {
    match fs::read_to_string(path) {
        Ok(text) => crate::discover::parse_port(text.reference()).ok_or(SendError::BadPort),
        Err(source) if source.kind() == io::ErrorKind::NotFound => SendError::NoPort.wrap_err(),
        Err(source) => SendError::ReadPort(ReadPort {
            path: path.to_owned(),
            source,
        })
        .wrap_err(),
    }
}
```

Drop `parse_port` from `send.rs`. Drop `use std::num::NonZeroU16` if nothing else in that file needs it.

`crates/isograph_cli/Cargo.toml` is unchanged.

## Tests

`cli.rs`. HOME isolation as today. Do not bring up VS Code. Do not assert semantic tokens (lsp-tokens.md). Do not add a production function only tests call. Nested start can take `START_TIMEOUT` (5s) plus `PORT_DEADLINE` (10s) before a byte is copied. Block on `Message::read` and on `child.wait()`. Do not `sleep` and then hope the response is there.

`ts_graphql_react_isograph_cli` tests write and read `lsp_server::Message` the way `send.rs` does. Do not hand-roll `Content-Length`.

```toml
# from crates/ts_graphql_react_isograph_cli/Cargo.toml (before)
[dev-dependencies]
prelude = { path = "../prelude" }
tempfile = "3"
```

```toml
# from crates/ts_graphql_react_isograph_cli/Cargo.toml (after)
[dev-dependencies]
lsp-server = { workspace = true }
lsp-types = { workspace = true }
prelude = { path = "../prelude" }
serde_json = { workspace = true }
tempfile = "3"
```

`cli.rs` already imports `Command`. Add `Stdio` and `BufReader`.

Add `Daemon::spawn` next to `Daemon::isograph`. Same env, cwd, binary. stdin and stdout piped. stderr inherit so a nested start cannot fill an unread stderr pipe and deadlock, and so a failure is on the test output.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
fn spawn(&self, args: &[&str]) -> std::process::Child {
    let home = self.dir.path().join("home");
    std::fs::create_dir_all(home.reference()).expect("a test can create its private HOME");
    Command::new(isograph_bin())
        .args(args)
        .current_dir(self.dir.path())
        .env("HOME", home.reference())
        .env("XDG_STATE_HOME", home.join("state"))
        .env("LOCALAPPDATA", home.join("appdata"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("the isograph binary runs")
}
```

`Daemon::isograph` stays `.output()`. Existing tests are unchanged.

Keep `ChildStdout` in the test as `BufReader` until after `child.wait()`. Do not move it into a thread that drops it after one message: that closes the pipe and kills the proxy with broken pipe, so a later stdin drop does not test stdin EOF.

Helper: write initialize (`capabilities: {}`, id 1, same params as `send::notify`). Read the first `Message::Response` on the test thread from `&mut impl BufRead`. Assert `error` is `None` and `result` is an object with a `capabilities` key. Do not send `initialized` unless a later assertion needs the session past handshake.

After the assertion: drop stdin, `child.wait()`, assert the status is success. Then `status` is running. Then `STOP`. Then `status` is not running. `Daemon`'s `Drop` still `--force`s.

If a test panics before dropping stdin, `Daemon::drop` stops the daemon, the proxy's socket EOF, `process::exit(0)`.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
fn write_initialize(stdin: &mut impl std::io::Write) {
    lsp_server::Message::Request(lsp_server::Request {
        id: lsp_server::RequestId::from(1),
        method: lsp_types::request::Initialize::METHOD.to_owned(),
        params: serde_json::json!({ "capabilities": {} }),
    })
    .write(stdin)
    .expect("writing initialize");
    stdin.flush().expect("flushing initialize");
}

fn read_initialize_result(stdout: &mut impl std::io::BufRead) -> lsp_server::Response {
    let message = lsp_server::Message::read(stdout)
        .expect("reading an lsp message")
        .expect("the proxy stayed open");
    let lsp_server::Message::Response(response) = message else {
        panic!("initialize must be answered with a response, got {message:?}");
    };
    assert!(response.error.is_none(), "{response:?}");
    let result = response
        .result
        .as_ref()
        .expect("initialize result is present");
    assert!(result.get("capabilities").is_some(), "{result}");
    response
}
```

- `lsp_is_in_help`: `isograph --help` contains `lsp`. Does not hide it. Still does not contain `send`. `isograph lsp --help` contains `config`.
- `lsp_with_the_daemon_stopped_starts_it`: fixture as `Daemon::start` (`{"source_files":[]}`), do not call `start` first. `spawn(["lsp"])`, write initialize, read a response with `capabilities`, drop stdin, `child.wait()` success, `status` running, `STOP`, `status` not running. Nested start uses default `Watch`. Empty `source_files` is already a watch path (`watch_of_empty_source_files_logs_scan_finished`).
- `lsp_with_the_daemon_already_running_dials_it`: `Daemon::start()` (injected). Record `status` stdout (includes the pid). Then `spawn(["lsp"])`, initialize, drop stdin, `child.wait()` success. `status` stdout equals the recorded line. The nested start adopts.
- `lsp_with_no_config_exits_1`: empty cwd, HOME isolation, `isograph lsp` as `.output()`, exit 1, stderr contains `no isograph.config`, stdout empty, `status` not running.
- `lsp_with_an_empty_object_config_exits_1`: `{}` config, same as `start_with_missing_source_files_exits_1`. Discover succeeds (the file exists). Nested start fails. Exit 1, stderr contains `source_files`. `status` not running.
- `lsp_with_empty_stdin_exits_0_and_leaves_the_daemon`: fixture with `source_files`, `spawn(["lsp"])`, drop stdin without writing, `child.wait()` success, `status` running, then `STOP`.
- `lsp_two_proxies_share_one_daemon`: `Daemon::start()`, record `status` stdout. Two `spawn(["lsp"])`, both initialize, both drop stdin and `wait` success. `status` stdout equals the recorded line. `STOP`.
- `lsp_forwards_config_to_nested_start`: write `{"source_files":[]}` at a path that walk-up from cwd would not find (cwd is an empty sibling directory). `spawn(["lsp", "--config", path])`, initialize, drop stdin, `wait` success, `status` running. Walk-up without the flag is the no-config failure.

Existing send / start / stop tests stay green.

## Call sites

- VS Code `languageClient.ts` `args = ['lsp']` plus optional `--config` -> `isograph lsp` -> nested `isograph start` if needed -> `{slug}.port` -> `connect_to_daemon` -> `accept_loop` -> `session`
- Zed `language_server_command` `args: "lsp"` (zed-and-vscode-extensions.md, after this lands)
- `isograph send` does not use this verb
- editor stdin EOF -> `Shutdown::Write` -> join daemon-to-stdout -> return 0 -> session `Drop` -> daemon stays
- `isograph stop` ends the daemon; a still-running proxy sees socket EOF and `process::exit(0)`
