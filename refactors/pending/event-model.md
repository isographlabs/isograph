# Event model

Requires config-discovery.md. pluggable-compiler.md names this file as the place the daemon reads the world and performs effects.

One daemon per config. Sources post events onto one channel. `handle` mutates in-memory state and returns effects. The event loop is the only caller of `handle`. Performers are the only code that does IO, except the sources themselves.

pico intern and memo sit inside `handle` once compilation exists. This section's state is a path-to-contents map. The event type does not change when pico replaces the map.

## What the user does

```
$ cd app && isograph start --filesystem watch
/Users/x/app/isograph.config.json started (pid 12345)
$ isograph send
{"kind":"IncomingEvent.DiskChanged","value":{"path":"/Users/x/app/src/Pet.tsx","presence":{"Present":{"contents":"export const x = iso(`field Pet.id { id }`);\n"}}}}
$ isograph logs
{"timestamp":"...","level":"INFO","fields":{"message":"isograph daemon up","config":"/Users/x/app/isograph.config.json","port":53124,"filesystem":"watch"}}
{"timestamp":"...","level":"INFO","fields":{"message":"disk changed","path":"/Users/x/app/src/Pet.tsx","presence":"present","file_count":1}}
```

A VS Code or Zed window on that project starts `isograph lsp`. That process is a stdio proxy: it starts the daemon if needed and copies LSP JSON-RPC between the editor and the daemon's multiplexer. Several editors share one daemon.

CI starts the daemon with `--filesystem injected`, sends the same `IncomingEvent` frames the CLI sends, and asserts on logs and on `handle` results. The daemon does not read the fixture paths off disk.

## Process layout

Per config, one process. Three sources post to one `tokio::sync::mpsc::unbounded_channel::<IsographEvent>()`:

- The file system watcher. OS notifications in, `DiskChanged` out. No CLI, no socket. notify's callback sends on the channel and returns.
- The event socket (`freddie_event_socket`). JSON frames in, `IncomingEvent` deserialized, the matching `IsographEvent` sent on the same channel. The CLI is a client of this socket. CI is a client of this socket.
- The LSP multiplexer. One TCP listener. Each accepted connection is a `ConnectionId`. An LSP message becomes `IsographEvent::Lsp`. A `Disconnected` event is posted when the connection ends.

The worker (the tokio runtime in `run_daemon`) owns `IsographState`, receives, calls `handle`, performs the returned effects. No `Mutex`. Sources do not read state.

`isograph lsp` is not a fourth copy of the compiler. It is a stdio proxy onto the multiplexer. `isograph send` is not a fourth copy either. It is a client of the event socket.

The three sources are theoretically separate daemons. They are one process because they share `IsographState` and because a socket hop on every save is the wrong latency and the wrong failure mode.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
use std::path::PathBuf;

use lsp_server::Message as LspMessage;

pub enum IsographEvent {
    DiskChanged(DiskChanged),
    Lsp(LspClientEvent),
}

#[derive(serde::Deserialize, Debug)]
pub struct DiskChanged {
    pub path: PathBuf,
    pub presence: PathPresence,
}

#[derive(serde::Deserialize, Debug)]
pub enum PathPresence {
    Present(Present),
    Absent,
}

#[derive(serde::Deserialize, Debug)]
pub struct Present {
    pub contents: String,
}

pub enum LspClientEvent {
    Message(LspClientMessage),
    Disconnected(ConnectionId),
}

pub struct LspClientMessage {
    pub connection: ConnectionId,
    pub message: LspMessage,
}

pub struct ConnectionId(pub u64);
```

`path` is absolute and canonical. A relative path is resolved by the source that constructed the event, never by `handle`.

`Present` is create or modify or the destination of a move. `Absent` is delete or the source of a move. There is no `Moved` variant. A rename the watcher sees becomes `Absent` then `Present`. pico sources are keyed by path; a rename is remove old plus intern new. A dedicated `Moved` would exist only if pico grew a rename API.

`handle` does not know about globs, gitignore, or "in scope". Scope is the watcher's job. The socket and the CLI may inject any path. That is how CI names fixtures that would not match `includes`.

```rust
// from crates/isograph_cli/src/effect.rs
use std::path::PathBuf;

use lsp_server::Message as LspMessage;

use crate::event::ConnectionId;

pub enum IsographEffect {
    Lsp(LspEffect),
    WriteFile(WriteFile),
}

pub enum LspEffect {
    Send(LspSend),
    Close(ConnectionId),
}

pub struct LspSend {
    pub connection: ConnectionId,
    pub message: LspMessage,
}

pub struct WriteFile {
    pub path: PathBuf,
    pub contents: String,
}
```

`WriteFile` is an artifact write, the outgoing counterpart of `common_lang_types::FileSystemOperation`. Incoming disk facts are `DiskChanged`. The two directions do not share a type.

The first shipped slice (filesystem-events.md) has no effect variants. `handle` returns `()`. The loop traces. The signature becomes `Vec<IsographEffect>` when the first performer exists.

```rust
// from crates/isograph_cli/src/state.rs
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::event::IsographEvent;

pub struct IsographState {
    pub files: BTreeMap<PathBuf, String>,
}

