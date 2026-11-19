# Debounced `publishDiagnostics` per client

Each live LSP client that accepts diagnostics receives `textDocument/publishDiagnostics` for the files it reads, computed through `file_contents(db, client, path)`: that client's open buffer when it has one, otherwise the `DiskFile`. A change that can alter diagnostics arms one 100ms debounce timer. When the timer fires, `handle` computes each client's current diagnostics, compares them with what that client was last sent, and returns one `PublishDiagnosticsEffect` per client that has a difference. A path whose errors went away is sent an empty list.

Builds on the current code: `file_contents` and the client-keyed iso literal memos (lsp-open-files.md change 7), `freddie_timer` timers (`IsographEffect::Timer`, `IsographEvent::Timer`), and `LiveSessions`.

Origin: isograph `crates/isograph_lsp/src/server.rs` (debounce, `SHORT_DEBOUNCE_TIME`) and `crates/isograph_lsp/src/diagnostic_notification.rs` (`publish_new_diagnostics_and_clear_old_diagnostics`). Deltas are in Upstream comparison.

## What the user does

The user opens `src/Home.ts` in VS Code and types `export const Home = iso(\`entrypoint\`)`. About 100ms after the last `didChange`, a red squiggle covers the error range inside the literal and the Problems panel lists the error with source `isograph` and the parse error's message. The user types `entrypoint Query.HomeRoute`; 100ms later the squiggle and the Problems entry disappear. Typing faster than the debounce publishes once, for the text the editor had at the last change.

`iso\`entrypoint Query.HomeRoute\`` (no parentheses) is a squiggle over the literal interior with the message `You must call the iso function with parentheses. "iso`...`" is not supported`.

What else appears depends on open decision 1:

- (a) open files only: only files open in that window have entries. Closing a tab clears its entries.
- (b) disk plus open (the design below): when the window connects, the Problems panel lists errors in every file on disk, including files not open. Closing a tab with unsaved broken edits replaces the buffer's errors with the disk file's errors (or clears them if the disk file is clean). Deleting a file on disk clears its entries.
- (c) upstream's disk with overlay: as (b), except a file open in the editor that has no `DiskFile` gets no entries.

Two VS Code windows on one project are two clients. Window A's unsaved errors appear only in A. A file B has not opened shows its disk errors in B. When A saves, the watcher posts `DiskChanged` and B's entries for that path (if B does not have it open) update to the saved text.

`isograph send` connections initialize with `{ "capabilities": {} }` and receive nothing (open decision 4).

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

`PublishedDiagnostics` and `PublishDiagnosticsSupport` are outbound I/O state: what the daemon has written to one connection. They are not pico sources. No memo reads them; only `handle` reads and writes them.

```rust
// from crates/isograph_cli/src/diagnostics.rs
/// What one client was last sent: every path whose last notification had a non-empty list.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PublishedDiagnostics(pub HashMap<RelativePathToSourceFile, Vec<lsp_types::Diagnostic>>);

/// Whether a client receives `publishDiagnostics`, and what it was last sent.
#[derive(Clone, Debug, PartialEq)]
pub enum PublishDiagnosticsSupport {
    Supported(PublishedDiagnostics),
    Unsupported,
}

impl PublishDiagnosticsSupport {
    /// `Supported` when the client declared `textDocument.publishDiagnostics`.
    pub fn from_client_capabilities(capabilities: &lsp_types::ClientCapabilities) -> Self {
        match capabilities
            .text_document
            .as_ref()
            .and_then(|text_document| text_document.publish_diagnostics.as_ref())
        {
            Some(_) => Self::Supported(PublishedDiagnostics::default()),
            None => Self::Unsupported,
        }
    }
}
```

The published map rides inside the `Supported` variant, so an `Unsupported` client has no published map to keep empty.

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
    pub publish_diagnostics: crate::diagnostics::PublishDiagnosticsSupport,
    pub outstanding: std::collections::HashSet<lsp_server::RequestId>,
}
```

`capabilities` is deleted. `run_session` reads `params.capabilities` once to build `PublishDiagnosticsSupport`; after that nothing in production reads the stored capabilities, and tests are not a reader. A later feature that needs another capability adds its own derived field the same way.

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
    pub(crate) diagnostics_debounce: Option<freddie_timer::TimerGuard>,
}
```

Construction sites that write `hello_world_debounce: None` write `diagnostics_debounce: None`: `daemon.rs` `serve` and its `empty_state` test helper, `lsp_socket.rs` `listen_and_reply`, `watch.rs` test fixture, `state.rs` `empty`.

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

Each function asks for one thing.

```text
publish_diagnostics(state)                          handle side, plain fn
  for each live client with Supported:
    client_diagnostics(db, client)                  plain fn
      db.paths_read_by(client)                      plain method
      lsp_diagnostics_for_file(db, client, path)    memo, isograph_lsp
        iso_literal_errors_in_file(client, path)    memo, isograph_compiler
          parsed_iso_literals_in_file(client, path)
          HostLanguage::host_errors(extraction, parsed)
        line_index(db, client, path)                memo, isograph_lsp
          file_contents(client, path)
    changed_diagnostics(published, current)         plain fn
    uri_for_path(db, path)                          plain fn
```

### `iso_literal_errors_in_file`

`crates/isograph_compiler/src/iso_literals.rs`, exported from `lib.rs`. Body moved from `file_literals` (`crates/isograph_extract_typescript/src/lib.rs:136-162`). Delta: generic over the host, calls `THostLanguage::host_errors` instead of the free `host_errors_for_extraction`, reads `parsed_iso_literals_in_file` instead of `extract_iso_literals` plus `parsed_iso_literal` per extraction, flattens to one list per file instead of one `FileLiteral` per literal, and is a memo.

