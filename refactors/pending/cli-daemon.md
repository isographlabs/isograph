# The isograph daemon and its CLI

The `isograph` binary is freddie_cli's lifecycle verbs around a daemon, and the daemon is keyed to one config: the `--config` flag's target when given, otherwise the nearest `isograph.config.json` at or above the current directory. Two paths to one config file are one daemon; two configs are two daemons, each with its own lock, log, and socket; and freddie_cli's instance lock is what makes "one daemon per config, maximum" true rather than intended.

The daemon has mercury's shape. Sources (the file watcher, the request socket, SIGTERM) send `IsographEvent`s into one channel; an event loop owns the state and dispatches each event, producing inert `IsographEffect`s; an effect loop performs them. Dispatch never does IO and the sources never touch the state, so there is one place state changes and one place the outside world is acted on. Unlike mercury there is no AppKit and no main-loop crate: the daemon is a tokio current-thread runtime and nothing else, on every platform.

Clients reach the daemon over a loopback TCP socket speaking one JSON frame per line, each response carrying the id of the request it answers. The port is OS-assigned at bind and published in a port file beside the instance's lock file, which is how the CLI, given only the config, finds the daemon for it.

The changes here are independent of the parser docs; the daemon does not read literals yet. The v1 request vocabulary is one capability, `TrackedFiles`, which is enough to make the watcher, the socket, the verbs, and the test harness real. The LSP proxy (`isograph lsp`) gets its own doc once this lands: it will translate each LSP method into the same requests defined here, so it adds vocabulary, not machinery.

## The standard: every capability is a CLI event

`IsographRequest` in `protocol.rs` is the single vocabulary for asking a running daemon anything. A capability exists when it has all three of:

- a variant in `IsographRequest`, with its response payload in `IsographResponse`;
- a CLI verb named after the variant in kebab-case (`TrackedFiles` → `isograph tracked-files`), which sends the request and prints the response payload as JSON on stdout;
- a test in `crates/isograph_cli/tests/` that drives the built binary against a fixture project and asserts facts about the payload.

The LSP proxy, when it lands, translates LSP methods into these same requests. It has only `IsographRequest` to speak, so a capability reachable through the LSP but not through a CLI verb is unrepresentable.

stdout discipline: the response payload is the only thing a request verb writes to stdout, so output is scriptable (`isograph tracked-files | jq .files`). Everything else — progress, errors, dispatch records — goes through `tracing`, which freddie_cli routes to the terminal and the log. The `print_stdout` lint stays `deny`, with one `#[expect]` at the one function that prints payloads.

Dispatch logging follows mercury: one record per dispatch, carrying the event, the effects it produced, and how long dispatch took. Source events log at `debug` (a scan of a large project is thousands of them), requests and quit at `info`. The full tracked set is never logged; it is available over the socket.

## What the user does

```
$ cd app/src/components && isograph        # walks up, finds app/isograph.config.json, starts its daemon
$ isograph status                          # running, with its pid
$ isograph tracked-files
{
  "files": [
    "/Users/x/app/src/components/Button.tsx",
    "/Users/x/app/src/entrypoint.ts"
  ]
}
$ isograph logs                            # follows the dispatch records
$ isograph stop
```

Bare `isograph` is `start`, freddie's `verb_for_bare_invocation`. Every verb resolves the config the same way, so `isograph stop` in a subdirectory stops the daemon that `isograph` in that subdirectory started. Outside any project:

```
$ cd /tmp && isograph status
error: no isograph.config.json at or above /tmp; create one, or name one with --config
```

A request verb talks to a running daemon and says so when there is none:

```
$ isograph tracked-files
ERROR request failed error=no daemon is running for /Users/x/app/isograph.config.json; `isograph start` starts one
```

## Change 1: config discovery and the per-config instance

New module `crates/isograph_cli/src/discover.rs`. The `App` impl's `Id` becomes the config flag, and `instance` keys the daemon to the canonical config path.

`crates/isograph_cli/Cargo.toml` gains one dependency:

```toml
thiserror = "2"
```

`src/discover.rs`, in full:

