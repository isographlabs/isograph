# Debounced `publishDiagnostics` per client

Each live LSP client that declares the `textDocument.publishDiagnostics` client capability receives `textDocument/publishDiagnostics` for the files that client has open, computed from that client's buffers through `file_contents(db, client, path)`. A `didOpen`, `didChange`, or `didClose` that changed a buffer arms one 100ms debounce timer. When the timer fires, `handle` computes each client's current diagnostics, compares them with what that client was last sent, and returns one `PublishDiagnosticsEffect` per client that has a difference. A path whose errors went away, or that the client closed, is sent an empty list.

Diagnostics are per client and reflect what that client sees. Client A's diagnostics cover only A's open files and are computed from A's buffers via `file_contents(A, path)`. Client B's open files and unsaved edits never affect A's diagnostics. Disk-only files (a `DiskFile` that no buffer of that client overlays) are never reported.

Builds on the current code: `file_contents` and the client-keyed iso literal memos (lsp-open-files.md change 7), `freddie_timer` timers (`IsographEffect::Timer`, `IsographEvent::Timer`), and `LiveSessions`.

Origin: isograph `crates/isograph_lsp/src/server.rs` (debounce, `SHORT_DEBOUNCE_TIME`) and `crates/isograph_lsp/src/diagnostic_notification.rs` (`publish_new_diagnostics_and_clear_old_diagnostics`). Deltas are in Upstream comparison.

## What the user does

The user opens `src/Home.ts` in VS Code and types `export const Home = iso(\`entrypoint\`)`. About 100ms after the last `didChange`, a red squiggle covers the error range inside the literal and the Problems panel lists the error with severity Error, source `isograph`, and the parse error's message. The user types `entrypoint Query.HomeRoute`; 100ms later the squiggle and the Problems entry disappear. Typing faster than the debounce publishes once, for the text the editor had at the last change.

Only parse errors appear. A literal that parses but has a host-language problem (`iso\`entrypoint Query.HomeRoute\`` with no parentheses, an unexported field, a field with no associated function) has no entry.

The Problems panel lists errors in open files only. A broken file on disk that the window has not opened has no entry. Closing a tab clears that file's entries, including when the closed buffer had unsaved broken edits. An open file that the config's `source_files` globs exclude, or that `THostLanguage::should_skip_source_file` skips (for example `README.md` containing `` iso`...` ``), has no entries.

"Open" means the client sent `didOpen` and has not sent `didClose`. LSP has no visibility signal. VS Code also sends `didOpen` for documents it loads without a visible tab (for example the target of a peek view), so such a document can have entries until VS Code sends `didClose` for it.

Two VS Code windows on one project are two clients. If both have `src/Home.ts` open with different unsaved text, each window shows errors for its own text only. Window A's unsaved errors never appear in B, and a file only A has open has entries only in A. Saving in A does not change B's entries: B's diagnostics come from B's buffer.

`isograph send` connections initialize with `{ "capabilities": {} }`, declare no `textDocument.publishDiagnostics`, and receive no diagnostics.

## Types

### Effect

```rust
// from crates/isograph_cli/src/effect.rs
#[derive(Debug)]
pub enum IsographEffect {
    LogHelloWorld,
    Kill,
    SendLspResponse(Box<SendLspResponseEffect>),
    PublishDiagnostics(Box<PublishDiagnosticsEffect>),
    Timer(TimerEffect<TimerPayload>),
}

/// `textDocument/publishDiagnostics` notifications for one client.
#[derive(Debug)]
pub struct PublishDiagnosticsEffect {
    pub sender: crossbeam::channel::Sender<lsp_server::Message>,
    pub notifications: Vec<lsp_server::Notification>,
}
```

`handle` serializes the params. `perform` sends. The notification is built with a struct literal, not `lsp_server::Notification::new`, because `new` calls `serde_json::to_value(params).unwrap()` (`lsp-server-0.7.8/src/msg.rs:235-237`).

### Per-client published state

`PublishedDiagnostics` is outbound I/O state: what the daemon has written to one connection. It is not a pico source. No memo reads it; only `handle` reads and writes it. It lives on `LiveSession`, so `Gone` drops it with the session and a replacing `Started` starts from an empty map. Upstream isograph keeps only a `BTreeSet<Uri>` of previously published URIs and resends every list on each publish; `PublishedDiagnostics` keeps the lists too, so a publish skips files whose diagnostics did not change.

```rust
// from crates/isograph_cli/src/diagnostics.rs
/// What one client was last sent: every path whose last notification had a non-empty list.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PublishedDiagnostics(pub HashMap<RelativePathToSourceFile, Vec<lsp_types::Diagnostic>>);
```

Before:

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug)]
pub struct LiveSession {
    pub sender: crossbeam::channel::Sender<lsp_server::Message>,
    #[cfg_attr(not(test), expect(dead_code))]
    pub capabilities: lsp_types::ClientCapabilities,
    pub outstanding: std::collections::HashSet<lsp_server::RequestId>,
}
```

After:

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug)]
pub struct LiveSession {
    pub sender: crossbeam::channel::Sender<lsp_server::Message>,
    pub capabilities: lsp_types::ClientCapabilities,
    pub published_diagnostics: crate::diagnostics::PublishedDiagnostics,
    pub outstanding: std::collections::HashSet<lsp_server::RequestId>,
}
```

`capabilities` stays the whole `lsp_types::ClientCapabilities`. `publish_to_client` reads it on every publish, so the `#[cfg_attr(not(test), expect(dead_code))]` is removed. A client gets diagnostics when `session.capabilities.text_document.as_ref().and_then(|text_document| text_document.publish_diagnostics.as_ref())` is `Some`; a client where it is `None` is skipped and its `published_diagnostics` stays empty.

### Source globs on `IsographState`

`SourceGlobs` (`crates/isograph_cli/src/watch.rs:210-259`) is private to the watcher today. It becomes `pub(crate)`, as do `SourceGlobs::parse` and `SourceGlobs::contains`. Bodies, `GlobPattern`, and `ExcludeGlob` are unchanged.

Before:

