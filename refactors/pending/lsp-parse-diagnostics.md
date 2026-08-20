# LSP parse diagnostics

Requires lsp-semantic-tokens.md. After a `didOpen` or `didChange`, the language server publishes parse errors for the iso literals in that file as `textDocument/publishDiagnostics`. Closing the file publishes an empty list for that URI, which clears the squiggles.

The pipeline is `file_literals` from lsp-semantic-tokens.md. Each `FileLiteral.errors` is `Vec<WithSpan<IsoLiteralError<THostLanguage>>>` from `Slot.extra`, already file-absolute. This doc turns those into `lsp_types::Diagnostic`.

Origin: isograph `crates/isograph_lsp/src/diagnostic_notification.rs` and the debounce-then-`validate_entire_schema` publish in `server.rs`. Delta: parse errors and host-language errors of the open file only, published on `didOpen` / `didChange` (no debounce, no schema, no file watcher). `didClose` clears. Messages are `Display` of the error types.

## What the user does

Open a file containing `iso(\`entrypoint\`)`. The `Type.name` is missing. A red squiggle appears on the literal with "Expected an identifier, found nothing more." (or whatever `ParseError` displays for that span). Fix the literal; the squiggle goes away. Close the file; diagnostics for that URI are cleared.

## Change 1: `Display` on `BracketError` and `CommaWithoutItem`

Owned by parse-iso-literal-entry.md Change 1. Same impls. Implement once.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum BracketError {
    UnmatchedOpen(WithSpan<OpenBracket>),
    UnmatchedClose(WithSpan<CloseBracket>),
}

impl fmt::Display for BracketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BracketError::UnmatchedOpen(open) => {
                write!(f, "Unclosed {}", open.item.0)
            }
            BracketError::UnmatchedClose(close) => {
                write!(f, "Unexpected {}", close.item.0)
            }
        }
    }
}

impl std::error::Error for BracketError {}
```

Before: `BracketError` has no `Display`. `OpenBracket` / `CloseBracket` are `pub struct OpenBracket(pub BracketKind)` / `pub struct CloseBracket(pub BracketKind)`. `BracketKind` already displays as `'('`, `'{'`, `'['`.

```rust
// from crates/isograph_parser/src/chunk.rs
use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CommaWithoutItem(pub Span);

impl fmt::Display for CommaWithoutItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "A comma with no item before it.")
    }
}

impl std::error::Error for CommaWithoutItem {}
```

Before: `CommaWithoutItem` has no `Display`.

`ParseError` already implements `Display` via `thiserror`.

Tests in those modules:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
    #[test]
    fn unmatched_open_displays_the_kind() {
        let err = BracketError::UnmatchedOpen(
            OpenBracket(BracketKind::Brace).with_span(Span::new(0, 1)),
        );
        assert_eq!(err.to_string(), "Unclosed '{'");
    }

    #[test]
    fn unmatched_close_displays_the_kind() {
        let err = BracketError::UnmatchedClose(
            CloseBracket(BracketKind::Parenthesis).with_span(Span::new(0, 1)),
        );
        assert_eq!(err.to_string(), "Unexpected '('");
    }
```

```rust
// from crates/isograph_parser/src/chunk.rs
    #[test]
    fn comma_without_item_displays() {
        assert_eq!(
            CommaWithoutItem(Span::new(0, 1)).to_string(),
            "A comma with no item before it."
        );
    }
```

## Change 2: file diagnostics

```rust
// from crates/isograph_lsp/src/diagnostics.rs
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use prelude::Postfix;
use span::Span;

use isograph_extract_typescript::TypeScriptHostLanguage;
use isograph_parser::HostLanguage;

use crate::file_literals::{FileLiteral, file_literals};

pub fn diagnostics_for_file(source: &str) -> Vec<Diagnostic> {
    file_literals(&TypeScriptHostLanguage, source)
        .iter()
        .flat_map(|literal| diagnostics_for_literal(source, literal))
        .collect()
}

fn diagnostics_for_literal<THostLanguage: HostLanguage>(
    source: &str,
    literal: &FileLiteral<'_, THostLanguage>,
) -> Vec<Diagnostic> {
    literal
        .errors
        .iter()
        .map(|error| diagnostic(source, error.location, error.item.to_string()))
        .collect()
}

fn diagnostic(source: &str, span: Span, message: String) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: char_index_to_position(source, span.start as usize),
            end: char_index_to_position(source, span.end as usize),
        },
        severity: DiagnosticSeverity::ERROR.wrap_some(),
        message,
        source: "isograph".to_owned().wrap_some(),
        ..Default::default()
    }
}

pub fn char_index_to_position(content: &str, char_index: usize) -> Position {
    let text_before = &content[..char_index];
    let mut line = 0;
    let mut last_line_start = 0;
    for (index, ch) in text_before.char_indices() {
        if ch == '\n' {
            line += 1;
            last_line_start = index + 1;
        }
    }
    let character = char_index - last_line_start;
    Position {
        line: line as u32,
        character: character as u32,
    }
}
```

