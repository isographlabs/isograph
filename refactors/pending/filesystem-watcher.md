# Filesystem watcher

Requires filesystem-events.md, config-includes.md, and `docs-website/docs/design-docs/event-model.md`. The daemon recvs `DiskChanged` and interns `DiskFile`. This file adds a source that walks once, then observes the OS, and posts `DiskChanged` on the same channel the socket uses.

The watcher posts in-process. It does not run `isograph send` and it does not write to the event socket. `isograph send` remains a source of the same event type.

Origin of the notify loop: isograph `crates/isograph_compiler/src/watch.rs` `create_debounced_file_watcher` and `categorize_and_filter_events`. Origin of posting `DiskChanged` rather than mutating the db: `docs-website/docs/design-docs/event-model.md`. Delta: OS events become `DiskChanged` with `Presence`; `handle` interns; no `ChangedFileKind`; no compile-on-change; `Filesystem::Watch` / `Filesystem::Injected` instead of isograph's `--watch` bool.

One shippable change.

## What the user does

```
$ isograph start --filesystem watch
# create, edit, rename, delete files under project_root
# each in-scope file becomes a DiskChanged, then a DiskFile
```

```
$ isograph start --filesystem injected
```

Writing a file on disk does not produce a log line. `isograph send` still works.

Default is `Watch`. `Injected` is how tests that must not watch start the daemon.

## Types

Most important first.

`IsographArgs` is `App::DaemonArgs`. Origin: event-loop.md / config-discovery.md `NoArgs`. Delta: `Filesystem`. `start` and `daemon` flatten `DaemonArgs`. `status` / `logs` / `stop` do not.

```rust
// from crates/isograph_cli/src/lib.rs
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum Filesystem {
    Watch,
    Injected,
}

#[derive(clap::Args, Debug)]
struct IsographArgs {
    /// How filesystem facts arrive. `watch` observes the OS. `injected` only accepts events.
    #[arg(long, value_enum, default_value_t = Filesystem::Watch)]
    pub filesystem: Filesystem,
}
```

```rust
// from crates/isograph_cli/src/lib.rs
    type DaemonArgs = IsographArgs;

    fn run_daemon(id: &ConfigFlag, args: &IsographArgs) {
        // instance_for_config_path, unlink port file, as today
        let config = match discover::load_config(path.reference()) {
            Ok(config) => config,
            Err(e) => {
                tracing::error!(error = %e, "could not load the config");
                return;
            }
        };
        crate::daemon::run(path, port_path, args.filesystem, config);
    }
```

`Injected` does not scan and does not watch. The event socket listens in both modes.

### Start

`run_daemon` already loads the config. Pass it into `serve`. Do not load it a second time.

```rust
// from crates/isograph_cli/src/daemon.rs
pub fn run(
    config_path: PathBuf,
    port_path: PathBuf,
    filesystem: Filesystem,
    config: isograph_config::IsographProjectConfig,
) {
    // runtime as today
    runtime.block_on(serve(config_path, port_path, filesystem, config));
}

async fn serve(
    config_path: PathBuf,
    port_path: PathBuf,
    filesystem: Filesystem,
    config: isograph_config::IsographProjectConfig,
) {
    // channel, socket, port file, SIGTERM as today

    let _watcher = match filesystem {
        Filesystem::Injected => None,
        Filesystem::Watch => {
            let scope = match crate::scope::SourceScope::from_config(
                config_path.reference(),
                config.reference(),
            ) {
                Ok(scope) => scope,
                Err(e) => {
                    tracing::error!(error = %e, "could not build the source scope");
                    return;
                }
            };
            match crate::watch::start(event_tx.clone(), scope) {
                Ok(watcher) => watcher.wrap_some(),
                Err(e) => {
                    tracing::error!(error = %e, "could not start the watcher");
                    return;
                }
            }
        }
    };

    tracing::info!(
        config = %config_path.display(),
        port,
        filesystem = ?filesystem,
        "isograph daemon up"
    );

    let _hold_events = event_tx;
    let state = IsographState::default();
    tokio::select! {
        () = run_event_loop(state, event_rx, effect_tx) => {}
        () = run_effect_loop(effect_rx) => {}
    }
    let _ = std::fs::remove_file(port_path.reference());
}
```

The `Watcher` is a local that outlives the `while let Some(event)` loop. Drop order is loop, then watcher, then socket.

