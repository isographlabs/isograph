# Event model

Two layers, one `handle`.

Inner: `handle(state, event) -> Vec<IsographEffect>`. No filesystem, no LSP, no socket. The test harness calls this function. The running binary calls this same function. There is not a second copy for tests.

Outer: the real process. It listens to the filesystem, speaks LSP, and performs effects. Every path into the process becomes an event, then `handle`, then effects.

The process does not read the filesystem inside `handle`. Facts about files arrive as events: a path is present with these contents, or a path is absent. Writing files is an effect.

This is figaro's shape. Figaro's events are keys and OS reports. Isograph's events are file changes, editor buffers, and completed work.

The building blocks are `Config`, `IsographEvent`, `IsographEffect`, `ClientId`, `DiskFile`, and `OpenFile`. The state is a pico database.

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

`Watch` starts a source that observes the OS and emits `DiskChanged`. `Injected` does not scan and does not watch. The LSP port listens in both modes. The CLI and the adapter submit `Internal` events into the same `handle`. `Watch` posts in-process. `Injected` still uses `isograph send`.

## Inner

```rust
fn handle(state: &mut IsographState, event: IsographEvent) -> Vec<IsographEffect>
```

`IsographState` is the process wrapper (`db: IsographDb`, live sessions, `source_globs`, `diagnostics_debounce`). The guard is not a pico source. `IsographDb` is the pico database. The test harness calls `handle`. It does not start a daemon, open a socket, or write a file. A test constructs a `DiskChanged` or an `LspNotification`, runs `handle`, and asserts the effects and the `DiskFile` sources. The binary's event loop calls the same `handle` with the same types.

`IsographEvent` is `Lsp(LspEvent)` | `Internal(InternalEvent)` | `ClientConnection(ClientConnectionEvent)` | `Timer(TimerFired<TimerPayload>)`. `LspEvent` is `Request` / `Notification` / `Response`. `handle` of `Lsp` matches those and runs isograph request/notification dispatch. A request leftover is `method_not_found`: one immediate `SendLspResponseEffect`. HelloWorld returns `LogHelloWorld`. An accepted `didOpen`, `didChange`, or `didClose` arms the diagnostics debounce, and `handle` of that `TimerFired` returns `PublishDiagnosticsEffect`s. That is the plumbing the previous sentence named: a handler returns a timer plus an event. A notification leftover is no effects. A notification that matches returns `Vec<IsographEffect>` and not `SendLspResponseEffect`. Bad `isograph/event` params are leftover (empty effects). An LSP request and an LSP response carry the `ClientId` of their connection. `handle` drops a response from a client not in `LiveSessions`; a response from a live client is no effects. `handle` of `Internal` is `HelloWorld` / `Quit` / `DiskChanged`. `handle` of `Timer` uses `trigger_if_matching`. `handle` does not sleep. `handle` does not touch the socket. The effect loop writes the `Response`.

Scope of disk files is the watcher's job. `handle` reads `source_globs` only to choose which open files get diagnostics. `handle` does not know about gitignore. `isograph send` may inject any path.

## Outer

The binary is the outer. Each source is outside `handle` and feeds it. One process, one channel, one worker that owns `IsographState`. Sources do not read state. Performers do not mutate it.

- Watcher: OS notifications become `DiskChanged` (path plus contents or absent). It may read the disk to fill `Present.contents`. `handle` does not. The watcher posts in-process on the event channel. It does not run the CLI and it does not write to the LSP port.
- LSP port: LSP JSON-RPC on `{slug}.port`. `isograph send` is a client: initialize, `initialized`, notification `isograph/event` whose params are `InternalEvent` (`HelloWorld` / `Quit` / `DiskChanged`). After initialize the session posts every message as `Lsp`. It clones the writer only for requests. It does not interpret methods. `handle` of `Lsp` is isograph's server-loop match. A request leftover is `MethodNotFound`. A notification leftover is no effects. `isograph/event` is the first notification arm and runs `handle` of `Internal`. Bad `isograph/event` params do not kill the daemon. Later, `textDocument/didOpen` / `didChange` / `didClose` are notification arms; hover and `semanticTokens/full` are request arms. Effects from `handle` (`PublishDiagnosticsEffect`) become LSP notifications (`publishDiagnostics`).
- Effect loop: performs `LogHelloWorld`, `Kill`, `SendLspResponseEffect`, `Timer`. `Timer` is `schedule_timer`, which posts `IsographEvent::Timer(timer.event)`.