```rust
// from crates/isograph_compiler/src/iso_literals.rs
/// Host and parse errors of every iso literal in `path` as `client` reads it, as file-absolute spans.
#[memo]
pub fn iso_literal_errors_in_file<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<WithSpan<IsoLiteralError<THostLanguage>>>> {
    let literals = parsed_iso_literals_in_file(db, client, path).as_ref()?;
    literals
        .iter()
        .flat_map(|(extraction, parsed)| {
            THostLanguage::host_errors(extraction, parsed)
                .into_iter()
                .map(|error| error.map(IsoLiteralError::Host))
                .chain(parsed.errors.iter().map(|error| {
                    IsoLiteralError::Parse(error.item.clone())
                        .with_span(error.location.with_offset(extraction.span().start))
                }))
        })
        .collect::<Vec<_>>()
        .wrap_some()
}
```

`None` is no text for that client. `Some(vec![])` is text with no errors (including no literals). Parse error spans are relative to the literal text in `ParsedIsoLiteral` (pico.md: a prepend does not make the parse `!=`); this memo shifts them by the literal's start. Host error spans are already file-absolute (`extraction.span()`).

### `HostLanguage::host_errors`

Before:

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait HostLanguage: Send + Sync + Sized + 'static {
    type Error: std::fmt::Display + std::error::Error + Clone + PartialEq + Eq + 'static;
    type LiteralContext: Clone + PartialEq + Eq + Debug + 'static;

    fn extract_iso_literals_from_text(contents: &str) -> Vec<IsoLiteralExtraction<Self>>;

    fn should_skip_source_file(relative_path: &Path) -> SkipSourceFile;
}
```

After:

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait HostLanguage: Send + Sync + Sized + 'static {
    type Error: std::fmt::Display + std::error::Error + Clone + PartialEq + Eq + 'static;
    type LiteralContext: Clone + PartialEq + Eq + Debug + 'static;

    fn extract_iso_literals_from_text(contents: &str) -> Vec<IsoLiteralExtraction<Self>>;

    /// Errors in the host-language code around one literal, as file-absolute spans.
    fn host_errors(
        extraction: &IsoLiteralExtraction<Self>,
        parsed: &ParsedIsoLiteral,
    ) -> Vec<WithSpan<Self::Error>>;

    fn should_skip_source_file(relative_path: &Path) -> SkipSourceFile;
}
```

A plain associated function, like `extract_iso_literals_from_text`. `host_language.rs` imports `isograph_parser::ParsedIsoLiteral` and `span::WithSpan`.

The TypeScript impl is the body of `host_errors_for_extraction` (`crates/isograph_extract_typescript/src/lib.rs:100-124`), verbatim. Delta: it is a trait method, and the return type names `Self::Error` instead of `TypeScriptHostError`. `item_of` stays a private free function.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
impl HostLanguage for TypeScriptHostLanguage {
    // type LiteralContext, type Error, extract_iso_literals_from_text, should_skip_source_file unchanged

    fn host_errors(
        extraction: &IsoLiteralExtraction<Self>,
        parsed: &ParsedIsoLiteral,
    ) -> Vec<span::WithSpan<Self::Error>> {
        let span = extraction.span();
        let mut errors = Vec::new();
        if let IsoCall::TaggedTemplate = extraction.context.call {
            errors.push(TypeScriptHostError::MissingParentheses.with_span(span));
        }
        let parsed_item = parsed.item.as_ref().and_then(item_of);
        if let Some(IsoLiteralItem::Selectable(selectable)) = parsed_item {
            if extraction.context.const_export_name.is_none() {
                errors.push(
                    TypeScriptHostError::MissingExport {
                        suggested_name: selectable.name.item,
                    }
                    .with_span(span),
                );
            }
            if let AssociatedJsFunction::Absent = extraction.context.associated_js_function {
                errors.push(TypeScriptHostError::MissingAssociatedFunction.with_span(span));
            }
        }
        errors
    }
}
```

The test hosts `TestHostLanguage` (`crates/isograph_compiler/src/database.rs` tests) and `InHost` (`crates/isograph_cli/src/watch.rs` tests) return `Vec::new()`.

Deleted from `crates/isograph_extract_typescript/src/lib.rs`: the free `host_errors_for_extraction`, `FileLiteral`, and `file_literals`. The imports `ClientId`, `IsoLiteralError`, `IsographDb`, `extract_iso_literals`, `parsed_iso_literal`, and `RelativePathToSourceFile` leave the non-test part of that file.

### `line_index`

`crates/isograph_lsp/src/line_index.rs`. Change 1 adds the `LineIndex` type; change 2 adds the memo.

`LineIndex` in `semantic_tokens.rs` today borrows the text (`struct LineIndex<'a> { text: &'a str, breaks: Vec<LineBreak> }`, `crates/isograph_lsp/src/semantic_tokens.rs:168-171`). A memo result is owned (pico.md: Memo), so a borrowed index cannot be the memo's value. The owned `LineIndex` stores the line breaks and the non-ASCII chars, so it converts a byte offset to a UTF-16 position without the text. The shape is rust-analyzer's `line-index` crate (`newlines` plus `WideChar`s). That crate is not used because it breaks lines on `\n` only; iso tokenizes `\n`, `\r\n`, and `\r` as line breaks (`line_breaks` doc comment), and diagnostics, semantic tokens, and formatting must agree with it.

```rust
// from crates/isograph_lsp/src/line_index.rs
/// Byte offsets of the line breaks and the non-ASCII chars of one text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineIndex {
    pub(crate) breaks: Vec<LineBreak>,
    wide_chars: Vec<WideChar>,
}

/// One line terminator. `start` is the first byte of `\n`, `\r`, or `\r\n`.
/// `after` is the first byte of the next line.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct LineBreak {
    pub(crate) start: u32,
    pub(crate) after: u32,
}