```rust
// from crates/isograph_cli/src/watch.rs
#[derive(Clone)]
struct SourceGlobs {
    patterns: Vec<GlobPattern>,
}

impl SourceGlobs {
    fn parse(source_files: &[String]) -> Result<Self, WatchError> {
        // unchanged
    }

    fn contains(&self, relative: &Path) -> bool {
        // unchanged
    }
}
```

After:

```rust
// from crates/isograph_cli/src/watch.rs
#[derive(Clone)]
pub(crate) struct SourceGlobs {
    patterns: Vec<GlobPattern>,
}

impl SourceGlobs {
    pub(crate) fn parse(source_files: &[String]) -> Result<Self, WatchError> {
        // unchanged
    }

    pub(crate) fn contains(&self, relative: &Path) -> bool {
        // unchanged
    }
}
```

`serve` (`daemon.rs`) parses the config's `source_files` once into `IsographState.source_globs`, before starting the watcher. An invalid glob logs `tracing::error!(error = %e, "invalid source_files")` and returns, the same outcome as today's watcher start failure; with `Filesystem::Injected` (which never parsed globs before) an invalid glob now also stops `serve`. The watcher keeps parsing its own copy from `source_files`.

```rust
// from crates/isograph_cli/src/daemon.rs
    let source_globs = match crate::watch::SourceGlobs::parse(config.source_files.as_slice()) {
        Ok(source_globs) => source_globs,
        Err(e) => {
            tracing::error!(error = %e, "invalid source_files");
            return;
        }
    };
    let mut state = IsographState {
        db: IsographDb::<THostLanguage>::default(),
        live_sessions: LiveSessions::default(),
        source_globs,
        diagnostics_debounce: None,
    };
```

### Timer payload and state

Before:

```rust
// from crates/isograph_cli/src/timer.rs
#[derive(Clone, Copy, Debug)]
pub enum TimerPayload {
    HelloWorldDebounced,
}
```

After:

```rust
// from crates/isograph_cli/src/timer.rs
#[derive(Clone, Copy, Debug)]
pub enum TimerPayload {
    DiagnosticsDebounce,
}
```

Before:

```rust
// from crates/isograph_cli/src/state.rs
pub struct IsographState<THostLanguage: HostLanguage> {
    pub db: IsographDb<THostLanguage>,
    pub live_sessions: LiveSessions,
    pub(crate) hello_world_debounce: Option<freddie_timer::TimerGuard>,
}
```

After:

```rust
// from crates/isograph_cli/src/state.rs
pub struct IsographState<THostLanguage: HostLanguage> {
    pub db: IsographDb<THostLanguage>,
    pub live_sessions: LiveSessions,
    pub(crate) source_globs: crate::watch::SourceGlobs,
    pub(crate) diagnostics_debounce: Option<freddie_timer::TimerGuard>,
}
```

Construction sites that write `hello_world_debounce: None` write `diagnostics_debounce: None` and a `source_globs`: `daemon.rs` `serve` (above) and its `empty_state` test helper, `lsp_socket.rs` `listen_and_reply`, `watch.rs` test fixture, `state.rs` `empty`. The `daemon.rs`, `lsp_socket.rs`, and `state.rs` test sites write `source_globs: crate::watch::SourceGlobs::parse(&["src/**".to_owned()]).expect("the test glob is valid")`; the `watch.rs` fixture parses its own `source_files` with the same `expect`.

### Errors

```rust
// from crates/isograph_cli/src/adapter.rs
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum PathUriError {
    #[error("CurrentWorkingDirectory is not interned")]
    MissingWorkingDirectory,
    #[error("the joined path is not absolute")]
    NotAbsolute,
    #[error("{0}")]
    InvalidUri(<lsp_types::Uri as std::str::FromStr>::Err),
}
```

`<lsp_types::Uri as FromStr>::Err` is `fluent_uri::ParseError` (`Clone, Copy, Debug, PartialEq, Eq`, `Display`); naming it through the trait avoids a direct `fluent_uri` dependency.

```rust
// from crates/isograph_cli/src/diagnostics.rs
#[derive(Debug, thiserror::Error)]
enum PublishDiagnosticsError {
    #[error("{0}")]
    Uri(#[from] crate::adapter::PathUriError),
    #[error("{0}")]
    Serialize(#[from] serde_json::Error),
}
```

## Computation

```text
publish_diagnostics(state)                              handle side, plain fn
  for each live client whose capabilities declare textDocument.publishDiagnostics:
    client_diagnostics(db, source_globs, client)        plain fn
      db.open_paths_of_client(client)                   plain method
      filter: source_globs.contains, should_skip_source_file
      lsp_diagnostics_for_file(db, client, path)        memo, isograph_lsp
        iso_literal_parse_errors_in_file(client, path)  memo, isograph_compiler
          parsed_iso_literals_in_file(client, path)
        line_index(db, client, path)                    memo, isograph_lsp
          file_contents(client, path)
    changed_diagnostics(published, current)             plain fn
    uri_for_path(db, path)                              plain fn
```

The memoized layer (`lsp_diagnostics_for_file` and below) works only with `ClientId` and `RelativePathToSourceFile`, plus the literal text for the parse. Absolute paths and URIs exist only at the outer layer: `relative_from_uri` converts the editor's URI to a `RelativePathToSourceFile` on the way in (`did_open`, `did_change`, `did_close`), and `uri_for_path` converts a `RelativePathToSourceFile` back to a URI on the way out, called only from `publish_diagnostics` when it builds notifications. No URI is stored on `LiveSession`, in a pico source, or in a memo key.

`publish_diagnostics` and its helpers are plain functions called from `handle`; none is memoized. The one-step-per-function rule in pico.md (Structuring functions and memos) governs the memo layers below `lsp_diagnostics_for_file`, and `publish_diagnostics` is not split into memo layers.

### `iso_literal_parse_errors_in_file`

