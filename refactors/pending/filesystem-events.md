# Filesystem events, the CLI, config globs, and the watcher

Requires event-loop.md (landed), send-events.md, `docs-website/docs/design-docs/event-model.md`, and config-discovery.md. The daemon already recvs, calls `handle`, performs effects, listens on the event socket, and `isograph send` writes one `IsographEvent` JSON frame. This file adds disk facts, config `includes`, `Presence`, and the watcher. Four shippable changes after send-events.md.

The watcher posts in-process. It does not run `isograph send` and it does not write to the event socket. The CLI and the socket are a separate source of the same event type. `isograph send` is send-events.md.

## What the user does

After change 2:

```
$ isograph start
$ isograph send <<'EOF'
{"kind":"IsographEvent.DiskChanged","value":{"path":"/tmp/proj/src/a.ts","contents":"export const a = 1;\n"}}
EOF
$ isograph logs
{"timestamp":"...","level":"INFO","fields":{"message":"disk changed","path":"/tmp/proj/src/a.ts","file_count":1}}
```

After change 4, the same send uses `presence`. After change 5:

```
$ isograph start --filesystem watch
# create, edit, rename, delete files under project_root
$ isograph logs
{"timestamp":"...","level":"INFO","fields":{"message":"disk changed","path":".../src/Pet.tsx","presence":"present","file_count":3}}
```

## Change 1: one event, this path has these contents

send-events.md already recvs, performs, and listens. This change adds `DiskChanged { path, contents }` and a path-to-contents map. Tests call `handle` and also drive the socket the way figaro's `tests/external.rs` does.

### Types

Most important first.

Origin: send-events.md `IsographEvent`. Delta: `DiskChanged` beside `HelloWorld`. `DiskChanged` is `Serialize` + `Deserialize`.

```rust
// from crates/isograph_cli/src/event.rs
use std::path::PathBuf;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum IsographEvent {
    #[serde(rename = "IsographEvent.HelloWorld")]
    HelloWorld,
    #[serde(rename = "IsographEvent.Quit")]
    Quit,
    #[serde(rename = "IsographEvent.DiskChanged")]
    DiskChanged(DiskChanged),
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct DiskChanged {
    pub path: PathBuf,
    pub contents: String,
}
```

```rust
// from crates/isograph_cli/src/state.rs
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::effect::IsographEffect;
use crate::event::{DiskChanged, IsographEvent};
use prelude::Postfix;

pub struct IsographState {
    pub files: BTreeMap<PathBuf, String>,
}

impl IsographState {
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
        }
    }

    pub fn handle(&mut self, event: IsographEvent) -> Vec<IsographEffect> {
        match event {
            IsographEvent::HelloWorld => IsographEffect::LogHelloWorld.wrap_vec(),
            IsographEvent::Quit => IsographEffect::Kill.wrap_vec(),
            IsographEvent::DiskChanged(change) => {
                self.files
                    .insert(change.path.clone(), change.contents.clone());
                Vec::new()
            }
        }
    }
}
```

Origin: send-events.md / event-loop.md `handle`. Delta: `files` and a `DiskChanged` arm. `BTreeMap` so a log of the map is sorted. A second event for the same path replaces the contents. There is no way to remove a path yet. `DiskChanged` returns no effects this change. The `disk changed` log for the e2e is change 2.

`on_message` is send-events.md: `from_str::<IsographEvent>`. No new arm. `DiskChanged` is `Serialize` + `Deserialize`.

The socket, `IsographArgs.port`, `listen`, the port file, and `select!` stay as send-events.md left them.

### Tests

In `state.rs`:

- An event inserts. `files.get(path)` is the contents.
- A second event on the same path replaces.
- An empty string is stored. `files.get(path)` is `Some("")`.
- Two paths are two entries.

`expect` in these tests names the fixture the test inserted.

In `external.rs`:

- A `IsographEvent.DiskChanged` frame with `path` and `contents` round-trips.
- `{"kind":"IsographEvent.DiskChanged",...}` does not deserialize.
- `"not json"` does not deserialize.