`isograph lsp` is a stdio proxy onto the port. Walk-up / `--config` is the same as every other verb. It starts the daemon if needed, dials the port, and copies stdin/stdout. Dropping the editor drops the proxy and that connection. The daemon stays up. Several editors share one process. The vscode-extension already spawns `isograph lsp` on stdio.

`isograph send` is a hidden LSP client of that port. It does not start the daemon. It is not in `--help`.

The sources are theoretically separate daemons. They are one process because they share `Database` and because a socket hop on every save is the wrong latency.

## Ports

Figaro is one process per machine, so a default port is enough. Isograph is one process per config. Two configs cannot share a port.

The LSP port binds `127.0.0.1:0`. The kernel assigns a port from its local/dynamic range. There is no `--port`. After the lock, `run_daemon` unlinks the leftover `{slug}.port` before loading the config, then `serve` binds and writes the port to a sibling of its lock (`{slug}.lock` → `{slug}.port`). On quit, after `Kill` ends the loops, `serve` unlinks the file, then `process::exit(0)`. Flock is released when the holder dies. `isograph send` reads the lock, then that file. `Held::Free` is not running and the file is not consulted. Lock held and the file absent means the daemon has taken the lock and has not bound yet, or is on the way out; send fails. Send does not wait.

There is one listener. There is no `{slug}.lsp` and no `freddie_event_socket`. `Watch` posts `DiskChanged` in-process. `Injected` uses `isograph send`, which carries `Present.contents` on the wire.

## Event

An event is something that happened, already carrying what the source knows.

```rust
enum IsographEvent {
    Lsp(LspEvent),
    Internal(InternalEvent),
    ClientConnection(ClientConnectionEvent),
    Timer(freddie_timer::TimerFired<TimerPayload>),
}

enum LspEvent {
    Request(LspRequest),
    Notification(lsp_server::Notification),
    Response(lsp_server::Response),
}

struct LspRequest {
    request: lsp_server::Request,
    reply: crossbeam::channel::Sender<lsp_server::Message>,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", content = "value")]
enum InternalEvent {
    DiskChanged(DiskChanged),
    AsyncWorkFinished,
    Quit,
}

#[derive(serde::Deserialize, serde::Serialize)]
enum DiskChanged {
    File(DiskFileChanged),
    FolderRemoved(FolderRemoved),
}

#[derive(serde::Deserialize, serde::Serialize)]
struct DiskFileChanged {
    path: PathBuf,
    presence: Presence,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct FolderRemoved {
    path: PathBuf,
}

#[derive(serde::Deserialize, serde::Serialize)]
enum Presence {
    Present(String),
    Absent,
}
```

`DiskFileChanged.path` and `FolderRemoved.path` are absolute and canonical. A relative path is resolved by the source that constructed the event, never by `handle`.

`File` `Present` is create or modify or the destination of a move of a file. `File` `Absent` is delete or the source of a move of a file. `FolderRemoved` is delete or the source of a move of a directory: notify does not deliver one `Remove` per child. There is no `Moved` variant. A rename the watcher sees becomes `FolderRemoved` then file `Present`s, or file `Absent` then file `Present`. pico sources are keyed by path; a rename is remove old plus intern new. A directory is not a `DiskFile`.

`InternalEvent` values come from outside `handle`: `DiskChanged`. The CLI submits `Internal`. The watcher submits `DiskChanged`. `Lsp` is not `Internal`; the session posts it from the socket.

The wire is `serde_json` of `InternalEvent`. A frame that is not a valid `InternalEvent` is logged and dropped. The connection stays up. `Lsp` is not a wire kind.