`daemon up` logs `filesystem`. E2E that must not watch uses `--filesystem injected`. Existing `Daemon::start` becomes `start` with `--filesystem injected`. Add `Daemon::start_watch` for the one real-notify test.

`run_daemon` today loads the config and discards it before `daemon::run`. The `Watch` arm loads it again for `SourceScope`. Loading twice is the same file; do not stash it in `IsographState`. Config as a pico singleton is a later compilation doc.

### Watch

```rust
// from crates/isograph_cli/src/watch.rs
use std::path::{Path, PathBuf};
use std::time::Duration;

use ignore::WalkBuilder;
use notify::event::{ModifyKind, RenameMode};
use notify::{EventKind, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, new_debouncer};
use prelude::Postfix;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

use crate::event::{DiskChanged, IsographEvent, Presence, Present};
use crate::scope::SourceScope;

const DEBOUNCE: Duration = Duration::from_millis(50);

#[derive(Debug)]
pub struct WatchRoot {
    pub path: PathBuf,
    pub source: notify::Error,
}

#[derive(Debug, thiserror::Error)]
pub enum WatchError {
    #[error("could not watch {}: {}", .0.path.display(), .0.source)]
    Watch(WatchRoot),
    #[error("could not start the notify watcher: {0}")]
    Notify(notify::Error),
}

pub struct Watcher {
    _debouncer: notify_debouncer_full::Debouncer<
        notify::RecommendedWatcher,
        notify_debouncer_full::FileIdMap,
    >,
}

pub fn start(
    event_tx: UnboundedSender<IsographEvent>,
    scope: SourceScope,
) -> Result<Watcher, WatchError> {
    scan(event_tx.reference(), scope.reference());
    let tx = event_tx.clone();
    let scope_for_events = scope.clone();
    let mut debouncer = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| {
        match result {
            Ok(events) => {
                for event in events {
                    dispatch(
                        tx.reference(),
                        scope_for_events.reference(),
                        event.kind,
                        event.paths.as_slice(),
                    );
                }
            }
            Err(errors) => {
                for e in errors {
                    warn!(error = %e, "notify");
                }
            }
        }
    })
    .map_err(WatchError::Notify)?;
    for root in &scope.watch_roots {
        debouncer
            .watch(root.reference(), RecursiveMode::Recursive)
            .map_err(|source| {
                WatchError::Watch(WatchRoot {
                    path: root.clone(),
                    source,
                })
            })?;
    }
    Watcher {
        _debouncer: debouncer,
    }
    .wrap_ok()
}
```

`SourceScope` derives `Clone`. The `Debouncer` type arguments are `notify-debouncer-full` 0.4 with `notify` 7. If inference can fill the field, omit the arguments. `new_debouncer(timeout, tick_rate, cb)` with `tick_rate: None` is the crate default. Workspace already pins both crates. isograph uses 100ms; this slice uses 50ms.

### Scan

```rust
// from crates/isograph_cli/src/watch.rs
fn scan(event_tx: &UnboundedSender<IsographEvent>, scope: &SourceScope) {
    let mut paths = Vec::new();
    for root in &scope.watch_roots {
        for entry in WalkBuilder::new(root).build() {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    warn!(error = %e, "walk");
                    continue;
                }
            };
            match entry.file_type() {
                Some(file_type) if file_type.is_file() => {}
                _ => continue,
            }
            let path = entry.path();
            if scope.contains(path) {
                paths.push(path.to_owned());
            }
        }
    }
    for always in scope.always.iter() {
        match always.is_file() {
            true => paths.push(always.to_owned()),
            false => {}
        }
    }
    paths.sort();
    paths.dedup();
    info!(n = paths.len(), "scan finished");
    for path in paths {
        post_present(event_tx, path.reference());
    }
}
```

`WalkBuilder::new(root).build()` uses the crate defaults: gitignore on, hidden files skipped, symlinks not followed. A gitignored schema that is `always` is still posted because `always` is merged after the walk.

`file_type.is_file()` and `always.is_file()` are std. Match rather than `if !`. Raise: `is_file` is std's API; the match makes the two cases visible.

### Dispatch