A tokio test in `crates/isograph_cli/tests/socket.rs`, copied from figaro `tests/external.rs` in shape: bind `listen(0, ...)`, connect with `tokio_tungstenite`, send one frame, `event_rx.try_recv()` is `DiskChanged` with that path and contents. A bad frame then a good frame: the good one still arrives. Dev-dependency: `tokio-tungstenite`, `futures-util`.

The e2e crate does not yet send; that is change 2. Existing start/status/logs/stop tests still pass. `isograph daemon up` still appears in the log, now with `port`.

`crates/ts_graphql_react_isograph_cli/tests/cli.rs` `the_log_contains_the_config_path` still polls for `isograph daemon up` and the config path.

## Change 2: send `DiskChanged`

`isograph send`, the socket, and the port file are send-events.md. This change adds `IsographEvent.DiskChanged` and an e2e that sends it.

Origin of the verb: send-events.md. Delta: a `DiskChanged` frame instead of `HelloWorld`. `on_message` does not change.

### Tests

A `IsographEvent.DiskChanged` frame with `path` and `contents` round-trips. A tokio test in `crates/isograph_cli/tests/socket.rs`: send one `DiskChanged` frame, `event_rx.try_recv()` is `DiskChanged` with that path and contents.

E2E in `crates/ts_graphql_react_isograph_cli/tests/cli.rs`:

- Start. `send` a frame with `path` and `contents` on stdin. Poll the log until `disk changed` and the path and `file_count` 1.
- `send --file` of a `DiskChanged` JSON file is the same as stdin.

send-events.md already covers a stopped daemon and `not json`. The harness already points `HOME` at the temp dir.

## Change 3: config `includes`

`project_root` is a single directory. The watcher and the compiler need glob arrays, negation, and gitignore.

Biome's `files.includes` is the model: an array of globs, `!` prefixes a negation, last match wins, processed in order. Patterns are relative to the directory that contains the config file. `*` is one path segment, `**` is recursive.

### Config types

`discover::IsographConfig` is an empty struct and ignores unknown fields. `isograph_config::IsographProjectConfig` is the real file and `deny_unknown_fields`. Change 3 parses with `IsographProjectConfig`, in `isograph_config`, and `discover::load_config` returns that type.

```rust
// from crates/isograph_config/src/compilation_options.rs
#[derive(Deserialize, JsonSchema, Debug)]
#[serde(deny_unknown_fields)]
pub struct IsographProjectConfig {
    #[serde(rename = "$schema")]
    pub json_schema: Option<String>,
    pub project_root: PathBuf,
    pub artifact_directory: Option<PathBuf>,
    pub schema: PathBuf,
    #[serde(default)]
    pub schema_extensions: Vec<PathBuf>,
    /// Glob patterns relative to the config file's directory. `!` at the start of a pattern
    /// excludes. Last match wins. When absent, the default is `{project_root}/**/*.{js,jsx,ts,tsx}`.
    pub includes: Option<Vec<String>>,
    #[serde(default)]
    pub options: ConfigFileOptions,
}
```

`includes: Option<Vec<String>>`. `None` is the default glob. `Some(vec![])` is nothing in scope. Those are not a bool.

`project_root` stays required. Existing demo configs keep working. `includes` absent means one include: `{project_root}/**/*.{js,jsx,ts,tsx}`.

Always out of scope for source walking, hardcoded, not a config flag: any path whose components include `node_modules` or `__isograph`. Schema and schema_extensions are still watched even if they sit in `node_modules`.

`.gitignore` is always honored, via the `ignore` crate. No `useIgnoreFile` bool.

### Load without panic, without mkdir

`create_config` today panics on missing files and creates `artifact_directory` and `project_root` at parse time. Parse is not an effect. Change 3 replaces `create_config` with a `Result`.

