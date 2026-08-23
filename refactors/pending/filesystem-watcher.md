# Filesystem watcher

Requires filesystem-events.md (landed), interned-source-path.md (landed), config-source-files.md (landed), lsp-port.md (landed), and `docs-website/docs/design-docs/event-model.md`. The daemon recvs `DiskChanged` and interns `DiskFile`. This file adds a source that walks once, then observes the OS, and posts `DiskChanged` on the same `event_tx` the LSP session uses.

The watcher posts in-process. It does not run `isograph send` and it does not write to the LSP port. `isograph send` remains a source of the same event type. `handle`, `run_event_loop`, and the session do not know whether the watcher is running. Tests construct `DiskChanged` or send it. CI e2e starts the daemon without the watcher and sends events.

Origin of the notify loop: isograph `crates/isograph_compiler/src/watch.rs` `create_debounced_file_watcher` and `categorize_and_filter_events`. Origin of the boot walk: isograph `crates/isograph_compiler/src/read_files.rs` `read_files_in_folder` / `visit_dirs_skipping_isograph`. Origin of posting `DiskChanged` rather than mutating the db: `docs-website/docs/design-docs/event-model.md`. Origin of `--watch` as a CLI switch: isograph `crates/isograph_cli/src/opt.rs` `CompileCommand.watch`. Origin of `Filesystem::Watch` / `Filesystem::Injected`: event-model.md. Origin of folder-delete removing every interned path under that prefix: isograph `IsographDatabase::remove_iso_literals_from_path`.

Delta: OS events become `DiskChanged` with `Presence`; `handle` interns; no `ChangedFileKind`; no compile-on-change; `Filesystem` instead of isograph's `--watch` bool; the watcher lives in `isograph_cli` (outer), not `isograph_compiler`; which files are interned is `HostLanguage::source_file_kind`, not config globs and not a hardcoded extension list in the walker; which directories the boot walk enters is `HostLanguage::directory_walk`; one recursive watch on the config file's parent (isograph watches `config_location`, `project_root`, `schema`, and schema extensions); `UnboundedSender::send` from the notify thread (isograph uses a bounded tokio channel and `Handle::spawn` because its send is async); no panic on a notify payload (isograph panics if a create/modify/remove does not contain exactly one path, or a Both-rename exactly two); `Path::starts_with` for descendant removal (isograph uses string `starts_with`, which treats `src` as a prefix of `src2`); `Create(_)` of a directory scans that directory (isograph ignores `CreateKind::Folder`); `RenameMode::From` / `To` are handled (isograph ignores them).

Config `source_files` is not read. Glob matching from that field is later. This slice's filter is a host-language function of a path relative to the config directory.

One shippable change.

## What the user does

```
$ isograph start
# create, edit, rename, delete TypeScript files under the config directory
# each file HostLanguage::source_file_kind names Source becomes a DiskChanged, then a DiskFile
```

```
$ isograph start --filesystem injected
```

Writing a file on disk does not intern. `isograph send` of `DiskChanged` still intern. The event loop, `handle`, and the LSP session are the same in both modes.

Default is `Watch`. `Injected` is how tests that must not watch start the daemon.

## Types

Most important first.

### HostLanguage filter

Origin of a per-language file filter: isograph `read_files_in_folder` (extensions `ts` / `tsx` / `js` / `jsx`, skip paths containing `__isograph`) plus `visit_dirs_skipping_isograph`. Delta: the policy is two associated functions on `HostLanguage`, not a walk inside `isograph_compiler`. They are not memos. They do not take the database. The argument is a path relative to the config directory, not interned (`RelativePathToSourceFile` is handle's job) and not the absolute OS path (later glob matching is relative to the config directory). A file name without its parent components cannot skip `node_modules/pkg/index.ts`.

The two cases of `source_file_kind`: intern this path as a `DiskFile`, or do not. The two cases of `directory_walk`: the boot walk descends into this directory, or it does not. Not a `bool`.

Before:

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait HostLanguage: Send + Sync + Sized + 'static {
    type Error: std::fmt::Display + std::error::Error + Clone + PartialEq + Eq + 'static;
    type LiteralContext: Clone + PartialEq + Eq + Debug + 'static;

    fn extract_iso_literals(
        db: &IsographState<Self>,
        path: RelativePathToSourceFile,
    ) -> &Option<Vec<IsoLiteralExtraction<Self>>>;
}
```

After. Add `use std::path::Path`.

```rust
// from crates/isograph_compiler/src/host_language.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFileKind {
    Source,
    NotSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectoryWalk {
    Descend,
    Skip,
}