```rust
// from crates/isograph_cli/src/watch.rs
fn dispatch(
    event_tx: &UnboundedSender<IsographEvent>,
    scope: &SourceScope,
    kind: EventKind,
    paths: &[PathBuf],
) {
    match kind {
        EventKind::Create(_) | EventKind::Modify(ModifyKind::Data(_)) => {
            if let Some(path) = paths.first() {
                on_maybe_present(event_tx, scope, path);
            }
        }
        EventKind::Remove(_) => {
            if let Some(path) = paths.first() {
                on_maybe_absent(event_tx, path);
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
            if let Some(from) = paths.first() {
                on_maybe_absent(event_tx, from);
            }
            if let Some(to) = paths.get(1) {
                on_maybe_present(event_tx, scope, to);
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
            if let Some(from) = paths.first() {
                on_maybe_absent(event_tx, from);
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
            if let Some(to) = paths.first() {
                on_maybe_present(event_tx, scope, to);
            }
        }
        _ => {}
    }
}

fn on_maybe_present(event_tx: &UnboundedSender<IsographEvent>, scope: &SourceScope, path: &Path) {
    let path = match path.canonicalize() {
        Ok(path) => path,
        Err(_) => return,
    };
    match path.is_file() {
        true => {}
        false => return,
    }
    if !scope.contains(path.reference()) {
        return;
    }
    post_present(event_tx, path.reference());
}

fn on_maybe_absent(event_tx: &UnboundedSender<IsographEvent>, path: &Path) {
    let path = match path.canonicalize() {
        Ok(path) => path,
        Err(_) => path.to_owned(),
    };
    post(
        event_tx,
        DiskChanged {
            path,
            presence: Presence::Absent,
        },
    );
}

fn post_present(event_tx: &UnboundedSender<IsographEvent>, path: &Path) {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(e) => {
            warn!(error = %e, path = %path.display(), "could not read");
            return;
        }
    };
    post(
        event_tx,
        DiskChanged {
            path: path.to_owned(),
            presence: Presence::Present(Present { contents }),
        },
    );
}

fn post(event_tx: &UnboundedSender<IsographEvent>, change: DiskChanged) {
    let _ = event_tx.send(IsographEvent::DiskChanged(change));
}
```

`Present` is scoped: out-of-scope creates do not enter the map. `Absent` is not scoped: a deleted path often fails canonicalize, and filtering it through `contains` would miss in-scope deletes. An out-of-scope `Absent` is a no-op in `handle`. Failed read after `Create` (vanished during debounce) drops the event; the following `Remove` posts `Absent`. `ModifyKind::Metadata` is ignored. Directory `Create` is dropped by `path.is_file()`.

isograph has `ChangedFileKind` and panics if a create event does not contain exactly one path. We take `paths.first()` / `paths.get(1)` and skip. No panic on any notify payload.

### Tests

Unit tests of `dispatch` with a fake channel and a `SourceScope` over a temp tree:

- `Create` of an in-scope file that exists: one `Present` with those contents.
- `Create` of an out-of-scope `.rs` file: no event.
- `Remove` of a path: one `Absent`.
- `RenameMode::Both` with two paths: `Absent` of from, `Present` of to (file must exist at `to`).
- `RenameMode::Both` out of scope to in scope: `Absent` of from is posted and `Present` of to; the test asserts the pair.
- Empty file: `Present` with `contents == ""`.
- Failed read (path does not exist on `Create`): no event.

`dispatch` is `pub(crate)` so the tests in the module can call it.

Scan: write `src/a.ts` and `src/b.rs` and `node_modules/c.ts` under a temp project with default includes. Give `scan` the channel and drain. Assert the paths posted are exactly `src/a.ts` and the schema and the config, sorted. `b.rs` and `node_modules` are absent.

Do not add a production function only the tests call. Drain the test channel.

E2E: `--filesystem watch` starts, the log has `scan finished`. HOME isolation as today. Deadline 10s, same as existing daemon tests.

Injected e2e from filesystem-events.md still does not start a watcher. All existing `Daemon::start` tests pass `--filesystem injected`.

### Cargo

```toml
# from crates/isograph_cli/Cargo.toml
ignore = { workspace = true }
notify = { workspace = true }
notify-debouncer-full = { workspace = true }
```

## Call sites

- `isograph start --filesystem watch|injected` -> `App::DaemonArgs` -> `run_daemon` -> `daemon::run`.
- `Filesystem::Watch` arm -> `watch::start`.
- notify callback -> `dispatch` -> `event_tx.send`.
- scan -> `post_present` -> `event_tx.send`.
- event loop -> `handle` (filesystem-events.md).
