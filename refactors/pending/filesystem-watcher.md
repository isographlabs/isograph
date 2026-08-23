# Filesystem watcher

Requires filesystem-events.md (landed), interned-source-path.md (landed), config-source-files.md (landed), lsp-port.md (landed), and `docs-website/docs/design-docs/event-model.md`.

This is isograph's watcher. The code is `crates/isograph_compiler/src/watch.rs`, `read_files.rs`, and the `update_sources` match in `source_files.rs`. It lives in `isograph_cli` because i2 posts `DiskChanged` instead of mutating the db on the notify thread.

Origin of the notify loop: `create_debounced_file_watcher` and `categorize_and_filter_events`. Origin of the boot walk: `read_files_in_folder` / `visit_dirs_skipping_isograph`. Origin of folder delete: `remove_iso_literals_from_path`. Origin of ingest: `update_sources` / `handle_update_source_file` / `handle_update_source_folder`. Origin of `--watch`: isograph `CompileCommand.watch`. Origin of `Filesystem::Watch` / `Filesystem::Injected`: event-model.md.

Deltas from those files, exhaustive:

- The notify callback sends `SourceFileEvent`. A loop in `serve` reads contents and posts `DiskChanged` on `event_tx`. `handle` intern. isograph's callback sends, then `update_sources` writes the db on the recv loop. Same split, `DiskChanged` instead of `db.insert_iso_literal`.
- `Filesystem` instead of `--watch: bool`. Default `Watch`. `Injected` does not create a debouncer.
- `source_files` is membership. isograph used `path.starts_with(project_root)`. A relative path is in scope if the ordered glob list says so (`!` excludes, last match wins). The watch/walk root for a positive glob is the static prefix before the first `*`, `?`, `[`, or `{` (the analog of `project_root`). `**/*.ts` has an empty prefix and watches the config directory. `[]` watches nothing and intern nothing.
- The extension check in `read_files_in_folder` is `HostLanguage::source_file_kind`. One associated function, two-variant enum. Not a memo. Not a second trait. The CLI is already generic over `THostLanguage` and does not depend on `isograph_extract_typescript`.
- `__isograph` is skipped in the walker, as `visit_dirs_skipping_isograph`. Not a HostLanguage concern. `ISOGRAPH_FOLDER` lives on `isograph_config` as in isograph.
- No `schema` / `schema_extensions` / `artifact_directory` watches. i2 has none of those fields. Config-file events are `ChangedFileKind::Config` and are dropped. The daemon does not reload config.
- Watch, then boot-walk. isograph compiled from the walk, then watched. A file created in that gap was missed. Watch first; a Create that races with the walk is `Present` twice with the same bytes; pico does not advance the epoch.
- `CreateKind::Folder` scans that folder. isograph ignores it (comment in `process_create_event`).
- `RenameMode::From` / `To` are handled. isograph ignores them (`_ => None`).
- `EventKind::Any` and `ModifyKind::Any` (not `Name`) do exists-then-Present else Absent, same as isograph's `RenameMode::Any`. isograph drops them. notify 7 Windows emits `EventKind::Any` and `ModifyKind::Any`; kqueue emits `ModifyKind::Any` for `Vnode::Link`.
- `EventKind::Other` with `Flag::Rescan` re-walks every watch root. isograph drops it. inotify and FSEvents emit it when events were lost.
- No panic on path count. Take `paths.first()` / `paths.get(1)` and skip.
- `Path::starts_with` for descendant removal. isograph uses string `starts_with`, which treats `src` as a prefix of `src2`.
- Failed `read_to_string` / non-UTF8 / non-regular file posts `Absent`, not a silent keep of the old `DiskFile`.
- `UnboundedSender::send` from the notify thread. isograph uses `channel(1)` and `Handle::spawn` because that send is async.
- `let _ = event_tx.send`. isograph `expect`s the debouncer and each `watch()`.

`isograph send` still intern any path. Scope is the watcher's job. `handle`, `run_event_loop`, and the session do not know whether the watcher is running.

Two shippable changes. Prefactor first.

Change 1 is `remove_disk_file` prefix removal. `handle` of `Absent` is already the reader. Folder-delete in change 2 needs it. It lands alone.

Change 2 is the watcher: flag, globs, walk, notify, `HostLanguage::source_file_kind`. That is one chunk. `source_file_kind` with no walker is unused. `--filesystem watch` with no debouncer is a no-op. `ISOGRAPH_FOLDER` with no walk is unused. `SourceGlobs` with no ingest is unused. Do not split those into their own docs.

## What the user does

```
$ cat isograph.config.json
{"source_files":["src/**/*.ts","src/**/*.tsx","!src/**/*.test.ts"]}
$ isograph start
# create src/a.ts -> intern
# create src/a.test.ts -> not intern
# create src/a.rs -> not intern
# create lib/b.ts -> not intern
```