```rust
//! Which config an invocation means, and the daemon instance keyed to it.

use std::path::{Path, PathBuf};

use freddie_cli::Instance;

pub const CONFIG_FILE_NAME: &str = "isograph.config.json";

#[derive(Debug, thiserror::Error)]
pub enum DiscoverError {
    #[error("could not resolve the config at {}: {source}", .path.display())]
    ConfigNotReadable {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("no {CONFIG_FILE_NAME} at or above {}; create one, or name one with --config", .start.display())]
    NotFound { start: PathBuf },
    #[error("could not read the current directory: {source}")]
    NoCurrentDir { source: std::io::Error },
    #[error(transparent)]
    NoUserDir(#[from] freddie_cli::NoUserDir),
}

/// The canonical path of the config `flag` names, or of the nearest `isograph.config.json` at or
/// above the current directory. Canonical, so two paths to one file name one daemon.
pub fn config_path(flag: Option<&Path>) -> Result<PathBuf, DiscoverError> {
    let named = match flag {
        Some(path) => path.to_owned(),
        None => {
            let start = std::env::current_dir()
                .map_err(|source| DiscoverError::NoCurrentDir { source })?;
            match nearest_config(&start) {
                Some(found) => found,
                None => return Err(DiscoverError::NotFound { start }),
            }
        }
    };
    named
        .canonicalize()
        .map_err(|source| DiscoverError::ConfigNotReadable { path: named, source })
}

/// The first directory at or above `start` holding a config file, joined with the file's name.
fn nearest_config(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|dir| dir.join(CONFIG_FILE_NAME))
        .find(|candidate| candidate.is_file())
}

/// The config path and the instance keyed to it, computed together so no caller can pair a
/// config with some other config's instance.
pub fn config_and_instance(flag: Option<&Path>) -> Result<(PathBuf, Instance), DiscoverError> {
    let config = config_path(flag)?;
    let instance = Instance::named("isograph", slug(&config), config.display().to_string())?;
    Ok((config, instance))
}

/// The slug for the daemon keyed to `config`: one filename per canonical path, stable across
/// invocations and builds.
fn slug(config: &Path) -> String {
    format!("isograph-{:016x}", fnv1a(config.as_os_str().as_encoded_bytes()))
}

/// FNV-1a, 64 bits, written out so the slug does not ride on a std hasher whose output may
/// change between releases.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::{CONFIG_FILE_NAME, nearest_config, slug};
    use std::path::Path;

    #[test]
    fn the_nearest_config_is_found_from_a_subdirectory() {
        let dir = tempfile::tempdir().expect("a test can create a temp directory");
        let project = dir.path().join("project");
        let deep = project.join("src/components");
        std::fs::create_dir_all(&deep).expect("a test can create directories");
        std::fs::write(project.join(CONFIG_FILE_NAME), "{}").expect("a test can write a config");
        let found = nearest_config(&deep).expect("the config above is found");
        assert_eq!(found, project.join(CONFIG_FILE_NAME));
    }

    #[test]
    fn no_config_above_is_none() {
        let dir = tempfile::tempdir().expect("a test can create a temp directory");
        assert_eq!(nearest_config(dir.path()), None);
    }

    #[test]
    fn two_configs_get_two_slugs_and_one_config_gets_one() {
        let a = slug(Path::new("/a/isograph.config.json"));
        let b = slug(Path::new("/b/isograph.config.json"));
        assert_ne!(a, b);
        assert_eq!(a, slug(Path::new("/a/isograph.config.json")));
    }
}
```

(`tempfile` moves into `[dev-dependencies]` here rather than in Change 4, since these tests use it first.)

`src/main.rs`: `Isograph`'s id becomes the config flag, and `IsographArgs` is deleted — with `Id` no longer `NoArgs`, the daemon's empty arg set can be `NoArgs` without the group-name collision the old comment described.

Before:

```rust
/// The flags the daemon takes: none yet.
///
/// Not [`NoArgs`], because `start` flattens [`App::Id`] and [`App::DaemonArgs`] into one clap
/// command, and clap requires the two derived argument groups to have distinct names.
#[derive(clap::Args, Debug)]
pub struct IsographArgs;

/// isograph, to the verbs that manage it.
pub struct Isograph;

impl App for Isograph {
    // One isograph daemon to a machine, so no flag names which.
    type Id = NoArgs;
    type DaemonArgs = IsographArgs;

    const NAME: &'static str = "isograph";

    fn instance(_: &NoArgs) -> Result<Instance, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Instance::global(Self::NAME)?)
    }

    fn run_daemon(_: &NoArgs, _: &IsographArgs) {
        tracing::info!("hello from isograph");
        loop {
            std::thread::park();
        }
    }
}
```

After:

```rust
mod discover;

/// Which daemon a verb means: the one keyed to this config.
#[derive(clap::Args, Debug)]
pub struct ConfigFlag {
    /// Path to the isograph config. When absent, the nearest isograph.config.json at or above
    /// the current directory.
    #[arg(long)]
    pub config: Option<std::path::PathBuf>,
}

/// isograph, to the verbs that manage it.
pub struct Isograph;

impl App for Isograph {
    // One daemon per config: the id is the config the verb means.
    type Id = ConfigFlag;
    type DaemonArgs = NoArgs;

    const NAME: &'static str = "isograph";

    fn instance(id: &ConfigFlag) -> Result<Instance, Box<dyn std::error::Error + Send + Sync>> {
        let (_, instance) = discover::config_and_instance(id.config.as_deref())?;
        Ok(instance)
    }

    fn run_daemon(_: &ConfigFlag, _: &NoArgs) {
        tracing::info!("hello from isograph");
        loop {
            std::thread::park();
        }
    }
}
```

This change ships alone: the daemon still only says hello, but `isograph start` in two projects now runs two daemons, `status`/`logs`/`stop` in a subdirectory find the right one, and outside a project every verb reports the discovery error through clap the way `run_lifecycle_verb` already reports a bad id.

## Change 2: the daemon — events, effects, watcher, socket

Three new modules: `protocol.rs` (the wire vocabulary), `socket.rs` (the request socket), `daemon.rs` (state, dispatch, and the loops). `run_daemon` stops parking and becomes the daemon.

`crates/isograph_cli/Cargo.toml`, `[dependencies]` after this change:

```toml
[dependencies]
clap = { version = "4.5.18", features = ["derive"] }
freddie_cli = { git = "https://github.com/freddiehg/freddie", rev = "04d3be57eef63608ef013004a7626505a4245bbf" }
futures-util = "0.3"
notify = "8"
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tokio = { version = "1", features = ["io-util", "macros", "net", "rt", "signal", "sync"] }
tokio-util = { version = "0.7", features = ["codec"] }
tracing = "0.1.41"

[dev-dependencies]
tempfile = "3"
```

### `src/protocol.rs`, in full