`crates/isograph_compiler/src/iso_literals.rs`, exported from `lib.rs` (`pub use iso_literals::{..., iso_literal_parse_errors_in_file}`). `iso_literals.rs` imports `isograph_parser::ParseError` and `span::{WithSpan, WithSpanPostfix}`.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
/// Parse errors of every iso literal in `path` as `client` reads it, as file-absolute spans.
#[memo]
pub fn iso_literal_parse_errors_in_file<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<WithSpan<ParseError>>> {
    let literals = parsed_iso_literals_in_file(db, client, path).as_ref()?;
    literals
        .iter()
        .flat_map(|(extraction, parsed)| {
            let offset = extraction.iso_literal_start_index.0 as u32;
            parsed.errors.iter().map(move |error| {
                error
                    .item
                    .clone()
                    .with_span(error.location.with_offset(offset))
            })
        })
        .collect::<Vec<_>>()
        .wrap_some()
}
```

`None` is no text for that client. `Some(vec![])` is text with no parse errors (including no literals). `ParsedIsoLiteral.errors` is `Vec<WithSpan<ParseError>>` with spans relative to `iso_literal_text` (pico.md: a prepend does not make the parse `!=`); this memo shifts each by the literal's `iso_literal_start_index`, the same offset `format_iso_literals` and `lsp_semantic_tokens_for_file` add. `Span::with_offset` (`crates/span/src/lib.rs:40`) adds the offset to both ends.

v0 reports parse errors only. Host errors (`TypeScriptHostError`: missing parentheses, missing export, missing associated function) are a later change. `host_errors_for_extraction`, `FileLiteral`, and `file_literals` in `crates/isograph_extract_typescript/src/lib.rs` stay as they are and remain test-only, tracked in `refactors/pending/tests-only-functionality.md`.

### `line_index`

`LineIndex` and the `#[memo] line_index(db, client, path) -> Option<LineIndex>` exist in `crates/isograph_lsp/src/line_index.rs` (landed in `d48dd131`), exported as `isograph_lsp::{LineIndex, line_index}`. `LineIndex::range(span: Span) -> lsp_types::Range` counts columns in UTF-16 code units and treats `\n`, `\r\n`, and `\r` as line breaks. Diagnostics read the memo and `range`; this doc does not change either.

### `lsp_diagnostics_for_file`

`crates/isograph_lsp/src/file_diagnostics.rs`, the same shape as `lsp_semantic_tokens_for_file` (`file_semantic_tokens.rs:10-25`). `lib.rs` adds `mod file_diagnostics;` and `pub use file_diagnostics::lsp_diagnostics_for_file;`.

```rust
// from crates/isograph_lsp/src/file_diagnostics.rs
const DIAGNOSTIC_SOURCE: &str = "isograph";

/// `lsp_types::Diagnostic`s of `path` as `client` reads it.
#[memo]
pub fn lsp_diagnostics_for_file<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::Diagnostic>> {
    let errors = iso_literal_parse_errors_in_file(db, client, path).as_ref()?;
    if errors.is_empty() {
        return Vec::new().wrap_some();
    }
    let line_index = line_index(db, client, path).as_ref()?;
    errors
        .iter()
        .map(|error| lsp_types::Diagnostic {
            range: line_index.range(error.location),
            severity: lsp_types::DiagnosticSeverity::ERROR.wrap_some(),
            source: DIAGNOSTIC_SOURCE.to_owned().wrap_some(),
            message: error.item.to_string(),
            ..Default::default()
        })
        .collect::<Vec<_>>()
        .wrap_some()
}
```

The early return means an open file with no errors does not compute or store a line index. The message is `ParseError`'s `Display` (`#[error("{0}")]` of the `AstError`, `BracketError`, or `CommaWithoutItem`). Every diagnostic has severity `ERROR` and no `code`.

### `open_paths_of_client`

```rust
// from crates/isograph_compiler/src/database.rs
impl<THostLanguage: HostLanguage> IsographDb<THostLanguage> {
    /// Every path `client` has open.
    pub fn open_paths_of_client(&self, client: ClientId) -> Vec<RelativePathToSourceFile> {
        self.get_open_file_map()
            .tracked()
            .0
            .keys()
            .filter(|key| key.client == client)
            .map(|key| key.path)
            .collect()
    }
}
```

A plain method. `handle` calls it with no memo on the stack, so the `tracked()` read records nothing (`dependency_stack.push_if_not_empty`, `crates/pico/src/database.rs:109-121`); `tracked()` is used because AGENTS.md forbids iterating `untracked()`. It does not read `disk_file_map`: a disk-only path is never reported. Each returned path has an `OpenFile` for `client`, so `file_contents(db, client, path)` for it returns that client's buffer, never disk text, and never misses the open map.

It returns a `Vec`, not `impl Iterator`. `View::tracked(&'a self) -> &'a T` (`crates/pico/src/view.rs:29-32`) borrows the `View`, and the `View` is a local, so an iterator over its keys cannot leave the function. `OpenFileKey` is `(client, path)`, so no path appears twice.

The name does not collide: no item named `open_paths_of_client` exists in the workspace.

### Publish

`crates/isograph_cli/src/diagnostics.rs`, new module (`mod diagnostics;` in `lib.rs`). Plain functions; no memo.

```rust
// from crates/isograph_cli/src/diagnostics.rs
/// Changed diagnostics for every live client that accepts them, one effect per client.
pub(crate) fn publish_diagnostics<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
) -> Vec<IsographEffect> {
    let db = state.db.reference();
    let source_globs = state.source_globs.reference();
    state
        .live_sessions
        .0
        .iter_mut()
        .filter_map(|(client, session)| {
            publish_to_client(db, source_globs, client.dereference(), session)
        })
        .collect()
}

fn publish_to_client<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    source_globs: &SourceGlobs,
    client: ClientId,
    session: &mut LiveSession,
) -> Option<IsographEffect> {
    session
        .capabilities
        .text_document
        .as_ref()
        .and_then(|text_document| text_document.publish_diagnostics.as_ref())?;
    let previous = std::mem::take(&mut session.published_diagnostics);
    let mut current = client_diagnostics(db, source_globs, client);
    let outcomes: Vec<_> = changed_diagnostics(previous.reference(), current.reference())
        .map(|(path, diagnostics)| (path, publish_notification(db, path, diagnostics)))
        .collect();
    let notifications: Vec<lsp_server::Notification> = outcomes
        .into_iter()
        .filter_map(|(path, outcome)| {
            outcome
                .map_err(|error| {
                    tracing::warn!(%error, path = path.lookup(), "publishDiagnostics");
                    match previous.0.get(&path) {
                        Some(sent) => current.insert(path, sent.clone()),
                        None => current.remove(&path),
                    };
                })
                .ok()
        })
        .collect();
    session.published_diagnostics = PublishedDiagnostics(current);
    if notifications.is_empty() {
        return None;
    }
    IsographEffect::PublishDiagnostics(
        PublishDiagnosticsEffect {
            sender: session.sender.clone(),
            notifications,
        }
        .boxed(),
    )
    .wrap_some()
}
```