```rust
// from crates/isograph_config/src/compilation_options.rs
#[derive(Debug)]
pub struct Unreadable {
    pub path: PathBuf,
    pub source: std::io::Error,
}

#[derive(Debug)]
pub struct Unparseable {
    pub path: PathBuf,
    pub source: serde_json::Error,
}

#[derive(Debug)]
pub struct NotCanonical {
    pub path: PathBuf,
    pub source: std::io::Error,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not read {}: {}", .0.path.display(), .0.source)]
    Unreadable(Unreadable),
    #[error("could not parse {}: {}", .0.path.display(), .0.source)]
    Unparseable(Unparseable),
    #[error("could not canonicalize {}: {}", .0.path.display(), .0.source)]
    NotCanonical(NotCanonical),
}

pub fn load_project_config(
    config_location: &Path,
) -> Result<IsographProjectConfig, ConfigError> {
    let config_contents = std::fs::read_to_string(config_location).map_err(|source| {
        ConfigError::Unreadable(Unreadable {
            path: config_location.to_owned(),
            source,
        })
    })?;
    serde_json::from_str(config_contents.reference()).map_err(|source| {
        ConfigError::Unparseable(Unparseable {
            path: config_location.to_owned(),
            source,
        })
    })
}
```

`discover::load_config` becomes a call to `isograph_config::load_project_config` after `config_json` (so `.js` / `.ts` still stringify first). `DiscoverError::Load` wraps `ConfigError` for JSON parse, and keeps the existing JS-runtime variants.

`create_config` remains for callers that still want `CompilerConfig` (artifact paths, canonical schema). It uses `load_project_config` and returns `Result<CompilerConfig, ConfigError>`. It does not `create_dir_all`. Creating the artifact directory is an effect of generation, not of load. That is a behavior change: a missing `project_root` directory is a later watcher/compile diagnostic, not a panic at daemon start.

`isograph_config` already panics. Touching it to return `Result` is the building block. `thiserror` in that crate: workspace `thiserror` is 1.0; `isograph_cli` uses 2. Use workspace `thiserror` in `isograph_config`.

### Scope