Origin of `char_index_to_position`: isograph `crates/isograph_lsp/src/format.rs`, verbatim. Origin of the publish shape: isograph `diagnostic_notification.rs` (`range`, `message`; we also set `severity` and `source`). Delta: no code-action `data`, no `isograph_location_to_lsp_location`, no pico. Spans on `IsoLiteralError` are already file-absolute.

`lib.rs` gains `mod diagnostics;`.

Tests:

```rust
// from crates/isograph_lsp/src/diagnostics.rs
#[cfg(test)]
mod tests {
    use lsp_types::{DiagnosticSeverity, Position};
    use prelude::Postfix;

    use super::{char_index_to_position, diagnostics_for_file};

    #[test]
    fn empty_file_has_no_diagnostics() {
        assert_eq!(diagnostics_for_file(""), vec![]);
    }

    #[test]
    fn valid_literal_has_no_diagnostics() {
        assert_eq!(
            diagnostics_for_file("iso(`entrypoint Query.HomeRoute`)"),
            vec![]
        );
    }

    #[test]
    fn tagged_template_is_a_host_error() {
        let diagnostics = diagnostics_for_file("iso`entrypoint Query.HomeRoute`");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("parentheses"));
    }

    #[test]
    fn incomplete_entrypoint_is_an_error_inside_the_literal() {
        let prefix = "export const Foo = iso(`";
        let source = "export const Foo = iso(`entrypoint`)(";
        let diagnostics = diagnostics_for_file(source);
        assert!(!diagnostics.is_empty());
        let first = &diagnostics[0];
        assert_eq!(first.source.as_deref(), "isograph".wrap_some());
        assert_eq!(first.severity, DiagnosticSeverity::ERROR.wrap_some());
        let start_offset = prefix.len();
        let start = char_index_to_position(source, start_offset);
        assert!(first.range.start.line >= start.line);
        assert!(first.range.end.character > 0 || first.range.end.line > start.line);
        assert!(first.message.contains("Expected"));
    }

    #[test]
    fn two_literals_errors_stay_on_the_failing_one() {
        let source = "\
iso(`entrypoint Query.HomeRoute`)
iso(`entrypoint`)";
        let diagnostics = diagnostics_for_file(source);
        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().all(|d| d.range.start.line >= 1));
    }

    #[test]
    fn char_index_to_position_counts_newlines() {
        let source = "ab\ncd";
        assert_eq!(
            char_index_to_position(source, 0),
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            char_index_to_position(source, 3),
            Position {
                line: 1,
                character: 0
            }
        );
    }
}
```

Need `use lsp_types::{DiagnosticSeverity, Position};` and `use prelude::Postfix;` in the test module.

`incomplete_entrypoint` asserts the range sits at or after the literal start, and the message is an `Expected` parse error. It does not snapshot the full diagnostic.

`two_literals_errors_stay_on_the_failing_one`: the second literal is on line 1 (0-based). All published ranges start on that line.

## Change 3: publish on open/change, clear on close

`LspState` gains the sender, so handlers can publish.

```rust
// from crates/isograph_lsp/src/lsp_state.rs
use std::collections::HashMap;

use lsp_server::Connection;

pub struct LspState<'a> {
    pub open_files: HashMap<String, String>,
    pub sender: &'a lsp_server::Sender,
}
```

Before (lsp-semantic-tokens.md): `LspState { open_files }` with no lifetime. After: borrows `connection.sender`.

`run` constructs `LspState { open_files: HashMap::new(), sender: &connection.sender }`.

```rust
// from crates/isograph_lsp/src/diagnostics.rs
use lsp_types::{
    Uri,
    notification::{Notification, PublishDiagnostics},
    PublishDiagnosticsParams,
};

