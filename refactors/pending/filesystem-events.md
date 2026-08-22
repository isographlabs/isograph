# Filesystem events: DiskFile in the pico database

Requires event-loop.md (landed), send-events.md (landed), and `docs-website/docs/design-docs/event-model.md`. The daemon recvs, calls `handle`, performs effects, listens on the event socket, and `isograph send` writes one `IsographEvent` JSON frame.

`IsographState` becomes the pico database. A path on disk is a `DiskFile` source. `DiskChanged` is the event that sets or removes that source. Files arrive through `isograph send`. `handle` does not read the filesystem.

Two shippable changes.

## What the user does

After change 2:

```
$ isograph logs
{"timestamp":"...","level":"INFO","fields":{"message":"isograph daemon up","config":"/tmp/proj/isograph.config.json","port":53124}}
$ printf '%s\n' '{"kind":"DiskChanged","value":{"path":"/tmp/proj/src/a.ts","presence":{"Present":{"contents":"export const a = 1;\n"}}}}' > /tmp/disk.json
$ isograph send --file /tmp/disk.json
```

`handle` interns that path as a `DiskFile`. A second send with `"presence":"Absent"` removes it. Neither send produces an effect.

## Change 1: pico database, DiskFile, DiskChanged

send-events.md already recvs, performs, and listens. This change makes `IsographState` a pico `#[derive(Db)]` database, adds `DiskFile` as a source, and adds `DiskChanged` with `Presence`. Tests call `handle` and also drive the socket the way figaro's `tests/external.rs` does.

### Types

Most important first.

Origin of `IsographState` as a pico database: isograph `crates/isograph_schema/src/isograph_database.rs` `IsographDatabase`. Delta: no `TCompilationProfile`; no `iso_literal_map` / `standard_sources` / `open_file_map`; one tracked `disk_file_map`; the type keeps the event-loop name `IsographState`.

Origin of `DiskFile`: isograph `IsoLiteralsSource`. Delta: `PathBuf` key instead of `RelativePathToSourceFile`; type name `DiskFile`.

Origin of `DiskFileMap`: isograph `IsoLiteralMap`. Delta: `PathBuf` key, `SourceId<DiskFile>` value. Same `HashMap`.

```rust
// from crates/isograph_cli/src/state.rs
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source};
use prelude::Postfix;

use crate::effect::IsographEffect;
use crate::event::{DiskChanged, IsographEvent, Presence};

#[derive(Default, Debug, Db)]
pub struct IsographState {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
}

#[derive(Debug, Default)]
pub struct DiskFileMap(pub HashMap<PathBuf, SourceId<DiskFile>>);

#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct DiskFile {
    #[key]
    pub path: PathBuf,
    pub contents: String,
}
```

`disk_file_map` is private. `#[tracked]` fields must be. Generated getters are `get_disk_file_map` and `get_disk_file_map_mut`.

Origin of the `Present` / `Absent` arms: isograph `insert_iso_literal` / `remove_iso_literal`, inlined in `handle`. Delta: `PathBuf` is not `Copy`, so `Present` clones `path` into the source and moves the original into the map.

```rust
// from crates/isograph_cli/src/state.rs
impl IsographState {
    pub fn handle(&mut self, event: IsographEvent) -> Vec<IsographEffect> {
        match event {
            IsographEvent::HelloWorld => IsographEffect::LogHelloWorld.wrap_vec(),
            IsographEvent::Quit => IsographEffect::Kill.wrap_vec(),
            IsographEvent::DiskChanged(change) => {
                self.handle_disk_changed(change);
                Vec::new()
            }
        }
    }

    fn handle_disk_changed(&mut self, change: DiskChanged) {
        match change.presence {
            Presence::Present(present) => {
                let source_id = self.set(DiskFile {
                    path: change.path.clone(),
                    contents: present.contents,
                });
                self.get_disk_file_map_mut()
                    .tracked()
                    .0
                    .insert(change.path, source_id);
            }
            Presence::Absent => {
                if let Some(source_id) = self
                    .get_disk_file_map_mut()
                    .tracked()
                    .0
                    .remove(&change.path)
                {
                    self.remove(source_id);
                }
            }
        }
    }
}
```

`Present` of a path that is already in the map replaces the `DiskFile` (pico `set` on the same key) and the map entry. `Absent` of a path that is not in the map is a no-op. `Present` of an empty string is present, not absent. `DiskChanged` returns no effects.

`handle` does not canonicalize `path`. The JSON author is the source. A relative path is stored as given.

Origin: send-events.md `IsographEvent`. Delta: `DiskChanged` beside `HelloWorld` and `Quit`. `DiskChanged` and `Presence` are `Serialize` + `Deserialize`.