pub trait HostLanguage: Send + Sync + Sized + 'static {
    type Error: std::fmt::Display + std::error::Error + Clone + PartialEq + Eq + 'static;
    type LiteralContext: Clone + PartialEq + Eq + Debug + 'static;

    fn extract_iso_literals(
        db: &IsographState<Self>,
        path: RelativePathToSourceFile,
    ) -> &Option<Vec<IsoLiteralExtraction<Self>>>;

    fn source_file_kind(relative_path: &Path) -> SourceFileKind;

    fn directory_walk(relative_path: &Path) -> DirectoryWalk;
}
```

`source_file_kind` is consulted for files (boot walk and notify Present). `directory_walk` is consulted for child directories during the boot walk and during a directory Present (create or rename-to of a folder). The config directory itself is always walked and always watched. `directory_walk` of the empty relative path is not consulted.

Notify still fires for files under a `Skip` directory because the OS watch is recursive on the config directory. `source_file_kind` drops those Present events. `directory_walk` exists so the boot walk does not read `node_modules`. A `yarn install` still produces a burst of dropped notify events. Excluding `Skip` trees from the OS watch is later.

`handle` does not call these. `isograph send` may inject any path. Scope is the watcher's job.

### TypeScript policy

Origin: isograph `read_files_in_folder` extension match and `__isograph` skip. Delta: also `NotSource` / `Skip` when any path component is `node_modules`. isograph watches `project_root` (typically `./src`), so `node_modules` is outside that watch. This slice watches the config directory.

Add `use std::path::Path` and `use isograph_compiler::{DirectoryWalk, SourceFileKind}` at the top of that file. Add these two methods to the existing `impl HostLanguage for TypeScriptHostLanguage`.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    fn source_file_kind(relative_path: &Path) -> SourceFileKind {
        if path_has_skipped_component(relative_path) {
            return SourceFileKind::NotSource;
        }
        match relative_path.extension().and_then(|e| e.to_str()) {
            Some("ts" | "tsx" | "js" | "jsx") => SourceFileKind::Source,
            _ => SourceFileKind::NotSource,
        }
    }

    fn directory_walk(relative_path: &Path) -> DirectoryWalk {
        if path_has_skipped_component(relative_path) {
            DirectoryWalk::Skip
        } else {
            DirectoryWalk::Descend
        }
    }

fn path_has_skipped_component(relative_path: &Path) -> bool {
    relative_path
        .components()
        .any(|c| matches!(c.as_os_str().to_str(), Some("node_modules" | "__isograph")))
}
```

`path_has_skipped_component` is private to that file. Do not put `node_modules` or `__isograph` in `isograph_cli`.

`.d.ts` is `Source` (extension `ts`). `.mjs` / `.cjs` / `.mts` / `.json` / `.graphql` / `.rs` are `NotSource`. The empty relative path (the config directory itself) is `NotSource` as a file and would be `Descend` as a directory. A file `a.ts` at the config directory is `Source`.

### Filesystem CLI

`IsographArgs` is `App::DaemonArgs`. Origin: event-loop.md / config-discovery.md `NoArgs`. Delta: `Filesystem`. `start`, `restart`, and `daemon` flatten `DaemonArgs`. `status` / `logs` / `stop` do not. clap requires `Id` and `DaemonArgs` to have distinct group names. `ConfigFlag` and `IsographArgs` are different types.

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

Before:

```rust
// from crates/isograph_cli/src/lib.rs
    type DaemonArgs = NoArgs;

    fn run_daemon(id: &ConfigFlag, _: &NoArgs) {
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
        if let Err(e) = discover::load_config(path.reference()) {
            tracing::error!(error = %e, "could not load the config");
            return;
        }
        crate::daemon::run::<THostLanguage>(path, port_path);
    }
```

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
        if let Err(e) = discover::load_config(path.reference()) {
            tracing::error!(error = %e, "could not load the config");
            return;
        }
        crate::daemon::run::<THostLanguage>(path, port_path, args.filesystem);
    }