```
$ isograph start --filesystem injected
```

Writing a file on disk does not intern. `isograph send` of `DiskChanged` still intern.

Default is `Watch`. CI e2e uses `Injected`. After change 1, `isograph send` of `Absent` of a directory path removes interned files under that path.

## Change 1: descendant Absent

Origin: `IsographDatabase::remove_iso_literals_from_path`. Delta: `Path::starts_with`.

Before: `remove_disk_file` removes one map key.

After. Add `use std::path::Path`.

```rust
// from crates/isograph_compiler/src/database.rs
    pub fn remove_disk_file(&mut self, path: RelativePathToSourceFile) {
        let ids: Vec<_> = self
            .get_disk_file_map_mut()
            .tracked()
            .0
            .extract_if(|key, _| key.as_ref().starts_with(path.as_ref()))
            .map(|(_, source_id)| source_id)
            .collect();
        for source_id in ids {
            self.remove(source_id);
        }
    }
```

`handle_disk_changed` still calls `remove_disk_file`. `Absent` of a prefix path from `isograph send` removes descendants. That is the same contract as isograph's folder remove.

Empty relative path: `pathdiff` of the config directory against itself is `""`. `Path::new("src/a.ts").starts_with(Path::new(""))` is true. `Absent` of the config directory removes every interned file.

`Absent` of a never-interned path that is not a prefix of any key is a no-op.

### Tests

`database.rs`:

- intern `src/a.ts`, `src/b.ts`, `src2/c.ts`; `remove_disk_file` of `src`; `src/a.ts` and `src/b.ts` gone; `src2/c.ts` remains.
- intern `src/a.ts`; `remove_disk_file` of `src/a.ts.bak` leaves `src/a.ts`.
- intern `src/a.ts`; `remove_disk_file` of interned `""`; `src/a.ts` gone.
- existing exact-path remove tests stay.

`state.rs` handle, `intern_config_directory` of `/tmp/proj/isograph.config.json`:

- Present `/tmp/proj/src/a.ts`, `/tmp/proj/src/b.ts`, `/tmp/proj/src2/c.ts`. `Absent` of `/tmp/proj/src`: first two gone, `src2/c.ts` remains. `handle` returns `Vec::new()`.
- `Absent` of `/tmp/proj`: every interned file gone.
- `Absent` of `/tmp/proj/never`: no-op.

## Change 2: watcher

Requires change 1. Types, start, watch, tests, cargo, and call sites below are this change.

Most important first.

### HostLanguage filter

Origin: the extension match in `read_files_in_folder`. Delta: an associated function on `HostLanguage` instead of a hardcoded list in the walker.

```rust
// from crates/isograph_compiler/src/host_language.rs
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFileKind {
    Source,
    NotSource,
}

pub trait HostLanguage: Send + Sync + Sized + 'static {
    type Error: std::fmt::Display + std::error::Error + Clone + PartialEq + Eq + 'static;
    type LiteralContext: Clone + PartialEq + Eq + Debug + 'static;

    fn extract_iso_literals(
        db: &IsographState<Self>,
        path: RelativePathToSourceFile,
    ) -> &Option<Vec<IsoLiteralExtraction<Self>>>;

    fn source_file_kind(relative_path: &Path) -> SourceFileKind;
}
```

Before, the trait ends at `extract_iso_literals`. After, it has `source_file_kind`. No `directory_walk`. The walker skips `__isograph`. The globs skip `node_modules` when the user did not write a glob that includes it.

TypeScript, copied from `read_files_in_folder`:

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    fn source_file_kind(relative_path: &Path) -> SourceFileKind {
        match relative_path.extension().and_then(|e| e.to_str()) {
            Some("ts" | "tsx" | "js" | "jsx") => SourceFileKind::Source,
            _ => SourceFileKind::NotSource,
        }
    }
```

`isograph.config.ts` is `Source` by extension. The watcher still does not intern it: `categorize` returns `ChangedFileKind::Config` when `path == config_path`, copied from isograph's `path == config.config_location`.

### ISOGRAPH_FOLDER

```rust
// from crates/isograph_config/src/compilation_options.rs
pub static ISOGRAPH_FOLDER: &str = "__isograph";
```

Origin: isograph `isograph_config::ISOGRAPH_FOLDER`. `isograph_config` already re-exports `compilation_options::*`.

### SourceFileEvent

Copied from `watch.rs`. Delta: no Schema / SchemaExtension. `JavaScriptSourceFile` / `JavaScriptSourceFolder` renamed `SourceFile` / `SourceFolder` because HostLanguage is not JavaScript.

```rust
// from crates/isograph_cli/src/watch.rs
#[derive(Debug, Clone)]
pub enum SourceEventKind {
    CreateOrModify(PathBuf),
    Rename((PathBuf, PathBuf)),
    Remove(PathBuf),
}