After a publish, the stored map is what the client has: `current`, except that a path whose notification could not be built keeps its previous entry (or stays absent). A new path whose URI fails is therefore not recorded and is retried on the next publish. `uri_for_path` depends only on `CurrentWorkingDirectory` and the path, so a path that was published once never fails later; the restore branch covers a serialize failure.

```rust
// from crates/isograph_cli/src/diagnostics.rs
/// Non-empty diagnostics of every source file `client` has open.
fn client_diagnostics<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    source_globs: &SourceGlobs,
    client: ClientId,
) -> HashMap<RelativePathToSourceFile, Vec<lsp_types::Diagnostic>> {
    db.open_paths_of_client(client)
        .into_iter()
        .filter(|path| {
            let relative = Path::new(path.lookup());
            source_globs.contains(relative)
                && matches!(
                    THostLanguage::should_skip_source_file(relative),
                    SkipSourceFile::Keep
                )
        })
        .filter_map(|path| {
            let diagnostics = isograph_lsp::lsp_diagnostics_for_file(db, client, path).as_ref()?;
            (!diagnostics.is_empty()).then(|| (path, diagnostics.clone()))
        })
        .collect()
}

/// Paths whose `current` list differs from `published`, and `vec![]` for published paths absent from `current`.
fn changed_diagnostics<'a>(
    published: &'a PublishedDiagnostics,
    current: &'a HashMap<RelativePathToSourceFile, Vec<lsp_types::Diagnostic>>,
) -> impl Iterator<Item = (RelativePathToSourceFile, Vec<lsp_types::Diagnostic>)> + 'a {
    let changed = current
        .iter()
        .filter(|(path, diagnostics)| published.0.get(*path) != diagnostics.reference().wrap_some())
        .map(|(path, diagnostics)| (path.dereference(), diagnostics.clone()));
    let cleared = published
        .0
        .keys()
        .filter(|path| !current.contains_key(*path))
        .map(|path| (path.dereference(), Vec::new()));
    changed.chain(cleared)
}

fn publish_notification<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    path: RelativePathToSourceFile,
    diagnostics: Vec<lsp_types::Diagnostic>,
) -> Result<lsp_server::Notification, PublishDiagnosticsError> {
    let uri = crate::adapter::uri_for_path(db, path)?;
    let params = serde_json::to_value(lsp_types::PublishDiagnosticsParams {
        uri,
        diagnostics,
        version: None,
    })?;
    lsp_server::Notification {
        method: <lsp_types::notification::PublishDiagnostics as lsp_types::notification::Notification>::METHOD.to_owned(),
        params,
    }
    .wrap_ok()
}
```

The filter is the watcher's disk filter (`watch.rs` `process_create_or_modify`: `globs.contains(relative)` then `should_skip_source_file`) applied to open paths, so an open buffer is diagnosed exactly when the watcher would ingest the same path from disk. A parent-relative key (`../other/a.ts`, from a `didOpen` outside the config directory) is diagnosed only if a glob matches it, as the watcher would. `previous` is moved out of `session.published_diagnostics` with `std::mem::take` (`PublishedDiagnostics: Default`).

A closed or fixed path is absent from `current` (it is no longer open, or its list is empty), so `changed_diagnostics` yields `vec![]` for it when it was published before. That is how the client clears squiggles, and how `didClose` clears a closed file's entries for that client only. Notifications within one effect are in `HashMap` order; the protocol does not order them and tests do not depend on it.

`version` is `None` in v0; storing document versions is a later change. A publish computed before a newer `didChange` arrives can be shown on the newer text until the next publish, which that `didChange` has already armed.

### `uri_for_path`

`crates/isograph_cli/src/adapter.rs`, next to its inverse `relative_from_uri` (`adapter.rs:120-131`).

```rust
// from crates/isograph_cli/src/adapter.rs
/// The `file:` URI of `path` under the config directory.
pub(crate) fn uri_for_path<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    path: RelativePathToSourceFile,
) -> Result<lsp_types::Uri, PathUriError> {
    let cwd = db
        .get_singleton::<CurrentWorkingDirectory>()
        .ok_or(PathUriError::MissingWorkingDirectory)?
        .dereference();
    let absolute = normalize_lexically(Path::new(cwd.lookup()).join(path.lookup()).as_path());
    let url = url::Url::from_file_path(absolute.as_path()).map_err(|()| PathUriError::NotAbsolute)?;
    url.as_str()
        .parse::<lsp_types::Uri>()
        .map_err(PathUriError::InvalidUri)
}

/// `path` with each `.` dropped and each `..` removing the component before it, without reading the filesystem.
fn normalize_lexically(path: &Path) -> PathBuf {
    path.components()
        .fold(PathBuf::new(), |mut normalized, component| {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    normalized.pop();
                }
                Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                    normalized.push(component);
                }
            }
            normalized
        })
}
```

`relative_from_uri` produces parent-relative keys (`../other/a.ts`) through `pathdiff`. `Url::from_file_path` keeps `..` segments (`url-2.5.8/src/lib.rs:2948-2983` percent-encodes each component), and VS Code does not resolve them, so `file:///tmp/proj/../other/a.ts` would not match the editor's `file:///tmp/other/a.ts`. std's `Path::normalize_lexically` is unstable and the workspace builds on stable; no crate in `Cargo.lock` provides it. Lexical `..` is correct here because `CurrentWorkingDirectory` is the parent of the canonicalized config path (`config_path`, `discover.rs:51-69`, canonicalizes both a `--config` flag and the `nearest_config` result) and `pathdiff` computes the relative part lexically from it. `Url::from_file_path` percent-encodes spaces, non-ASCII bytes, and other reserved bytes; `relative_from_uri` decodes them (`Url::to_file_path`), so the round trip is exact.

