# Filesystem watcher

Requires filesystem-events.md (landed), interned-source-path.md (landed), config-source-files.md (landed), lsp-port.md (landed), and `docs-website/docs/design-docs/event-model.md`.

This is isograph's watcher. Copy `crates/isograph_compiler/src/watch.rs`, `read_files.rs` `visit_dirs_skipping_isograph` / `read_files_in_folder`, and `source_files.rs` `update_sources`. It lives in `isograph_cli` because i2 posts `DiskChanged` instead of writing the db from the notify thread.

The notify callback categorizes and sends `SourceFileEvent`. `run_filesystem_watcher` reads file bytes and posts `DiskChanged`. That is isograph's callback-then-`update_sources` split.

Deltas from those files, exhaustive:

- `Filesystem` instead of `--watch: bool`. Default `Watch`. `Injected` does not create a debouncer.
- `source_files` is which files intern. isograph used `path.starts_with(project_root)` plus an extension check. Ordered globs, `!` excludes, last match wins. The watch/walk root is the config directory, isograph's `project_root` analog. Folder events under that directory are in scope even when the folder path itself does not match a file glob (`src` vs `src/**/*.ts`). `[]` intern nothing. Watching the config directory when it is the repo root is the `max_user_watches` case isograph avoided by pointing `project_root` at `./src`.
- The extension check in `read_files_in_folder` is `HostLanguage::should_skip_source_file`. One associated function, two-variant enum (`Skip` / `Keep`). Not a memo. Not a second trait: the CLI is already generic over `THostLanguage`.
- `__isograph` is skipped in the walker (`ISOGRAPH_FOLDER` on `isograph_config`). Not HostLanguage.
- No schema / schema-extension / artifact-directory watches. Config-file events are `ChangedFileKind::Config` and are dropped. The daemon does not reload config.
- Watch, then boot-walk. isograph compiled then watched and missed the gap.
- Copy `categorize_and_filter_events` / `process_create_event` / `process_modify_event` / `process_remove_event`. Then: `CreateKind::Folder` scans; `RenameMode::From` / `To`; `EventKind::Any` and `ModifyKind::Any` (not `Name`) like isograph's `RenameMode::Any`; `need_rescan()` re-walks the config directory; no panic on path count (`paths.first()` / `paths.get(1)`).
- `RemoveKind::File` uses the file glob after the path is gone. `RemoveKind::Folder` is in scope if the path is under the config directory.
- `apply` is `update_sources`: `SourceFile` -> `DiskChanged::File`, `SourceFolder` remove/rename-from -> `FolderRemoved`. Failed read of an interned file posts `File` `Absent`. Skip non-regular files and non-UTF8 paths; do not call `handle` with a path that will not stringify.
- `Path::starts_with` in `remove_disk_files_from_path`. isograph uses string `starts_with`.
- Unbounded send from the notify thread. isograph `channel(1)` plus `Handle::spawn`. `watch()` errors are `WatchError`, not `expect`.

`isograph send` still intern any path. Scope is the watcher's job. `handle`, `run_event_loop`, and the session do not know whether the watcher is running.

Two shippable changes. Prefactor first.

Change 1 is file vs folder on `DiskChanged`, and two remove functions. Change 2 is the watcher. Do not split HostLanguage, the flag, or globs out: they have no reader until ingest is wired.

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

`apply` (change 2) and isograph `update_sources` already branch file vs folder. A directory is not a `DiskFile`. Notify of a folder delete is one `Remove`, not one per child. `handle` must see that distinction.

Origin of two methods: isograph `remove_iso_literal` and `remove_iso_literals_from_path`. Origin of `DiskChanged` as path plus `Presence`: event-model.md. Delta: `DiskChanged` is an enum; `FolderRemoved` cannot carry contents; `remove_disk_files_from_path` uses `Path::starts_with`.

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

Add `use std::path::Path`. `FolderRemoved` of the config directory (relative `""`) removes every interned file. `FolderRemoved` of a never-interned path that is not a prefix of any key is a no-op. File `Absent` of `/tmp/proj/src` removes only interned `src`, not `src/a.ts`.

### Tests

`database.rs`: existing `remove_disk_file` tests stay exact. Add `remove_disk_files_from_path`:

- intern `src/a.ts`, `src/b.ts`, `src2/c.ts`; `remove_disk_files_from_path` of `src`; `src/a.ts` and `src/b.ts` gone; `src2/c.ts` remains.
- intern `src/a.ts`; `remove_disk_files_from_path` of `src/a.ts.bak` leaves `src/a.ts`.
- intern `src/a.ts`; `remove_disk_files_from_path` of interned `""`; `src/a.ts` gone.

`state.rs` handle, `intern_config_directory` of `/tmp/proj/isograph.config.json`. Existing Present / Absent tests use `DiskChanged::File`. Add:

- File Present of three files, `FolderRemoved` of `/tmp/proj/src`: `src/a.ts` and `src/b.ts` gone, `src2/c.ts` remains.
- `FolderRemoved` of `/tmp/proj`: every interned file gone.
- `FolderRemoved` of `/tmp/proj/never`: no-op.
- File `Absent` of `/tmp/proj/src` after Present of `/tmp/proj/src/a.ts`: `src/a.ts` remains.