pub enum ChangedFileKind {
    Config,
    SourceFile,
    SourceFolder,
}

pub type SourceFileEvent = (SourceEventKind, ChangedFileKind);
```

### Filesystem CLI

`IsographArgs` is `App::DaemonArgs`. Origin: `NoArgs`. Delta: `Filesystem`. clap groups: `ConfigFlag` vs `IsographArgs`.

```rust
// from crates/isograph_cli/src/lib.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
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

`run_daemon` passes `args.filesystem` and the `IsographProjectConfig` from `load_config`. It does not discard the config.

Before: `type DaemonArgs = NoArgs` and `daemon::run::<THostLanguage>(path, port_path)` after `load_config` whose value is ignored.

After:

```rust
// from crates/isograph_cli/src/lib.rs
    type DaemonArgs = IsographArgs;

    fn run_daemon(id: &ConfigFlag, args: &IsographArgs) {
        let (path, instance) = match discover::instance_for_config_path(id.config.as_deref()) {
            Ok(pair) => pair,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "the config went away between naming this daemon and starting it"
                );
                return;
            }
        };
        let port_path = discover::port_file(instance.lock_file());
        let _ = std::fs::remove_file(port_path.reference());
        let config = match discover::load_config(path.reference()) {
            Ok(config) => config,
            Err(e) => {
                tracing::error!(error = %e, "could not load the config");
                return;
            }
        };
        crate::daemon::run::<THostLanguage>(path, port_path, args.filesystem, config);
    }
```

Drop `NoArgs` from the `freddie_cli` import.

Non-UTF8 `Absent` from the watcher: if the absolute path is not UTF-8, skip the post. Do not call `handle`. `relative_path_from_absolute_and_working_directory` `expect`s stringify.

## Start

Intern `CurrentWorkingDirectory` before any `DiskChanged` is posted. Bind the LSP socket, intern, start the watcher (watch roots, then boot walk), write the port file.

`discover` hands a canonical config file path. A file path has a parent. The type system does not. Same `expect` as `intern_config_directory`.

```rust
// from crates/isograph_cli/src/daemon.rs
use isograph_config::IsographProjectConfig;

use crate::watch::SourceFileEvent;
use crate::Filesystem;

pub fn run<THostLanguage: HostLanguage>(
    config_path: PathBuf,
    port_path: PathBuf,
    filesystem: Filesystem,
    config: IsographProjectConfig,
) {
    // runtime as today
    runtime.block_on(serve::<THostLanguage>(
        config_path,
        port_path,
        filesystem,
        config,
    ));
}

async fn serve<THostLanguage: HostLanguage>(
    config_path: PathBuf,
    port_path: PathBuf,
    filesystem: Filesystem,
    config: IsographProjectConfig,
) {
    let (event_tx, event_rx) = unbounded_channel::<IsographEvent>();
    let (effect_tx, effect_rx) = unbounded_channel::<IsographEffect>();
    let (watch_tx, watch_rx) = unbounded_channel::<Vec<SourceFileEvent>>();
    // bind listener, read port, as today

    let mut state = IsographState::<THostLanguage>::default();
    intern_config_directory(&mut state, config_path.reference());
    let config_directory = config_path
        .parent()
        .expect("a config file path has a parent directory")
        .to_owned();
    let _hold_watch = watch_tx.clone();

    let _watcher = match filesystem {
        Filesystem::Injected => None,
        Filesystem::Watch => {
            match crate::watch::start::<THostLanguage>(
                watch_tx,
                config_path.reference(),
                config.source_files.as_slice(),
            ) {
                Ok(watcher) => watcher.wrap_some(),
                Err(e) => {
                    tracing::error!(error = %e, "could not start the watcher");
                    return;
                }
            }
        }
    };

    // write port file, "isograph daemon up", SIGTERM, as today

    let _hold_events = event_tx.clone();
    tokio::select! {
        () = run_event_loop(state, event_rx, effect_tx) => {}
        () = run_effect_loop(effect_rx) => {}
        () = crate::lsp_socket::accept_loop(listener, event_tx.clone()) => {}
        () = ingest_watch_events::<THostLanguage>(watch_rx, event_tx, config_directory) => {}
    }
    let _ = std::fs::remove_file(port_path.reference());
    std::process::exit(0);
}

async fn ingest_watch_events<THostLanguage: HostLanguage>(
    mut watch_rx: UnboundedReceiver<Vec<SourceFileEvent>>,
    event_tx: UnboundedSender<IsographEvent>,
    config_directory: PathBuf,
) {
    while let Some(events) = watch_rx.recv().await {
        crate::watch::apply::<THostLanguage>(
            event_tx.reference(),
            config_directory.reference(),
            events,
        );
    }
}
```

