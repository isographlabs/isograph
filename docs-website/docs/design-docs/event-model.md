# Event model

Two layers, one `handle`.

Inner: `handle(state, event) -> Vec<IsographEffect>`. No filesystem, no LSP, no socket. The test harness calls this function. The running binary calls this same function. There is not a second copy for tests.

Outer: the real process. It listens to the filesystem, speaks LSP, accepts CLI frames, and performs effects. Every path into the process becomes an event, then `handle`, then effects.

The process does not read the filesystem. Facts about files arrive as events: a path is present with these contents, or a path is absent. Writing files is an effect.

This is figaro's shape. Figaro's events are keys and OS reports. Isograph's events are file changes, editor buffers, and completed work.

The building blocks are `Config`, `Event`, `Effect`, `DiskFile`, and `OpenFile`. The state is a pico database.

A binary is compiled with one `HostLanguage`, one `NetworkProtocol`, and one `RuntimeFramework`. Those are the type parameters in the mental model. They are static for the process. The config file's fields depend on them.

## Config

There is one isograph process per config file.

```rust
struct Config {
    path: PathBuf,
}
```

Figaro has one process on the machine (`Instance::global`). Isograph has one process per `Config` (`Instance::named`). `freddie_cli` is the same crate in both.

The process is found by `--config`, or by the nearest `isograph.config.json`, `isograph.config.js`, or `isograph.config.ts` at or above the current directory. At one directory, that order: json, then js, then ts. Two paths to one file are one process.

A `.json` config is data. A `.js` or `.ts` config is a module that exports the config object (`export default` or `module.exports`). Loading it runs the file with an executor and reads JSON from stdout. The executor is the first that works, same shape as barnum: `bun`, then `deno`, then `node` plus `tsx/cli` from `node_modules` walking up from the config, then `pnpm exec tsx`, `npx tsx`, `yarn exec tsx`, then `node`.

Watch mode and the LSP are that process. They share the pico database.

```rust
enum Filesystem {
    Watch,
    Injected,
}
```

`Watch` starts a source that observes the OS and emits `DiskChanged`. `Injected` does not. The CLI, the LSP adapter, and the socket all submit ingested events into the same `handle`.

## Inner

```rust
fn handle(state: &mut Database, event: IsographEvent) -> Vec<IsographEffect>
```

The test harness calls this. It does not start a daemon, open a socket, or write a file. A test constructs a `DiskChanged` or `EditorChanged`, runs `handle`, and asserts the effects. The binary's event loop calls the same `handle` with the same types.

## Outer

The binary is the outer. Each adapter is outside `handle` and feeds it.

- Watcher: OS notifications become `DiskChanged` (path plus contents or absent). It may read the disk to fill `Present.contents`. `handle` does not.
- LSP adapter: an LSP request (`textDocument/didOpen`, `didChange`, `didClose`, hover, completions, …) becomes one or more ingested events, usually `EditorChanged`. Effects come back as LSP responses and notifications (`publishDiagnostics`, `semanticTokens/full`, …). The adapter is request/response. `handle` is not.
- CLI: every ingested event can be submitted as a frame, including `DiskChanged` and `EditorChanged`. Same events the watcher and the LSP adapter would have produced.
- Effect loop: performs `WriteArtifacts`, `ReportDiagnostics`, LSP replies, `Kill`.

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

Ingested events come from outside `handle`: `DiskChanged`, `EditorChanged`. The CLI can submit any of them. The watcher submits `DiskChanged`. The LSP adapter submits `EditorChanged` (and any later LSP-originated events).

`DiskChanged` is a filesystem fact. `Present` carries the contents. `handle` does not open the path. Boot scan is a burst of `DiskChanged` from the watcher; Injected mode has no such burst.

`EditorChanged` is an editor-buffer fact. The LSP adapter produces it from `didOpen` / `didChange` / `didClose`. The CLI can produce the same event without an editor.

`AsyncWorkFinished` is the answer to work `handle` asked the effect loop to do off-thread: a compilation, a schema fetch. Tests may inject it.

`Quit` is `isograph stop` and SIGTERM.

## Effect

An effect is inert data. Performing it is the effect loop's job. The effect loop never mutates the state.

```rust
enum IsographEffect {
    WriteArtifacts,
    ReportDiagnostics,
    StartAsyncWork,
    Kill,
}
```

`WriteArtifacts` writes generated files. `ReportDiagnostics` prints or publishes errors. `StartAsyncWork` runs something off the event thread; its result returns as `AsyncWorkFinished`. `Kill` ends the process.

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

The LSP reads `OpenFile` when it exists for that path, otherwise `DiskFile`. The LSP does not generate artifacts. Whether it should is open.

## Dispatch

`DiskChanged` with `Present` sets `DiskFile` from the payload. `DiskChanged` with `Absent` removes it. `EditorChanged` with `Open` sets `OpenFile`. `EditorChanged` with `Closed` removes it. `AsyncWorkFinished` records the result. `Quit` returns `Kill`.

## Watch and batch

Watch mode is the process listening for `DiskChanged` and compiling.

Batch mode is watch mode from a fresh start. Boot is a burst of `DiskChanged`; the same `handle` runs.