`DiskChanged` is a filesystem fact. `File` `Present` carries the contents. `handle` does not open the path. Boot scan is a burst of `File` `Present`. Injected mode has no such burst. `FolderRemoved` is a directory gone from disk.

`AsyncWorkFinished` is the answer to work `handle` asked the effect loop to do off-thread: a compilation, a schema fetch. Tests and the CLI may inject it.

`Quit` is SIGTERM, and a `Quit` frame on the socket (`isograph send` and Windows `stop`). `handle` returns `Kill`.

## Effect

An effect is inert data. Performing it is the effect loop's job. The effect loop never mutates the state.

```rust
enum IsographEffect {
    LogHelloWorld,
    Kill,
    SendLspResponse(Box<SendLspResponseEffect>),
    PublishDiagnostics(Box<PublishDiagnosticsEffect>),
    Timer(TimerEffect<TimerPayload>),
}
```

`Kill` ends the effect loop. `run` returns. The process exits.

event-loop.md ships `HelloWorld` / `LogHelloWorld`, `Quit` / `Kill`, tokio `run_event_loop` / `run_effect_loop`, and `handle` returning `Vec<IsographEffect>`. send-events.md ships the event socket, the port file, `Serialize` + `Deserialize` on `IsographEvent`, and `isograph send`. filesystem-events.md makes `IsographDb` a pico database, adds `DiskChanged` with `Presence`, and interns `DiskFile`.

## State

`IsographDb` is the pico database. Disk files and open editor buffers are source nodes. Compilation is derived.

```rust
#[derive(Debug, Db)]
struct IsographDb<THostLanguage: HostLanguage> {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
    #[tracked]
    open_file_map: OpenFileMap,
    phantom_data: PhantomData<THostLanguage>,
}

struct DiskFileMap(pub HashMap<PathBuf, SourceId<DiskFile>>);

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct DiskFile {
    #[key]
    path: PathBuf,
    contents: String,
}

struct OpenFileMap(pub HashMap<OpenFileKey, SourceId<OpenFile>>);

struct OpenFileKey {
    client: ClientId,
    path: RelativePathToSourceFile,
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct OpenFile {
    #[key]
    key: OpenFileKey,
    contents: String,
}
```

A path may have a `DiskFile`, one `OpenFile` per client that has it open, both, or neither. The tracked maps are which paths currently have a `DiskFile` and which `(client, path)` pairs have an `OpenFile`. `handle` is the only writer of `DiskFile` and `OpenFile`.

Artifact generation will read through `file_contents` as its own client with no `OpenFile`; how it gets its `ClientId` is not designed yet.

`file_contents(db, client, path)` reads that client's `OpenFile` when it exists, otherwise `DiskFile`. It is the only reader of file text. Semantic tokens, formatting, and `willSaveWaitUntil` read through it with the asking client's `ClientId`. The LSP adapter does not generate artifacts.

event-loop.md ships `IsographDb` with no fields. filesystem-events.md makes `IsographDb` the pico database and interns `DiskFile`. lsp-open-files.md adds `OpenFile` and `open_file_map`.

## Dispatch

`DiskChanged::File` with `Present` sets `DiskFile` from the payload. `DiskChanged::File` with `Absent` calls `remove_disk_file`. `DiskChanged::FolderRemoved` calls `remove_disk_files_from_path`. `DidOpenTextDocument` and `DidChangeTextDocument` (one change, no range) call `set_open_file` for the notification's client. `DidCloseTextDocument` calls `remove_open_file`. `ClientConnectionEvent::Gone` calls `remove_open_files_of_client`. An LSP notification carries the `ClientId` of its connection. `handle` drops a notification from a client not in `LiveSessions`. `AsyncWorkFinished` records the result. `Quit` returns `Kill`.

## Watch and batch

Watch mode is the process listening for `DiskChanged` and compiling.

Batch mode is watch mode from a fresh start. Boot is a burst of `DiskChanged`; the same `handle` runs.