```rust
// from crates/isograph_cli/src/scope.rs
use std::path::{Path, PathBuf};

use globset::{Glob, GlobMatcher};

#[derive(Clone)]
pub enum GlobRule {
    Include(GlobMatcher),
    Exclude(GlobMatcher),
}

#[derive(Clone)]
pub struct SourceScope {
    pub config_dir: PathBuf,
    pub rules: Vec<GlobRule>,
    pub always: AlwaysWatched,
    pub watch_roots: Vec<PathBuf>,
}

#[derive(Clone)]
pub struct AlwaysWatched {
    pub config: PathBuf,
    pub schema: PathBuf,
    pub schema_extensions: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct GlobError {
    pub pattern: String,
    pub source: globset::Error,
}

#[derive(Debug, thiserror::Error)]
pub enum ScopeError {
    #[error("invalid glob {}: {}", .0.pattern, .0.source)]
    Glob(GlobError),
    #[error("config path {} has no parent directory", .0.display())]
    NoParent(PathBuf),
}

impl SourceScope {
    pub fn from_config(
        config_path: &Path,
        config: &isograph_config::IsographProjectConfig,
    ) -> Result<Self, ScopeError> {
        let config_dir = match config_path.parent() {
            Some(dir) => dir.to_owned(),
            None => return ScopeError::NoParent(config_path.to_owned()).wrap_err(),
        };
        let patterns = match &config.includes {
            Some(patterns) => patterns.clone(),
            None => {
                let root = config.project_root.to_string_lossy().replace('\\', "/");
                format!("{root}/**/*.{{js,jsx,ts,tsx}}").wrap_vec()
            }
        };
        let mut rules = Vec::new();
        let mut watch_roots = Vec::new();
        for pattern in &patterns {
            rules.push(compile_rule(pattern)?);
            match pattern.strip_prefix('!') {
                Some(_) => {}
                None => watch_roots.push(watch_root_for(config_dir.reference(), pattern)),
            }
        }
        let schema = config_dir.join(config.schema.reference());
        let schema_extensions = config
            .schema_extensions
            .iter()
            .map(|p| config_dir.join(p.reference()))
            .collect::<Vec<_>>();
        let always = AlwaysWatched {
            config: config_path.to_owned(),
            schema,
            schema_extensions,
        };
        for always_path in always.iter() {
            if let Some(parent) = always_path.parent() {
                watch_roots.push(parent.to_owned());
            }
        }
        watch_roots.sort();
        watch_roots.dedup();
        Self {
            config_dir,
            rules,
            always,
            watch_roots,
        }
        .wrap_ok()
    }

    pub fn contains(&self, path: &Path) -> bool {
        if self.always.contains(path) {
            return true;
        }
        if has_component(path, "node_modules") || has_component(path, "__isograph") {
            return false;
        }
        let relative = match path.strip_prefix(self.config_dir.reference()) {
            Ok(relative) => relative,
            Err(_) => return false,
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        let mut included = false;
        for rule in &self.rules {
            match rule {
                GlobRule::Include(matcher) if matcher.is_match(relative.as_str()) => {
                    included = true;
                }
                GlobRule::Exclude(matcher) if matcher.is_match(relative.as_str()) => {
                    included = false;
                }
                _ => {}
            }
        }
        included
    }
}

impl AlwaysWatched {
    fn contains(&self, path: &Path) -> bool {
        path == self.config.as_path()
            || path == self.schema.as_path()
            || self.schema_extensions.iter().any(|p| p.as_path() == path)
    }

    pub fn iter(&self) -> impl Iterator<Item = &PathBuf> {
        std::iter::once(&self.config)
            .chain(std::iter::once(&self.schema))
            .chain(self.schema_extensions.iter())
    }
}

fn has_component(path: &Path, name: &str) -> bool {
    path.components().any(|c| c.as_os_str() == name)
}

fn compile_rule(pattern: &str) -> Result<GlobRule, ScopeError> {
    let body = match pattern.strip_prefix('!') {
        Some(rest) => rest,
        None => pattern,
    };
    let matcher = Glob::new(body)
        .map_err(|source| {
            ScopeError::Glob(GlobError {
                pattern: pattern.to_owned(),
                source,
            })
        })?
        .compile_matcher();
    match pattern.strip_prefix('!') {
        Some(_) => GlobRule::Exclude(matcher).wrap_ok(),
        None => GlobRule::Include(matcher).wrap_ok(),
    }
}

fn watch_root_for(config_dir: &Path, pattern: &str) -> PathBuf {
    let prefix = match pattern.split(['*', '{']).next() {
        Some(prefix) => prefix.trim_end_matches('/'),
        None => "",
    };
    if prefix.is_empty() {
        config_dir.to_owned()
    } else {
        config_dir.join(prefix)
    }
}
```

`config_path.parent()` of a canonical absolute file is `Some`. `None` is `ScopeError::NoParent`. Brace expansion in the default `{js,jsx,ts,tsx}` is globset's `{a,b}` support.

Exclude patterns do not add watch roots. `always` paths add their parent. Dedup the `Vec`. `split` always yields at least one piece; the `None` arm is the empty prefix, same as watching `config_dir`. Do not `expect` on `next`.

### Cargo

Workspace already does not list `globset` or `ignore`. Add:

```toml
# from Cargo.toml workspace.dependencies
globset = "0.4"
ignore = "0.4"
```

`isograph_cli` depends on `globset` and `isograph_config`. `ignore` is used in change 5. Add `ignore` to `isograph_cli` in change 5, not here. Change 3 only matches paths; it does not walk.

### Tests

E2e today writes `{}\n`. After this change that file is `Unparseable` / missing fields. Every e2e fixture becomes:

```json
{
  "project_root": ".",
  "schema": "./schema.graphql"
}
```

plus a `schema.graphql` file of `type Query { __typename: String }\n`. `Daemon::start` writes both.

Scope unit tests, paths relative to a temp `config_dir`:

- Default includes: `src/a.ts` under `project_root: "src"` is in. `src/a.rs` is out. `README.md` is out.
- `includes: ["src/**/*.ts"]`: `src/a.ts` in, `src/a.tsx` out, `lib/a.ts` out.
- `includes: ["**", "!**/*.test.ts"]`: `src/a.ts` in, `src/a.test.ts` out.
- Last match wins: `["**", "!**/*.test.ts", "**/special.test.ts"]`: `special.test.ts` in.
- `includes: []`: nothing in, except `always` (config, schema).
- `node_modules/foo.ts` out even if `includes: ["**"]`.
- `__isograph/foo.ts` out even if `includes: ["**"]`.
- Schema path `vendor/schema.graphql` in even if under a negated glob.
- Path outside `config_dir` out, unless it is an `always` path.
- Empty file name matching: `includes: ["src/**"]` and path `src/` (a directory) — `contains` is for files the watcher will post; directories can be true or false as long as the watcher only posts files. Test files only.
- Windows separators in the path become `/` before match.

`from_config` of a bad glob (`[`) is `ScopeError::Glob`.

`load_project_config` of `{}\n` is `Unparseable`. Of a missing file, `Unreadable`. Of a valid demo-shaped JSON, `Ok` with `includes == None`.

## Change 4: created, deleted, moved

`DiskChanged` is one always-present file. The design-doc event is a path plus `Presence`. Created and modified are `Present`. Deleted is `Absent`. Moved is `Absent` of the old path then `Present` of the new path. There is no `Moved` variant.

Before:

```rust
// from crates/isograph_cli/src/event.rs
pub struct DiskChanged {
    pub path: PathBuf,
    pub contents: String,
}
```

After:

```rust
// from crates/isograph_cli/src/event.rs
pub struct DiskChanged {
    pub path: PathBuf,
    pub presence: Presence,
}

pub enum Presence {
    Present(Present),
    Absent,
}

pub struct Present {
    pub contents: String,
}
```

`handle_disk_changed`:

```rust
// from crates/isograph_cli/src/state.rs
    fn handle_disk_changed(&mut self, change: &DiskChanged) {
        match &change.presence {
            Presence::Present(present) => {
                self.files
                    .insert(change.path.clone(), present.contents.clone());
            }
            Presence::Absent => {
                self.files.remove(change.path.reference());
            }
        }
    }
```

`Absent` of a path that is not in the map is a no-op.

The log line gains `presence` (`present` / `absent`).

Wire frames change. Change 2's `{"path","contents"}` no longer deserializes.

```json
{"kind":"IsographEvent.DiskChanged"},"value":{"path":"/tmp/proj/src/a.ts","presence":{"Present":{"contents":"export const a = 1;\n"}}}}
{"kind":"IsographEvent.DiskChanged"},"value":{"path":"/tmp/proj/src/a.ts","presence":"Absent"}}
```

A move is two frames, in that order: `Absent` of `from`, `Present` of `to`.

The socket test and the send e2e use the `presence` shape. Existing change 2 tests that sent `contents` at the top level are rewritten.

Tests in `state.rs`:

- `Present` inserts.
- A second `Present` on the same path replaces.
- `Absent` removes.
- `Absent` of a path that was never present leaves the map unchanged.
- `Present` of an empty string is present, not absent.
- Two events `Absent` then `Present` on different paths is a move: old path gone, new path present with those contents.

`on_message` is still `from_str::<IsographEvent>`. `Presence` is `Serialize` + `Deserialize`.

## Change 5: the watcher

A source. It walks once, then observes. Each fact is a `DiskChanged` sent on the same channel the socket uses.

`IsographArgs` gains how facts arrive. Not a bool.

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

    /// Loopback port for the event socket. Assigned by the OS when absent.
    #[arg(long)]
    pub port: Option<u16>,
}
```

`Injected` does not scan and does not watch. The event socket listens in both modes. Default `Watch`.

### Start

In `daemon::serve`, the `Filesystem::Watch` arm:

```rust
// from crates/isograph_cli/src/daemon.rs
        Filesystem::Watch => {
            let scope = match crate::scope::SourceScope::from_config(
                config_path.reference(),
                &_config,
            ) {
                Ok(scope) => scope,
                Err(e) => {
                    error!(error = %e, "could not build the source scope");
                    return;
                }
            };
            let watcher = match crate::watch::start(event_tx.clone(), scope) {
                Ok(watcher) => watcher,
                Err(e) => {
                    error!(error = %e, "could not start the watcher");
                    return;
                }
            };
            // `watcher` lives until the event loop ends, next to `socket`.
            let _watcher = watcher;
        }