`let _watcher` binds until `serve` returns. `let _ = watcher` drops the debouncer immediately. `_hold_watch` keeps `ingest_watch_events` from returning on `Injected`, same as `_hold_events` for the event loop.

Do not log `filesystem` on `isograph daemon up`.

`ingest_watch_events` is the analog of isograph's `while let Some(res) = file_system_receiver.recv().await` plus `update_sources`. It never returns while the watcher lives (`watch_tx` is held by the callback). `select!` ends on `Kill`.

## Watch

Copied from `create_debounced_file_watcher`. Delta: `watch_tx` is unbounded; send is sync; watch roots come from `source_files`; boot walk after `watch()`; errors are `WatchError` not `expect`.

```rust
// from crates/isograph_cli/src/watch.rs
use std::path::{Path, PathBuf};
use std::time::Duration;

use globset::Glob;
use isograph_compiler::{HostLanguage, SourceFileKind};
use isograph_config::ISOGRAPH_FOLDER;
use notify::event::{CreateKind, ModifyKind, RemoveKind, RenameMode};
use notify::{EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use prelude::Postfix;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, info, warn};

use crate::event::{DiskChanged, IsographEvent, Presence};

const DEBOUNCE: Duration = Duration::from_millis(100);

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
    #[error("invalid source_files glob {glob}: {source}")]
    Glob {
        glob: String,
        source: globset::Error,
    },
}

pub struct Watcher {
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
}

pub fn start<THostLanguage: HostLanguage>(
    watch_tx: UnboundedSender<Vec<SourceFileEvent>>,
    config_path: &Path,
    source_files: &[String],
) -> Result<Watcher, WatchError> {
    let config_directory = config_path
        .parent()
        .expect("a config file path has a parent directory");
    let globs = SourceGlobs::parse(source_files)?;
    let roots = globs.watch_roots(config_directory);
    let tx = watch_tx.clone();
    let globs_for_events = globs.clone();
    let config_path_for_events = config_path.to_owned();
    let mut debouncer = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| {
        match result {
            Ok(events) => {
                if let Some(source_file_events) = categorize_and_filter_events(
                    events.as_slice(),
                    globs_for_events.reference(),
                    config_path_for_events.reference(),
                ) {
                    let _ = tx.send(source_file_events);
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
    for root in &roots {
        let mode = if root.is_file() {
            RecursiveMode::NonRecursive
        } else {
            RecursiveMode::Recursive
        };
        debouncer.watch(root.reference(), mode).map_err(|source| {
            WatchError::Watch(WatchRoot {
                path: root.clone(),
                source,
            })
        })?;
    }
    let mut paths = Vec::new();
    for root in &roots {
        if root.is_dir() {
            visit_dirs_skipping_isograph(root.reference(), &mut |entry| {
                paths.push(entry.path());
            });
        } else if root.is_file() {
            paths.push(root.clone());
        }
    }
    let boot: Vec<SourceFileEvent> = paths
        .into_iter()
        .filter_map(|path| {
            categorize_path(
                path.reference(),
                globs.reference(),
                config_path,
            )
            .and_then(|kind| match kind {
                ChangedFileKind::SourceFile => (
                    SourceEventKind::CreateOrModify(path),
                    ChangedFileKind::SourceFile,
                )
                    .wrap_some(),
                _ => None,
            })
        })
        .collect();
    info!(n = boot.len(), "scan finished");
    if !boot.is_empty() {
        let _ = watch_tx.send(boot);
    }
    Watcher {
        _debouncer: debouncer,
    }
    .wrap_ok()
}
```

`new_debouncer(timeout, tick_rate, cb)` with `tick_rate: None` is the crate default. 100ms as in isograph. `DebouncedEvent` Deref to `notify::Event`.

### source_files globs

