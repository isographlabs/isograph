# LSP hover

The server answers `textDocument/hover`. A hover inside an iso literal returns the name of the `IsographResolutionNode` variant that the character under the cursor resolves to, as Markdown. A hover anywhere else returns `null`.

## End-user experience

The editor advertises hover because `initialize` returns `"hoverProvider": true`. The user rests the mouse on `Query` in

```ts
export const Home = iso(`entrypoint Query.HomeRoute`);
```

and the editor shows `EntityNameWrapper`. On `HomeRoute` it shows `SelectableNameWrapper`. On `entrypoint`, on `.`, and on the space between `entrypoint` and `Query` it shows `EntrypointDeclaration`. On `id` in `` iso(`field Query.x{id}`) `` it shows `SelectionNameWrapper`. On `fieldd` in `` iso(`fieldd Query.foo`) `` (a parse error) it shows `IsoLiteralSlot`.

The editor shows nothing (the result is `null`) on `export`, on `iso(`, on either backtick, past the end of a line, on a line past the end of the file, inside an empty or whitespace-only literal, for a non-file URI, and for a path with no buffer and no disk file.

Hover reads the requesting client's open buffer, and disk when that client has no buffer for the path. Columns are UTF-16 code units, and `\n`, `\r\n`, and `\r` are line breaks, the same model semantic tokens and formatting use.

## The chain

Each function asks the layer below for one thing (pico.md, Structuring functions and memos).

```text
hover_response(db, client, params)                          isograph_cli adapter.rs
  -> hover_for_position(db, client, path, position)          isograph_lsp hover.rs
    -> resolved_position_at_lsp_position(...)                isograph_lsp hover.rs
      -> ast_at_lsp_position(...)                            isograph_lsp hover.rs
        -> iso_literal_at_lsp_position(...)                  isograph_lsp hover.rs
          -> byte_offset_at_lsp_position(...)                isograph_lsp line_index.rs
            -> line_index(db, client, path)                  memo, isograph_lsp line_index.rs
              -> file_contents(db, client, path)
          -> extract_iso_literals(db, client, path)
        -> parsed_iso_literal_by_id(db, literal_id)          isograph_compiler iso_literals.rs
          -> iso_literal_extraction(db, literal_id)
          -> parsed_iso_literal(db, iso_literal_text)
      -> isograph_parser::resolve_offset(parsed, cursor)     isograph_parser parse_iso_literal.rs
```

`line_index` is the only new memo. Every function that takes a position is a plain function, so moving the cursor creates no derived node per position.

### `hover_for_position`

```rust
// from crates/isograph_lsp/src/hover.rs
/// The hover at `position` in `path` as `client` reads it.
pub fn hover_for_position<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<lsp_types::Hover> {
    let node = resolved_position_at_lsp_position(db, client, path, position)?;
    lsp_types::Hover {
        contents: lsp_types::HoverContents::Markup(lsp_types::MarkupContent {
            kind: lsp_types::MarkupKind::Markdown,
            value: node.to_string(),
        }),
        range: None,
    }
    .wrap_some()
}
```

Caller: `hover_response`. `lib.rs` adds `mod hover;` and `pub use hover::hover_for_position;`.

### `resolved_position_at_lsp_position`

```rust
// from crates/isograph_lsp/src/hover.rs
fn resolved_position_at_lsp_position<'db, THostLanguage: HostLanguage>(
    db: &'db IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<IsographResolutionNode<'db>> {
    let (parsed, cursor) = ast_at_lsp_position(db, client, path, position)?;
    resolve_offset(parsed, cursor)
}
```

A plain function: `IsographResolutionNode<'db>` borrows the parse stored in `parsed_iso_literal`, and a memo cannot return a borrow (pico.md, Memos return owned values).

### `ast_at_lsp_position`

```rust
// from crates/isograph_lsp/src/hover.rs
fn ast_at_lsp_position<'db, THostLanguage: HostLanguage>(
    db: &'db IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<(&'db ParsedIsoLiteral, Span)> {
    let (literal_id, cursor) = iso_literal_at_lsp_position(db, client, path, position)?;
    (parsed_iso_literal_by_id(db, literal_id)?, cursor).wrap_some()
}
```

### `iso_literal_at_lsp_position`