```

`serve` currently names `_config`. Change 3's `config_and_instance` already returns `IsographProjectConfig`. Stop discarding it: `Ok((path, instance, config))`, pass `config` into `serve`. The `Watcher` is a local that outlives the `while let Some(event)` loop; put it in an outer binding before the loop so drop order is loop, then watcher, then socket.

`daemon up` logs `filesystem`. E2E that must not watch uses `--filesystem injected`. Add `Daemon::start_injected`.

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

`SourceScope` derives `Clone`. The `Debouncer` type arguments are `notify-debouncer-full` 0.4 with `notify` 7. If inference can fill the field, omit the arguments. `new_debouncer(timeout, tick_rate, cb)` with `tick_rate: None` is the crate default. Workspace already pins both crates.

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
            let Some(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_file() {
                continue;
            }
            let path = entry.path();
            if scope.contains(path) {
                paths.push(path.to_owned());
            }
        }
    }
    for always in scope.always.iter() {
        if always.is_file() {
            paths.push(always.to_owned());
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

`file_type.is_file()` is a bool from `std`. Match it rather than `if !`. Raise: `is_file` is std's API; the match makes the two cases visible. Same for `always.is_file()`.

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
    if !path.is_file() {
        return;
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

### Tests

Unit tests of `dispatch` with a fake channel and a `SourceScope` over a temp tree:

- `Create` of an in-scope file that exists: one `Present` with those contents.
- `Create` of an out-of-scope `.rs` file: no event.
- `Remove` of a path: one `Absent`.
- `RenameMode::Both` with two paths: `Absent` of from, `Present` of to (file must exist at `to`).
- `RenameMode::Both` out of scope to in scope: only `Present` of to (`Absent` of from is also posted; the test asserts the pair).
- Empty file: `Present` with `contents == ""`.
- Failed read (path does not exist on `Create`): no event.

`dispatch` is `pub(crate)` so the tests in the module can call it.

Scan: write `src/a.ts` and `src/b.rs` and `node_modules/c.ts` under a temp project with default includes. Collect posted events (a test-only `scan` that returns `Vec<DiskChanged>` is wrong; instead give `scan` the channel and drain). Assert the paths posted are exactly `src/a.ts` and the schema and the config, sorted. `b.rs` and `node_modules` are absent.

Do not add a production function only the tests call. Drain the test channel.

E2E: `--filesystem watch`, write `src/a.ts`, poll logs for `disk changed` and that path. Delete the file, poll for `absent`. HOME isolation as today. This is the one real-notify test. Deadline 10s, same as existing daemon tests.

Injected e2e from change 2 still does not start a watcher: writing a file on disk must not produce a log line.

### Cargo

```toml
# from crates/isograph_cli/Cargo.toml
ignore = { workspace = true }
notify = { workspace = true }
notify-debouncer-full = { workspace = true }
```

## CI

No new workflow file. `cargo test --manifest-path crates/ts_graphql_react_isograph_cli/Cargo.toml --tests` already runs on every platform in `build-cli.yml`. Change 2's send tests and change 5's watch test ride that job.

`cargo test` for `isograph_cli` unit tests (handle, deserialize, scope, dispatch) rides `cargo-test` in `ci.yml` once `isograph_cli` is in the workspace test set. It is a workspace member. `cargo test` at the root includes it.

Do not launch VS Code or Zed. Editor CI is zed-and-vscode-extensions.md.

`freddie_event_socket`'s 64 KiB cap: a send fixture larger than that is not added. Production contents do not go over the socket.

## Call sites

- `run_daemon` -> `daemon::run` (change 1).
- `CliVerb::Send` -> `send::run` (change 2).
- `discover::load_config` -> `isograph_config::load_project_config` (change 3).
- `Filesystem::Watch` arm -> `watch::start` (change 5).
- notify callback -> `dispatch` -> `event_tx.send`.
- socket callback -> `on_message` -> `event_tx.send`.
- event loop -> `state.handle`.