```rust
//! What a client may ask a running daemon, and what it answers.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Everything a client may ask. One variant per capability; each has a CLI verb of the same
/// name in kebab-case, and the LSP proxy translates each LSP method into one of these.
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind", content = "value")]
pub enum IsographRequest {
    /// The files the daemon is tracking under the config's project_root.
    #[serde(rename = "IsographRequest.TrackedFiles")]
    TrackedFiles,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind", content = "value")]
pub enum IsographResponse {
    #[serde(rename = "IsographResponse.TrackedFiles")]
    TrackedFiles(TrackedFilesResponse),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TrackedFilesResponse {
    /// Absolute paths, sorted.
    pub files: Vec<PathBuf>,
}

/// One request on the wire: a line of JSON.
#[derive(Serialize, Deserialize, Debug)]
pub struct RequestFrame {
    pub id: u64,
    pub request: IsographRequest,
}

/// One response on the wire, carrying the id of the request it answers.
#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseFrame {
    pub id: u64,
    pub response: IsographResponse,
}
```

### `src/socket.rs`, in full

In freddie_event_socket's mold, with the reply path that crate does not have: a frame in is a request, and the daemon owes a frame out on the same connection. Plain TCP with line-delimited JSON rather than WebSocket, because every client is a local process this crate also implements; there is no browser to gate at a handshake.

```rust
//! The loopback socket a daemon answers on: one JSON frame per line, each response carrying the
//! id of the request it answers.

use std::io;
use std::net::{Ipv4Addr, SocketAddr, TcpListener as StdTcpListener};

use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::sync::watch;
use tokio_util::codec::{FramedRead, LinesCodec};
use tracing::{debug, warn};

use crate::daemon::{IncomingRequest, IsographEvent};
use crate::protocol::{IsographResponse, RequestFrame, ResponseFrame};

/// A line past this closes the connection that sent it: nothing that belongs on this socket is
/// large, and a client must not be able to make the daemon allocate without bound.
const MAX_LINE_BYTES: usize = 64 * 1024;

/// The listener. Dropping it stops accepting and ends every connection's tasks.
pub struct RequestSocket {
    _shutdown: watch::Sender<()>,
}

/// Routes one response to the connection and request it answers.
#[derive(Debug)]
pub struct ReplyHandle {
    connection: UnboundedSender<ResponseFrame>,
    id: u64,
}

impl ReplyHandle {
    /// Send the response back. A connection that has gone away drops it, which is what closing
    /// means.
    pub fn respond(self, response: IsographResponse) {
        let _ = self.connection.send(ResponseFrame {
            id: self.id,
            response,
        });
    }
}

/// Bind `127.0.0.1` on an OS-assigned port and turn each well-formed frame into an
/// [`IsographEvent::Request`]. Returns the port, for the daemon to publish in its port file.
///
/// The bind is synchronous, through std, so a failure is an `Err` from this call rather than a
/// failure inside a spawned task.
pub fn listen(event_tx: UnboundedSender<IsographEvent>) -> io::Result<(RequestSocket, u16)> {
    let std_listener = StdTcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))?;
    std_listener.set_nonblocking(true)?;
    let port = std_listener.local_addr()?.port();
    let listener = TcpListener::from_std(std_listener)?;

    let (shutdown, closed) = watch::channel(());

    tokio::spawn({
        let mut closed = closed.clone();
        async move {
            loop {
                let accepted = tokio::select! {
                    () = dropped(&mut closed) => break,
                    accepted = listener.accept() => accepted,
                };
                match accepted {
                    Ok((stream, peer)) => {
                        debug!(%peer, "accepted");
                        tokio::spawn(serve(stream, event_tx.clone(), closed.clone()));
                    }
                    // A refused connection is that client's problem; the listener keeps
                    // accepting.
                    Err(e) => debug!(error = %e, "accept failed"),
                }
            }
            debug!("the request socket closed");
        }
    });

    Ok((
        RequestSocket {
            _shutdown: shutdown,
        },
        port,
    ))
}

/// Resolves once the [`RequestSocket`] has been dropped, taking the only sender with it.
async fn dropped(closed: &mut watch::Receiver<()>) {
    while closed.changed().await.is_ok() {}
}

/// One connection: read frames into events until the peer hangs up or the socket is dropped.
/// Responses come back through the channel the connection's [`ReplyHandle`]s hold, written by
/// one task that owns the write half.
async fn serve(
    stream: TcpStream,
    event_tx: UnboundedSender<IsographEvent>,
    mut closed: watch::Receiver<()>,
) {
    let (read, write) = stream.into_split();
    let (response_tx, response_rx) = unbounded_channel::<ResponseFrame>();
    tokio::spawn(write_responses(write, response_rx));

    let mut lines = FramedRead::new(read, LinesCodec::new_with_max_length(MAX_LINE_BYTES));
    loop {
        let line = tokio::select! {
            () = dropped(&mut closed) => break,
            line = lines.next() => line,
        };
        match line {
            Some(Ok(line)) => match serde_json::from_str::<RequestFrame>(&line) {
                Ok(frame) => {
                    let reply = ReplyHandle {
                        connection: response_tx.clone(),
                        id: frame.id,
                    };
                    // A closed channel means the event loop has ended, which is the daemon
                    // leaving.
                    let _ = event_tx.send(IsographEvent::Request(IncomingRequest {
                        request: frame.request,
                        reply,
                    }));
                }
                // A client speaking nonsense is a client bug, not a reason to tear the
                // connection down.
                Err(e) => warn!(error = %e, frame = %line, "undeserializable frame"),
            },
            Some(Err(e)) => {
                debug!(error = %e, "connection ended");
                break;
            }
            None => break,
        }
    }
}

/// The write half's task: serialize each response as one line. Ends when the read loop and
/// every outstanding [`ReplyHandle`] for this connection are gone.
async fn write_responses(mut write: OwnedWriteHalf, mut responses: UnboundedReceiver<ResponseFrame>) {
    while let Some(frame) = responses.recv().await {
        let line = match serde_json::to_string(&frame) {
            Ok(line) => line,
            Err(e) => {
                warn!(error = %e, "unserializable response");
                continue;
            }
        };
        if let Err(e) = write.write_all(format!("{line}\n").as_bytes()).await {
            debug!(error = %e, "connection ended while responding");
            break;
        }
    }
}
```