/// A non-ASCII char. Its UTF-8 length exceeds its UTF-16 length.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct WideChar {
    start: u32,
    utf8_len: u8,
    utf16_len: u8,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        Self {
            breaks: line_breaks(text),
            wide_chars: text
                .char_indices()
                .filter(|(_, character)| !character.is_ascii())
                .map(|(start, character)| WideChar {
                    start: start as u32,
                    utf8_len: character.len_utf8() as u8,
                    utf16_len: character.len_utf16() as u8,
                })
                .collect(),
        }
    }

    /// 0-based line, and UTF-16 code units from the line start to `offset`.
    pub fn position(&self, offset: u32) -> lsp_types::Position {
        let line = self
            .breaks
            .partition_point(|line_break| line_break.after <= offset);
        let line_start = match line {
            0 => 0,
            n => self.breaks[n - 1].after,
        };
        let first = self
            .wide_chars
            .partition_point(|wide| wide.start < line_start);
        let last = self
            .wide_chars
            .partition_point(|wide| wide.start + u32::from(wide.utf8_len) <= offset);
        let narrowed: u32 = self.wide_chars[first..last]
            .iter()
            .map(|wide| u32::from(wide.utf8_len - wide.utf16_len))
            .sum();
        lsp_types::Position {
            line: line as u32,
            character: offset - line_start - narrowed,
        }
    }

    pub fn range(&self, span: Span) -> lsp_types::Range {
        lsp_types::Range {
            start: self.position(span.start),
            end: self.position(span.end),
        }
    }
}
```

`position` does not slice the text and does not panic. `n - 1` is in range because `n <= breaks.len()`. `first <= last` because `line_start <= offset`. Only chars that end at or before `offset` are counted, so the subtraction cannot underflow; an offset that is not a char boundary counts the partial char's bytes as code units.

`line_breaks` moves from `semantic_tokens.rs:299-327` to `line_index.rs` verbatim (with its doc comment and `LineBreak`).

```rust
// from crates/isograph_lsp/src/line_index.rs
/// The line index of `path` as `client` reads it.
#[memo]
pub fn line_index<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<LineIndex> {
    let contents = file_contents(db, client, path).as_ref()?;
    LineIndex::new(contents).wrap_some()
}
```

`lib.rs` adds `mod line_index;` and `pub use line_index::{LineIndex, line_index};`.

The memo is also the planned reader for hover's `LineChar` conversion. Semantic tokens and formatting keep building a `LineIndex` per call from `text_through_last_iso_literal`, not from this memo: the memo reads the whole file, so typing after the last literal would re-invoke them, where today `text_through_last_iso_literal` backdates and they do not.

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
    let errors = iso_literal_errors_in_file(db, client, path).as_ref()?;
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

The early return means a file with no errors does not compute or store a line index; with (b) that is most of the project, per client. The message is `IsoLiteralError`'s `Display` (`#[error("{0}")]` of the host or parse error). Severity and `code` are open decision 5.

### `paths_read_by`

```rust
// from crates/isograph_compiler/src/database.rs
impl<THostLanguage: HostLanguage> IsographDb<THostLanguage> {
    /// Every path with a `DiskFile`, plus every path `client` has open.
    pub fn paths_read_by(&self, client: ClientId) -> HashSet<RelativePathToSourceFile> {
        let disk_file_map = self.get_disk_file_map();
        let open_file_map = self.get_open_file_map();
        disk_file_map
            .tracked()
            .0
            .keys()
            .copied()
            .chain(
                open_file_map
                    .tracked()
                    .0
                    .keys()
                    .filter(|key| key.client == client)
                    .map(|key| key.path),
            )
            .collect()
    }
}
```

A plain method. `handle` calls it with no memo on the stack, so the `tracked()` reads record nothing (`dependency_stack.push_if_not_empty`, `crates/pico/src/database.rs:109-121`); `tracked()` is used because AGENTS.md forbids iterating `untracked()`. As a memo it would hit the never-written counter gap (Known risks): before the first `set_open_file` it would record no dependency on `open_file_map` and never see a later open.

It returns a `HashSet`, not `impl Iterator`. `View::tracked(&'a self) -> &'a T` (`crates/pico/src/view.rs:29-32`) borrows the `View`, and the `View` is a local, so an iterator over its keys cannot leave the function. The set is also the deduplication of a path both on disk and open.

This is (b). (a) is the `open_file_map` half only; (c) is the `disk_file_map` half only.

### Publish

`crates/isograph_cli/src/diagnostics.rs`, new module (`mod diagnostics;` in `lib.rs`). Plain functions; no memo.

```rust
// from crates/isograph_cli/src/diagnostics.rs
/// Changed diagnostics for every live client that accepts them, one effect per client.
pub(crate) fn publish_diagnostics<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
) -> Vec<IsographEffect> {
    let db = state.db.reference();
    state
        .live_sessions
        .0
        .iter_mut()
        .filter_map(|(client, session)| publish_to_client(db, client.dereference(), session))
        .collect()
}

fn publish_to_client<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    session: &mut LiveSession,
) -> Option<IsographEffect> {
    let PublishDiagnosticsSupport::Supported(published) = &mut session.publish_diagnostics else {
        return None;
    };
    let previous = std::mem::take(published);
    let mut current = client_diagnostics(db, client);
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
    *published = PublishedDiagnostics(current);
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
/// Non-empty diagnostics of every path `client` reads.
fn client_diagnostics<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
) -> HashMap<RelativePathToSourceFile, Vec<lsp_types::Diagnostic>> {
    db.paths_read_by(client)
        .into_iter()
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

A closed, deleted, or fixed path is absent from `current` (it is not read, or its list is empty), so `changed_diagnostics` yields `vec![]` for it when it was published before. That is how the client clears squiggles. Notifications within one effect are in `HashMap` order; the protocol does not order them and tests do not depend on it.

`version` is `None` (open decision 7).

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

`relative_from_uri` produces parent-relative keys (`../other/a.ts`) through `pathdiff`. `Url::from_file_path` keeps `..` segments (`url-2.5.8/src/lib.rs:2948-2983` percent-encodes each component), and VS Code does not resolve them, so `file:///tmp/proj/../other/a.ts` would not match the editor's `file:///tmp/other/a.ts`. std's `Path::normalize_lexically` is unstable and the workspace builds on stable; no crate in `Cargo.lock` provides it. Lexical `..` is correct here because `CurrentWorkingDirectory` is the parent of the canonicalized config path (`discover.rs:63`) and `pathdiff` computes the relative part lexically from it. `Url::from_file_path` percent-encodes spaces and other reserved bytes.

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

