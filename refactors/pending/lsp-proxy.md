# `isograph lsp`

Requires lsp-port.md (landed). Independent of lsp-tokens.md, lsp-sessions.md, lsp-diagnostics.md.

VS Code and Zed spawn a process on stdio. They do not dial `{slug}.port`. `isograph lsp` is that process: the same walk-up / `--config` as every other verb, `isograph start`, dial the port, copy stdin/stdout. Dropping the editor drops the proxy. The daemon stays up.

Origin: `docs-website/docs/design-docs/event-model.md`. Origin of spawn args: `vscode-extension/src/languageClient.ts`. Origin of start: nested `isograph start`, not an in-process call. Origin of the port file: `send.rs` `parse_port` / `read_port`. Delta: the binary has the verb; the daemon is already the LSP server; this process does not parse LSP; `parse_port` moves to `discover.rs` so send and the proxy share it.

```ts
// from vscode-extension/src/languageClient.ts
  const args = ['lsp'];

  if (config.pathToConfig != null) {
    args.push('--config');
    args.push(config.pathToConfig);
  }
```

One shippable change.

## What the user does

```
$ isograph lsp
```

A config at or above cwd (or `--config`). The daemon is running, or this process starts it. Stdio is LSP. The process stays until stdin EOF or the TCP connection ends, then exits 0. `isograph status` is still running.

```
$ isograph --help
```

Lists start, restart, status, logs, stop, config-path, lsp. Does not list send.

```
$ isograph lsp --help
```

`--config` is `ConfigFlag`. No `--filesystem`. Nested start uses start's default (`Watch`). An already-running daemon, including `--filesystem injected`, is adopted.

```
$ isograph lsp
no isograph.config.json, isograph.config.js, or isograph.config.ts at or above /tmp; create one, or pass one using the --config flag
```

Exit 1. stdout empty. Same `DiscoverError` as config-path.

## Types

```rust
// from crates/isograph_cli/src/lib.rs
    /// Speak LSP on stdio with the daemon for this config.
    Lsp(ConfigFlag),
```

Not hidden. `ConfigFlag` is `--config`. Do not add `IsographArgs`.

```rust
// from crates/isograph_cli/src/lib.rs
        Some(CliVerb::Lsp(id)) => lsp_stdio::run(id.reference()),
```

`lib.rs`: `mod lsp_stdio;`.

`freddie_cli::client::start` is `pub(crate)` and writes "started" / "already running" on stdout. Stdout of this process is LSP. Nested `Command::new(current_exe())` `start`: stdin and stdout `Stdio::null()`, stderr inherit, cwd and `HOME` / `XDG_STATE_HOME` / `LOCALAPPDATA` inherited. Forward `--config` when the flag was set. Always invoke start. Start adopts if the lock is held.

`isograph start` returning is lock held, not listen done. `run_daemon` unlinks `{slug}.port` after the lock, then binds, then writes the file. Loop until connect succeeds or 10s.

```rust
// from crates/isograph_cli/src/lsp_stdio.rs
use std::fs;
use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, TcpStream};
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

`ReadPort` / `Connect` / `NoPort` are send's payloads and wording.

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

fn connect_to_daemon(path: &Path) -> Result<TcpStream, LspError> {
    let start = Instant::now();
    let mut last_connect = None;
    loop {
        match fs::read_to_string(path) {
            Ok(text) => {
                if let Some(port) = crate::discover::parse_port(text.reference()) {
                    match TcpStream::connect((Ipv4Addr::LOCALHOST, port)) {
                        Ok(stream) => return stream.wrap_ok(),
                        Err(source) => last_connect = Connect { port, source }.wrap_some(),
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

fn copy_stdio(stream: TcpStream) -> Result<(), LspError> {
    let mut to_daemon = stream.try_clone().map_err(LspError::Clone)?;
    let mut from_daemon = stream.try_clone().map_err(LspError::Clone)?;
    let (done_tx, done_rx) = mpsc::channel();
    let incoming_done = done_tx.clone();
    thread::spawn(move || {
        let mut stdin = io::stdin();
        let _ = copy_flush(&mut stdin, &mut to_daemon);
        let _ = incoming_done.send(());
    });
    thread::spawn(move || {
        let mut stdout = io::stdout();
        let _ = copy_flush(&mut from_daemon, &mut stdout);
        let _ = done_tx.send(());
    });
    let _ = done_rx.recv();
    std::process::exit(0);
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

Do not parse LSP. No `Message::read`. No `initialize`. Flush after every write: pipe stdout is block-buffered and `std::io::copy` can hold an initialize result. Either direction finishing `process::exit(0)` so a thread blocked on stdin dies with the process. After a successful connect, copy errors are exit 0. `run`'s `Ok` arm is for the type; the success path does not reach it.

No tokio. No `Connection`.

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

`crates/isograph_cli/Cargo.toml` is unchanged. The tests crate already has `lsp-server` / `lsp-types` / `serde_json`.

## Tests

`cli.rs`. HOME isolation as today. Do not bring up VS Code. Do not assert semantic tokens.

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

stderr inherit so a nested start cannot fill an unread stderr pipe.

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

Keep `ChildStdout` as `BufReader` until after `child.wait()`. After initialize: drop stdin, `child.wait()` success, `status` running, `STOP`, `status` not running. `Daemon`'s `Drop` still `--force`s.

- `lsp_is_in_help`: `--help` contains `lsp`, not `send`. `lsp --help` contains `config`.
- `lsp_with_the_daemon_stopped_starts_it`: fixture `{"source_files":[]}`, do not call `start` first. `spawn(["lsp"])`, initialize, drop stdin, wait 0, `status` running, `STOP`.
- `lsp_with_the_daemon_already_running_dials_it`: `Daemon::start()`, record `status` stdout, `spawn(["lsp"])`, initialize, drop stdin, wait 0, `status` stdout equals the recorded line.
- `lsp_with_no_config_exits_1`: empty cwd, `.output()`, exit 1, stderr contains `no isograph.config`, stdout empty.
- `lsp_with_an_empty_object_config_exits_1`: `{}` config. Nested start fails. Exit 1, stderr contains `source_files`.
- `lsp_with_empty_stdin_exits_0_and_leaves_the_daemon`: `spawn(["lsp"])`, drop stdin without writing, wait 0, `status` running, `STOP`.
- `lsp_two_proxies_share_one_daemon`: `Daemon::start()`, two `spawn(["lsp"])`, both initialize, both wait 0, `status` unchanged, `STOP`.
- `lsp_forwards_config_to_nested_start`: config in a sibling of cwd. `spawn(["lsp", "--config", path])`, initialize, wait 0, `status` running.

## Call sites

- VS Code `args = ['lsp']` plus optional `--config` -> nested `isograph start` -> `{slug}.port` -> copy stdin/stdout
- Zed `args: "lsp"` (zed-and-vscode-extensions.md)
- editor stdin EOF or daemon gone -> `process::exit(0)` -> TCP close -> session `Drop` -> daemon stays