### `src/daemon.rs`, in full

```rust
//! Be the isograph daemon for one config: watch its project and answer requests, event-driven.
//!
//! Sources send [`IsographEvent`]s into one channel; the event loop owns the state and
//! dispatches, producing inert [`IsographEffect`]s; the effect loop performs them. Dispatch
//! does no IO and the sources never touch the state.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::time::Instant;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tracing::{debug, error, info, warn};

use crate::discover;
use crate::protocol::{IsographRequest, IsographResponse, TrackedFilesResponse};
use crate::socket::ReplyHandle;

/// The extensions tracked under project_root.
const TRACKED_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx"];

/// Directories never looked inside: generated artifacts and dependencies.
const SKIPPED_DIRECTORIES: &[&str] = &["__isograph", "node_modules"];

/// Everything that can happen to a running daemon, one variant per source.
#[derive(Debug)]
pub enum IsographEvent {
    /// The watcher, or the initial scan, says this path's tracked-ness changed.
    Source(SourceEvent),
    /// A client asked something over the socket.
    Request(IncomingRequest),
    /// SIGTERM, which is what `isograph stop` sends.
    Quit,
}

#[derive(Debug)]
pub struct SourceEvent {
    pub path: PathBuf,
    pub change: SourceChange,
}

#[derive(Debug)]
pub enum SourceChange {
    /// The path exists with a tracked extension: track it.
    Present,
    /// The path is gone: forget it.
    Absent,
}

/// A request and the way its answer gets back to whoever asked.
#[derive(Debug)]
pub struct IncomingRequest {
    pub request: IsographRequest,
    pub reply: ReplyHandle,
}

/// What dispatch asks the effect loop to do. Inert data; performing it is the effect loop's
/// job, and it never mutates the state.
#[derive(Debug)]
pub enum IsographEffect {
    Respond(Respond),
    /// End the daemon: the effect loop breaks and destructors run.
    Kill,
}

#[derive(Debug)]
pub struct Respond {
    pub reply: ReplyHandle,
    pub response: IsographResponse,
}

/// What the daemon knows. Mutated only by dispatch, on the one runtime thread.
#[derive(Debug)]
struct IsographState {
    /// Tracked source files, absolute. A `BTreeSet`, so every answer derived from the set is
    /// sorted and deterministic.
    files: BTreeSet<PathBuf>,
}

/// The one field of the config the daemon reads today.
///
/// The full config type is `isograph_config::IsographProjectConfig`, pinned to the workspace
/// lockfile this crate is excluded from, so the daemon deserializes the field it needs and
/// ignores the rest of the file.
#[derive(serde::Deserialize, Debug)]
struct DaemonConfig {
    project_root: PathBuf,
}

#[derive(Debug, thiserror::Error)]
enum ProjectRootError {
    #[error("could not read {}: {source}", .path.display())]
    Unreadable {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not parse {}: {source}", .path.display())]
    Unparseable {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("{} has no parent directory to resolve project_root against", .path.display())]
    NoParent { path: PathBuf },
    #[error("the project root {} does not resolve: {source}", .path.display())]
    Unresolvable {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Be the daemon for the config `config_flag` names. Returns when it has stopped or when it
/// could not start; freddie_cli drops the lock after either.
pub fn run(config_flag: Option<&Path>) {
    let (config_path, instance) = match discover::config_and_instance(config_flag) {
        Ok(pair) => pair,
        // `instance` passed in this same process moments ago, so this is a race against the
        // filesystem; refusing to start is the whole response.
        Err(e) => {
            error!(error = %e, "the config went away between naming this daemon and starting it");
            return;
        }
    };
    let project_root = match project_root(&config_path) {
        Ok(root) => root,
        Err(e) => {
            error!(error = %e, "not starting");
            return;
        }
    };
    let port_file = instance.lock_file().with_extension("port");
    let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(e) => {
            error!(error = %e, "could not build the runtime");
            return;
        }
    };
    runtime.block_on(serve(project_root, port_file));
}

/// `project_root` out of the config, resolved against the config's directory, canonical.
fn project_root(config_path: &Path) -> Result<PathBuf, ProjectRootError> {
    let contents = std::fs::read_to_string(config_path).map_err(|source| {
        ProjectRootError::Unreadable {
            path: config_path.to_owned(),
            source,
        }
    })?;
    let config: DaemonConfig =
        serde_json::from_str(&contents).map_err(|source| ProjectRootError::Unparseable {
            path: config_path.to_owned(),
            source,
        })?;
    let config_dir = config_path.parent().ok_or_else(|| ProjectRootError::NoParent {
        path: config_path.to_owned(),
    })?;
    let root = config_dir.join(&config.project_root);
    match root.canonicalize() {
        Ok(root) => Ok(root),
        Err(source) => Err(ProjectRootError::Unresolvable { path: root, source }),
    }
}

/// Everything the daemon does, on the runtime thread. Returns when a `Kill` effect has been
/// performed, which is the only way out: the event loop cannot end while the watcher holds a
/// sender.
async fn serve(project_root: PathBuf, port_file: PathBuf) {
    let (event_tx, event_rx) = unbounded_channel::<IsographEvent>();
    let (effect_tx, effect_rx) = unbounded_channel::<IsographEffect>();

    // `isograph stop` sends SIGTERM. Route it into the event channel, so a stopped daemon
    // leaves through the same dispatch as everything else and destructors run.
    #[cfg(unix)]
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
        Ok(mut term) => {
            let event_tx = event_tx.clone();
            tokio::spawn(async move {
                if term.recv().await.is_some() {
                    info!("SIGTERM: quitting");
                    let _ = event_tx.send(IsographEvent::Quit);
                }
            });
        }
        Err(e) => {
            warn!(error = %e, "no SIGTERM handler; `isograph stop` will need --force");
        }
    }

    // The watcher before the scan, so a change landing mid-scan arrives as an event and the
    // set converges on the filesystem.
    let _watcher = match watch(&project_root, event_tx.clone()) {
        Ok(watcher) => watcher,
        Err(e) => {
            error!(error = %e, root = %project_root.display(), "could not watch the project root");
            return;
        }
    };
    scan(&project_root, &event_tx);

    // The socket after the scan has been queued: the seed events sit ahead of any request in
    // the one event channel, so the earliest answer a client can get is already seeded.
    let (socket, port) = match crate::socket::listen(event_tx.clone()) {
        Ok(bound) => bound,
        Err(e) => {
            error!(error = %e, "could not bind the request socket");
            return;
        }
    };
    if let Err(e) = std::fs::write(&port_file, port.to_string()) {
        error!(error = %e, path = %port_file.display(), "could not write the port file");
        return;
    }
    info!(port, root = %project_root.display(), "isograph daemon up");

    let state = IsographState {
        files: BTreeSet::new(),
    };
    tokio::select! {
        () = run_event_loop(state, event_rx, effect_tx) => {}
        () = run_effect_loop(effect_rx) => {}
    }

    drop(socket);
    // Best effort: a daemon that dies without reaching this leaves a stale file, which a
    // client's refused connect already reads as "not running".
    let _ = std::fs::remove_file(&port_file);
}

/// The event loop: read the event channel and dispatch each event.
async fn run_event_loop(
    mut state: IsographState,
    mut event_rx: UnboundedReceiver<IsographEvent>,
    effect_tx: UnboundedSender<IsographEffect>,
) {
    while let Some(event) = event_rx.recv().await {
        dispatch_event(&mut state, event, &effect_tx);
    }
}

/// Dispatch one event and enqueue whatever effects it produced. One record per dispatch;
/// source events at debug, since a scan is thousands of them.
fn dispatch_event(
    state: &mut IsographState,
    event: IsographEvent,
    effect_tx: &UnboundedSender<IsographEffect>,
) {
    let from_source = matches!(event, IsographEvent::Source(_));
    let summary = format!("{event:?}");
    let start = Instant::now();
    let effects = handle(state, event);
    let duration_us = u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX);
    if from_source {
        debug!(event = %summary, effects = ?effects, duration_us, files = state.files.len(), "dispatch");
    } else {
        info!(event = %summary, effects = ?effects, duration_us, files = state.files.len(), "dispatch");
    }
    for effect in effects {
        let _ = effect_tx.send(effect);
    }
}

fn handle(state: &mut IsographState, event: IsographEvent) -> Vec<IsographEffect> {
    match event {
        IsographEvent::Source(SourceEvent { path, change }) => {
            match change {
                SourceChange::Present => {
                    state.files.insert(path);
                }
                SourceChange::Absent => {
                    state.files.remove(&path);
                }
            }
            Vec::new()
        }
        IsographEvent::Request(IncomingRequest { request, reply }) => {
            let response = respond(state, &request);
            vec![IsographEffect::Respond(Respond { reply, response })]
        }
        IsographEvent::Quit => vec![IsographEffect::Kill],
    }
}

/// Answer one request out of the state. Reading the answer is dispatch's job; sending it is an
/// effect.
fn respond(state: &IsographState, request: &IsographRequest) -> IsographResponse {
    match request {
        IsographRequest::TrackedFiles => IsographResponse::TrackedFiles(TrackedFilesResponse {
            files: state.files.iter().cloned().collect(),
        }),
    }
}

/// The effect loop: read the effect channel and perform each effect, until one of them says to
/// stop.
async fn run_effect_loop(mut effect_rx: UnboundedReceiver<IsographEffect>) {
    while let Some(effect) = effect_rx.recv().await {
        if perform_effect(effect).is_break() {
            break;
        }
    }
}

/// Perform one effect. The effect itself is already on the dispatch record; `Kill` breaks
/// rather than exiting the process, so the way out runs destructors.
fn perform_effect(effect: IsographEffect) -> ControlFlow<()> {
    match effect {
        IsographEffect::Respond(Respond { reply, response }) => reply.respond(response),
        IsographEffect::Kill => {
            info!("kill: exiting");
            return ControlFlow::Break(());
        }
    }
    ControlFlow::Continue(())
}

/// Install the recursive watcher. The callback runs on notify's thread: it classifies and
/// sends, and the event loop does the rest.
fn watch(
    project_root: &Path,
    event_tx: UnboundedSender<IsographEvent>,
) -> notify::Result<notify::RecommendedWatcher> {
    use notify::Watcher as _;

    let mut watcher =
        notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
            let event = match result {
                Ok(event) => event,
                Err(e) => {
                    warn!(error = %e, "watch error");
                    return;
                }
            };
            for path in event.paths {
                if let Some(source) = classify(path) {
                    // A closed channel means the daemon is leaving; the watcher goes with it.
                    let _ = event_tx.send(IsographEvent::Source(source));
                }
            }
        })?;
    watcher.watch(project_root, notify::RecursiveMode::Recursive)?;
    Ok(watcher)
}

/// What one notified path means, if anything. The filesystem is the truth: the notification
/// only names a path to re-check, so create, write, rename, and remove all funnel into
/// present-or-absent.
fn classify(path: PathBuf) -> Option<SourceEvent> {
    if !is_tracked_source(&path) {
        return None;
    }
    let change = if path.is_file() {
        SourceChange::Present
    } else {
        SourceChange::Absent
    };
    Some(SourceEvent { path, change })
}

fn is_tracked_source(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(OsStr::to_str) else {
        return false;
    };
    TRACKED_EXTENSIONS.contains(&extension)
        && !path.components().any(|component| {
            SKIPPED_DIRECTORIES
                .iter()
                .any(|skipped| component.as_os_str() == OsStr::new(skipped))
        })
}

fn is_skipped_directory(path: &Path) -> bool {
    SKIPPED_DIRECTORIES
        .iter()
        .any(|skipped| path.file_name() == Some(OsStr::new(skipped)))
}

/// Seed the tracked set: every tracked source under `dir`, sent through the same channel the
/// watcher uses, so the state has one way in.
fn scan(dir: &Path, event_tx: &UnboundedSender<IsographEvent>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            warn!(error = %e, dir = %dir.display(), "unreadable directory skipped");
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        // `file_type` does not follow symlinks, so a symlinked directory is not descended into.
        if file_type.is_dir() {
            if !is_skipped_directory(&path) {
                scan(&path, event_tx);
            }
        } else if file_type.is_file() && is_tracked_source(&path) {
            let _ = event_tx.send(IsographEvent::Source(SourceEvent {
                path,
                change: SourceChange::Present,
            }));
        }
    }
}
```