Overwriting `diagnostics_debounce` drops the previous `TimerGuard`, which closes its oneshot and cancels its sleep in `schedule_timer` (`daemon.rs:216-233`). A firing that raced the drop fails `trigger_if_matching` because its `TimerId` differs.

Callers: `on_did_open`, `on_did_change`, `on_did_close` when the write succeeded; `handle_disk_changed` when `apply_disk_changed` succeeded (`File` `Present`, `File` `Absent`, `FolderRemoved`); `handle_client_connection` on `Started`. Not on `Gone`: the gone client's published state goes with its `LiveSession`, and other clients do not read its buffers. Not on `HelloWorld`, `Quit`, requests, or responses.

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
        InternalEvent::DiskChanged(change) => handle_disk_changed(state, change),
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

### `handle_disk_changed`

Before:

```rust
// from crates/isograph_cli/src/state.rs
fn handle_disk_changed<THostLanguage: HostLanguage>(
    state: &mut IsographDb<THostLanguage>,
    change: DiskChanged,
) -> Vec<IsographEffect> {
    apply_disk_changed(state, change)
        .unwrap_or_else(|error| tracing::warn!(%error, "DiskChanged path"));
    no_effects()
}
```

After:

```rust
// from crates/isograph_cli/src/state.rs
fn handle_disk_changed<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    change: DiskChanged,
) -> Vec<IsographEffect> {
    apply_disk_changed(&mut state.db, change).map_or_else(
        |error| {
            tracing::warn!(%error, "DiskChanged path");
            no_effects()
        },
        |()| crate::diagnostics::arm_diagnostics_debounce(state),
    )
}
```

### `handle_client_connection`

Before:

```rust
// from crates/isograph_cli/src/state.rs
        ClientConnectionEvent::Started(started) => {
            state
                .live_sessions
                .0
                .insert(started.client, started.session);
            no_effects()
        }
```

After:

```rust
// from crates/isograph_cli/src/state.rs
        ClientConnectionEvent::Started(started) => {
            state
                .live_sessions
                .0
                .insert(started.client, started.session);
            crate::diagnostics::arm_diagnostics_debounce(state)
        }
```

A `Started` for an id already live replaces the session, including its published map; the new session starts from an empty map and receives the full set.

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
                publish_diagnostics:
                    crate::diagnostics::PublishDiagnosticsSupport::from_client_capabilities(
                        params.capabilities.reference(),
                    ),
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

None added. `PublishedDiagnostics` lives on `LiveSession` inside `IsographState`, written only by `handle`. The debounce guard is a field on `IsographState`. `freddie_timer::TimerId::mint` uses a `static AtomicU64` inside freddie; that is existing behavior of the dependency, reached already by `HelloWorld`.

## Open decisions

None of these is decided. The design above is written with the first-listed option where one is named in the sections above.