A symlinked config directory does not produce a URI that differs from the editor's. `CurrentWorkingDirectory` is canonical (`/private/tmp/proj` on macOS). A window opened at `/tmp/proj` sends `file:///tmp/proj/src/a.ts`; `relative_from_uri` does not fail on it and stores the open file under the parent-relative key `../../../tmp/proj/src/a.ts`. `uri_for_path` joins that key to `/private/tmp/proj` and normalizes lexically to `/tmp/proj/src/a.ts`, which is the URI the editor sent. Only open files are published, so no disk key `src/a.ts` publishes a second URI for the same file. Such a key is published only if a `source_files` glob matches it (see Known risks).

On error the caller logs `warn!`, skips the path, and does not record it as published.

## Trigger, debounce, effect

```rust
// from crates/isograph_cli/src/diagnostics.rs
pub(crate) const DIAGNOSTICS_DEBOUNCE: Duration = Duration::from_millis(100);

/// Replaces the pending diagnostics timer with one that fires after `DIAGNOSTICS_DEBOUNCE`.
pub(crate) fn arm_diagnostics_debounce<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
) -> Vec<IsographEffect> {
    let (guard, effect) = freddie_timer::timer_effect_and_guard(
        DIAGNOSTICS_DEBOUNCE,
        TimerPayload::DiagnosticsDebounce,
    );
    state.diagnostics_debounce = guard.wrap_some();
    IsographEffect::Timer(effect).wrap_vec()
}
```

There is one global timer. The interval is 100ms, upstream `SHORT_DEBOUNCE_TIME`; vscode-languageclient batches full-sync `didChange` for up to 250ms (lsp-open-files.md, What the user does), so 100ms normally fires between batches. Every arming event resets the timer (trailing debounce, as upstream). Typing in one window therefore delays another window's publish, and one publish computes every client. A timer per client would need `HashMap<ClientId, TimerGuard>` on `IsographState`, because `TimerGuard` is not `Clone` and `LiveSession` is `Clone` (`ClientStartedEvent` and `IsographEvent` derive `Clone`).

Overwriting `diagnostics_debounce` drops the previous `TimerGuard`, which closes its oneshot and cancels its sleep in `schedule_timer` (`daemon.rs:216-233`). A firing that raced the drop fails `trigger_if_matching` because its `TimerId` differs.

Callers: `on_did_open`, `on_did_change`, `on_did_close` when the write succeeded. Not on `DiskChanged`: every path a client is diagnosed for has that client's `OpenFile`, so `file_contents(client, path)` does not read the `DiskFile` and a disk change cannot alter any client's diagnostics. Not on `Started`: a new client has no open files, so a publish for it sends nothing. Not on `Gone`: the gone client's published state goes with its `LiveSession`, and other clients do not read its buffers. Not on `HelloWorld`, `Quit`, requests, or responses.

### `handle_timer_fired` and HelloWorld

`HelloWorldDebounced` was a placeholder for this timer. It is deleted; `HelloWorld` returns `LogHelloWorld` directly. `InternalEvent::HelloWorld` stays: the CLI tests use it as a ping (`crates/ts_graphql_react_isograph_cli/tests/cli.rs`).

Before:

```rust
// from crates/isograph_cli/src/state.rs
fn handle_internal_event<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    internal: InternalEvent,
) -> Vec<IsographEffect> {
    match internal {
        InternalEvent::HelloWorld => handle_hello_world(state),
        InternalEvent::Quit => IsographEffect::Kill.wrap_vec(),
        InternalEvent::DiskChanged(change) => handle_disk_changed(&mut state.db, change),
    }
}

fn handle_hello_world<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
) -> Vec<IsographEffect> {
    let (guard, effect) =
        freddie_timer::timer_effect_and_guard(Duration::ZERO, TimerPayload::HelloWorldDebounced);
    state.hello_world_debounce = guard.wrap_some();
    IsographEffect::Timer(effect).wrap_vec()
}

fn handle_timer_fired<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    fired: freddie_timer::TimerFired<TimerPayload>,
) -> Vec<IsographEffect> {
    match fired.trigger_if_matching(state.hello_world_debounce.as_ref()) {
        Some(&TimerPayload::HelloWorldDebounced) => {
            state.hello_world_debounce = None;
            IsographEffect::LogHelloWorld.wrap_vec()
        }
        None => no_effects(),
    }
}
```

After:

```rust
// from crates/isograph_cli/src/state.rs
fn handle_internal_event<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    internal: InternalEvent,
) -> Vec<IsographEffect> {
    match internal {
        InternalEvent::HelloWorld => IsographEffect::LogHelloWorld.wrap_vec(),
        InternalEvent::Quit => IsographEffect::Kill.wrap_vec(),
        InternalEvent::DiskChanged(change) => handle_disk_changed(&mut state.db, change),
    }
}

fn handle_timer_fired<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    fired: freddie_timer::TimerFired<TimerPayload>,
) -> Vec<IsographEffect> {
    match fired.trigger_if_matching(state.diagnostics_debounce.as_ref()) {
        Some(&TimerPayload::DiagnosticsDebounce) => {
            state.diagnostics_debounce = None;
            crate::diagnostics::publish_diagnostics(state)
        }
        None => no_effects(),
    }
}
```

`use std::time::Duration;` leaves `state.rs`. A stale firing, a firing on a never-armed state, and a second delivery of the same firing return no effects.

### Document sync handlers

Before:

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn on_did_open<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    client: ClientId,
    params: lsp_types::DidOpenTextDocumentParams,
) -> Vec<IsographEffect> {
    did_open(&mut state.db, client, params).unwrap_or_else(|error| warn!(%error, "didOpen"));
    no_effects()
}
```

After:

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn on_did_open<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    client: ClientId,
    params: lsp_types::DidOpenTextDocumentParams,
) -> Vec<IsographEffect> {
    did_open(&mut state.db, client, params).map_or_else(
        |error| {
            warn!(%error, "didOpen");
            no_effects()
        },
        |()| crate::diagnostics::arm_diagnostics_debounce(state),
    )
}
```