```rust
// from crates/isograph_cli/src/event.rs
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum IsographEvent {
    HelloWorld,
    Quit,
    DiskChanged(DiskChanged),
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct DiskChanged {
    pub path: PathBuf,
    pub presence: Presence,
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum Presence {
    Present(Present),
    Absent,
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Present {
    pub contents: String,
}
```

Wire:

```json
{"kind":"DiskChanged","value":{"path":"/tmp/proj/src/a.ts","presence":{"Present":{"contents":"export const a = 1;\n"}}}}
{"kind":"DiskChanged","value":{"path":"/tmp/proj/src/a.ts","presence":"Absent"}}
```

A move is two frames, in that order: `Absent` of `from`, `Present` of `to`. There is no `Moved` variant. pico sources are keyed by path; a rename is remove old plus intern new.

`on_message` is send-events.md: `from_str::<IsographEvent>`. No new arm. `DiskChanged` deserializes because it is a variant of that enum.

`IsographEffect` and `perform` do not change.

`serve` constructs the database with `Default`. Origin: send-events.md / event-loop.md `let state = IsographState`. Delta: `IsographState::default()`.

```rust
// from crates/isograph_cli/src/daemon.rs
    let state = IsographState::default();
```

The same replacement in `run_event_loop` tests in `daemon.rs` that currently write `IsographState`.

### Cargo

```toml
# from crates/isograph_cli/Cargo.toml
pico = { path = "../pico" }
pico_macros = { path = "../pico_macros" }
```

`Storage` implements `Default`. `#[derive(Db, Default)]` is the pico test pattern (`crates/pico/tests/basic.rs`).

### Tests

In `state.rs`. A `#[cfg(test)]` helper in that tests module (not on `IsographState`) reads a path:

```rust
fn disk_file<'a>(state: &'a IsographState, path: &Path) -> Option<&'a DiskFile> {
    state
        .get_disk_file_map()
        .untracked()
        .0
        .get(path)
        .map(|id| state.get(*id))
}
```

`get` panics on a missing `SourceId`. The helper does not call `get` unless the map has the path. `use pico::Database` in the tests module.

- `HelloWorld` still returns `LogHelloWorld`. Construction is `IsographState::default()`.
- `Quit` still returns `Kill`.
- `Present` inserts. `disk_file(state, path).expect("the test inserted this path").contents` is the payload. `handle` returns `Vec::new()`.
- A second `Present` on the same path replaces contents. The map has one entry.
- An empty string is stored. `disk_file` is `Some` with `contents == ""`.
- Two paths are two entries.
- `Absent` removes. `disk_file` is `None`. `handle` returns `Vec::new()`.
- `Absent` of a path that was never present leaves the map unchanged. `handle` returns `Vec::new()`.
- `Present` of an empty string is present, not absent.
- Two events `Absent` then `Present` on different paths is a move: old path gone, new path present with those contents.

`expect` in these tests names the fixture the test inserted.

In `external.rs`:

- A `DiskChanged` frame with `Present` round-trips. A tokio test: send one frame, `event_rx.try_recv()` is `DiskChanged` with that path, `Present`, and those contents.
- A `DiskChanged` frame with `Absent` round-trips.
- `"not json"` still does not deserialize (existing test).
- A bad frame then a good `DiskChanged` frame: the good one still arrives.

The e2e crate does not yet send `DiskChanged`; that is change 2. Existing start/status/logs/stop/send-HelloWorld tests still pass.

## Change 2: send `DiskChanged`

`isograph send` and the socket are send-events.md. This change adds a `DiskChanged` e2e.

Origin of the verb: send-events.md. Delta: a `DiskChanged` frame instead of `HelloWorld`. `on_message` does not change.

`freddie_event_socket` caps a frame at 64 KiB. The e2e contents stay under that.

### Tests

E2E in `crates/ts_graphql_react_isograph_cli/tests/cli.rs`:

- The log has `isograph daemon up`. Write a temp JSON file with `Present` of `/tmp/proj/src/a.ts` and contents `export const a = 1;\n`. `isograph send --file` that file. The process exits 0. `--file` is unlinked.
- A second send in the same test with `Absent` of that path. The process exits 0.

send-events.md already covers a stopped daemon and `not json`. The harness already points `HOME` at the temp dir.

## Call sites

- `run_event_loop` -> `state.handle`.
- `IsographEvent::DiskChanged` -> `handle_disk_changed` (`db.set` / `db.remove` plus the tracked map). No effects.
- `CliVerb::Send` -> `send::run` (send-events.md). A `DiskChanged` frame is one `IsographEvent`.
- socket callback -> `on_message` -> `event_tx.send`.
