# Drive the isograph binary from CI by injecting events

Requires the event model. The parked daemon in `isograph-cli.md` is not enough: CI needs a process that accepts filesystem facts as events.

CI tests inner `handle`: feed events, assert effects. No daemon required for that.

Driving the binary is the outer path: `Filesystem::Injected`, submit the same ingested events (`DiskChanged`, `EditorChanged`, …) the watcher or LSP adapter would have produced.

Whether we also want tests that write real files and watch them is open. This doc is the injected path.

## What CI does

```
isograph daemon --filesystem injected --port 3884
```

Then a client sends JSON frames, one `IncomingEvent` per line, the same socket shape as figaro.

```json
{"kind":"IncomingEvent.DiskChanged","value":{"path":"/tmp/proj/src/a.ts","presence":{"Present":{"contents":"export const a = 1;\n"}}}}
```

The daemon handles that event. It never opens `/tmp/proj/src/a.ts`. Artifacts and diagnostics are effects.

`isograph start --filesystem injected` is the same mode, backgrounded. CI that needs the socket in-process uses the foreground `daemon` verb.

## Change 1: `Filesystem` and `--port` on the daemon

Not a bool `--noop`. Two ways filesystem facts arrive.

```rust
// from crates/isograph_cli/src/main.rs
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum Filesystem {
    Watch,
    Injected,
}

#[derive(clap::Args, Debug)]
pub struct IsographArgs {
    /// How filesystem facts arrive. `watch` observes the OS. `injected` only accepts events.
    #[arg(long, value_enum, default_value_t = Filesystem::Watch)]
    pub filesystem: Filesystem,

    /// Loopback port for incoming events. Figaro's shape.
    #[arg(long, default_value_t = 3884)]
    pub port: u16,
}
```

`App::DaemonArgs` is `IsographArgs` again (it already is, empty). `Id` stays whatever config-discovery set, or `NoArgs` until that lands.

`run_daemon` matches `filesystem`:

- `Watch`: start the watcher source (when it exists), listen on `port`.
- `Injected`: do not scan, do not watch, listen on `port`.

Until the event loop exists, `Injected` still parks after binding the socket, and a well-formed frame is logged and dropped. The socket bind is what this change ships.

## Change 2: `IncomingEvent` and the socket

`FigaroEvent` is not on the wire. Same split as figaro: a narrower deserialize enum.

```rust
// from crates/isograph_cli/src/external.rs
#[derive(serde::Deserialize, Debug)]
#[serde(tag = "kind", content = "value")]
pub enum IncomingEvent {
    #[serde(rename = "IncomingEvent.DiskChanged")]
    DiskChanged(DiskChanged),
    #[serde(rename = "IncomingEvent.EditorChanged")]
    EditorChanged(EditorChanged),
}
```

`Quit` is not constructible from the wire.

`crates/isograph_cli/Cargo.toml` gains `serde` with `derive`, `serde_json`, `freddie_event_socket` at the same freddie rev, and `tokio` with `rt`, `macros`, `net`, `sync`.

`run_daemon` builds a current-thread runtime, binds `freddie_event_socket::listen(port, ...)`, and on each frame deserializes `IncomingEvent` and sends the matching `IsographEvent`. A bad frame is logged and dropped, like figaro. Ingested events on the wire are the same ones `handle` takes, including `EditorChanged`.

`Watch` without a watcher yet: bind the socket and park. `Injected`: the same, no watcher to skip.

## Change 3: a CI job that starts Injected and submits one event

A workflow (or a step on `cargo-test-cli`) builds the crate, starts `isograph daemon --filesystem injected --port 3884` in the background, writes one `DiskChanged` `Present` to `127.0.0.1:3884`, and asserts the process is still running (`isograph status`) and the log recorded a dispatch (or, until dispatch exists, that the frame was accepted). Then `isograph stop`.

HOME (and `XDG_STATE_HOME` / `LOCALAPPDATA`) point at a temp directory, same as `tests/cli.rs`.

A crate integration test may do this instead of YAML: spawn `CARGO_BIN_EXE_isograph` with `daemon --filesystem injected --port 0` if the port is published, or a fixed high port. Prefer the integration test; the workflow only needs `cargo test --manifest-path crates/isograph_cli/Cargo.toml` from `cli-ci-build.md`.

## Open

Tests that write a file on disk, run `Filesystem::Watch`, and assert the daemon noticed. The injected path does not replace that, and does not require it.