`cli.rs` send Present then Absent: the new File JSON. Add send of `FolderRemoved` of `/tmp/proj/src` after Present of a file under it: exit 0.

`docs-website/docs/design-docs/event-model.md` `DiskChanged` struct becomes this enum.

## Change 2: watcher

Requires change 1.

### HostLanguage filter

Origin: the extension match in `read_files_in_folder`.

```rust
// from crates/isograph_compiler/src/host_language.rs
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipSourceFile {
    Skip,
    Keep,
}
```

Add `fn should_skip_source_file(relative_path: &Path) -> SkipSourceFile` to `HostLanguage`. TypeScript:

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    fn should_skip_source_file(relative_path: &Path) -> SkipSourceFile {
        match relative_path.extension().and_then(|e| e.to_str()) {
            Some("ts" | "tsx" | "js" | "jsx") => SkipSourceFile::Keep,
            _ => SkipSourceFile::Skip,
        }
    }
```

`isograph.config.ts` is `Source` by extension. The watcher does not intern it: path equals `config_path` is `ChangedFileKind::Config`.

```rust
// from crates/isograph_config/src/compilation_options.rs
pub static ISOGRAPH_FOLDER: &str = "__isograph";
```

### SourceFileEvent

Copied from isograph `watch.rs`. Delta: no Schema / SchemaExtension. `JavaScriptSourceFile` / `JavaScriptSourceFolder` renamed `SourceFile` / `SourceFolder`.

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

`App::DaemonArgs = IsographArgs`. clap groups: `ConfigFlag` vs `IsographArgs`. `run_daemon` passes `args.filesystem` and the loaded `IsographProjectConfig` into `daemon::run`. Drop `NoArgs`.

### Start

Intern `CurrentWorkingDirectory` before any `DiskChanged`. Bind, intern, start the watcher, write the port file.

`discover` hands a canonical config file path. A file path has a parent. The type system does not. Same `expect` as `intern_config_directory`.

`serve` does not match on `Filesystem`.

```rust
// from crates/isograph_cli/src/watch.rs
pub fn start_if_watching<THostLanguage: HostLanguage>(
    filesystem: Filesystem,
    watch_tx: UnboundedSender<Vec<SourceFileEvent>>,
    config_path: &Path,
    source_files: &[String],
) -> Result<Option<Watcher>, WatchError> {
    match filesystem {
        Filesystem::Injected => None.wrap_ok(),
        Filesystem::Watch => start::<THostLanguage>(watch_tx, config_path, source_files)
            .map(|watcher| watcher.wrap_some()),
    }
}
```

`Filesystem` is `pub(crate)` in `lib.rs`. `watch.rs` uses `crate::Filesystem`.

```rust
// from crates/isograph_cli/src/daemon.rs
    intern_config_directory(&mut state, config_path.reference());
    let config_directory = config_path
        .parent()
        .expect("a config file path has a parent directory")
        .to_owned();
    let _hold_watch = watch_tx.clone();
    let _watcher = match crate::watch::start_if_watching::<THostLanguage>(
        filesystem,
        watch_tx,
        config_path.reference(),
        config.source_files.as_slice(),
    ) {
        Ok(watcher) => watcher,
        Err(e) => {
            tracing::error!(error = %e, "could not start the watcher");
            return;
        }
    };
```

`let _watcher` outlives `select!`. `_hold_watch` keeps `run_filesystem_watcher` from returning on `Injected`. Do not log `filesystem` on `isograph daemon up`.

```rust
// from crates/isograph_cli/src/daemon.rs
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

`daemon::run` takes `filesystem` and `config` and passes them into `serve`. Channels, bind, port file, SIGTERM, `select!` of event loop / effect loop / accept_loop / `run_filesystem_watcher` stay as in the current `serve`, plus the watcher lines above.

### Watch

Copy `create_debounced_file_watcher`: `new_debouncer` 100ms, `RecommendedWatcher`, `RecommendedCache`, recursive `watch()` of the config directory. Callback: `categorize_and_filter_events`, `watch_tx.send`. Then boot-walk that directory with `visit_dirs_skipping_isograph` and send those `SourceFile` creates.

```rust
// from crates/isograph_cli/src/watch.rs
pub struct Watcher {
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
}

pub fn start<THostLanguage: HostLanguage>(
    watch_tx: UnboundedSender<Vec<SourceFileEvent>>,
    config_path: &Path,
    source_files: &[String],
) -> Result<Watcher, WatchError>;
```

`WatchError`: failed `new_debouncer`, failed `watch()` of the config directory (path plus `notify::Error`), invalid glob.

`source_files` membership is `SourceGlobs`: compile each glob with `globset`, `!` is exclude, last match wins. Copy `visit_dirs_skipping_isograph` from isograph (`ISOGRAPH_FOLDER`). Warn and skip a `read_dir` error. A directory symlink to an ancestor loops; isograph does not detect that.