`on_did_change` (`"didChange"`, `did_change`) and `on_did_close` (`"didClose"`, `did_close`) change the same way. A bad URI, a ranged change, or a change count other than one arms nothing.

### `run_session`

Before:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
            session: crate::event::LiveSession {
                sender: connection.sender.clone(),
                capabilities: params.capabilities,
                outstanding: std::collections::HashSet::new(),
            },
```

After:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
            session: crate::event::LiveSession {
                sender: connection.sender.clone(),
                capabilities: params.capabilities,
                published_diagnostics: crate::diagnostics::PublishedDiagnostics::default(),
                outstanding: std::collections::HashSet::new(),
            },
```

The server capabilities are unchanged: `publishDiagnostics` is a server-to-client notification with no server capability.

### `perform`

```rust
// from crates/isograph_cli/src/daemon.rs
        IsographEffect::PublishDiagnostics(publish) => {
            send_notifications(*publish);
            ControlFlow::Continue(())
        }
```

```rust
// from crates/isograph_cli/src/daemon.rs
/// Sends each notification in order; a disconnected session drops the rest.
fn send_notifications(publish: PublishDiagnosticsEffect) {
    let PublishDiagnosticsEffect {
        sender,
        notifications,
    } = publish;
    notifications
        .into_iter()
        .map(lsp_server::Message::Notification)
        .try_for_each(|message| sender.send(message))
        .unwrap_or_else(|_disconnected| tracing::debug!("publishDiagnostics to a closed session"));
}
```

`run_event_loop` and `run_effect_loop` are unchanged.

## Panics

This doc adds no panic and no `UnexpectedError`. `publish_diagnostics` is not a request; there is no response to carry an `UnexpectedError`, so failures are `warn!`. `LineIndex::position` indexes only with bounds it established (see its section). Serialization goes through `serde_json::to_value` with `?`, not `Notification::new`.

## Shared state

None added. `PublishedDiagnostics` lives on `LiveSession` inside `IsographState`, written only by `handle`. The debounce guard and `source_globs` are fields on `IsographState`. `freddie_timer::TimerId::mint` uses a `static AtomicU64` inside freddie; that is existing behavior of the dependency, reached already by `HelloWorld`.

## Known risks