```rust
// from crates/isograph_cli/src/watch.rs
#[derive(Clone)]
struct SourceGlobs {
    patterns: Vec<GlobPattern>,
}

#[derive(Clone)]
struct GlobPattern {
    raw: String,
    exclude: ExcludeGlob,
    matcher: globset::GlobMatcher,
}

#[derive(Clone, Copy)]
enum ExcludeGlob {
    Include,
    Exclude,
}

impl SourceGlobs {
    fn parse(source_files: &[String]) -> Result<Self, WatchError> {
        let patterns = source_files
            .iter()
            .map(|raw| {
                let (exclude, glob_str) = match raw.strip_prefix('!') {
                    Some(rest) => (ExcludeGlob::Exclude, rest),
                    None => (ExcludeGlob::Include, raw.as_str()),
                };
                let matcher = Glob::new(glob_str)
                    .map_err(|source| WatchError::Glob {
                        glob: raw.clone(),
                        source,
                    })?
                    .compile_matcher();
                GlobPattern {
                    raw: glob_str.to_owned(),
                    exclude,
                    matcher,
                }
                .wrap_ok()
            })
            .collect::<Result<Vec<_>, _>>()?;
        SourceGlobs { patterns }.wrap_ok()
    }

    fn contains(&self, relative: &Path) -> bool {
        self.patterns
            .iter()
            .fold(false, |allowed, pattern| {
                if pattern.matcher.is_match(relative) {
                    match pattern.exclude {
                        ExcludeGlob::Include => true,
                        ExcludeGlob::Exclude => false,
                    }
                } else {
                    allowed
                }
            })
    }

    fn watch_roots(&self, config_directory: &Path) -> Vec<PathBuf> {
        let mut roots: Vec<PathBuf> = self
            .patterns
            .iter()
            .filter_map(|pattern| match pattern.exclude {
                ExcludeGlob::Exclude => None,
                ExcludeGlob::Include => config_directory
                    .join(static_prefix(pattern.raw.as_str()))
                    .wrap_some(),
            })
            .collect();
        roots.sort();
        roots.dedup();
        roots
    }
}

fn static_prefix(glob: &str) -> PathBuf {
    let end = glob.find(['*', '?', '[', '{']).unwrap_or(glob.len());
    PathBuf::from(glob[..end].trim_end_matches('/'))
}
```

`static_prefix("src/**/*.ts")` is `src`. `static_prefix("**/*.ts")` is `""`, and `config_directory.join("")` is `config_directory`. `static_prefix("foo.ts")` is `foo.ts`.

`contains` last match wins. `["src/**/*.ts", "!src/**/*.test.ts"]`: `src/a.ts` true, `src/a.test.ts` false. Empty list: `fold` starts false, nothing intern.

Do not use HostLanguage to skip `node_modules`. A glob that does not include `node_modules` does not intern those files and does not watch them unless the static prefix is the config directory (`**/*.ts`).

### categorize

Copied from `categorize_and_filter_events` / `process_create_event` / `process_modify_event` / `process_remove_event` / `categorize_changed_file_and_filter_changes_in_artifact_directory`. Deltas named in the opening list.

```rust
// from crates/isograph_cli/src/watch.rs
fn categorize_and_filter_events(
    events: &[notify_debouncer_full::DebouncedEvent],
    globs: &SourceGlobs,
    config_path: &Path,
) -> Option<Vec<SourceFileEvent>> {
    if events.iter().any(|event| event.need_rescan()) {
        let config_directory = config_path.parent()?;
        let roots = globs.watch_roots(config_directory);
        if roots.is_empty() {
            return None;
        }
        return roots
            .into_iter()
            .map(|root| {
                (
                    SourceEventKind::CreateOrModify(root),
                    ChangedFileKind::SourceFolder,
                )
            })
            .collect::<Vec<_>>()
            .wrap_some();
    }
    let source_file_events: Vec<_> = events
        .iter()
        .filter_map(|event| match event.kind {
            EventKind::Create(create_kind) => {
                process_create_event(globs, config_path, create_kind, event.paths.as_slice())
            }
            EventKind::Modify(modify_kind) => {
                process_modify_event(globs, config_path, modify_kind, event.paths.as_slice())
            }
            EventKind::Remove(remove_kind) => {
                process_remove_event(globs, config_path, remove_kind, event.paths.as_slice())
            }
            EventKind::Any => event.paths.first().and_then(|path| {
                if path.exists() {
                    process_create_or_modify(globs, config_path, path)
                } else {
                    process_remove_path(globs, config_path, path)
                }
            }),
            _ => None,
        })
        .collect();
    if source_file_events.is_empty() {
        None
    } else {
        source_file_events.wrap_some()
    }
}
```

`DebouncedEvent` Deref to `notify::Event`. `need_rescan` is that method.