`src/main.rs`: `run_daemon` hands over to the module.

Before:

```rust
    fn run_daemon(_: &ConfigFlag, _: &NoArgs) {
        tracing::info!("hello from isograph");
        loop {
            std::thread::park();
        }
    }
```

After:

```rust
    fn run_daemon(id: &ConfigFlag, _: &NoArgs) {
        daemon::run(id.config.as_deref());
    }
```

with `mod daemon; mod protocol; mod socket;` added beside `mod discover;`.

## Change 3: the request verbs

New module `client.rs`, and the verb enum grows the first request verb. This is the pattern every future capability follows.

### `src/client.rs`, in full

```rust
//! One request to a running daemon, over the port published beside its lock.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::PathBuf;

use freddie_cli::Instance;

use crate::protocol::{IsographRequest, IsographResponse, RequestFrame, ResponseFrame};

/// The id a CLI request carries. A verb opens a connection, asks once, and reads one answer,
/// so nothing distinguishes ids within the connection.
const CLI_REQUEST_ID: u64 = 1;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("no daemon is running for {daemon}; `isograph start` starts one")]
    NotRunning { daemon: String },
    #[error("the port file {} does not hold a port", .path.display())]
    BadPortFile { path: PathBuf },
    #[error("the connection failed: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },
    #[error("a wire frame did not encode or decode: {source}")]
    Json {
        #[from]
        source: serde_json::Error,
    },
    #[error("the daemon answered id {answered} to request id {asked}")]
    WrongId { asked: u64, answered: u64 },
}

/// Send one request to the daemon `instance` names and wait for its answer.
pub fn request(
    instance: &Instance,
    request: IsographRequest,
) -> Result<IsographResponse, ClientError> {
    let port_file = instance.lock_file().with_extension("port");
    let Ok(contents) = std::fs::read_to_string(&port_file) else {
        return Err(ClientError::NotRunning {
            daemon: instance.display_name().to_owned(),
        });
    };
    let Ok(port) = contents.trim().parse::<u16>() else {
        return Err(ClientError::BadPortFile { path: port_file });
    };
    let stream = match TcpStream::connect(SocketAddr::from((Ipv4Addr::LOCALHOST, port))) {
        Ok(stream) => stream,
        // A port file with nothing listening is a daemon that died without cleaning up, which
        // is the same answer as no daemon at all.
        Err(_) => {
            return Err(ClientError::NotRunning {
                daemon: instance.display_name().to_owned(),
            });
        }
    };
    let frame = serde_json::to_string(&RequestFrame {
        id: CLI_REQUEST_ID,
        request,
    })?;
    let mut writer = &stream;
    writer.write_all(frame.as_bytes())?;
    writer.write_all(b"\n")?;
    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line)?;
    let response: ResponseFrame = serde_json::from_str(&line)?;
    if response.id != CLI_REQUEST_ID {
        return Err(ClientError::WrongId {
            asked: CLI_REQUEST_ID,
            answered: response.id,
        });
    }
    Ok(response.response)
}
```

