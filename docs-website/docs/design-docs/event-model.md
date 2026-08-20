# Event model

An isograph process is a pure function of state and event. Sources send events. Dispatch mutates the state and returns inert effects. An effect loop performs them. Dispatch does no IO. Sources do not touch the state.

This is figaro's shape. Figaro's events are keys and OS reports. Isograph's events are file changes and completed work.

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
    Present,
    Absent,
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

`DiskChanged` is the file watcher: created, written, or removed. Boot scan is a burst of `DiskChanged` with `Present` for every project file.

`EditorChanged` is the LSP: the buffer for an open file, or that the file is no longer open.

`AsyncWorkFinished` is the answer to work dispatch asked the effect loop to do off-thread: a compilation, a schema fetch.

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

```rust
fn handle(state: &mut Database, event: IsographEvent) -> Vec<IsographEffect>
```

`DiskChanged` with `Present` sets `DiskFile`. `DiskChanged` with `Absent` removes it. `EditorChanged` with `Open` sets `OpenFile`. `EditorChanged` with `Closed` removes it. `AsyncWorkFinished` records the result. `Quit` returns `Kill`.

## Watch and batch

Watch mode is the process listening for `DiskChanged` and compiling.

Batch mode is watch mode from a fresh start. Boot is a burst of `DiskChanged`; the same `handle` runs.