```rust
fn process_create_event(
    globs: &SourceGlobs,
    config_path: &Path,
    create_kind: CreateKind,
    paths: &[PathBuf],
) -> Option<SourceFileEvent> {
    match create_kind {
        CreateKind::File => paths
            .first()
            .and_then(|path| process_create_or_modify(globs, config_path, path)),
        CreateKind::Folder => paths.first().and_then(|path| {
            categorize_path(path, globs, config_path).map(|kind| {
                (SourceEventKind::CreateOrModify(path.clone()), kind)
            })
        }),
        _ => None,
    }
}

fn process_modify_event(
    globs: &SourceGlobs,
    config_path: &Path,
    modify_kind: ModifyKind,
    paths: &[PathBuf],
) -> Option<SourceFileEvent> {
    match modify_kind {
        ModifyKind::Data(_) => {
            let path = paths.first()?;
            if path.is_file() {
                process_create_or_modify(globs, config_path, path)
            } else {
                None
            }
        }
        ModifyKind::Any => {
            let path = paths.first()?;
            if path.exists() {
                process_create_or_modify(globs, config_path, path)
            } else {
                process_remove_path(globs, config_path, path)
            }
        }
        ModifyKind::Name(RenameMode::Any) => {
            let path = paths.first()?;
            if path.exists() {
                process_create_or_modify(globs, config_path, path)
            } else {
                process_remove_path(globs, config_path, path)
            }
        }
        ModifyKind::Name(RenameMode::Both) => {
            let from = paths.first()?;
            let to = paths.get(1)?;
            categorize_path(to, globs, config_path).map(|kind| {
                (
                    SourceEventKind::Rename((from.clone(), to.clone())),
                    kind,
                )
            })
        }
        ModifyKind::Name(RenameMode::From) => {
            paths.first().and_then(|path| process_remove_path(globs, config_path, path))
        }
        ModifyKind::Name(RenameMode::To) => {
            paths
                .first()
                .and_then(|path| process_create_or_modify(globs, config_path, path))
        }
        _ => None,
    }
}

fn process_remove_event(
    globs: &SourceGlobs,
    config_path: &Path,
    remove_kind: RemoveKind,
    paths: &[PathBuf],
) -> Option<SourceFileEvent> {
    match remove_kind {
        RemoveKind::File | RemoveKind::Folder | RemoveKind::Any => {
            paths.first().and_then(|path| process_remove_path(globs, config_path, path))
        }
        RemoveKind::Other => None,
    }
}
```

```rust
fn process_create_or_modify(
    globs: &SourceGlobs,
    config_path: &Path,
    path: &Path,
) -> Option<SourceFileEvent> {
    categorize_path(path, globs, config_path)
        .map(|kind| (SourceEventKind::CreateOrModify(path.to_owned()), kind))
}

fn process_remove_path(
    globs: &SourceGlobs,
    config_path: &Path,
    path: &Path,
) -> Option<SourceFileEvent> {
    categorize_path(path, globs, config_path)
        .map(|kind| (SourceEventKind::Remove(path.to_owned()), kind))
}

fn categorize_path(
    path: &Path,
    globs: &SourceGlobs,
    config_path: &Path,
) -> Option<ChangedFileKind> {
    if path == config_path {
        return ChangedFileKind::Config.wrap_some();
    }
    let config_directory = config_path.parent()?;
    let relative = pathdiff::diff_paths(path, config_directory)?;
    if !globs.contains(relative.reference()) {
        return None;
    }
    if path.is_file() {
        ChangedFileKind::SourceFile.wrap_some()
    } else {
        ChangedFileKind::SourceFolder.wrap_some()
    }
}
```

`categorize_path` does not call `source_file_kind`. isograph's categorize does not check extensions; `read_files_in_folder` does. `apply` / the boot walk call `source_file_kind` when reading a file.

Config events: `apply` ignores `ChangedFileKind::Config`. No reload.

### Walk

Copied from `visit_dirs_skipping_isograph`. Warn on `read_dir` error instead of returning `io::Result` up to a diagnostic.

```rust
// from crates/isograph_cli/src/watch.rs
fn visit_dirs_skipping_isograph(dir: &Path, cb: &mut dyn FnMut(&std::fs::DirEntry)) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            warn!(error = %e, path = %dir.display(), "walk");
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                warn!(error = %e, path = %dir.display(), "walk");
                continue;
            }
        };
        let path = entry.path();
        if path.is_dir() {
            if !dir.ends_with(ISOGRAPH_FOLDER) {
                visit_dirs_skipping_isograph(path.reference(), cb);
            }
        } else {
            cb(&entry);
        }
    }
}
```

A directory symlink to an ancestor loops. isograph does not detect that. Same here.

### apply

Copied from `update_sources`. Reads files here, not in the notify callback. Posts `DiskChanged`.

