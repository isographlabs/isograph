# Event model

Two layers, one `handle`.

Inner: `handle(state, event) -> Vec<IsographEffect>`. No filesystem, no LSP, no socket. The test harness calls this function. The running binary calls this same function. There is not a second copy for tests.

Outer: the real process. It listens to the filesystem, speaks LSP, accepts CLI frames, and performs effects. Every path into the process becomes an ingested event, then `handle`, then effects.

The process does not read the filesystem inside `handle`. Facts about files arrive as events: a path is present with these contents, or a path is absent. Writing files is an effect.

This is figaro's shape. Figaro's events are keys and OS reports. Isograph's events are file changes, editor buffers, and completed work.

The building blocks are `Config`, `IsographEvent`, `IsographEffect`, `DiskFile`, and `OpenFile`. The state is a pico database.

A binary is compiled with one `HostLanguage`, one `NetworkProtocol`, and one `RuntimeFramework`. Those are the type parameters in the mental model. They are static for the process. The config file's fields depend on them.

Landing order for the implementation slices is `refactors/pending/event-model.md`.

## Config

There is one isograph process per config file.

```rust
struct Config {
    path: PathBuf,
}
```

Figaro has one process on the machine (`Instance::global`). Isograph has one process per `Config` (`Instance::named`). `freddie_cli` is the same crate in both.

The process is found by `--config`, or by the nearest `isograph.config.json`, `isograph.config.js`, or `isograph.config.ts` at or above the current directory. At one directory, that order: json, then js, then ts. Two paths to one file are one process.

A `.json` config is data. A `.js` or `.ts` config is a module that exports the config object (`export default` or `module.exports`). Loading it runs the file with an executor and reads JSON from stdout.

Watch mode and the LSP adapter are that process. They share the pico database.

```rust
enum Filesystem {
    Watch,
    Injected,
}
```

`Watch` starts a source that observes the OS and emits `DiskChanged`. `Injected` does not scan and does not watch. The event socket listens in both modes. The CLI, the LSP adapter, and the socket all submit ingested events into the same `handle`.

## Inner

```rust
fn handle(state: &mut Database, event: IsographEvent) -> Vec<IsographEffect>
```

The test harness calls this. It does not start a daemon, open a socket, or write a file. A test constructs a `DiskChanged` or `EditorChanged`, runs `handle`, and asserts the effects. The binary's event loop calls the same `handle` with the same types.

`handle` does not know about globs, gitignore, or "in scope". Scope is the watcher's job. The socket and the CLI may inject any path.

## Outer

The binary is the outer. Each source is outside `handle` and feeds it. One process, one channel, one worker that owns `Database`. Sources do not read state. Performers do not mutate it.

- Watcher: OS notifications become `DiskChanged` (path plus contents or absent). It may read the disk to fill `Present.contents`. `handle` does not. The watcher posts in-process on the event channel. It does not run the CLI and it does not write to the event socket.
- Event socket (`freddie_event_socket`): JSON `IncomingEvent` frames. The CLI is a client of this socket. CI is a client of this socket.
- LSP adapter: an LSP notification (`textDocument/didOpen`, `didChange`, `didClose`) becomes `EditorChanged`. An LSP request (hover, `semanticTokens/full`, …) is request/response in the adapter: it reads `OpenFile` if present else `DiskFile`, computes, replies. `handle` is not request/response. Effects from `handle` (`ReportDiagnostics`) become LSP notifications (`publishDiagnostics`).
- Effect loop: performs `WriteArtifacts`, `ReportDiagnostics`, `StartAsyncWork`, `Kill`.

`isograph lsp` is a stdio proxy onto the adapter. Walk-up / `--config` is the same as every other verb. It starts the daemon if needed, dials the adapter, and copies stdin/stdout. Dropping the editor drops the proxy and that connection. The daemon stays up. Several editors share one process. The vscode-extension already spawns `isograph lsp` on stdio.

`isograph send` is a client of the event socket. It does not start the daemon.

The three sources are theoretically separate daemons. They are one process because they share `Database` and because a socket hop on every save is the wrong latency.

## Ports

Figaro is one process per machine, so a default port is enough. Isograph is one process per config. Two configs cannot share a port.

The event socket binds `127.0.0.1:port`. `port` is `--port` when given, otherwise `0` (the OS assigns). The bound port is `EventSocket::local_addr().port()` (freddie `event-socket-local-addr.md`). It is written to `{log_dir}/{slug}.port` as decimal digits and a newline. `isograph send` reads that file unless `--port` is given.

The LSP adapter is a second listener, `{log_dir}/{slug}.lsp`. The event socket is JSON frames. The adapter is LSP JSON-RPC. They are not the same protocol and not the same port.

`freddie_event_socket` refuses web-page `Origin` headers and caps a frame at 64 KiB. Production file contents never go over the socket: the watcher reads the file and posts in-process. CI fixtures stay small.

## Event

