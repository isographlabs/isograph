# `isograph lsp`

Requires start-returns-when-listening.md (landed). Independent of lsp-tokens.md, lsp-sessions.md, lsp-diagnostics.md.

VS Code and Zed spawn a process on stdio. They do not dial `{slug}.port`. `isograph lsp` is that process: discover, `isograph start`, one `connect_to_daemon`, copy bytes, die. Do not parse LSP. Do not become the daemon. Dropping the editor drops the proxy. The daemon stays up.

Origin: `docs-website/docs/design-docs/event-model.md`. Origin of spawn args: `vscode-extension/src/languageClient.ts`. Origin of start: nested `isograph start` (stdout isolation). Origin of connect: `send.rs` `read_port` + `TcpStream::connect`. Origin of the copy: `std::io::copy`. Delta: the binary has the verb; start already waited until the port accepts, so connect is once; send and lsp share `connect_to_daemon`.

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

`--config` is `ConfigFlag`. No `--filesystem`. Nested start uses start's default (`Watch`). An already-running daemon, including `--filesystem injected`, is adopted. `--stdio` is hidden and ignored (`vscode-languageclient` appends it for `TransportKind.stdio`).

```
$ isograph lsp
no isograph.config.json, isograph.config.js, or isograph.config.ts at or above /tmp; create one, or pass one using the --config flag
```

Exit 1. stdout empty. Same `DiscoverError` as config-path.

## Types

```rust
// from crates/isograph_cli/src/lib.rs
    /// Speak LSP on stdio with the daemon for this config.
    Lsp(LspArgs),
```

```rust
// from crates/isograph_cli/src/lib.rs
#[derive(clap::Args, Debug)]
struct LspArgs {
    #[command(flatten)]
    pub id: ConfigFlag,

    /// Ignored. vscode-languageclient appends this when the transport is stdio.
    #[arg(long, hide = true)]
    pub stdio: bool,
}
```

`stdio` is a clap presence flag. Both values are ignored. Not hidden on `Lsp` itself. Do not add `IsographArgs`.

```rust
// from crates/isograph_cli/src/lib.rs
        Some(CliVerb::Lsp(args)) => lsp_stdio::run(args.reference()),
```

`lib.rs`: `mod lsp_stdio;`.

Always invoke nested `isograph start`. Start adopts if the lock is held. Checking the lock first is a TOCTOU. Nested `Command::new(current_exe())`: stdin and stdout `Stdio::null()` so `{config} started (pid …)` is not LSP, stderr inherit, cwd and `HOME` / `XDG_STATE_HOME` / `LOCALAPPDATA` inherited. Forward `--config` when the flag was set. After start-returns-when-listening.md, that process does not return until the port accepts.

### Shared connect

Move `ReadPort`, `Connect`, `read_port` from `send.rs` next to `parse_port` (already in `discover.rs` after start-returns-when-listening.md). One helper, used by send and lsp.

```rust
// from crates/isograph_cli/src/discover.rs
#[derive(Debug)]
pub(crate) struct ReadPort {
    pub path: PathBuf,
    pub source: io::Error,
}

#[derive(Debug)]
pub(crate) struct Connect {
    pub port: u16,
    pub source: io::Error,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum PortError {
    #[error("the daemon has not recorded its port yet")]
    NoPort,
    #[error("the daemon's port file is not a port")]
    BadPort,
    #[error("could not read {}: {}", .0.path.display(), .0.source)]
    ReadPort(ReadPort),
    #[error("could not connect to 127.0.0.1:{}: {}", .0.port, .0.source)]
    Connect(Connect),
}

pub(crate) fn connect_to_daemon(path: &Path) -> Result<TcpStream, PortError> {
    let port = read_port(path)?;
    TcpStream::connect((Ipv4Addr::LOCALHOST, port)).map_err(|source| {
        PortError::Connect(Connect { port, source })
    })
}

pub(crate) fn read_port(path: &Path) -> Result<u16, PortError> {
    match fs::read_to_string(path) {
        Ok(text) => parse_port(text.reference()).ok_or(PortError::BadPort),
        Err(source) if source.kind() == io::ErrorKind::NotFound => PortError::NoPort.wrap_err(),
        Err(source) => PortError::ReadPort(ReadPort {
            path: path.to_owned(),
            source,
        })
        .wrap_err(),
    }
}
```

`read_port` body is send's, error type `PortError` instead of `SendError`.

```rust
// from crates/isograph_cli/src/send.rs
    #[error("{0}")]
    #[from]
    Port(crate::discover::PortError),
```

Drop `SendError::NoPort` / `ReadPort` / `BadPort` / `Connect` and the `ReadPort` / `Connect` structs from `send.rs`. `run_inner` after `require_running`:

```rust
// from crates/isograph_cli/src/send.rs
    let stream = crate::discover::connect_to_daemon(&crate::discover::port_file(
        instance.lock_file(),
    ))?;
```

`read_port_of_a_missing_file_is_no_port` matches `SendError::Port(PortError::NoPort)`. Display strings are unchanged.

### Proxy