pub fn publish_diagnostics_for_uri(state: &LspState<'_>, uri: &Uri) {
    let diagnostics = match state.open_files.get(uri.as_str()) {
        Some(source) => diagnostics_for_file(source),
        None => Vec::new(),
    };
    let _ = state.sender.send(
        lsp_server::Notification::new(
            PublishDiagnostics::METHOD.to(),
            PublishDiagnosticsParams {
                uri: uri.clone(),
                diagnostics,
                version: None,
            },
        )
        .to(),
    );
}
```

Origin: isograph `publish_new_diagnostics_and_clear_old_diagnostics`. Delta: one URI, parse errors of that file, empty list when the URI is not open. Send failure ends the publish; the loop in `run` already dies on a dead connection.

`text_document.rs` handlers call it:

```rust
// from crates/isograph_lsp/src/text_document.rs
pub fn on_did_open_text_document(
    state: &mut LspState<'_>,
    params: <DidOpenTextDocument as Notification>::Params,
) -> LSPRuntimeResult<()> {
    let DidOpenTextDocumentParams { text_document } = params;
    let TextDocumentItem { text, uri, .. } = text_document;
    state.open_files.insert(uri.as_str().to_owned(), text);
    crate::diagnostics::publish_diagnostics_for_uri(state, &uri);
    ().wrap_ok()
}

pub fn on_did_close_text_document(
    state: &mut LspState<'_>,
    params: <DidCloseTextDocument as Notification>::Params,
) -> LSPRuntimeResult<()> {
    let uri = params.text_document.uri;
    state.open_files.remove(uri.as_str());
    crate::diagnostics::publish_diagnostics_for_uri(state, &uri);
    ().wrap_ok()
}

pub fn on_did_change_text_document(
    state: &mut LspState<'_>,
    params: <DidChangeTextDocument as Notification>::Params,
) -> LSPRuntimeResult<()> {
    let DidChangeTextDocumentParams {
        content_changes,
        text_document,
    } = params;
    let Some(content_changed) = content_changes.first() else {
        return ().wrap_ok();
    };
    state.open_files.insert(
        text_document.uri.as_str().to_owned(),
        content_changed.text.to_owned(),
    );
    crate::diagnostics::publish_diagnostics_for_uri(state, &text_document.uri);
    ().wrap_ok()
}
```

`on_did_close` removes first, then publish sees `None` and sends an empty list.

The existing open-file tests construct `LspState` without a sender. They become:

The tests that only check the map do not go through the handler, or they use a local channel:

```rust
    fn state() -> (lsp_server::Connection, LspState<'static>) {
        // cannot borrow from a local Connection for 'static
    }
```

Keep the map assertions by splitting: handlers that publish need a `Connection`. `lsp_server::Connection::memory()` returns `(server, client)`.

```rust
// from crates/isograph_lsp/src/text_document.rs
    #[test]
    fn did_open_publishes_diagnostics() {
        let (server, client) = lsp_server::Connection::memory();
        let mut state = LspState {
            open_files: Default::default(),
            sender: &server.sender,
        };
        on_did_open_text_document(
            &mut state,
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri(),
                    language_id: "typescript".to_owned(),
                    version: 1,
                    text: "iso(`entrypoint`)".to_owned(),
                },
            },
        )
        .expect("didOpen succeeds");
        let msg = client.receiver.recv().expect("a publishDiagnostics was sent");
        let lsp_server::Message::Notification(notif) = msg else {
            assert!(
                false,
                "didOpen publishes a notification, not a request or response"
            );
            return;
        };
        assert_eq!(notif.method, PublishDiagnostics::METHOD);
        let params: PublishDiagnosticsParams =
            serde_json::from_value(notif.params).expect("params are PublishDiagnosticsParams");
        assert!(!params.diagnostics.is_empty());
    }
```

`panic!` in that `else`: write `assert!(matches!(msg, lsp_server::Message::Notification(_)));` then unwrap via if-let.

```rust
        let lsp_server::Message::Notification(notif) = msg else {
            assert!(
                false,
                "didOpen publishes a notification, not a request or response"
            );
            return;
        };
```

`Connection::memory` origin: `lsp-server` crate. isograph's diagnostic tests do not exist; this is the test for publish.

The Change 3 tests from lsp-semantic-tokens.md that build `LspState { open_files: Default::default() }` gain `sender: &server.sender` with a memory connection in each test, including `did_open_stores_the_text`, `did_change_replaces_the_text`, `did_close_removes_the_text`, and `request_uses_the_open_file_text`. Those tests ignore the client receiver.

`start` / `run` in `server.rs` pass the sender:

```rust
fn run(connection: Connection) {
    let mut state = LspState {
        open_files: HashMap::new(),
        sender: &connection.sender,
    };
    for msg in &connection.receiver {
        // same loop as lsp-semantic-tokens.md
    }
}
```

`LspState` now has a lifetime. `dispatch_request(request, &state)` and `dispatch_notification(notification, &mut state)` stay.

## Order

1. Change 1; `Display` on `BracketError` and `CommaWithoutItem`.
2. Change 2; `diagnostics_for_file`, `char_index_to_position`, unit tests.
3. Change 3; publish on open/change, clear on close, `LspState` sender, handler tests with `Connection::memory()`.