An event is something that happened, already carrying what the source knows.

```rust
enum IsographEvent {
    DiskChanged(DiskChanged),
    EditorChanged(EditorChanged),
    AsyncWorkFinished(AsyncWorkFinished),
    Quit(Quit),
}

struct DiskChanged {
    path: PathBuf,
    presence: Presence,
}

enum Presence {
    Present(Present),
    Absent,
}

struct Present {
    contents: String,
}

struct EditorChanged {
    path: PathBuf,
    buffer: Buffer,
}

enum Buffer {
    Open(String),
    Closed,
}

struct AsyncWorkFinished;

struct Quit;
```

`path` is absolute and canonical. A relative path is resolved by the source that constructed the event, never by `handle`.

`Present` is create or modify or the destination of a move. `Absent` is delete or the source of a move. There is no `Moved` variant. A rename the watcher sees becomes `Absent` then `Present`. pico sources are keyed by path; a rename is remove old plus intern new.

Ingested events come from outside `handle`: `DiskChanged`, `EditorChanged`. The CLI can submit any of them. The watcher submits `DiskChanged`. The LSP adapter submits `EditorChanged`.

`IsographEvent` does not derive `Deserialize`. A wire client cannot construct `AsyncWorkFinished` or `Quit`.

```rust
#[derive(serde::Deserialize, Debug)]
#[serde(tag = "kind", content = "value")]
enum IncomingEvent {
    #[serde(rename = "IncomingEvent.DiskChanged")]
    DiskChanged(DiskChanged),
    #[serde(rename = "IncomingEvent.EditorChanged")]
    EditorChanged(EditorChanged),
}
```

A frame that is not a valid `IncomingEvent` is logged and dropped. The connection stays up.

`DiskChanged` is a filesystem fact. `Present` carries the contents. `handle` does not open the path. Boot scan is a burst of `DiskChanged` from the watcher; Injected mode has no such burst.

`EditorChanged` is an editor-buffer fact. The LSP adapter produces it from `didOpen` / `didChange` / `didClose`. The CLI can produce the same event without an editor.

`AsyncWorkFinished` is the answer to work `handle` asked the effect loop to do off-thread: a compilation, a schema fetch. Tests may inject it. It is not on the wire.

`Quit` is `isograph stop` and SIGTERM, once the daemon maps those onto an event and a `Kill` effect. It is not on the wire.

## Effect

An effect is inert data. Performing it is the effect loop's job. The effect loop never mutates the state.

```rust
enum IsographEffect {
    WriteArtifacts(WriteArtifacts),
    ReportDiagnostics(ReportDiagnostics),
    StartAsyncWork(StartAsyncWork),
    Kill,
}

struct WriteArtifacts {
    pub files: Vec<Artifact>,
}

struct Artifact {
    pub path: PathBuf,
    pub contents: String,
}

struct ReportDiagnostics {
    pub diagnostics: Vec<Diagnostic>,
}

struct StartAsyncWork;
```

`WriteArtifacts` is generation's output written to disk. Producing `Artifact` values is the generation seam (pluggable-compiler.md); writing them is this effect. Incoming disk facts are `DiskChanged`. The two directions do not share a type.

`ReportDiagnostics` is what the adapter turns into `publishDiagnostics` and what a CLI compile prints.

`StartAsyncWork` runs something off the event thread; its result returns as `AsyncWorkFinished`.

`Kill` ends the effect loop. `run` returns. The process exits.

event-loop.md ships `HelloWorld` / `LogHelloWorld`, `Quit` / `Kill`, and `handle` returning `Vec<IsographEffect>`. filesystem-events.md adds `DiskChanged`. `Presence` is that doc's created/deleted/moved change.

## State

The state is a pico database. Disk files and open editor buffers are source nodes. Compilation is derived.

```rust
struct DiskFile {
    path: PathBuf,
    contents: String,
}

struct OpenFile {
    path: PathBuf,
    contents: String,
}
```

A path may have a `DiskFile`, an `OpenFile`, both, or neither.

Artifact generation and watch mode read `DiskFile`. They do not read `OpenFile`.

The LSP adapter reads `OpenFile` when it exists for that path, otherwise `DiskFile`. The LSP adapter does not generate artifacts.

event-loop.md ships `IsographState` with no fields. filesystem-events.md adds a path-to-contents map. pico intern of `DiskFile` replaces that map; `DiskChanged` stays. `OpenFile` lands with the adapter.

## Dispatch

`DiskChanged` with `Present` sets `DiskFile` from the payload. `DiskChanged` with `Absent` removes it. `EditorChanged` with `Open` sets `OpenFile`. `EditorChanged` with `Closed` removes it. `AsyncWorkFinished` records the result. `Quit` returns `Kill`.

## Watch and batch

Watch mode is the process listening for `DiskChanged` and compiling.

Batch mode is watch mode from a fresh start. Boot is a burst of `DiskChanged`; the same `handle` runs.