### `src/main.rs`

Before:

```rust
#[derive(Parser)]
#[command(name = "isograph", version, about = "The isograph compiler.", long_about = None)]
struct IsographCli {
    #[command(subcommand)]
    verb: Option<freddie_cli::Verb<Isograph>>,
}
```

After (with `mod client;` added, and `Subcommand` joining the clap imports):

```rust
#[derive(Parser)]
#[command(name = "isograph", version, about = "The isograph compiler.", long_about = None)]
struct IsographCli {
    #[command(subcommand)]
    verb: Option<IsographVerb>,
}

#[derive(Subcommand)]
enum IsographVerb {
    /// start, restart, status, logs, stop, and the hidden daemon.
    ///
    /// Flattened first, so `--help` lists them ahead of the request verbs.
    #[command(flatten)]
    Lifecycle(freddie_cli::Verb<Isograph>),

    /// Print the files the daemon is tracking, as JSON.
    TrackedFiles(freddie_cli::IdArgs<ConfigFlag>),
}
```

`main`, before:

```rust
    match cli.verb {
        Some(verb) => freddie_cli::run_lifecycle_verb::<Isograph>(verb, &matches),
        None => freddie_cli::run_lifecycle_verb::<Isograph>(
            freddie_cli::verb_for_bare_invocation::<Isograph>(),
            &matches,
        ),
    }
```