```rust
// from crates/isograph_lsp/src/hover.rs
/// The literal whose interior contains `position`, and the span of the char under
/// `position` relative to that interior.
fn iso_literal_at_lsp_position<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<(LiteralId, Span)> {
    let offset = byte_offset_at_lsp_position(db, client, path, position)?;
    let extractions = extract_iso_literals(db, client, path).as_ref()?;
    extractions
        .iter()
        .enumerate()
        .find_map(|(index, extraction)| {
            let within = offset.checked_sub(extraction.iso_literal_start_index.0 as u32)?;
            let character = extraction
                .iso_literal_text
                .get(within as usize..)?
                .chars()
                .next()?;
            (
                LiteralId {
                    client,
                    path,
                    index,
                },
                Span::new(within, within + character.len_utf8() as u32),
            )
                .wrap_some()
        })
}
```

The interior is start inclusive, end exclusive, so the opening and closing backticks are outside every literal. `get(within..)` is `None` past the interior; `chars().next()` is `None` at its end. The cursor span is the char under the cursor (`offset..offset + len_utf8`), so it is never zero-width and never crosses the interior end.

The second element is a `Span` rather than a `u32` offset. `ParsedIsoLiteral` does not hold the literal text, so the char length is known only here, where the extraction is in hand.

### `byte_offset_at_lsp_position`

```rust
// from crates/isograph_lsp/src/line_index.rs
pub(crate) fn byte_offset_at_lsp_position<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<u32> {
    line_index(db, client, path).as_ref()?.byte_offset(position)
}
```

### `line_index` and the owned `LineIndex`

This is Change 1 of `refactors/pending/lsp-diagnostics.md` (owned `LineIndex`, semantic tokens and formatting moved to it, `lsp_range_for_byte_range` deleted) and its `line_index` memo, plus `len` and `byte_offset`. The hand-rolled index (rather than rust-analyzer's `line-index` crate) is because that crate breaks lines on `\n` only; iso tokenizes `\n`, `\r\n`, and `\r` as line breaks.

```rust
// from crates/isograph_lsp/src/line_index.rs
/// Byte offsets of the line breaks and the non-ASCII chars of one text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineIndex {
    pub(crate) breaks: Vec<LineBreak>,
    wide_chars: Vec<WideChar>,
    len: u32,
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
            len: text.len() as u32,
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

    /// The byte offset of `position`. `None` when the line is past the last line, the
    /// column is past the line's end, or the column falls between the two code units
    /// of a surrogate pair.
    pub fn byte_offset(&self, position: lsp_types::Position) -> Option<u32> {
        let line = position.line as usize;
        let line_start = match line {
            0 => 0,
            n => self.breaks.get(n - 1)?.after,
        };
        let line_end = self
            .breaks
            .get(line)
            .map_or(self.len, |line_break| line_break.start);
        let first = self
            .wide_chars
            .partition_point(|wide| wide.start < line_start);
        let last = self
            .wide_chars
            .partition_point(|wide| wide.start < line_end);
        let walked = self.wide_chars[first..last].iter().try_fold(
            (line_start, position.character),
            |(offset, remaining), wide| {
                let narrow = wide.start - offset;
                if remaining <= narrow {
                    return ControlFlow::Break((offset + remaining).wrap_some());
                }
                match (remaining - narrow).checked_sub(u32::from(wide.utf16_len)) {
                    Some(rest) => {
                        ControlFlow::Continue((wide.start + u32::from(wide.utf8_len), rest))
                    }
                    None => ControlFlow::Break(None),
                }
            },
        );
        match walked {
            ControlFlow::Break(offset) => offset,
            ControlFlow::Continue((offset, remaining)) => {
                (remaining <= line_end - offset).then(|| offset + remaining)
            }
        }
    }
}

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

`line_breaks` moves from `semantic_tokens.rs` to `line_index.rs` verbatim, with its doc comment.

`byte_offset` does not slice the text and does not panic. `wide.start - offset` cannot underflow because wide chars are sorted and `offset` is the line start or the end of the previous wide char. `line_end - offset` cannot underflow because no wide char is a line break. A column equal to the line length is the byte of the line break (or the end of the text).

Semantic tokens and formatting build a `LineIndex` per call from `text_through_last_iso_literal`, not from the `line_index` memo. The memo reads the whole file, so typing after the last literal would re-invoke them, where today `text_through_last_iso_literal` backdates and they do not.

Semantic tokens: the private borrowed `LineIndex<'a>` is deleted. `LineCursor` holds the text and the owned index.

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

`lsp_semantic_tokens` builds `let index = LineIndex::new(page_content);` and passes `LineCursor::new(page_content, index.reference())` to `lsp_semantic_tokens_with`, which takes `mut cursor: LineCursor<'_>` in place of `index: &LineIndex` and drops `let mut cursor = index.cursor();`. In `LineCursor`'s methods and in `lsp_semantic_token`, `index.text` becomes `text`. The encoding is unchanged.

Formatting, before:

```rust
// from crates/isograph_cli/src/adapter.rs
            let start = extraction.iso_literal_start_index.0 as u32;
            let end = start + extraction.iso_literal_text.len() as u32;
            lsp_types::TextEdit {
                range: isograph_lsp::lsp_range_for_byte_range(page_content, start, end),
```

After (`let line_index = isograph_lsp::LineIndex::new(page_content);` before the iterator):

```rust
// from crates/isograph_cli/src/adapter.rs
            lsp_types::TextEdit {
                range: line_index.range(extraction.span()),
```

`lib.rs`: `mod line_index;`, `pub use line_index::{LineIndex, line_index};`, and `pub use semantic_tokens::{lsp_semantic_tokens, semantic_token_legend};`.

### `parsed_iso_literal_by_id`

```rust
// from crates/isograph_compiler/src/iso_literals.rs
pub fn parsed_iso_literal_by_id<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    literal_id: LiteralId,
) -> Option<&ParsedIsoLiteral> {
    let extraction = iso_literal_extraction(db, literal_id).as_ref()?;
    parsed_iso_literal(db, extraction.iso_literal_text.clone()).wrap_some()
}
```

The parse stays keyed by the literal text. `lib.rs` exports it.

### `resolve_offset` and `strum::Display`

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
/// The resolution leaf of `span`, a range within the literal text. `None` for an empty
/// or whitespace-only literal.
pub fn resolve_offset(parsed: &ParsedIsoLiteral, span: Span) -> Option<IsographResolutionNode<'_>> {
    parsed.item.as_ref().map(|item| item.resolve((), span))
}
```

`item`'s span is the whole literal text (the chunked root's span), so every span within the text satisfies `resolve`'s precondition.

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
#[derive(Debug, derive_more::From, strum::Display)]
#[non_exhaustive]
pub enum IsographResolutionNode<'a> {
```