```

`load_config` still runs and still discards the value. A bad config still refuses to start. The watcher does not read `IsographProjectConfig`. Do not stash the config in `IsographState`.

Drop `NoArgs` from `use freddie_cli::{App, Instance, NoArgs}`.

`Injected` does not scan and does not watch. The LSP port listens in both modes. `event_tx` is the same channel.

### Descendant Absent

Origin: isograph `remove_iso_literals_from_path`. Delta: `Path::starts_with` (whole components), so interned `src` does not remove interned `src2`. One method: `Absent` of a path means that path is gone from the filesystem, so the interned key and every interned key under it go away. File delete of `src/a.ts` removes that key and would remove `src/a.ts/foo` if one existed. Directory delete of `src` removes `src` and `src/a.ts`.

This changes `isograph send` of `Absent` for a prefix path: `Absent` of `/tmp/proj/src` removes every interned file under `src`. `Absent` of `/tmp/proj/src/a.ts` still removes only that file (`src/a.ts.bak` is a different last component).

Before:

```rust
// from crates/isograph_compiler/src/database.rs
    pub fn remove_disk_file(&mut self, path: RelativePathToSourceFile) {
        if let Some(source_id) = self.get_disk_file_map_mut().tracked().0.remove(&path) {
            self.remove(source_id);
        }
    }
```

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

`AsRef<Path>` is on `RelativePathToSourceFile` via `string_key_newtype!`. Do not add `intern` as a production dependency of `isograph_compiler`. Collect then `remove` because `extract_if` already borrows the map. `handle_disk_changed` still calls `remove_disk_file`. Existing exact-path tests still pass.

### Start

`run_daemon` already has the canonical config path. Pass `filesystem` into `serve`. Intern `CurrentWorkingDirectory` before `watch::start` posts `DiskChanged`. Start the watcher after intern and after the LSP bind, before writing the port file, so a watch failure does not leave a port file.

The watch root is `config_path.parent()`, the same path `intern_config_directory` intern. Do not canonicalize that parent again. `config_path` is already canonical from `discover`. Posted `Present` paths are canonicalized in `post_present` so they pathdiff against that interned directory.

```rust
// from crates/isograph_cli/src/daemon.rs
use crate::Filesystem;

pub fn run<THostLanguage: HostLanguage>(
    config_path: PathBuf,
    port_path: PathBuf,
    filesystem: Filesystem,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            tracing::error!(error = %e, "could not start the tokio runtime");
            return;
        }
    };
    runtime.block_on(serve::<THostLanguage>(config_path, port_path, filesystem));
}