After:

```rust
    match cli.verb {
        Some(IsographVerb::Lifecycle(verb)) => {
            freddie_cli::run_lifecycle_verb::<Isograph>(verb, &matches)
        }
        Some(IsographVerb::TrackedFiles(args)) => {
            run_request_verb(&args.id, protocol::IsographRequest::TrackedFiles)
        }
        None => freddie_cli::run_lifecycle_verb::<Isograph>(
            freddie_cli::verb_for_bare_invocation::<Isograph>(),
            &matches,
        ),
    }
```

The two new functions at the bottom of `main.rs`:

```rust
/// Name the daemon, send one request, print the response payload as JSON on stdout.
fn run_request_verb(id: &ConfigFlag, request: protocol::IsographRequest) -> ExitCode {
    let instance = match Isograph::instance(id) {
        Ok(instance) => instance,
        // The same reporting path `run_lifecycle_verb` uses for an id that names no daemon.
        Err(e) => {
            clap::Error::raw(clap::error::ErrorKind::ValueValidation, format!("{e}\n")).exit()
        }
    };
    freddie_cli::init_client_logging(&instance);
    match client::request(&instance, request) {
        Ok(response) => print_response(&response),
        Err(e) => {
            tracing::error!(error = %e, "request failed");
            ExitCode::FAILURE
        }
    }
}

/// The response payload is a request verb's output, so the one print in the crate is here.
fn print_response(response: &protocol::IsographResponse) -> ExitCode {
    let payload = match response {
        protocol::IsographResponse::TrackedFiles(payload) => serde_json::to_value(payload),
    };
    match payload {
        Ok(payload) => {
            #[expect(clippy::print_stdout)]
            {
                println!("{payload:#}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            tracing::error!(error = %e, "unprintable response");
            ExitCode::FAILURE
        }
    }
}
```

## Change 4: the CLI tests

The CLI tests live in `crates/isograph_cli/tests/`, not in `crates/tests`: the crate is excluded from the workspace, and `CARGO_BIN_EXE_isograph` only exists for tests compiled in the binary's own package. Fixture projects live under `crates/isograph_cli/fixtures/`; each test copies one into a temp directory and points `HOME` (and its platform siblings) at a private directory, so every daemon's lock, log, and port file are isolated from the developer's own and from other tests.

### The fixture

`fixtures/hello/isograph.config.json`:

```json
{
  "project_root": "./src",
  "schema": "./schema.graphql"
}
```

`fixtures/hello/schema.graphql`:

```graphql
type Query {
  hello: String
}
```

`fixtures/hello/src/a.ts`:

```ts
export const a = 1;
```

`fixtures/hello/src/nested/b.tsx`:

```tsx
export const b = 2;
```

`fixtures/hello/src/__isograph/generated.ts` (must never be tracked):

```ts
export const generated = true;
```

`fixtures/hello/src/notes.md` (wrong extension, must never be tracked):

```md
not a source file
```

### `tests/cli.rs`, in full