```rust
// from crates/isograph_cli/src/watch.rs
pub fn apply<THostLanguage: HostLanguage>(
    event_tx: &UnboundedSender<IsographEvent>,
    config_directory: &Path,
    events: Vec<SourceFileEvent>,
) {
    for (kind, changed) in events {
        match changed {
            ChangedFileKind::Config => {}
            ChangedFileKind::SourceFile => match kind {
                SourceEventKind::CreateOrModify(path) => {
                    post_file::<THostLanguage>(event_tx, config_directory, path.reference());
                }
                SourceEventKind::Rename((from, to)) => {
                    post_absent(event_tx, from.reference());
                    post_file::<THostLanguage>(event_tx, config_directory, to.reference());
                }
                SourceEventKind::Remove(path) => {
                    post_absent(event_tx, path.reference());
                }
            },
            ChangedFileKind::SourceFolder => match kind {
                SourceEventKind::CreateOrModify(folder) => {
                    scan_folder::<THostLanguage>(event_tx, config_directory, folder.reference());
                }
                SourceEventKind::Rename((from, to)) => {
                    post_absent(event_tx, from.reference());
                    scan_folder::<THostLanguage>(event_tx, config_directory, to.reference());
                }
                SourceEventKind::Remove(path) => {
                    post_absent(event_tx, path.reference());
                }
            },
        }
    }
}

fn scan_folder<THostLanguage: HostLanguage>(
    event_tx: &UnboundedSender<IsographEvent>,
    config_directory: &Path,
    folder: &Path,
) {
    let mut paths = Vec::new();
    visit_dirs_skipping_isograph(folder, &mut |entry| {
        paths.push(entry.path());
    });
    for path in paths {
        post_file::<THostLanguage>(event_tx, config_directory, path.reference());
    }
}

fn post_file<THostLanguage: HostLanguage>(
    event_tx: &UnboundedSender<IsographEvent>,
    config_directory: &Path,
    path: &Path,
) {
    let path = match path.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            warn!(error = %e, path = %path.display(), "could not canonicalize");
            post_absent(event_tx, path);
            return;
        }
    };
    let metadata = match std::fs::metadata(path.reference()) {
        Ok(metadata) => metadata,
        Err(e) => {
            warn!(error = %e, path = %path.display(), "could not stat");
            post_absent(event_tx, path.reference());
            return;
        }
    };
    if !metadata.is_file() {
        return;
    }
    let relative = match pathdiff::diff_paths(path.reference(), config_directory) {
        Some(relative) => relative,
        None => return,
    };
    match THostLanguage::source_file_kind(relative.reference()) {
        SourceFileKind::NotSource => return,
        SourceFileKind::Source => {}
    }
    if path.to_str().is_none() {
        warn!(path = %path.display(), "skipping non-UTF8 path");
        return;
    }
    let contents = match std::fs::read_to_string(path.reference()) {
        Ok(contents) => contents,
        Err(e) => {
            warn!(error = %e, path = %path.display(), "could not read");
            post_absent(event_tx, path.reference());
            return;
        }
    };
    debug!(path = %path.display(), "disk present");
    post(
        event_tx,
        DiskChanged {
            path,
            presence: Presence::Present(contents),
        },
    );
}
```

A fifo named `a.ts`: `metadata.is_file()` is false on Unix. Skip. Do not `read_to_string`.

```rust
fn post_absent(event_tx: &UnboundedSender<IsographEvent>, path: &Path) {
    let path = match path.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            if path.is_absolute() {
                path.to_owned()
            } else {
                return;
            }
        }
    };
    if path.to_str().is_none() {
        warn!(path = %path.display(), "skipping non-UTF8 path");
        return;
    }
    post(
        event_tx,
        DiskChanged {
            path,
            presence: Presence::Absent,
        },
    );
}

fn post(event_tx: &UnboundedSender<IsographEvent>, change: DiskChanged) {
    let _ = event_tx.send(IsographEvent::DiskChanged(change));
}
```

Posted `Present` paths are absolute and canonical. Posted `Absent` paths are absolute; canonical when canonicalize succeeded.

## Tests (change 2)

### TypeScript `source_file_kind`

- `src/a.ts`, `src/a.tsx`, `src/a.js`, `src/a.jsx`, `a.ts`, `src/a.d.ts` are `Source`.
- `src/a.rs`, `src/a.json`, `src/a.mjs`, `src/a.mts`, `src/a.graphql`, empty path are `NotSource`.
- `node_modules/pkg/index.ts` is `Source` (extension `ts`). The glob, not HostLanguage, keeps it out.
- `src/__isograph/foo.ts` is `Source` by extension. The walker does not visit `__isograph`.

### `ISOGRAPH_FOLDER`

Equals `"__isograph"`.

### Globs

- `["src/**/*.ts"]` contains `src/a.ts`, not `src/a.tsx`, not `lib/a.ts`, not `src/a.test.ts` unless that glob includes it.
- `["src/**/*.ts", "!src/**/*.test.ts"]` contains `src/a.ts`, not `src/a.test.ts`.
- `[]` contains nothing.
- `static_prefix("src/**/*.ts")` is `src`. `static_prefix("**/*.ts")` is empty. `static_prefix("foo.ts")` is `foo.ts`.