1. Which files each client gets.
   - (a) Open files only. `client_diagnostics` iterates this client's `open_file_map` keys. No per-client copy of disk text (see Known risks, memo growth). `didClose` always clears. Errors in files the user has not opened are invisible until opened.
   - (b) Disk plus open (`paths_read_by` above). The Problems panel is project-wide. Each client computes `file_contents`, extract, parse, and errors for every disk path under its own `ClientId`, which stores one copy of every disk file's text per client. A variant reads disk-only paths under one shared reader `ClientId` so clients share those memo slots; `event-model.md` says how such a reader gets its `ClientId` is not designed.
   - (c) Upstream: disk paths with overlay. A buffer with no `DiskFile` (a new file before its first save is still a `file:` URI only after save, so this is mainly files outside the watched set) gets no diagnostics.
   - Filtering. `did_open` stores any `file:` URI, including `README.md` and paths outside the config directory (`../other/a.ts`). `handle` does not know globs (`event-model.md`). Options: no filtering (a markdown file containing `` iso`...` `` gets diagnostics); filter in `did_open` with `THostLanguage::should_skip_source_file` (no globs needed, extension only, also changes what semantic tokens and formatting see); filter `paths_read_by` the same way (diagnostics only); or carry `source_files` globs into `IsographState` (contradicts event-model.md's "handle does not know about globs" and needs that doc amended).

2. Debounce interval and timer granularity.
   - Interval: 100ms is upstream `SHORT_DEBOUNCE_TIME`. vscode-languageclient batches full-sync `didChange` for up to 250ms (lsp-open-files.md, What the user does), so 100ms normally fires between batches.
   - One global timer (above): any client's change delays every client's publish, and one publish computes every client. Simple and one field.
   - Per client: `TimerGuard` is not `Clone` (`freddie_timer/src/timer.rs`, no derive), and `LiveSession` is `Clone` because `ClientStartedEvent` and `IsographEvent` derive `Clone`, so the guard cannot sit on `LiveSession`; it would be `HashMap<ClientId, TimerGuard>` on `IsographState`, removed on `Gone`. `TimerFired`'s payload is private and readable only through `trigger_if_matching(guard)`, so `TimerPayload::DiagnosticsDebounce(ClientId)` cannot route the firing; `handle_timer_fired` tries each client's guard in turn. `DiskChanged` arms every client's timer.

3. Where published state lives.
   - (a) `LiveSession` (above). `Gone` drops it with the session; a replacing `Started` resets it.
   - (b) `HashMap<ClientId, PublishedDiagnostics>` on `IsographState`. `Gone` and a replacing `Started` must also clear the entry, a second place to keep in sync with `LiveSessions`.
   - (c) A set of published paths only, like upstream's `BTreeSet<Uri>`. Every publish resends every non-empty list and clears paths that left the set. No stored `Vec<Diagnostic>`; traffic on each fire proportional to the number of files with errors, not to the number that changed.

4. Capability gating. The spec's `textDocument.publishDiagnostics` client capability lists optional features (`relatedInformation`, `tagSupport`, `versionSupport`, `codeDescriptionSupport`, `dataSupport`; `lsp-types-0.97.0/src/lib.rs:1270-1304`); the spec does not say a client without it rejects the notification.
   - (a) Gate on presence (above). `isograph send` and the socket and CLI tests send `{ "capabilities": {} }` and receive nothing; their `read_message` sequences stay valid. `capabilities` leaves `LiveSession`.
   - (b) Publish to every client. `PublishDiagnosticsSupport` is replaced by `PublishedDiagnostics` on every session. `isograph send` connections receive notifications until they close (sends then fail and are ignored). The `lsp_socket.rs` and `cli.rs` tests that read the next message as a response must skip notifications, because a `DiskChanged` they inject arms a real 100ms timer.

5. Severity and code. (a) All `ERROR` (above). (b) Host errors `WARNING`, parse errors `ERROR`: `lsp_diagnostics_for_file` matches `IsoLiteralError` variants. A per-variant `code` (`NumberOrString::String("missing-export")`, through `strum` or a match) lets later code actions and filters key on the error; it adds one name per variant of `TypeScriptHostError` and `ParseError`.

6. Host error ranges. Today every host error spans the literal interior (`extraction.span()`), so a missing-parentheses squiggle covers the whole literal. Alternative: `TypeScriptLiteralContext` records the byte ranges of `iso`, `(`, and `)(` from the regex captures, and `host_errors` uses them (`MissingParentheses` on `iso`, `MissingAssociatedFunction` after the closing paren, `MissingExport` on `iso(`). The context then carries spans; extract is already `!=` after a prepend because `iso_literal_start_index` moves, so no new invalidation.

7. `version`. `None` (above), as upstream. Sending the buffer version requires storing the `didOpen` / `didChange` version: on `OpenFile` (a version-only change becomes a source change; `file_contents` re-invokes and backdates) or in a map on `IsographState`. Disk-only paths have no version either way. A client may use the version to drop diagnostics computed for an older buffer.

8. URIs rebuilt from `CurrentWorkingDirectory` plus path (above) or remembered from the client. The config directory is canonical (`/private/tmp/proj` on macOS). A window opened at `/tmp/proj` sends `file:///tmp/proj/src/a.ts`, which `relative_from_uri` turns into `../../../tmp/proj/src/a.ts`, a different key from the disk key `src/a.ts`. With rebuilt URIs and lexical normalization the open key publishes to `file:///tmp/proj/src/a.ts`, which matches the editor, and the disk key publishes to `file:///private/tmp/proj/src/a.ts`, a second Problems entry for a file VS Code considers unopened. Remembering the client's URI per `(client, path)` reproduces the client's spelling for open paths but does not merge the two keys; the duplicate is a `relative_from_uri` canonicalization gap that this doc does not fix.

9. Order relative to the never-written counter fix (Known risks). Change 2 makes the gap reachable on every daemon start. Options: land `refactors/pending/pico-missing-key-dependency.md` (or upstream pico PR https://github.com/isographlabs/isograph/pull/994) first; land lsp-open-files.md change 3 option (a) (`IsographDb::default` writes each counter once) first; or ship change 2 with the gap.

## Known risks

- `perform` runs on the effect loop of a current-thread tokio runtime (`daemon.rs:24`). Each session's writer is `bounded(0)` (`lsp_socket.rs:48-56`), so `sender.send` blocks until the writer thread takes the message, and the writer thread blocks in `Message::write` when the socket buffer is full. A client that stops reading stalls its writer, then the next send blocks the runtime thread: the event loop, timers, the accept loop, and every other client stop. Responses already have this; diagnostics add traffic the client did not request, so a client that never sends a request can still stall the daemon.
- Traffic. The first publish after `Started` sends one notification per path with errors (with (b), across the project). Later publishes send only changed paths.
- Never-written counter. pico records no dependency when a memo reads a tracked map whose counter was never written (lsp-open-files.md Known gap, `refactors/pending/pico-missing-key-dependency.md`; pico PR https://github.com/isographlabs/isograph/pull/994 fixes it). The `open_file_map` counter is first written by the first `set_open_file` in the process. A window that connects with no file open gets a publish 100ms after `Started`; with (b) that runs `file_contents(client, p)` for every disk path, each missing the open map with no recorded dependency. The first `didOpen` of such a `p` sets a vacant counter and a vacant source and does not advance the epoch, so `file_contents(client, p)` keeps returning disk text. Later `didChange`s do not re-invoke it either, because its only recorded dependency is the `DiskFile`. Diagnostics, semantic tokens, and formatting for that `(client, p)` then use disk text until the disk file changes; formatting computes edits from disk ranges and applies them to the buffer. With (a) the publish never reads a path the client has not opened, so this path to the gap does not exist.
- Memo growth. `accept_loop` mints a new `ClientId` per connection (`lsp_socket.rs:96-101`); a window reload is a new client. Memo slots keyed by `(client, path)` are never collected: i2 never calls `run_garbage_collection`, and `Gone` removes sources, not memo slots. With (b) each diagnostics client stores a `file_contents` copy of every disk file, plus extract, parse pairs, errors, and diagnostics.

## Upstream comparison

isograph `server.rs` arms `debounce_timer` for 100ms on every notification and every file-system batch, and on fire runs `validate_entire_schema` and `publish_new_diagnostics_and_clear_old_diagnostics`.

Copied:

- 100ms (`SHORT_DEBOUNCE_TIME`).
- Re-arm on each change, publish on quiet.
- Clear a path by sending an empty list when it was published before and has nothing now (`diagnostic_notification.rs`: `map.entry(uri).or_default()` for each old URI).
- `Display` messages (`diagnostic.printable(noop_print_location_fn())`).
- URIs built from the working directory plus the relative path (`location_utils.rs:isograph_location_to_lsp_location`).

Not copied:

- `validate_entire_schema` is one memo slot for the whole project; a change to any file re-invokes it. Here each `(client, path)` is its own slot.
- `char_index_to_position` (`format.rs:73-91`) counts bytes as columns and breaks lines on `\n` only. Here columns are UTF-16 code units and `\n`, `\r\n`, `\r` are breaks.
- Diagnostics with no embedded location, or whose path is not in the iso literal map, or whose URI fails, are dropped with `continue` and no log. Here a URI or serialize failure is a `warn!` and the path is retried.
- `serde_json::to_value(...).expect(...)` and `lsp_server::Notification::new` (which unwraps). Here both are `?`.
- Every publish resends every path with diagnostics. Here only changed paths are sent.
- Every notification arms the timer, including ones that change nothing. Here only a successful `didOpen` / `didChange` / `didClose`, an applied `DiskChanged`, and `Started` arm it.
- One connection, one set. Here each client has its own text and its own published state.

## Changes

AGENTS.md: an API only tests call should not exist. `iso_literal_errors_in_file`, `line_index`, `lsp_diagnostics_for_file`, `paths_read_by`, and `uri_for_path` have one production reader chain, which ends at `publish_diagnostics`. They therefore ship together with it. The `LineIndex` type has production readers today (formatting and semantic tokens) and ships first.

### Change 1: owned `LineIndex` (prefactor)

`crates/isograph_lsp/src/line_index.rs` with `LineIndex`, `LineBreak`, `WideChar`, `line_breaks` (no memo yet). Production readers: `format_iso_literals` and `lsp_semantic_tokens`.

`lsp_range_for_byte_range` is deleted. Its one caller builds the index once per request instead of once per literal.

Before:

```rust
// from crates/isograph_cli/src/adapter.rs
    let literals = isograph_compiler::parsed_iso_literals_in_file(db, client, path).as_ref()?;
    let page_content =
        isograph_compiler::text_through_last_iso_literal(db, client, path).as_ref()?;
    literals
        .iter()
        .filter_map(|(extraction, parsed)| {
            if !parsed.errors.is_empty() {
                return None;
            }
            let start = extraction.iso_literal_start_index.0 as u32;
            let end = start + extraction.iso_literal_text.len() as u32;
            lsp_types::TextEdit {
                range: isograph_lsp::lsp_range_for_byte_range(page_content, start, end),
                new_text: isograph_parser::format_recorded(
                    extraction.iso_literal_text.as_str(),
                    parsed.tokens.as_slice(),
                ),
            }
            .wrap_some()
        })
        .collect::<Vec<_>>()
        .wrap_some()
```

After:

```rust
// from crates/isograph_cli/src/adapter.rs
    let literals = isograph_compiler::parsed_iso_literals_in_file(db, client, path).as_ref()?;
    let page_content =
        isograph_compiler::text_through_last_iso_literal(db, client, path).as_ref()?;
    let line_index = isograph_lsp::LineIndex::new(page_content);
    literals
        .iter()
        .filter_map(|(extraction, parsed)| {
            if !parsed.errors.is_empty() {
                return None;
            }
            lsp_types::TextEdit {
                range: line_index.range(extraction.span()),
                new_text: isograph_parser::format_recorded(
                    extraction.iso_literal_text.as_str(),
                    parsed.tokens.as_slice(),
                ),
            }
            .wrap_some()
        })
        .collect::<Vec<_>>()
        .wrap_some()
```

Semantic tokens: the private borrowed `LineIndex<'a>` (`semantic_tokens.rs:168-201`) is deleted. `LineCursor` holds the text and the owned index.

Before:

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
/// `break_index` is the current line. Offsets must not go backward.
struct LineCursor<'a> {
    index: &'a LineIndex<'a>,
    break_index: usize,
}
```

After:

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
/// `break_index` is the current line. Offsets must not go backward.
struct LineCursor<'a> {
    text: &'a str,
    index: &'a LineIndex,
    break_index: usize,
}

impl<'a> LineCursor<'a> {
    fn new(text: &'a str, index: &'a LineIndex) -> Self {
        Self {
            text,
            index,
            break_index: 0,
        }
    }
}
```

`lsp_semantic_tokens` builds `let index = LineIndex::new(page_content);` and passes `LineCursor::new(page_content, index.reference())` to `lsp_semantic_tokens_with`, which takes `mut cursor: LineCursor<'_>` in place of `index: &LineIndex` and drops `let mut cursor = index.cursor();`. In `LineCursor`'s methods, `self.index.text` becomes `self.text`; `self.index.breaks` is unchanged (`breaks` is `pub(crate)`). The semantic-tokens encoding is unchanged. `LineIndex::new` also collects wide chars, which semantic tokens do not read; that is one extra pass over the text through the last literal.

`lib.rs`: `pub use semantic_tokens::{lsp_semantic_tokens, semantic_token_legend};` and `pub use line_index::LineIndex;`.

Tests (in `line_index.rs`): `line_breaks_records_crlf_as_one_break` moves from `semantic_tokens.rs`. `position` of offset 0 in `""` is `0:0`; after `"a\n"`, `"a\r\n"`, `"a\r"` the next line's first byte is `1:0`; an offset between `\r` and `\n` is on line 0; `é` (2 bytes) before the offset counts 1; `😀` (4 bytes) counts 2; a wide char on an earlier line does not change the column; `range` of a span crossing a line break. The existing formatting tests (`adapter.rs`, `state.rs`) pin that formatting ranges are unchanged, including multi-byte text and CRLF.

### Change 2: diagnostics

One shippable change. It lands as commits in this order, each compiling with tests passing; `pub` items in library crates do not trip `dead_code` between commits, and every `pub(crate)` item lands in the commit with its caller.

1. `HostLanguage::host_errors`, the TS impl, test-host impls, `iso_literal_errors_in_file`; delete `host_errors_for_extraction`, `FileLiteral`, `file_literals`; move their tests.
2. `line_index` memo.
3. `lsp_diagnostics_for_file` and its tests.
4. `IsographDb::paths_read_by` and its tests.
5. `PublishDiagnosticsSupport`, `PublishedDiagnostics`, `LiveSession.publish_diagnostics` (deleting `capabilities`), `uri_for_path`, `PathUriError`, `diagnostics.rs`, `PublishDiagnosticsEffect` and its `perform` arm, `TimerPayload::DiagnosticsDebounce` replacing `HelloWorldDebounced`, `diagnostics_debounce` replacing `hello_world_debounce`, the arming call sites, and the `handle` tests.

## Tests

No sleeps, no polling. A timer fires by the test injecting `IsographEvent::Timer(timer.event)` from the returned `TimerEffect`. A notification is observed by calling `crate::daemon::perform` on the effect (the existing `perform_send` helper) and then `rx.recv()` on the session's unbounded crossbeam receiver; after `perform` returns the sends have completed, so `rx.try_recv()` returning `Empty` is the assertion that nothing else was sent. Tests collect notifications into a `HashMap<String, Vec<lsp_types::Diagnostic>>` keyed by URI, since order within an effect is unspecified.

### `handle` tests (`crates/isograph_cli/src/state.rs`)

Helpers: `dummy_session(sender, publish_diagnostics: PublishDiagnosticsSupport)` replaces the `capabilities` parameter; `diagnostics_client(state, client) -> TestClient` starts a client with `Supported(PublishedDiagnostics::default())` and drains the `Started` timer by firing it; `only_timer(effects) -> TimerEffect<TimerPayload>` panics unless `effects` is exactly one `Timer`; `fire(state, effects) -> Vec<IsographEffect>` is `handle(state, IsographEvent::Timer(only_timer(effects).event))`; `published(client, effects) -> HashMap<String, Vec<Diagnostic>>` performs and drains. Fixture URIs are under `file:///tmp/proj` (`with_config`).

- `didOpen` of `iso(\`entrypoint\`)` returns one `Timer`; firing it returns one `PublishDiagnostics` whose one notification is `file:///tmp/proj/src/x.ts` with non-empty diagnostics, all `ERROR`, source `isograph`.
- UTF-16 range: `didOpen` of `"const é = 1;\nconst 😀 = iso\`entrypoint Query.HomeRoute\`"`; the one diagnostic (`MissingParentheses`) starts at line 1, character `"const 😀 = iso\`".encode_utf16().count()` and ends at that plus `"entrypoint Query.HomeRoute".len()`.
- CRLF: the same literal after `"const a = 1;\r\n"` starts on line 1.
- A file with no literals: fire returns no effects.
- Whitespace-only literal `iso(\` \`)`: one diagnostic whose message is `IsoLiteralError::<TypeScriptHostLanguage>::Parse(ParseError::Ast(AstError::EmptyLiteral)).to_string()`.
- Broken then fixed: second fire publishes that URI with `[]`; a third arm and fire returns no effects.
- Fixed then broken: first fire returns no effects; second publishes the errors.
- Two fires with no change: arm with `DiskChanged` of identical contents after a publish; the fire returns no effects.
- Two `didChange`s before a fire: firing the first timer returns no effects (its guard was dropped); firing the second publishes the second text's errors once.
- The same firing delivered twice: the second returns no effects.
- A firing on a never-armed state (timer taken from another state) returns no effects.
- Close with (b): broken buffer over a clean `DiskFile`, `didClose`, fire: `[]`. Broken buffer with no `DiskFile`, `didClose`, fire: `[]`. Broken buffer over a disk file with different errors: the disk errors. With (a): every close publishes `[]` for a published path.
- Two clients, one path, different buffers: client 1 broken, client 2 clean buffer: only client 1 receives a notification; client 2's `try_recv` is `Empty`.
- `Gone` before the fire: fire returns no effects for the gone client; another live client is unaffected.
- Capability absent: a client with `Unsupported` and a broken buffer receives nothing.
- `DiskChanged` `Absent` of a published disk path, and `FolderRemoved` of its directory: `[]`.
- `Started` with broken disk files while client 1 already received them: firing the `Started` timer publishes to client 2 only.
- `didOpen` of a non-file URI, `didChange` with a ranged change, and `didOpen` from a never-started client return no effects and leave `diagnostics_debounce` unchanged.
- Tagged template host error range: covered by the UTF-16 test (`MissingParentheses` spans the literal interior).

Existing tests that change:

- Assert one `Timer` (via `only_timer`) instead of `effects.is_empty()`: `disk_changed_present_interns_a_path_relative_to_the_config`, `disk_changed_present_outside_the_config_dir_is_a_parent_relative_path`, `disk_changed_absent_removes_the_relative_path`, `folder_removed_of_src_removes_files_under_src_not_src2`, `folder_removed_of_the_config_directory_removes_every_interned_file`, `folder_removed_of_a_never_interned_path_is_a_noop` (renamed `folder_removed_of_a_never_interned_path_removes_nothing`), `file_absent_of_a_directory_path_does_not_remove_files_under_it`, `did_open_stores_the_buffer_for_that_client_only`, `did_change_with_one_full_change_replaces_the_buffer`, `did_close_falls_back_to_disk`, `lsp_isograph_event_disk_changed_interns`, `client_started_stores_the_session`, `client_started_twice_for_the_same_id_keeps_the_second_session`.
- `client_started_stores_the_session` asserts `session.publish_diagnostics` instead of `session.capabilities`.
- `hello_world_returns_a_timer` becomes `hello_world_returns_log_hello_world`; `lsp_isograph_event_hello_world_returns_a_timer` becomes `lsp_isograph_event_hello_world_returns_log_hello_world`.
- Deleted (their behavior is covered by the diagnostics timer tests above): `matching_hello_world_debounced_returns_log_hello_world`, `stale_hello_world_debounced_returns_no_effects`, `a_second_hello_world_drops_the_first_guard`, `a_matching_hello_world_debounced_fed_twice_logs_once`, `lsp_isograph_event_hello_world_then_its_timer_returns_log_hello_world`.
- `timer_fired_on_a_never_armed_state_returns_no_effects` arms with `didOpen` in a second state.
- `lsp_notification_from_a_client_that_never_started_returns_no_effects` asserts `diagnostics_debounce.is_none()`; `lsp_notification_after_gone_returns_no_effects` drops its `hello_world_debounce` assertion (its `Started` armed the debounce).
- `crates/isograph_cli/src/daemon.rs`: `sending_hello_world_emits_a_timer` becomes `sending_hello_world_emits_log_hello_world`; the three timer tests use `TimerPayload::DiagnosticsDebounce`.

### Plain-function tests (`crates/isograph_cli/src/diagnostics.rs`)

- `changed_diagnostics`: empty and empty yields nothing; a new path yields it; an equal list yields nothing; a different list yields the new list; a published path absent from current yields `[]`.
- `PublishDiagnosticsSupport::from_client_capabilities`: default capabilities are `Unsupported`; `text_document` without `publish_diagnostics` is `Unsupported`; `publish_diagnostics: Some(default)` is `Supported` with an empty map.

### `uri_for_path` (`crates/isograph_cli/src/adapter.rs` tests)

- `src/a.ts` is `file:///tmp/proj/src/a.ts`, and `relative_from_uri` of it is `src/a.ts`.
- `src/a b.ts` is `file:///tmp/proj/src/a%20b.ts` and round-trips.
- `../other/a.ts` is `file:///tmp/other/a.ts` and round-trips.
- `src/./a.ts` is `file:///tmp/proj/src/a.ts`.
- No `CurrentWorkingDirectory` is `MissingWorkingDirectory`.

### `paths_read_by` (`crates/isograph_compiler/src/database.rs` tests)

Empty database is empty; disk paths only; one client's open path only; a path both on disk and open appears once; another client's open path is excluded.

### Memo tests

`iso_literal_errors_in_file` (`crates/isograph_extract_typescript/src/lib.rs` `memo_tests`):

- Missing file is `None`. A file with no literals is `Some(vec![])`.
- `iso(\`entrypoint\`)`: a `Parse` error whose span starts at or after the literal start (file-absolute).
- Prepending `"const x = 1;\n"` shifts every error span by that length.
- Two clients: client 1's broken buffer has errors, client 2 reading clean disk has none.

The `host_errors_for_extraction` tests call `TypeScriptHostLanguage::host_errors(&extraction, &parsed)`: `tagged_template_is_missing_parentheses`, `entrypoint_without_export_is_valid`, `field_without_export_is_missing_export`, `field_without_associated_function_is_missing_associated_function`, `exported_field_with_associated_function_is_valid`, `tagged_template_field_reports_parentheses_and_export_and_associated`, `host_error_span_is_the_extraction_span`, `valid_extraction_has_no_errors`, `failed_field_parse_is_not_missing_export`. The `file_literals` tests read `iso_literal_errors_in_file`: `incomplete_entrypoint_is_a_parse_error`, `missing_disk_file_has_no_file_literals` (renamed `missing_disk_file_has_no_iso_literal_errors`), `present_file_with_no_iso_has_empty_file_literals` (renamed `present_file_with_no_iso_has_no_iso_literal_errors`), `file_literals_parse_error_is_file_absolute` (renamed `iso_literal_errors_parse_error_is_file_absolute`), `tagged_template_incomplete_entrypoint_is_parentheses_only`.

Before:

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    #[test]
    fn incomplete_entrypoint_is_a_parse_error() {
        let (db, path, extraction, parsed) = one_literal("iso(`entrypoint`)");
        assert!(!parsed.errors.is_empty());
        assert!(host_errors_for_extraction(&extraction, &parsed).is_empty());
        let literals = file_literals(&db, client(), path).expect("the test interned this path");
        assert!(
            literals
                .iter()
                .flat_map(|literal| literal.errors.iter())
                .any(|error| matches!(error.item, IsoLiteralError::Parse(_)))
        );
    }
```

After:

```rust
// from crates/isograph_extract_typescript/src/lib.rs
    #[test]
    fn incomplete_entrypoint_is_a_parse_error() {
        let (db, path, extraction, parsed) = one_literal("iso(`entrypoint`)");
        assert!(!parsed.errors.is_empty());
        assert!(TypeScriptHostLanguage::host_errors(&extraction, &parsed).is_empty());
        let errors = iso_literal_errors_in_file(&db, client(), path)
            .as_ref()
            .expect("the test interned this path");
        assert!(
            errors
                .iter()
                .any(|error| matches!(error.item, IsoLiteralError::Parse(_)))
        );
    }
```

`lsp_diagnostics_for_file` (`crates/isograph_lsp/src/file_diagnostics.rs`, with `isograph_extract_typescript` as the existing dev-dependency):

- Missing file is `None`; no literals is `Some(vec![])`.
- Each message equals the `to_string()` of the corresponding `iso_literal_errors_in_file` item; severity `ERROR`; source `isograph`.
- CRLF and lone `\r` before the literal put the range on line 1.
- A surrogate pair (`😀`) before the literal on its line adds 2 to the start character.

## Docs this change amends

- `docs-website/docs/design-docs/event-model.md`: `IsographEffect` gains `PublishDiagnostics(Box<PublishDiagnosticsEffect>)`; "HelloWorld arms a timer; `handle` of that `TimerFired` is `trigger_if_matching` and `LogHelloWorld`" becomes "HelloWorld returns `LogHelloWorld`; an applied `DiskChanged`, document sync notification, or `Started` arms the diagnostics debounce, and `handle` of that `TimerFired` returns `PublishDiagnosticsEffect`s"; "Effects from `handle` (`ReportDiagnostics`)" becomes `PublishDiagnosticsEffect`; the effect loop list adds `PublishDiagnostics`; `IsographState` names `diagnostics_debounce`.
- `docs-website/docs/design-docs/pico.md`, Iso literals: add the chain `lsp_diagnostics_for_file(client, path) -> iso_literal_errors_in_file(client, path) -> parsed_iso_literals_in_file(client, path)`, plus `line_index(client, path) -> file_contents(client, path)`.
- `refactors/pending/tests-only-functionality.md`: the Diagnostics block (`host_errors_for_extraction`, `FileLiteral`, `file_literals`) and the sentence "`IsoLiteralError` is the error type on `FileLiteral.errors`" are removed; "`parsed_iso_literal` is called from those and from `file_literals`" drops `file_literals`; the `WithErrors` section's "`file_literals` builds `FileLiteral` instead" becomes "`iso_literal_errors_in_file` returns the errors as a list".
- `refactors/pending/format-file-from-segments.md`: its `lsp_range_for_byte_range` calls become `LineIndex::new(text).range(span)`.
- `refactors/pending/lsp-open-files.md` change 7 "Docs this change amends" describes the previous shape of this doc (`report_diagnostics`, `file_literals`, `PublishDiagnosticsCap`); it is history of that change and is not edited.
