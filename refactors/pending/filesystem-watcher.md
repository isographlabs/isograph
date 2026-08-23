# Filesystem watcher

Requires filesystem-events.md (landed), interned-source-path.md (landed), config-source-files.md (landed), lsp-port.md (landed), and `docs-website/docs/design-docs/event-model.md`.

This is isograph's watcher. The code is `crates/isograph_compiler/src/watch.rs`, `read_files.rs`, and the `update_sources` match in `source_files.rs`. It lives in `isograph_cli` because i2 posts `DiskChanged` instead of mutating the db on the notify thread.

Origin of the notify loop: `create_debounced_file_watcher` and `categorize_and_filter_events`. Origin of the boot walk: `read_files_in_folder` / `visit_dirs_skipping_isograph`. Origin of folder delete: `remove_iso_literals_from_path`. Origin of ingest: `update_sources` / `handle_update_source_file` / `handle_update_source_folder`. Origin of `--watch`: isograph `CompileCommand.watch`. Origin of `Filesystem::Watch` / `Filesystem::Injected`: event-model.md.

Deltas from those files, exhaustive:

- The notify callback sends `SourceFileEvent`. A loop in `serve` reads contents and posts `DiskChanged` on `event_tx`. `handle` intern. isograph's callback sends, then `update_sources` writes the db on the recv loop. Same split, `DiskChanged` instead of `db.insert_iso_literal`.
- `Filesystem` instead of `--watch: bool`. Default `Watch`. `Injected` does not create a debouncer.
- `source_files` is membership. isograph used `path.starts_with(project_root)`. A relative path is in scope if the ordered glob list says so (`!` excludes, last match wins). The watch/walk root for a positive glob is the static prefix before the first `*`, `?`, `[`, or `{` (the analog of `project_root`). `**/*.ts` has an empty prefix and watches the config directory. `[]` watches nothing and intern nothing.
- The extension check in `read_files_in_folder` is `HostLanguage::source_file_kind`. One associated function, two-variant enum. Not a memo. The CLI is already generic over `THostLanguage` and does not depend on `isograph_extract_typescript`. A second trait in `isograph_compiler` would be the same `&Path` on the compiler crate with another name. The method is not a pico memo and does not take the db. `extract_iso_literals` stays the memo over interned paths.
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

Change 1 is file vs folder on `DiskChanged`, and two remove functions. `apply` already knows `SourceFile` vs `SourceFolder`. `handle` has to know too. `remove_disk_file` stays exact. `remove_disk_files_from_path` is isograph's `remove_iso_literals_from_path`. `isograph send` of `FolderRemoved` is the reader so the new function is not unused.

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

Default is `Watch`. CI e2e uses `Injected`. After change 1, `isograph send` of `FolderRemoved` of a directory path removes interned files under that path. File `Absent` still removes one key.

## Change 1: file vs folder on DiskChanged

`apply` (change 2) and isograph `update_sources` already branch file vs folder. A directory is not a `DiskFile`. Notify of a folder delete is one `Remove`, not one per child. `handle` must see that distinction. `remove_disk_file` is the wrong name for the folder case and the wrong function.

Origin of two methods: isograph `remove_iso_literal` and `remove_iso_literals_from_path`. Origin of `DiskChanged` as path plus `Presence`: event-model.md / filesystem-events.md. Delta: `DiskChanged` is an enum; `FolderRemoved` cannot carry contents; `remove_disk_files_from_path` uses `Path::starts_with` (isograph uses string `starts_with`, which treats `src` as a prefix of `src2`).

Before:

```rust
// from crates/isograph_cli/src/event.rs
pub struct DiskChanged {
    pub path: PathBuf,
    pub presence: Presence,
}

pub enum Presence {
    Present(String),
    Absent,
}
```