### `database.rs` `TestHostLanguage`

`source_file_kind` returns `Source`. Existing insert/remove tests stay.

### `watch.rs`

A `#[cfg(test)]` host: `source_file_kind` is `Source` iff the extension is `in`.

Unit tests of `categorize_and_filter_events` / `apply` with a fake channel. `source_files: ["**/*.in"]`. Config path is `temp/isograph.config.json`. Write files to disk for Present cases.

- `CreateKind::File` of `src/a.in`: after `apply`, one `Present` with those contents.
- `CreateKind::File` of `src/b.rs`: no event (`**/*.in` does not match; also `NotSource`).
- `CreateKind::File` of `isograph.config.json` at the config path: no `DiskChanged` (`Config`).
- `CreateKind::Folder` of a dir containing `a.in` and `b.rs`: one `Present` of `a.in`.
- `ModifyKind::Data` of a directory: no event.
- `ModifyKind::Data` of `src/a.in`: `Present`.
- `RemoveKind::File` of a path: `Absent`.
- `RemoveKind::Folder` of a path: `Absent`.
- `RenameMode::Both`: `Absent` of from, `Present` of to.
- `RenameMode::Any` exists: `Present`. does not exist: `Absent`.
- `EventKind::Any` exists `Source` file: `Present`. missing: `Absent`.
- `ModifyKind::Any` same.
- `Flag::Rescan`: `apply` walks each watch root; `a.in` is `Present` again.
- Empty `paths`: no event.
- Empty file: `Present` of `""`.
- Missing path on `CreateKind::File`: `categorize` may still yield `SourceFolder` (`!is_file`). `apply` `post_file` stats, fails, posts `Absent`.
- Failed `read_to_string`: chmod 0 on a matching file if the user is not root; otherwise skip this case on that platform. After a successful Present, a second CreateOrModify that cannot be read posts `Absent`. The interned file is gone after `handle`.
- `visit_dirs_skipping_isograph`: `src/a.in` is visited; `src/__isograph/c.in` is not.
- `source_files: []`: boot list is empty; `watch_roots` is empty.

Live notify, same crate, one test: temp dir, `source_files: ["src/**/*.in"]`, `watch::start`, intern_config_directory, `ingest` via `apply` on recvd events. Write `src/a.in` before `start` (boot) or after (notify). `handle` the `DiskChanged`. `disk_file` of `src/a.in` is those contents. Deadline 10s. This is the test that pins intern against disk. Do not add a production API only this test calls.

### e2e `cli.rs`

Every existing `Daemon::start` passes `--filesystem injected`.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
        let output = daemon.isograph(["start", "--filesystem", "injected"].reference());
```

`Daemon::start_watch(source_files_json: &str)` writes that config and runs `["start"]`.

- Empty `source_files`, Watch: log has `scan finished` and `isograph daemon up`. `n` is 0.
- Config `{"source_files":["src/**/*.ts"]}`. Write `src/Home.ts` with `export const Home = iso(\`entrypoint Query.HomeRoute\`)` before start. Watch. Log contains `scan finished` and `disk present` and `Home.ts`. Then write `src/Other.ts` the same way. Log contains `Other.ts` within 10s. Then write `src/skip.rs`. Log does not gain a `disk present` for `skip.rs` within the poll. Then write `src/Home.test.ts` if the glob is `src/**/*.ts` with no negation: it will intern (extension `ts`). Use `["src/**/*.ts","!src/**/*.test.ts"]` and assert `Home.test.ts` is not logged as `disk present`.

`isograph start --help` contains `filesystem`.

`a_second_start_adopts_the_running_daemon` still calls `["start"]` against an Injected daemon.

## Cargo

```toml
# from crates/isograph_cli/Cargo.toml
globset = "0.3"
notify = { workspace = true }
notify-debouncer-full = { workspace = true }
pathdiff = { workspace = true }
```

```toml
# from crates/isograph_cli/src/lib.rs (modules)
mod watch;
```

`isograph_cli` already depends on `isograph_config`. Use `ISOGRAPH_FOLDER` from there.

No `ignore` crate. No `directory_walk`.

## Call sites

- `isograph start --filesystem watch|injected` -> `DaemonArgs` -> `run_daemon` -> `daemon::run`.
- `Filesystem::Watch` -> `watch::start` (watch roots, then boot walk).
- notify callback -> `categorize_and_filter_events` -> `watch_tx`.
- `ingest_watch_events` -> `apply` -> `event_tx.send(DiskChanged)`.
- `Filesystem::Injected` -> no debouncer, no boot walk.
- event loop -> `handle`. Same `handle` for watcher posts and for `isograph send`.