The default `strum::Display` writes the variant name and ignores the payload.

### `hover_response`

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn hover_response<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    params: lsp_types::HoverParams,
) -> LSPRuntimeResult<<lsp_types::request::HoverRequest as lsp_types::request::Request>::Result> {
    let position = params.text_document_position_params;
    let path = relative_from_uri(db, position.text_document.uri.reference()).map_err(lsp_error)?;
    isograph_lsp::hover_for_position(db, client, path, position.position).wrap_ok()
}
```

`HoverRequest::Result` is `Option<Hover>`. A non-file URI is `ExpectedError`, which the dispatcher sends as a `null` result, as for the other handlers.

Registration in `dispatch_lsp_request`, after `WillSaveWaitUntil`:

```rust
// from crates/isograph_cli/src/state.rs
            .on_request_sync::<lsp_types::request::HoverRequest>(
                crate::adapter::hover_response::<THostLanguage>,
            )?
```

Capability in `run_session` (`lsp_socket.rs`), added to `ServerCapabilities`:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
        hover_provider: lsp_types::HoverProviderCapability::Simple(true).wrap_some(),
```

## Deletions

From `crates/isograph_compiler/src/iso_literals.rs`: `LineChar`, `literal_id_at_location`, `find_iso_literal_index`, `position_in_range`, `line_and_byte`. `lib.rs` stops exporting `LineChar` and `literal_id_at_location`. From `crates/isograph_lsp/src/semantic_tokens.rs`: the borrowed `LineIndex<'a>`, its `cursor`, `lsp_range_for_byte_range`, `LineBreak`, and `line_breaks` (the last two move to `line_index.rs`).

The `isograph_extract_typescript` memo tests that call `literal_id_at_location` move to `hover.rs` tests of `iso_literal_at_lsp_position` and `ast_at_lsp_position`. `one_literal` builds `LiteralId { client, path, index: 0 }` directly, since its fixtures have one literal. `missing_disk_file_has_no_file_literals` drops its `literal_id_at_location` assertion.