After:

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum DiskChanged {
    File(DiskFileChanged),
    FolderRemoved(FolderRemoved),
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct DiskFileChanged {
    pub path: PathBuf,
    pub presence: Presence,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct FolderRemoved {
    pub path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum Presence {
    Present(String),
    Absent,
}
```

Wire. Existing send frames break; update them.

```json
{"kind":"DiskChanged","value":{"File":{"path":"/tmp/proj/src/a.ts","presence":{"Present":"export const a = 1;\n"}}}}
{"kind":"DiskChanged","value":{"File":{"path":"/tmp/proj/src/a.ts","presence":"Absent"}}}
{"kind":"DiskChanged","value":{"FolderRemoved":{"path":"/tmp/proj/src"}}}
```

`lsp_socket.rs` tests that construct `DiskChanged { path, presence }` become `DiskChanged::File(DiskFileChanged { ... })`.

Before `handle_disk_changed` matches `change.presence`. After:

```rust
// from crates/isograph_cli/src/state.rs
fn handle_disk_changed<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    change: DiskChanged,
) {
    match change {
        DiskChanged::File(change) => {
            let path = relative_path_to_source_file(state, &change.path);
            match change.presence {
                Presence::Present(contents) => {
                    state.insert_disk_file(path, contents);
                }
                Presence::Absent => {
                    state.remove_disk_file(path);
                }
            }
        }
        DiskChanged::FolderRemoved(folder) => {
            let path = relative_path_to_source_file(state, &folder.path);
            state.remove_disk_files_from_path(path);
        }
    }
}
```

`remove_disk_file` is unchanged: one map key.

```rust
// from crates/isograph_compiler/src/database.rs
    pub fn remove_disk_files_from_path(&mut self, path: RelativePathToSourceFile) {
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

Add `use std::path::Path`. Empty relative path: `pathdiff` of the config directory against itself is `""`. `Path::new("src/a.ts").starts_with(Path::new(""))` is true. `FolderRemoved` of the config directory removes every interned file. `FolderRemoved` of a never-interned path that is not a prefix of any key is a no-op. File `Absent` of `/tmp/proj/src` removes only interned `src`, not `src/a.ts`.

### Tests

`database.rs`: existing `remove_disk_file` tests stay exact. Add `remove_disk_files_from_path`:

- intern `src/a.ts`, `src/b.ts`, `src2/c.ts`; `remove_disk_files_from_path` of `src`; `src/a.ts` and `src/b.ts` gone; `src2/c.ts` remains.
- intern `src/a.ts`; `remove_disk_files_from_path` of `src/a.ts.bak` leaves `src/a.ts`.
- intern `src/a.ts`; `remove_disk_files_from_path` of interned `""`; `src/a.ts` gone.

`state.rs` handle, `intern_config_directory` of `/tmp/proj/isograph.config.json`. Existing Present / Absent tests use `DiskChanged::File`. Add:

- File Present of three files, `FolderRemoved` of `/tmp/proj/src`: `src/a.ts` and `src/b.ts` gone, `src2/c.ts` remains. `handle` returns `Vec::new()`.
- `FolderRemoved` of `/tmp/proj`: every interned file gone.
- `FolderRemoved` of `/tmp/proj/never`: no-op.
- File `Absent` of `/tmp/proj/src` after Present of `/tmp/proj/src/a.ts`: `src/a.ts` remains.

`cli.rs` send Present then Absent: the new File JSON. Add send of `FolderRemoved` of `/tmp/proj/src` after Present of a file under it: exit 0.

`docs-website/docs/design-docs/event-model.md` `DiskChanged` struct becomes this enum. Dispatch: `File` Present intern, `File` Absent `remove_disk_file`, `FolderRemoved` `remove_disk_files_from_path`.

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

`HostLanguage` is the host seam the CLI is already generic over. Pico forbids `PathBuf` on source keys and memo arguments. This method is neither: the outer walker calls it with a relative `Path` before intern. Putting the same function on a new trait next to `HostLanguage` duplicates the bound `isograph_cli` already threads.

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
        () = run_filesystem_watcher::<THostLanguage>(watch_rx, event_tx, config_directory) => {}
    }
    let _ = std::fs::remove_file(port_path.reference());
    std::process::exit(0);
}

async fn run_filesystem_watcher<THostLanguage: HostLanguage>(
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

`let _watcher` binds until `serve` returns. `let _ = watcher` drops the debouncer immediately. `_hold_watch` keeps `run_filesystem_watcher` from returning on `Injected`, same as `_hold_events` for the event loop.

Do not log `filesystem` on `isograph daemon up`.

`run_filesystem_watcher` is the analog of isograph's `while let Some(res) = file_system_receiver.recv().await` plus `update_sources`. It never returns while the watcher lives (`watch_tx` is held by the callback). `select!` ends on `Kill`.

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

use crate::event::{DiskChanged, DiskFileChanged, FolderRemoved, IsographEvent, Presence};

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

A file glob does not match the directory that contains the files. `src` does not match `src/**/*.ts`. Folder create/delete uses watch-root membership, not `contains`. Otherwise deleting `src` is dropped and interned `src/a.ts` stays.

Do not use HostLanguage to skip `node_modules`. A glob that does not include `node_modules` does not intern those files and does not watch them unless the static prefix is the config directory (`**/*.ts`). That last glob is the Linux `max_user_watches` case isograph avoided by watching `project_root` (usually `./src`). The user wrote that glob; the daemon watches the config directory. `watch()` returning `Err` still fails start before the port file.

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
    let path = paths.first()?;
    let kind = match remove_kind {
        RemoveKind::File => categorize_file(path, globs, config_path)?,
        RemoveKind::Folder => categorize_folder(path, globs, config_path)?,
        RemoveKind::Any => categorize_path(path, globs, config_path)?,
        RemoveKind::Other => return None,
    };
    (SourceEventKind::Remove(path.clone()), kind).wrap_some()
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
    if path.is_file() {
        categorize_file(path, globs, config_path)
    } else {
        categorize_folder(path, globs, config_path)
    }
}

fn categorize_file(
    path: &Path,
    globs: &SourceGlobs,
    config_path: &Path,
) -> Option<ChangedFileKind> {
    if path == config_path {
        return ChangedFileKind::Config.wrap_some();
    }
    let config_directory = config_path.parent()?;
    let relative = pathdiff::diff_paths(path, config_directory)?;
    if globs.contains(relative.reference()) {
        ChangedFileKind::SourceFile.wrap_some()
    } else {
        None
    }
}

fn categorize_folder(
    path: &Path,
    globs: &SourceGlobs,
    config_path: &Path,
) -> Option<ChangedFileKind> {
    if path == config_path {
        return ChangedFileKind::Config.wrap_some();
    }
    let config_directory = config_path.parent()?;
    if globs
        .watch_roots(config_directory)
        .iter()
        .any(|root| path.starts_with(root))
    {
        ChangedFileKind::SourceFolder.wrap_some()
    } else {
        None
    }
}
```

`categorize_file` / `categorize_folder` do not call `source_file_kind`. isograph's categorize does not check extensions; `read_files_in_folder` does. `apply` / the boot walk call `source_file_kind` when reading a file.

`RemoveKind::File` uses `categorize_file` even though the path is already gone (`is_file()` is false). `RemoveKind::Folder` uses `categorize_folder` even though `src` does not match `src/**/*.ts`. `RemoveKind::Any` still uses `is_file()`: a vanished file is treated as a folder; `FolderRemoved` of `src/a.ts` then `remove_disk_files_from_path` drops that key and nothing else (`src/a.ts.bak` is a different last component). That is isograph's post-delete `is_file()` branch, with `Path::starts_with` instead of string prefix.

Config events: `apply` ignores `ChangedFileKind::Config`. The daemon does not reload config. An implementer does not invent a rescan on `isograph.config.json` Data.

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
                    post_file_absent(event_tx, from.reference());
                    post_file::<THostLanguage>(event_tx, config_directory, to.reference());
                }
                SourceEventKind::Remove(path) => {
                    post_file_absent(event_tx, path.reference());
                }
            },
            ChangedFileKind::SourceFolder => match kind {
                SourceEventKind::CreateOrModify(folder) => {
                    scan_folder::<THostLanguage>(event_tx, config_directory, folder.reference());
                }
                SourceEventKind::Rename((from, to)) => {
                    post_folder_removed(event_tx, from.reference());
                    scan_folder::<THostLanguage>(event_tx, config_directory, to.reference());
                }
                SourceEventKind::Remove(path) => {
                    post_folder_removed(event_tx, path.reference());
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
            post_file_absent(event_tx, path);
            return;
        }
    };
    let metadata = match std::fs::metadata(path.reference()) {
        Ok(metadata) => metadata,
        Err(e) => {
            warn!(error = %e, path = %path.display(), "could not stat");
            post_file_absent(event_tx, path.reference());
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
            post_file_absent(event_tx, path.reference());
            return;
        }
    };
    debug!(path = %path.display(), "disk present");
    post(
        event_tx,
        DiskChanged::File(DiskFileChanged {
            path,
            presence: Presence::Present(contents),
        }),
    );
}
```

A fifo named `a.ts`: `metadata.is_file()` is false on Unix. Skip. Do not `read_to_string`.

```rust
fn gone_path(path: &Path) -> Option<PathBuf> {
    match path.canonicalize() {
        Ok(path) => path.wrap_some(),
        Err(_) => {
            if path.is_absolute() {
                path.to_owned().wrap_some()
            } else {
                None
            }
        }
    }
    .and_then(|path| {
        if path.to_str().is_none() {
            warn!(path = %path.display(), "skipping non-UTF8 path");
            None
        } else {
            path.wrap_some()
        }
    })
}

fn post_file_absent(event_tx: &UnboundedSender<IsographEvent>, path: &Path) {
    let Some(path) = gone_path(path) else {
        return;
    };
    post(
        event_tx,
        DiskChanged::File(DiskFileChanged {
            path,
            presence: Presence::Absent,
        }),
    );
}

fn post_folder_removed(event_tx: &UnboundedSender<IsographEvent>, path: &Path) {
    let Some(path) = gone_path(path) else {
        return;
    };
    post(
        event_tx,
        DiskChanged::FolderRemoved(FolderRemoved { path }),
    );
}

fn post(event_tx: &UnboundedSender<IsographEvent>, change: DiskChanged) {
    let _ = event_tx.send(IsographEvent::DiskChanged(change));
}
```

Posted `File` `Present` paths are absolute and canonical. Posted `File` `Absent` and `FolderRemoved` paths are absolute; canonical when canonicalize succeeded.

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
- `["src/**/*.in"]` `contains` `src/a.in`, not `src`. `watch_roots` is `src`. `categorize_folder` of `src` is `SourceFolder`.

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
- `RemoveKind::File` of a path: `DiskChanged::File` `Absent`.
- `RemoveKind::Folder` of a path: `DiskChanged::FolderRemoved`.
- `RenameMode::Both` of files: `File` `Absent` of from, `File` `Present` of to.
- `RenameMode::Both` of folders: `FolderRemoved` of from, then `Present` of files under to.
- `RenameMode::Any` exists: `Present`. does not exist: `File` `Absent` or `FolderRemoved` from `categorize_path` (`!is_file` is folder).
- `EventKind::Any` exists `Source` file: `Present`. missing: `FolderRemoved` if `categorize` yields `SourceFolder`.
- `ModifyKind::Any` same.
- `Flag::Rescan`: `apply` walks each watch root; `a.in` is `Present` again.
- Empty `paths`: no event.
- Empty file: `Present` of `""`.
- Missing path on `CreateKind::File`: `is_file` is false, `SourceFolder`, `scan_folder` of a missing dir walks nothing.
- Failed `read_to_string`: after a successful `File` `Present`, a second CreateOrModify that cannot be read posts `File` `Absent`. The interned file is gone after `handle`. chmod 0 if the user is not root; otherwise skip this case on that platform.
- `visit_dirs_skipping_isograph`: `src/a.in` is visited; `src/__isograph/c.in` is not.
- `source_files: []`: boot list is empty; `watch_roots` is empty.

Live notify, same crate. Temp dir, `source_files: ["src/**/*.in"]`, intern_config_directory, `watch::start`, `apply` on recvd events, `handle`. Deadline 10s. Do not add a production API only these tests call.

- Boot: write `src/a.in` before `start`. After `scan finished`, `disk_file` of `src/a.in` is those contents.
- Notify: start on an empty `src/`, then write `src/a.in`. `disk_file` of `src/a.in` is those contents.

The binary e2e cannot read intern (no query verb). These crate tests are the intern pin. The e2e log line `disk present` only shows the watcher posted.

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
- `run_filesystem_watcher` -> `apply` -> `event_tx.send(DiskChanged)`.
- `Filesystem::Injected` -> no debouncer, no boot walk.
- event loop -> `handle`. Same `handle` for watcher posts and for `isograph send`.