- `perform` runs on the effect loop of a current-thread tokio runtime (`daemon.rs:24`). Each session's writer is `bounded(0)` (`lsp_socket.rs:48-56`), so `sender.send` blocks until the writer thread takes the message, and the writer thread blocks in `Message::write` when the socket buffer is full. A client that stops reading stalls its writer, then the next send blocks the runtime thread: the event loop, timers, the accept loop, and every other client stop. Responses already have this; diagnostics add traffic the client did not request, so a client that never sends a request can still stall the daemon.
- Traffic. A publish sends one notification per open path whose diagnostics changed, per client.
- Symlinked project directory. A window opened through a symlink (`/tmp/proj` when the config resolves to `/private/tmp/proj`) stores its open files under parent-relative keys (`../../../tmp/proj/src/a.ts`). A glob such as `src/**` does not match those keys, so that window gets no diagnostics. The watcher filters disk paths the same way, and semantic tokens and formatting already work on those keys. Fixing it is a `relative_from_uri` canonicalization change, not part of this doc.
- Never-written counter. pico records no dependency when a memo reads a tracked map whose counter was never written (lsp-open-files.md Known gap, `refactors/pending/pico-missing-key-dependency.md`; pico PR https://github.com/isographlabs/isograph/pull/994). Diagnostics cannot hit this gap, so this change does not wait for that fix. Every diagnosed path was opened by a `didOpen`, which writes `open_file_map` (and its counter) before any diagnostics memo reads it, and `file_contents(client, path)` for that path hits the `OpenFile`.
- Memo growth. `accept_loop` mints a new `ClientId` per connection (`lsp_socket.rs:96-101`); a window reload is a new client. Memo slots keyed by `(client, path)` are never collected: i2 never calls `run_garbage_collection`, and `Gone` removes sources, not memo slots. Each client adds `line_index`, `iso_literal_parse_errors_in_file`, and `lsp_diagnostics_for_file` slots for the files it opened, alongside the `file_contents`, extract, and parse slots semantic tokens already create for them.

## Upstream comparison

isograph `server.rs` arms `debounce_timer` for 100ms on every notification and every file-system batch, and on fire runs `validate_entire_schema` and `publish_new_diagnostics_and_clear_old_diagnostics`.

Copied:

- 100ms (`SHORT_DEBOUNCE_TIME`).
- Re-arm on each change, publish on quiet.
- Clear a path by sending an empty list when it was published before and has nothing now (`diagnostic_notification.rs`: `map.entry(uri).or_default()` for each old URI).
- `Display` messages (`diagnostic.printable(noop_print_location_fn())`).
- `PublishDiagnosticsParams.version: None`.
- URIs built from the working directory plus the relative path (`location_utils.rs:isograph_location_to_lsp_location`).

Not copied:

- `validate_entire_schema` is one memo slot for the whole project; a change to any file re-invokes it. Here each `(client, path)` is its own slot.
- `char_index_to_position` (`format.rs:73-91`) counts bytes as columns and breaks lines on `\n` only. Here columns are UTF-16 code units and `\n`, `\r\n`, `\r` are breaks.
- Diagnostics with no embedded location, or whose path is not in the iso literal map, or whose URI fails, are dropped with `continue` and no log. Here a URI or serialize failure is a `warn!` and the path is retried.
- `serde_json::to_value(...).expect(...)` and `lsp_server::Notification::new` (which unwraps). Here both are `?`.
- `BTreeSet<Uri>` of previously published URIs, and every publish resends every path with diagnostics. Here `PublishedDiagnostics` stores the lists and only changed paths are sent.
- Every notification and every file-system batch arms the timer, including ones that change nothing. Here only an accepted `didOpen` / `didChange` / `didClose` arms it.
- Diagnostics for every file in the project, read from disk with the open buffer as an overlay. Here only the client's open files, read from that client's buffers.
- Host-language validation errors. Here only parse errors.
- One connection, one set. Here each client has its own text and its own published state, and a client that does not declare `textDocument.publishDiagnostics` receives nothing.

## Changes

AGENTS.md: an API only tests call should not exist. `iso_literal_parse_errors_in_file`, `lsp_diagnostics_for_file`, `open_paths_of_client`, `uri_for_path`, and the `pub(crate)` `SourceGlobs` on `IsographState` have one production reader chain, which ends at `publish_diagnostics`. They therefore ship together with it.

### Change 1: owned `LineIndex` and the `line_index` memo (landed)

Landed in `d48dd131` (hover plan, `refactors/past/hover.md`).

### Change 2: diagnostics

One shippable change. It lands as commits in this order, each compiling with tests passing; `pub` items in library crates do not trip `dead_code` between commits, and every `pub(crate)` item lands in the commit with its caller.

1. `iso_literal_parse_errors_in_file` and its tests.
2. `lsp_diagnostics_for_file` and its tests.
3. `IsographDb::open_paths_of_client` and its tests.
4. `PublishedDiagnostics`, `LiveSession.published_diagnostics`, removal of the `expect(dead_code)` on `LiveSession.capabilities`, `SourceGlobs` made `pub(crate)`, `IsographState.source_globs` and its parse in `serve`, `uri_for_path`, `PathUriError`, `diagnostics.rs`, `PublishDiagnosticsEffect` and its `perform` arm, `TimerPayload::DiagnosticsDebounce` replacing `HelloWorldDebounced`, `diagnostics_debounce` replacing `hello_world_debounce`, the three arming call sites, and the `handle` tests.

## Tests

No sleeps, no polling. A timer fires by the test injecting `IsographEvent::Timer(timer.event)` from the returned `TimerEffect`. A notification is observed by calling `crate::daemon::perform` on the effect (the existing `perform_send` helper) and then `rx.recv()` on the session's unbounded crossbeam receiver; after `perform` returns the sends have completed, so `rx.try_recv()` returning `Empty` is the assertion that nothing else was sent. Tests collect notifications into a `HashMap<String, Vec<lsp_types::Diagnostic>>` keyed by URI, since order within an effect is unspecified.

### `handle` tests (`crates/isograph_cli/src/state.rs`)

Helpers: `dummy_session(sender, capabilities)` keeps its parameters and writes `published_diagnostics: PublishedDiagnostics::default()`; `diagnostics_capabilities() -> lsp_types::ClientCapabilities` is the default with `text_document: Some(TextDocumentClientCapabilities { publish_diagnostics: Some(Default::default()), ..Default::default() })`; `diagnostics_client(state, client) -> TestClient` starts a client with `diagnostics_capabilities()`; `only_timer(effects) -> TimerEffect<TimerPayload>` panics unless `effects` is exactly one `Timer`; `fire(state, effects) -> Vec<IsographEffect>` is `handle(state, IsographEvent::Timer(only_timer(effects).event))`; `published(client, effects) -> HashMap<String, Vec<Diagnostic>>` performs and drains. Fixture URIs are under `file:///tmp/proj` (`with_config`); `empty` sets `source_globs` to `src/**`.

- `didOpen` of `src/x.ts` with `iso(\`entrypoint\`)` returns one `Timer`; firing it returns one `PublishDiagnostics` whose one notification is `file:///tmp/proj/src/x.ts` with non-empty diagnostics, all `ERROR`, source `isograph`, no `code`.
- UTF-16 range: `didOpen` of `"const é = 1;\nconst 😀 = iso(\`entrypoint\`)"`; with `parsed = parse_iso_literal("entrypoint")`, the first diagnostic starts at line 1, character `"const 😀 = iso(\`".encode_utf16().count() as u32 + parsed.errors[0].location.start`.
- CRLF: the same literal after `"const a = 1;\r\n"` starts on line 1.
- A file with no literals: fire returns no effects.
- Whitespace-only literal `iso(\` \`)`: one diagnostic whose message is `ParseError::Ast(AstError::EmptyLiteral).to_string()`.
- A literal with only a host-language problem (`iso\`entrypoint Query.HomeRoute\``, which parses): fire returns no effects.
- Broken then fixed: second fire publishes that URI with `[]`; a third arm and fire returns no effects.
- Fixed then broken: first fire returns no effects; second publishes the errors.
- Two fires with no change: after a publish, `didChange` with identical text arms; the fire returns no effects.
- Two `didChange`s before a fire: firing the first timer returns no effects (its guard was dropped); firing the second publishes the second text's errors once.
- The same firing delivered twice: the second returns no effects.
- A firing on a never-armed state (timer taken from another state) returns no effects.
- Close clears: broken buffer published, `didClose`, fire: that URI with `[]`, with and without a `DiskFile` for that path, including a broken `DiskFile`.
- Disk-only broken file produces nothing: `DiskChanged` `Present` of `src/y.ts` with `iso(\`entrypoint\`)` returns no effects; a client then `didOpen`s a clean `src/x.ts`, and the fire returns no effects.
- `DiskChanged` returns no `Timer`: `Present`, `Absent`, and `FolderRemoved` each return no effects, including when a client has that path open.
- `Started` returns no `Timer`: `handle` of `Started` for a client with diagnostics capabilities returns no effects.
- Glob and host filtering: `didOpen` of `other/x.ts` (outside `src/**`) and of `src/README.md` (skipped by `should_skip_source_file`), each with `iso(\`entrypoint\`)`, arms; the fire returns no effects.
- Two clients, one path, different buffers: client 1 and client 2 both `didOpen` `src/x.ts`, client 1 with `iso(\`entrypoint\`)`, client 2 with `iso(\` \`)`. One fire returns two effects. Client 1's diagnostics equal `lsp_diagnostics_for_file(db, client 1, path)` and contain no `EmptyLiteral` message; client 2's diagnostics are exactly the one `EmptyLiteral` diagnostic. Client 2 then `didChange`s to clean text: the fire publishes `[]` to client 2 only, and client 1's `try_recv` is `Empty`.
- Another client's open file: client 1 opens broken `src/x.ts`, client 2 opens nothing: client 2 receives nothing.
- `Gone` before the fire: fire returns no effects for the gone client; another live client is unaffected.
- Capability absent: a client started with `ClientCapabilities::default()` and a broken buffer receives nothing; a client with `text_document: Some(Default::default())` (so `publish_diagnostics: None`) receives nothing.
- `didOpen` of a non-file URI, `didChange` with a ranged change, and `didOpen` from a never-started client return no effects and leave `diagnostics_debounce` unchanged.

Existing tests that change:

- Assert one `Timer` (via `only_timer`) instead of `effects.is_empty()`: `did_open_stores_the_buffer_for_that_client_only`, `did_change_with_one_full_change_replaces_the_buffer`, `did_close_falls_back_to_disk`.
- `client_started_stores_the_session` keeps `assert!(effects.is_empty())` and its `session.capabilities` assertion, and asserts `session.published_diagnostics == PublishedDiagnostics::default()`.
- `hello_world_returns_a_timer` becomes `hello_world_returns_log_hello_world`; `lsp_isograph_event_hello_world_returns_a_timer` becomes `lsp_isograph_event_hello_world_returns_log_hello_world`.
- Deleted (their behavior is covered by the diagnostics timer tests above): `matching_hello_world_debounced_returns_log_hello_world`, `stale_hello_world_debounced_returns_no_effects`, `a_second_hello_world_drops_the_first_guard`, `a_matching_hello_world_debounced_fed_twice_logs_once`, `lsp_isograph_event_hello_world_then_its_timer_returns_log_hello_world`.
- `timer_fired_on_a_never_armed_state_returns_no_effects` arms with `didOpen` in a second state.
- `lsp_notification_from_a_client_that_never_started_returns_no_effects` and `lsp_notification_after_gone_returns_no_effects` assert `diagnostics_debounce.is_none()` in place of `hello_world_debounce.is_none()`.
- `crates/isograph_cli/src/daemon.rs`: `sending_hello_world_emits_a_timer` becomes `sending_hello_world_emits_log_hello_world`; the three timer tests use `TimerPayload::DiagnosticsDebounce`.

### Plain-function tests (`crates/isograph_cli/src/diagnostics.rs`)

- `changed_diagnostics`: empty and empty yields nothing; a new path yields it; an equal list yields nothing; a different list yields the new list; a published path absent from current yields `[]`.

### `uri_for_path` (`crates/isograph_cli/src/adapter.rs` tests)

- `src/a.ts` is `file:///tmp/proj/src/a.ts`, and `relative_from_uri` of it is `src/a.ts`.
- `src/a b/é.ts` is `file:///tmp/proj/src/a%20b/%C3%A9.ts`, and `relative_from_uri` of it is `src/a b/é.ts`.
- `../other/a.ts` is `file:///tmp/other/a.ts` and round-trips.
- `src/./a.ts` is `file:///tmp/proj/src/a.ts`.
- No `CurrentWorkingDirectory` is `MissingWorkingDirectory`.

### `open_paths_of_client` (`crates/isograph_compiler/src/database.rs` tests)

Empty database is empty; disk paths only is empty; one client's open path is returned; a path both on disk and open appears once; another client's open path is excluded; after `remove_open_file` the path is gone.

### Memo tests

`iso_literal_parse_errors_in_file` (`crates/isograph_extract_typescript/src/lib.rs` `memo_tests`, which already builds `IsographDb<TypeScriptHostLanguage>` fixtures):

- Missing file is `None`. A file with no literals is `Some(vec![])`.
- `iso(\`entrypoint\`)`: the errors equal `parse_iso_literal("entrypoint").errors` with each span shifted by the literal's `iso_literal_start_index`.
- Prepending `"const x = 1;\n"` shifts every error span by that length and leaves every `item` equal.
- Two literals in one file: the errors of both, each shifted by its own start.
- Two clients: client 1's broken buffer has errors, client 2 reading clean disk has none.

`lsp_diagnostics_for_file` (`crates/isograph_lsp/src/file_diagnostics.rs`, with `isograph_extract_typescript` as the existing dev-dependency):

- Missing file is `None`; no literals is `Some(vec![])`.
- Each message equals the `to_string()` of the corresponding `iso_literal_parse_errors_in_file` item; severity `ERROR`; source `isograph`; `code` `None`.
- CRLF and lone `\r` before the literal put the range on line 1.
- A surrogate pair (`😀`) before the literal on its line adds 2 to the start character.

## Docs this change amends

- `docs-website/docs/design-docs/event-model.md`: `IsographEffect` gains `PublishDiagnostics(Box<PublishDiagnosticsEffect>)`; "HelloWorld arms a timer; `handle` of that `TimerFired` is `trigger_if_matching` and `LogHelloWorld`" becomes "HelloWorld returns `LogHelloWorld`; an accepted `didOpen`, `didChange`, or `didClose` arms the diagnostics debounce, and `handle` of that `TimerFired` returns `PublishDiagnosticsEffect`s"; "Effects from `handle` (`ReportDiagnostics`)" becomes `PublishDiagnosticsEffect`; the effect loop list adds `PublishDiagnostics`; `IsographState` names `source_globs` and `diagnostics_debounce`; "`handle` does not know about globs, gitignore, or \"in scope\". Scope is the watcher's job." becomes "Scope of disk files is the watcher's job. `handle` reads `source_globs` only to choose which open files get diagnostics."
- `docs-website/docs/design-docs/pico.md`, Iso literals: add the chain `lsp_diagnostics_for_file(client, path) -> iso_literal_parse_errors_in_file(client, path) -> parsed_iso_literals_in_file(client, path)`, plus `line_index(client, path)`. Outbound I/O state: the `IsographState` snippet names `source_globs` and `diagnostics_debounce`, and "the planned last-published diagnostics per client" becomes `LiveSession.published_diagnostics`.
- `refactors/pending/tests-only-functionality.md`: the Diagnostics block's "Production caller: lsp-diagnostics.md via `file_literals`" becomes "Production caller: a later host-errors change; lsp-diagnostics.md reports parse errors only".
- `refactors/pending/lsp-open-files.md` change 7 "Docs this change amends" describes the previous shape of this doc (`report_diagnostics`, `file_literals`, `PublishDiagnosticsCap`); it is history of that change and is not edited.