Copy `categorize_and_filter_events` and the `process_*` functions from isograph `watch.rs`. Files are in scope if `source_files` matches. Folders are in scope if they are under the config directory. Deltas are the bullet list at the top of this file. Do not re-implement notify's event matrix from scratch.

```rust
// from crates/isograph_cli/src/watch.rs
pub fn apply<THostLanguage: HostLanguage>(
    event_tx: &UnboundedSender<IsographEvent>,
    config_directory: &Path,
    events: Vec<SourceFileEvent>,
)
```

Copy `update_sources`. `SourceFile` create/modify reads the file and posts `DiskChanged::File` `Present` (absolute canonical path, UTF-8 contents). `SourceFile` remove posts `File` `Absent`. `SourceFile` rename is `File` `Absent` of from then `Present` of to. `SourceFolder` create/modify walks the folder and posts file `Present`s. `SourceFolder` remove/rename-from posts `FolderRemoved`. `Config` is ignored.

`should_skip_source_file` runs when posting a file `Present`. `Skip` drops the file. Failed read of a path that was interned posts `File` `Absent`. Skip non-regular files. Skip non-UTF8 paths; do not call `handle`.

Posted `File` `Present` paths are absolute and canonical. `File` `Absent` and `FolderRemoved` are absolute; canonical when canonicalize succeeded.

### Tests

TypeScript `should_skip_source_file`: `ts` / `tsx` / `js` / `jsx` (including `a.ts`, `src/a.d.ts`) are `Keep`. `rs` / `json` / `mjs` / `mts` / `graphql` / empty path are `Skip`. `node_modules/pkg/index.ts` and `src/__isograph/foo.ts` are `Keep` by extension.

`ISOGRAPH_FOLDER` is `"__isograph"`.

Globs: `["src/**/*.ts"]` contains `src/a.ts`, not `src/a.tsx`, not `lib/a.ts`. `["src/**/*.ts", "!src/**/*.test.ts"]` contains `src/a.ts`, not `src/a.test.ts`. `[]` contains nothing. `["src/**/*.in"]` contains `src/a.in`, not `src`. Folder `src` under the config directory is still `SourceFolder`.

`database.rs` `TestHostLanguage::should_skip_source_file` returns `Keep`.

`watch.rs` tests a host where `should_skip_source_file` is `Keep` iff the extension is `in`. Fake channel. Config `["src/**/*.in"]` or `["**/*.in"]` as the case needs.

Behavior, not notify internals:

- Create of `src/a.in` intern that file. Create of `src/b.rs` does not.
- Create of the config path is `Config`, no `DiskChanged`.
- Create of folder `src` intern `src/a.in` even though `src` does not match the glob.
- Remove of file `src/a.in` is `File` `Absent`. Remove of folder `src` is `FolderRemoved`; after `handle`, `src/a.in` is gone.
- Rename file: `File` `Absent` of from, `Present` of to. Rename folder: `FolderRemoved` of from, then `Present` of files under to.
- Empty `source_files`: boot posts nothing.
- Walk skips `__isograph`.
- Failed read after a successful `Present` posts `File` `Absent`; after `handle` the interned file is gone.
- `need_rescan` re-walks the config directory and intern matching files again.

Live notify, same crate. Temp dir, intern_config_directory, `start`, `apply`, `handle`. Deadline 10s.

- Boot: write `src/a.in` before `start`. After `scan finished`, `disk_file` of `src/a.in` is those contents.
- Notify: start on empty `src/`, then write `src/a.in`. `disk_file` is those contents.

The binary e2e cannot read intern. These crate tests are the intern pin.

### e2e `cli.rs`

Every existing `Daemon::start` passes `--filesystem injected`.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
        let output = daemon.isograph(["start", "--filesystem", "injected"].reference());
```

`Daemon::start_watch` writes the config and runs `["start"]`.

- Empty `source_files`, Watch: log has `scan finished` and `isograph daemon up`.
- `{"source_files":["src/**/*.ts","!src/**/*.test.ts"]}`. Write `src/Home.ts` with an iso literal before start. Log has `disk present` and `Home.ts`. Write `src/Other.ts` after start; log has `Other.ts` within 10s. Write `src/skip.rs` and `src/Home.test.ts`; those paths do not appear as `disk present`.

`isograph start --help` contains `filesystem`. `a_second_start_adopts_the_running_daemon` still calls `["start"]` against an Injected daemon.

### Cargo

```toml
# from crates/isograph_cli/Cargo.toml
globset = "0.3"
notify = { workspace = true }
notify-debouncer-full = { workspace = true }
pathdiff = { workspace = true }
```

```rust
// from crates/isograph_cli/src/lib.rs
mod watch;
```

## Call sites

- `isograph start --filesystem watch|injected` -> `DaemonArgs` -> `run_daemon` -> `daemon::run`.
- `start_if_watching` -> `watch::start` or `None`.
- notify callback -> `categorize_and_filter_events` -> `watch_tx`.
- `run_filesystem_watcher` -> `apply` -> `event_tx.send(DiskChanged)`.
- event loop -> `handle`. Same `handle` for watcher posts and for `isograph send`.