impl IsographState {
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
        }
    }

    pub fn handle(&mut self, event: &IsographEvent) -> Vec<IsographEffect> {
        match event {
            IsographEvent::DiskChanged(change) => self.handle_disk_changed(change),
            IsographEvent::Lsp(lsp) => self.handle_lsp(lsp),
        }
    }
}
```

`files` is the stand-in for pico sources. pico intern of `FileContent` replaces the map; `DiskChanged` stays.

LSP overlays (unsaved buffer text on top of disk text) refine the value type, not the event type. They land with the multiplexer, not with disk events.

```rust
// from crates/isograph_cli/src/external.rs
#[derive(serde::Deserialize, Debug)]
#[serde(tag = "kind", content = "value")]
pub enum IncomingEvent {
    #[serde(rename = "IncomingEvent.DiskChanged")]
    DiskChanged(DiskChanged),
}
```

`IsographEvent` does not derive `Deserialize`. A wire client cannot construct `Lsp` or any future internal variant. Same split as figaro's `IncomingEvent` / `FigaroEvent`.

A frame that is not a valid `IncomingEvent` is logged and dropped. The connection stays up.

```rust
// from crates/isograph_cli/src/external.rs
use tokio::sync::mpsc::UnboundedSender;
use tracing::warn;

use crate::event::IsographEvent;

pub fn on_message(text: &str, event_tx: &UnboundedSender<IsographEvent>) {
    match serde_json::from_str::<IncomingEvent>(text) {
        Ok(IncomingEvent::DiskChanged(change)) => {
            let _ = event_tx.send(IsographEvent::DiskChanged(change));
        }
        Err(e) => warn!(error = %e, frame = text, "undeserializable frame"),
    }
}
```

## Ports

Figaro is one process per machine, so a default port is enough. Isograph is one process per config. Two configs cannot share a port.

`run_daemon` binds `127.0.0.1:port` for the event socket. `port` is `--port` when given, otherwise `0` (the OS assigns). The bound port is written to `{log_dir}/{slug}.port` as decimal digits and a newline, and is logged on `isograph daemon up`. `isograph send` reads that file unless `--port` is given.

The multiplexer gets the same treatment later: `{log_dir}/{slug}.lsp`, a second listener. The event socket is JSON frames. The multiplexer is LSP JSON-RPC. They are not the same protocol and not the same port.

`freddie_event_socket` refuses web-page `Origin` headers and caps a frame at 64 KiB. File contents on the wire therefore cannot exceed that. Production contents never go over the socket: the watcher reads the file and posts in-process. CI fixtures stay small. A fixture that exceeds 64 KiB cannot be injected until the cap changes in freddie. That change is not this work.

## `isograph lsp`

```rust
// from crates/isograph_cli/src/lsp_proxy.rs
use std::process::ExitCode;

use crate::discover::ConfigFlag;

pub fn run_lsp_proxy(id: &ConfigFlag) -> ExitCode
```

Walk-up / `--config` is the same as every other verb. `ensure_started` (freddie_cli's start) starts the daemon if it is not running. The proxy reads `{slug}.lsp`, dials it, and copies stdin/stdout to the connection. Dropping the editor drops the proxy, which closes that multiplexer connection, which posts `LspClientEvent::Disconnected`. The daemon stays up.

The vscode-extension already spawns `isograph lsp` on stdio (`vscode-extension/src/languageClient.ts`). The proxy is what that spawn becomes. Until the multiplexer exists, `isograph lsp` is not implemented; the extension's spawn fails. lsp-semantic-tokens.md's token encoding and `file_literals` are the computation the multiplexer will call. Its standalone stdio server loop is not the process model.

## Filesystem flag

```rust
// from crates/isograph_cli/src/lib.rs
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum Filesystem {
    Watch,
    Injected,
}
```

`Watch` starts the watcher source, which scans then observes. `Injected` does not scan and does not watch. The event socket listens in both modes. This is an enum, not a `--noop` bool: the two cases are how filesystem facts arrive.

## Constraints

Highlighting an open `iso(\`...\`)` needs the open buffer, `file_literals`, and LSP semantic tokens. It does not need the watcher, the globs, or pico. The first shipped slice is still the event loop, because the extension's long-term client is the multiplexer, and the multiplexer is a source on this loop.

Zed's default `semantic_tokens` is `off`. An extension cannot add tree-sitter injections to Zed's built-in TypeScript grammar. Iso-literal coloring in Zed is LSP semantic tokens, and the user has to set `semantic_tokens` to `combined` or `full` for TypeScript and TSX. vscode-languageclient requests semantic tokens by default. That gap is zed-and-vscode-extensions.md.

## Sequence

1. filesystem-events.md. Event loop, `DiskChanged`, socket, `isograph send`, config `includes`, watcher. End state: injected CI and a watched project both show the path-to-contents map in the log.
2. LSP multiplexer (later doc). `IsographEvent::Lsp`, effects, `{slug}.lsp`, `isograph lsp` proxy. Token encoding from lsp-semantic-token-encoding.md. `file_literals` from lsp-semantic-tokens.md.
3. zed-and-vscode-extensions.md. Zed extension that spawns `isograph lsp`. VS Code extension already does; it keeps working when the proxy exists.

pico intern of disk files can land anywhere after (1), as a replacement of `IsographState::files`.