```rust
// from crates/isograph_cli/src/lsp_stdio.rs
use std::io::{self, Write, copy};
use std::net::TcpStream;
use std::process::{Command, ExitCode, ExitStatus, Stdio};
use std::thread;

use prelude::Postfix;

use crate::LspArgs;
use crate::discover::DiscoverError;

#[derive(Debug, thiserror::Error)]
enum LspError {
    #[error("{0}")]
    Discover(#[from] DiscoverError),
    #[error("could not find this executable: {0}")]
    CurrentExe(io::Error),
    #[error("could not spawn isograph start: {0}")]
    Spawn(io::Error),
    #[error("{0}")]
    #[from]
    Port(crate::discover::PortError),
    #[error("could not clone the stream: {0}")]
    Clone(io::Error),
}

struct FlushStdout(io::Stdout);

impl Write for FlushStdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.0.write(buf)?;
        self.0.flush()?;
        n.wrap_ok()
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

#[expect(clippy::print_stderr)]
pub fn run(args: &LspArgs) -> ExitCode {
    match start_daemon(args) {
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
        Ok(status) if !status.success() => exit_from_status(status),
        Ok(_) => match connect_pair(args) {
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
            Ok((to_daemon, from_daemon)) => copy_stdio(to_daemon, from_daemon),
        },
    }
}

fn start_daemon(args: &LspArgs) -> Result<ExitStatus, LspError> {
    let _ = crate::discover::instance_for_config_path(args.id.config.as_deref())?;
    let exe = std::env::current_exe().map_err(LspError::CurrentExe)?;
    let mut command = Command::new(exe);
    command
        .arg("start")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    if let Some(config) = args.id.config.as_deref() {
        command.arg("--config").arg(config);
    }
    command.status().map_err(LspError::Spawn)
}

fn connect_pair(args: &LspArgs) -> Result<(TcpStream, TcpStream), LspError> {
    let (_, instance) = crate::discover::instance_for_config_path(args.id.config.as_deref())?;
    let stream =
        crate::discover::connect_to_daemon(&crate::discover::port_file(instance.lock_file()))?;
    let to_daemon = stream.try_clone().map_err(LspError::Clone)?;
    (to_daemon, stream).wrap_ok()
}

fn exit_from_status(status: ExitStatus) -> ExitCode {
    match status.code() {
        Some(code) => u8::try_from(code)
            .map(ExitCode::from)
            .unwrap_or(ExitCode::FAILURE),
        None => ExitCode::FAILURE,
    }
}

fn copy_stdio(mut to_daemon: TcpStream, mut from_daemon: TcpStream) -> ! {
    thread::spawn(move || {
        let _ = copy(&mut io::stdin(), &mut to_daemon);
        std::process::exit(0);
    });
    let _ = copy(&mut from_daemon, &mut FlushStdout(io::stdout()));
    std::process::exit(0);
}
```

`FlushStdout` is the copy to the editor. `io::stdout()` on a pipe holds bytes until 8KiB or drop; compact JSON has no newline, so `LineWriter` would also hold. Whichever `copy` finishes first `process::exit(0)`: stdin EOF or the daemon closing. A thread blocked on the other side dies with the process. One `try_clone`. The original stream is `from_daemon`. `copy_stdio` is `!`. Nested start failure: child stderr already has the message; this process returns that exit code.

Missing config fails in `start_daemon` before spawn. `connect_pair` discovers again for the port path.

No tokio. No `Connection`. No `Message::read`. No `notify`. `crates/isograph_cli/Cargo.toml` is unchanged.

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

Keep `ChildStdout` as `BufReader` until after `child.wait()`. Dropping the pipe early is SIGPIPE on the next daemon write. After initialize: drop stdin, `child.wait()` success, `status` running, `STOP`, `status` not running. `write_initialize` does not send `initialized`. That is enough to prove the copy and that `started` is not on stdout.

- `lsp_is_in_help`: `--help` contains `lsp`. `lsp --help` contains `config`, does not contain `stdio`.
- `lsp_with_the_daemon_stopped_starts_it`: fixture `{"source_files":[]}`, do not call `start` first. `spawn(["lsp"])`, initialize, drop stdin, wait 0, `status` running, `STOP`.
- `lsp_with_the_daemon_already_running_dials_it`: `Daemon::start()` (injected). Record `status` stdout. `spawn(["lsp"])`, initialize, drop stdin, wait 0. `status` stdout equals the recorded line. Nested start without `--filesystem` is Watch; this is adopt, not replace.
- `lsp_with_no_config_exits_1`: empty cwd, `.output()`, exit 1, stderr contains `no isograph.config`, stdout empty.
- `lsp_with_an_empty_object_config_exits_1`: `{}` config. Nested start fails. Exit 1, stderr contains `source_files`, does not contain `could not start the daemon`.
- `lsp_with_empty_stdin_exits_0_and_leaves_the_daemon`: `spawn(["lsp"])`, drop stdin without writing, wait 0, `status` running, `STOP`.
- `lsp_two_proxies_share_one_daemon`: `Daemon::start()`, two `spawn(["lsp"])`, both initialize, both wait 0, `status` unchanged, `STOP`.
- `lsp_forwards_config_to_nested_start`: config in a sibling of cwd. `spawn(["lsp", "--config", path])`, initialize, wait 0, `status` running.
- `lsp_stdio_flag_is_ignored`: `Daemon::start()`, `spawn(["lsp", "--stdio"])`, initialize, drop stdin, wait 0.

The tests crate already has `lsp-server` / `lsp-types` / `serde_json`.

## Call sites

- VS Code `args = ['lsp']` plus optional `--config` -> nested `isograph start` -> `connect_to_daemon` -> `copy` both ways
- Zed `args: "lsp"` (zed-and-vscode-extensions.md)
- editor stdin EOF or daemon gone -> `process::exit(0)` -> session `Drop` -> daemon stays
- `isograph send` -> `connect_to_daemon` after `require_running`