async fn serve<THostLanguage: HostLanguage>(
    config_path: PathBuf,
    port_path: PathBuf,
    filesystem: Filesystem,
) {
    let (event_tx, event_rx) = unbounded_channel::<IsographEvent>();
    let (effect_tx, effect_rx) = unbounded_channel::<IsographEffect>();
    let listener = match tokio::net::TcpListener::bind(std::net::SocketAddr::from((
        std::net::Ipv4Addr::LOCALHOST,
        0,
    )))
    .await
    {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!(error = %e, "could not bind the lsp socket");
            return;
        }
    };
    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(e) => {
            tracing::error!(error = %e, "could not read the lsp socket address");
            return;
        }
    };

    let mut state = IsographState::<THostLanguage>::default();
    intern_config_directory(&mut state, config_path.reference());

    let _watcher = match filesystem {
        Filesystem::Injected => None,
        Filesystem::Watch => {
            match crate::watch::start::<THostLanguage>(event_tx.clone(), config_path.reference()) {
                Ok(watcher) => watcher.wrap_some(),
                Err(e) => {
                    tracing::error!(error = %e, "could not start the watcher");
                    return;
                }
            }
        }
    };

    if let Err(e) = std::fs::write(port_path.reference(), format!("{port}\n")) {
        tracing::error!(
            error = %e,
            path = %port_path.display(),
            "could not write the event socket port"
        );
        return;
    }
    tracing::info!(config = %config_path.display(), port, "isograph daemon up");

    // SIGTERM task as today

    let _hold_events = event_tx.clone();
    tokio::select! {
        () = run_event_loop(state, event_rx, effect_tx) => {}
        () = run_effect_loop(effect_rx) => {}
        () = crate::lsp_socket::accept_loop(listener, event_tx) => {}
    }
    let _ = std::fs::remove_file(port_path.reference());
    std::process::exit(0);
}
```

Scan posts onto `event_tx` before `select!` recvs. Those frames sit in the unbounded channel. `intern_config_directory` has already run. `handle` of `DiskChanged` on a default database panics; that is why intern is before `start`.

`let _watcher` binds until the end of `serve`. `let _ = watcher` would drop the debouncer immediately and stop notify. Drop order after `select!` ends is reverse declaration: loops have finished, then `_hold_events`, then `_watcher`.

Do not log `filesystem` on `isograph daemon up`. Existing e2e asserts that line, `config`, and `port`. Whether the watcher started is not part of that record.

`run_event_loop` does not change. `handle` does not change except `remove_disk_file` as above. `lsp_socket` does not change.

Existing `Daemon::start` becomes `start` with `--filesystem injected`. Add `Daemon::start_watch` for the watch e2e.

### Watch

`notify_debouncer_full::DebouncedEvent` Deref to `notify::Event`, so `event.kind` and `event.paths` are the notify fields. Same as isograph.

```rust
// from crates/isograph_cli/src/watch.rs
use std::path::{Path, PathBuf};
use std::time::Duration;

use isograph_compiler::{DirectoryWalk, HostLanguage, SourceFileKind};
use notify::event::{ModifyKind, RenameMode};
use notify::{EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use prelude::Postfix;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

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
}

pub struct Watcher {
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
}