```rust
//! Drive the built `isograph` binary against fixture projects. Every daemon's lock, log, and
//! port live under a per-fixture private HOME, so tests touch nothing of the developer's and
//! nothing of each other's.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

/// How long a fact that becomes true asynchronously (a spawned daemon, a watcher delivery)
/// gets before its test fails.
const DEADLINE: Duration = Duration::from_secs(10);

/// One started daemon against one private copy of a fixture.
struct DaemonFixture {
    /// Holds the copied project and the private HOME; removed when the test ends.
    dir: tempfile::TempDir,
}

impl DaemonFixture {
    /// Copy `fixture` out of `fixtures/`, and `isograph start` in the copy.
    fn start(fixture: &str) -> DaemonFixture {
        let dir = tempfile::tempdir().expect("a test can create a temp directory");
        let started = DaemonFixture { dir };
        copy_tree(&fixture_source(fixture), &started.project());
        let output = started.isograph(&["start"]);
        assert!(
            output.status.success(),
            "start failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        started
    }

    fn project(&self) -> PathBuf {
        self.dir.path().join("project")
    }

    /// A second project under the same private HOME, so its daemon and this one's run side by
    /// side in one state directory. The caller stops it.
    fn add_project(&self, fixture: &str, name: &str) -> PathBuf {
        let target = self.dir.path().join(name);
        copy_tree(&fixture_source(fixture), &target);
        target
    }

    fn isograph(&self, args: &[&str]) -> Output {
        self.isograph_in(&self.project(), args)
    }

    /// Run the built binary in `dir`, with every per-user path under this fixture's private
    /// HOME.
    fn isograph_in(&self, dir: &Path, args: &[&str]) -> Output {
        let home = self.dir.path().join("home");
        std::fs::create_dir_all(&home).expect("a test can create its private HOME");
        Command::new(env!("CARGO_BIN_EXE_isograph"))
            .args(args)
            .current_dir(dir)
            .env("HOME", &home)
            .env("XDG_STATE_HOME", home.join("state"))
            .env("LOCALAPPDATA", home.join("appdata"))
            .output()
            .expect("the isograph binary runs")
    }

    fn tracked_files(&self) -> Vec<PathBuf> {
        self.tracked_files_in(&self.project())
    }

    /// The daemon's tracked files, once it answers: `start` returns before the daemon has its
    /// socket up, so this retries until the deadline.
    fn tracked_files_in(&self, dir: &Path) -> Vec<PathBuf> {
        poll(|| {
            let output = self.isograph_in(dir, &["tracked-files"]);
            if !output.status.success() {
                return None;
            }
            let payload: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
            Some(
                payload["files"]
                    .as_array()?
                    .iter()
                    .filter_map(|file| file.as_str().map(PathBuf::from))
                    .collect(),
            )
        })
    }
}

impl Drop for DaemonFixture {
    /// Stop the daemon before the directory under it is removed, so no test leaks a daemon
    /// watching a directory that is gone. `--force`, because a wedged daemon is exactly what
    /// this must clean up after.
    fn drop(&mut self) {
        let _ = self.isograph(&["stop", "--force"]);
    }
}

fn fixture_source(fixture: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(fixture)
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("a test can create directories");
    for entry in std::fs::read_dir(from)
        .expect("the fixture directory exists")
        .flatten()
    {
        let target = to.join(entry.file_name());
        let file_type = entry.file_type().expect("the fixture's entries are readable");
        if file_type.is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("the fixture's files copy");
        }
    }
}

/// Retry `f` every 50ms until it answers, panicking at the deadline. The facts these tests
/// assert become true asynchronously, and the deadline is what bounds them.
fn poll<T>(mut f: impl FnMut() -> Option<T>) -> T {
    let start = Instant::now();
    loop {
        if let Some(value) = f() {
            return value;
        }
        assert!(start.elapsed() < DEADLINE, "deadline passed");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn contains_suffix(files: &[PathBuf], suffix: &str) -> bool {
    files.iter().any(|file| file.ends_with(suffix))
}

#[test]
fn the_project_files_are_tracked() {
    let daemon = DaemonFixture::start("hello");
    let files = daemon.tracked_files();
    assert!(contains_suffix(&files, "src/a.ts"), "{files:?}");
    assert!(contains_suffix(&files, "src/nested/b.tsx"), "{files:?}");
}

#[test]
fn generated_and_non_source_files_are_not_tracked() {
    let daemon = DaemonFixture::start("hello");
    let files = daemon.tracked_files();
    assert!(!contains_suffix(&files, "generated.ts"), "{files:?}");
    assert!(!contains_suffix(&files, "notes.md"), "{files:?}");
}

#[test]
fn a_created_file_becomes_tracked() {
    let daemon = DaemonFixture::start("hello");
    daemon.tracked_files(); // the daemon is up
    std::fs::write(daemon.project().join("src/c.ts"), "export const c = 3;\n")
        .expect("a test can write into its copy");
    poll(|| contains_suffix(&daemon.tracked_files(), "src/c.ts").then_some(()));
}

#[test]
fn a_removed_file_is_forgotten() {
    let daemon = DaemonFixture::start("hello");
    poll(|| contains_suffix(&daemon.tracked_files(), "src/a.ts").then_some(()));
    std::fs::remove_file(daemon.project().join("src/a.ts"))
        .expect("a test can edit its copy");
    poll(|| (!contains_suffix(&daemon.tracked_files(), "src/a.ts")).then_some(()));
}

#[test]
fn stop_ends_the_daemon() {
    let daemon = DaemonFixture::start("hello");
    daemon.tracked_files(); // the daemon is up
    let stopped = daemon.isograph(&["stop"]);
    assert!(stopped.status.success());
    poll(|| (!daemon.isograph(&["tracked-files"]).status.success()).then_some(()));
}

#[test]
fn two_configs_are_two_daemons() {
    let daemon = DaemonFixture::start("hello");
    let second = daemon.add_project("hello", "second");
    let started = daemon.isograph_in(&second, &["start"]);
    assert!(started.status.success());
    std::fs::write(second.join("src/only_second.ts"), "export const s = 1;\n")
        .expect("a test can write into its copy");
    poll(|| contains_suffix(&daemon.tracked_files_in(&second), "only_second.ts").then_some(()));
    assert!(!contains_suffix(&daemon.tracked_files(), "only_second.ts"));
    let _ = daemon.isograph_in(&second, &["stop", "--force"]);
}

#[test]
fn a_second_start_for_one_config_is_the_running_daemon() {
    // The one-daemon-per-config guarantee is freddie_cli's lock; this exercises the wiring:
    // a second start is not an error, and the daemon still answers after it.
    let daemon = DaemonFixture::start("hello");
    daemon.tracked_files(); // the daemon is up
    let again = daemon.isograph(&["start"]);
    assert!(again.status.success());
    assert!(contains_suffix(&daemon.tracked_files(), "src/a.ts"));
}
```