`pico.md` sentences that name `LineChar` as a second line model and `literal_id_at_location` as the current per-position memo are rewritten to the chain above.

## Tests

`line_index.rs`: `line_breaks_records_crlf_as_one_break` (moved). `byte_offset`: ASCII; after `é` (2 bytes, 1 unit); after `😀` (4 bytes, 2 units); the column between the two units of `😀` is `None`; a column equal to the line length is the line end; one past is `None`; a line past the last is `None`; the line after `\r\n` and after a lone `\r`; `0:0` of `""` is `0` and `1:0` of `""` is `None`. `position`: `0:0` of `""`; the first byte after `\n`, `\r\n`, `\r`; between `\r` and `\n` is line 0; `é` and `😀` before the offset; a wide char on an earlier line does not change the column; `range` across a line break.

`isograph_parser` (`parse_iso_literal.rs`): `resolve_offset` of `entrypoint Query.HomeRoute` on `Query` is `EntityNameWrapper`, on `HomeRoute` is `SelectableNameWrapper`, on `.` is `EntrypointDeclaration`; of `field Query.x{id}` on `id` is `SelectionNameWrapper`; of `""` and `"  "` is `None`; the `Display` of each is the variant name.

`hover.rs`: `iso_literal_at_lsp_position` on the interior's first and last byte is `LiteralId { index: 0 }` with the one-char span; on `iso(`, on each backtick, and past the interior is `None`; two literals on one line are index 0 and 1 and the JS between them is `None`; a multiline literal on its interior line; a newline prepended moves the position to line 1 with the same `LiteralId`; lengthening an earlier line keeps the `LiteralId` at the same position; a buffer over disk names the buffer's literal for its client and disk's for another client; no file is `None`. `ast_at_lsp_position` of the same text in two files is the same parse.

`state.rs` (the `handle` seam, through `request_result` and `rx.recv`), disk `src/Home.ts` = `` export const Home = iso(`entrypoint Query.HomeRoute`) ``:

- `Q` is `EntityNameWrapper`; `H` of `HomeRoute` is `SelectableNameWrapper`; `e` of `entrypoint` is `EntrypointDeclaration`; `.` is `EntrypointDeclaration`; the first interior byte is non-null.
- `x` of `export`, the opening backtick, and the closing backtick are `null`.
- Buffer over disk: didOpen `` iso(`field Query.x{id}`) `` from client 1; `id` is `SelectionNameWrapper` for client 1; client 2 at the same position reads disk; after didClose client 1 reads disk.
- didChange then hover reads the changed buffer.
- Two literals on two lines and on one line: each resolves in its own literal.
- An empty literal and a literal of two spaces are `null`.
- UTF-16: `` const s = "😀"; iso(`entrypoint Query.A`) `` and the same with `é`: the UTF-16 column of `Q` is `EntityNameWrapper`.
- CRLF and lone `\r` files: `Q` on line 1 is `EntityNameWrapper`.
- `` iso(`fieldd Query.foo`) ``: `fieldd` is `IsoLiteralSlot`.
- A column past the line end and a line past the end of the file are `null`.
- A non-file URI is `null`.
- A hover from a client that never started returns no effects.
- The result's `contents.kind` is `markdown`.

`lsp_request_returns_method_not_found` uses `textDocument/definition`, which stays unregistered. `lsp_socket.rs`'s initialize capabilities test asserts `hoverProvider` is `true`.

Every `handle` test that reads a client's buffer sends that client's didOpen before its first read of the path, so `file_contents` has a dependency on the open-file map (pico.md, Keyed reads depend on their key).

## Changes

1. Owned `LineIndex`, `line_index` memo, `byte_offset`; semantic tokens and formatting on the owned type; `lsp_range_for_byte_range` deleted.
2. `strum::Display` on `IsographResolutionNode`; `resolve_offset`.
3. `parsed_iso_literal_by_id`, `byte_offset_at_lsp_position`, `hover.rs`; deletions of `LineChar`, `literal_id_at_location`, `find_iso_literal_index`, `position_in_range`, `line_and_byte`; tests moved; `pico.md` updated.
4. `hover_response`, registration, capability, `handle` tests.