pub fn start<THostLanguage: HostLanguage>(
    event_tx: UnboundedSender<IsographEvent>,
    config_path: &Path,
) -> Result<Watcher, WatchError> {
    let config_directory = config_path
        .parent()
        .expect("a config file path has a parent directory");
    let mut paths = scan_files::<THostLanguage>(config_directory, config_directory);
    paths.sort();
    paths.dedup();
    info!(n = paths.len(), "scan finished");
    for path in paths {
        post_present(event_tx.reference(), path.reference());
    }
    let tx = event_tx.clone();
    let config_directory_for_events = config_directory.to_owned();
    let mut debouncer = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| {
        match result {
            Ok(events) => {
                for event in events {
                    dispatch::<THostLanguage>(
                        tx.reference(),
                        config_directory_for_events.reference(),
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
    debouncer
        .watch(config_directory, RecursiveMode::Recursive)
        .map_err(|source| {
            WatchError::Watch(WatchRoot {
                path: config_directory.to_owned(),
                source,
            })
        })?;
    Watcher {
        _debouncer: debouncer,
    }
    .wrap_ok()
}
```

The parent `expect` is the same invariant as `intern_config_directory`. The type system does not require a config `Path` to have a parent. `discover` hands `start` a canonical config file path; that path has a parent.

`new_debouncer(timeout, tick_rate, cb)` with `tick_rate: None` is the crate default. Workspace already pins `notify` 7 and `notify-debouncer-full` 0.4. isograph uses 100ms. The `Debouncer` type arguments are `RecommendedWatcher` and `RecommendedCache` as in isograph `create_debounced_file_watcher`. If inference can fill the field, omit the arguments.

The notify callback is not on the tokio runtime. `UnboundedSender::send` is not async. Do not `Handle::current().spawn`. That is the delta from isograph's bounded channel.

`HostLanguage` methods are associated functions. The move closure calls `THostLanguage::source_file_kind` / `directory_walk`. `HostLanguage` is `'static`. The host value is not cloned.

Scan then watch. A file created in that gap is missed until a later event. isograph compiles from its boot walk, then starts the watcher; same gap.

### Scan

Origin: isograph `visit_dirs_skipping_isograph` plus the extension filter in `read_files_in_folder`. Delta: skip and filter go through `HostLanguage`; the walk is in `isograph_cli`; return the file paths, then `start` / directory Present posts.

```rust
// from crates/isograph_cli/src/watch.rs
fn scan_files<THostLanguage: HostLanguage>(
    config_directory: &Path,
    dir: &Path,
) -> Vec<PathBuf> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            warn!(error = %e, path = %dir.display(), "walk");
            return Vec::new();
        }
    };
    entries
        .filter_map(|entry| match entry {
            Ok(entry) => entry.wrap_some(),
            Err(e) => {
                warn!(error = %e, path = %dir.display(), "walk");
                None
            }
        })
        .flat_map(|entry| {
            let path = entry.path();
            let relative = match relative_to_config(config_directory, path.reference()) {
                Some(relative) => relative,
                None => return Vec::new(),
            };
            if path.is_dir() {
                match THostLanguage::directory_walk(relative.reference()) {
                    DirectoryWalk::Descend => {
                        scan_files::<THostLanguage>(config_directory, path.reference())
                    }
                    DirectoryWalk::Skip => Vec::new(),
                }
            } else {
                match THostLanguage::source_file_kind(relative.reference()) {
                    SourceFileKind::Source => path.wrap_vec(),
                    SourceFileKind::NotSource => Vec::new(),
                }
            }
        })
        .collect()
}

fn relative_to_config(config_directory: &Path, absolute: &Path) -> Option<PathBuf> {
    pathdiff::diff_paths(absolute, config_directory)
}
```

`is_dir` / `is_file` / `exists` / `is_absolute` are std. `clippy::match_bool` is deny, so these are `if` / `else`, not `match`.

`scan_files` returns the paths `read_dir` produced. It does not canonicalize. `start` sorts and dedups before posting so the boot burst is stable. `n` in `scan finished` is the deduped length. `post_present` canonicalizes before posting.

A gitignored file that `source_file_kind` names `Source` is posted. There is no `ignore` crate. isograph does not consult gitignore either.

Symlinks: `is_dir` / `is_file` follow. A directory symlink to an ancestor loops. isograph does not detect that. Same here.

### Dispatch

```rust
// from crates/isograph_cli/src/watch.rs
fn dispatch<THostLanguage: HostLanguage>(
    event_tx: &UnboundedSender<IsographEvent>,
    config_directory: &Path,
    kind: EventKind,
    paths: &[PathBuf],
) {
    match kind {
        EventKind::Create(_) | EventKind::Modify(ModifyKind::Data(_)) => {
            if let Some(path) = paths.first() {
                on_maybe_present::<THostLanguage>(event_tx, config_directory, path);
            }
        }
        EventKind::Remove(_) => {
            if let Some(path) = paths.first() {
                on_maybe_absent(event_tx, config_directory, path);
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
            if let Some(from) = paths.first() {
                on_maybe_absent(event_tx, config_directory, from);
            }
            if let Some(to) = paths.get(1) {
                on_maybe_present::<THostLanguage>(event_tx, config_directory, to);
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
            if let Some(from) = paths.first() {
                on_maybe_absent(event_tx, config_directory, from);
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
            if let Some(to) = paths.first() {
                on_maybe_present::<THostLanguage>(event_tx, config_directory, to);
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::Any)) => {
            if let Some(path) = paths.first() {
                if path.exists() {
                    on_maybe_present::<THostLanguage>(event_tx, config_directory, path);
                } else {
                    on_maybe_absent(event_tx, config_directory, path);
                }
            }
        }
        _ => {}
    }
}

fn on_maybe_present<THostLanguage: HostLanguage>(
    event_tx: &UnboundedSender<IsographEvent>,
    config_directory: &Path,
    path: &Path,
) {
    let path = match path.canonicalize() {
        Ok(path) => path,
        Err(_) => return,
    };
    if path.is_dir() {
        let relative = match relative_to_config(config_directory, path.reference()) {
            Some(relative) => relative,
            None => return,
        };
        match THostLanguage::directory_walk(relative.reference()) {
            DirectoryWalk::Skip => {}
            DirectoryWalk::Descend => {
                let mut paths = scan_files::<THostLanguage>(config_directory, path.reference());
                paths.sort();
                paths.dedup();
                for path in paths {
                    post_present(event_tx, path.reference());
                }
            }
        }
    } else if path.is_file() {
        let relative = match relative_to_config(config_directory, path.reference()) {
            Some(relative) => relative,
            None => return,
        };
        match THostLanguage::source_file_kind(relative.reference()) {
            SourceFileKind::NotSource => {}
            SourceFileKind::Source => post_present(event_tx, path.reference()),
        }
    }
}

fn on_maybe_absent(
    event_tx: &UnboundedSender<IsographEvent>,
    config_directory: &Path,
    path: &Path,
) {
    let path = match path.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            if path.is_absolute() {
                path.to_owned()
            } else {
                config_directory.join(path)
            }
        }
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
    let path = match path.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            warn!(error = %e, path = %path.display(), "could not canonicalize");
            return;
        }
    };
    if path.to_str().is_none() {
        warn!(path = %path.display(), "skipping non-UTF8 path");
        return;
    }
    let contents = match std::fs::read_to_string(path.reference()) {
        Ok(contents) => contents,
        Err(e) => {
            warn!(error = %e, path = %path.display(), "could not read");
            return;
        }
    };
    post(
        event_tx,
        DiskChanged {
            path,
            presence: Presence::Present(contents),
        },
    );
}

fn post(event_tx: &UnboundedSender<IsographEvent>, change: DiskChanged) {
    let _ = event_tx.send(IsographEvent::DiskChanged(change));
}
```

`dispatch` and `scan_files` stay private. Tests live in `watch.rs` as a child module and call `super::`.

Present is scoped: `NotSource` creates do not enter the map. Absent is not scoped: a deleted path often fails canonicalize, a directory has no extension so `source_file_kind` would be `NotSource`, and filtering Absent through `source_file_kind` would miss directory deletes. An Absent of a path that was never interned is a no-op in `handle`. Failed canonicalize after `Create` (vanished during debounce) drops the event; the following `Remove` posts `Absent`. `ModifyKind::Metadata` is ignored. Directory `Create` scans that directory. `RenameMode::From` / `To` are handled. `RenameMode::Any` copies isograph: exists then Present, else Absent.

Take `paths.first()` / `paths.get(1)` and skip. No panic on any notify payload.

Posted `Present` paths are absolute and canonical. Posted `Absent` paths are absolute; they are canonical when canonicalize succeeded. event-model.md says the event path is absolute and canonical. A vanished path cannot be canonicalized. Joining a relative notify path onto `config_directory` is the source resolving a relative path, never `handle`.

Non-UTF8 paths: `handle` `expect`s the relative path to stringify. Skip `post_present` when `path.to_str()` is `None`.

### Tests

#### TypeScript

In `crates/isograph_extract_typescript/src/lib.rs`:

- `src/a.ts`, `src/a.tsx`, `src/a.js`, `src/a.jsx` are `Source`.
- `a.ts` is `Source`.
- `src/a.d.ts` is `Source`.
- `src/a.rs`, `src/a.json`, `src/a.mjs`, `src/a.mts`, `src/a.graphql`, empty path are `NotSource`.
- `node_modules/pkg/index.ts` is `NotSource`.
- `src/node_modules/pkg/index.ts` is `NotSource`.
- `src/__isograph/foo.ts` is `NotSource`.
- `__isograph/foo.ts` is `NotSource`.
- `src` is `Descend`. Empty path is `Descend`.
- `node_modules`, `src/node_modules`, `__isograph`, `src/__isograph` are `Skip`.
- `src/components` is `Descend`.

#### `database.rs` `TestHostLanguage`

Implement the two new methods. `source_file_kind` returns `Source`. `directory_walk` returns `Descend`. The existing insert/remove tests do not call them.

Add:

- intern `src/a.ts` and `src/b.ts` and `src2/c.ts`; `remove_disk_file` of `src`; `src/a.ts` and `src/b.ts` gone; `src2/c.ts` remains; interned `src` itself gone if it was interned.
- intern `src/a.ts`; `remove_disk_file` of `src/a.ts`; that key gone. This is the existing test, still valid.
- intern `src/a.ts`; `remove_disk_file` of `src/a.ts.bak` (never present) leaves `src/a.ts`.

#### `state.rs` handle

`intern_config_directory` of `/tmp/proj/isograph.config.json`. Present `/tmp/proj/src/a.ts` and `/tmp/proj/src/b.ts` and `/tmp/proj/src2/c.ts`. Absent of `/tmp/proj/src`. `src/a.ts` and `src/b.ts` gone. `src2/c.ts` remains. `handle` returns `Vec::new()`.

#### `watch.rs`

A `#[cfg(test)]` host in that tests module. `source_file_kind` is `Source` iff the extension is `in`. `directory_walk` is `Skip` iff the last component is `skip`. `extract_iso_literals` returns `&NONE` as `database.rs` `TestHostLanguage` does. `Error` is a `thiserror` unit. `LiteralContext` is `()`.

Unit tests of `dispatch` with a fake channel and a temp tree. Drain the test channel. `config_directory` is the temp dir. Notify paths in these tests are absolute. `Create` uses `EventKind::Create(CreateKind::File)` for files and `EventKind::Create(CreateKind::Folder)` for directories. Write the file (or directory) to disk before a Present case, because `on_maybe_present` canonicalizes and stats.

- `Create` of `src/a.in` that exists: one `Present` with those contents. Canonical path.
- `Create` of `src/b.rs` that exists: no event.
- `Create` with an empty `paths` slice: no event.
- `Remove` of an absolute path: one `Absent`. The file may already be gone.
- `Remove` of a directory path: one `Absent` of that path.
- `RenameMode::Both` with two paths: `Absent` of from, `Present` of to (`to` exists and is `Source`).
- `RenameMode::Both` out of scope to in scope: `Absent` of from is posted and `Present` of to; the test asserts the pair.
- `RenameMode::From` of a path: one `Absent`.
- `RenameMode::To` of a `Source` file that exists: one `Present`.
- `RenameMode::Any` of a path that exists and is `Source`: `Present`.
- `RenameMode::Any` of a path that does not exist: `Absent`.
- Empty file: `Present` with `contents == ""`.
- `Create` of a path that does not exist: no event (canonicalize fails).
- `Create` of a directory that contains `a.in` and `b.rs`: one `Present` of `a.in`.
- `Create` of a `skip/` directory that contains `c.in`: no event.

Scan: write `src/a.in`, `src/b.rs`, `skip/c.in`. `node_modules` is not special for this host (not named `skip`). Give `scan_files` the config directory. Assert the returned paths, after canonicalize, are exactly the canonical `src/a.in`. `b.rs` and `skip/c.in` are absent. `scan_files` itself does not sort; the test may sort.

Do not start a real `notify` watcher in unit tests.

#### e2e `cli.rs`

`--filesystem injected` on every existing `Daemon::start`. `isograph send` of `DiskChanged` is how those tests intern files. They do not watch.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
        let output = daemon.isograph(["start", "--filesystem", "injected"].reference());
```

`Daemon::start_watch` is the same fixture except the argv is `["start"]` (default `Watch`). One test: that start, the log has `scan finished` and `isograph daemon up`. HOME isolation as today. Deadline 10s. The fixture's config is `{"source_files":[]}` and no `.ts` files, so `n` is 0.

`isograph start --help` contains `filesystem`. Do not assert the flag on top-level `--help`; DaemonArgs are flattened onto `start` / `restart` / `daemon`.

`a_second_start_adopts_the_running_daemon` still calls `["start"]` without the flag against an Injected daemon. The running daemon is adopted. DaemonArgs of the second invocation do not take effect (freddie_cli).

### Cargo

```toml
# from crates/isograph_cli/Cargo.toml
notify = { workspace = true }
notify-debouncer-full = { workspace = true }
pathdiff = { workspace = true }
```

```rust
// from crates/isograph_cli/src/lib.rs
mod config_path;
mod daemon;
mod discover;
mod effect;
mod event;
mod lsp_socket;
mod send;
mod state;
mod watch;
```

No `ignore` crate.

## Call sites

- `isograph start --filesystem watch|injected` -> `App::DaemonArgs` -> `run_daemon` -> `daemon::run`.
- `Filesystem::Watch` arm -> `watch::start`.
- `Filesystem::Injected` arm -> no scan, no notify.
- notify callback -> `dispatch` -> `event_tx.send`.
- scan -> `post_present` -> `event_tx.send`.
- event loop -> `handle` (filesystem-events.md). Same `handle` for watcher posts and for `isograph send`.
