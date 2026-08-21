# LSP semantic token encoding

`Vec<WithSpan<IsographSemanticToken>>` is literal-relative byte spans. LSP `textDocument/semanticTokens/full` wants `lsp_types::SemanticToken`: `delta_line`, `delta_start`, `length`, `token_type` index into a legend, `token_modifiers_bitset`. `delta_start` and `length` are UTF-16 code units. A token does not include a line break.

`LineIndex` records every line-break index in `page_content` once. A `LineCursor` walks that vec as tokens are encoded (file order). `position(offset)` advances past breaks with `after <= offset`; `break_index` is the line. The encoder delta-encodes those positions: `delta_line = line - prev_line`; `delta_start` is `col - prev_col` on the same line and `col` after a line break; `length` is UTF-16 of the piece. Spans are file-absolute, mutually exclusive, ordered, in range, on char boundaries, and not strictly inside a line break. A violation is `EncodeError`. The caller rebases a literal-relative span with `with_offset` before concatenating literals. The server (lsp-semantic-tokens.md) logs the error and answers with empty tokens.

Origin: `crates/isograph_lsp/src/semantic_tokens.rs` and `crates/isograph_lang_types/src/semantic_token_legend/mod.rs` in isograph. This crate is `crates/isograph_lsp`. It does not start the server. lsp-semantic-tokens.md adds `file_literals` and the stdio loop, and calls the functions here.

The parser type is `IsographSemanticToken`. `lsp_types::SemanticToken` is `LspSemanticToken` at the use site.

One shippable change.

## What the user does

No user-facing change until lsp-semantic-tokens.md. Tests construct a source string, parse a literal, encode, and assert the LSP integers.

## Types

Most important first.

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
use isograph_parser::IsographSemanticToken;
use prelude::Postfix;
use span::{Span, WithSpan};
use thiserror::Error;

/// File-absolute parser tokens as LSP semantic-token deltas.
///
/// Co-iterates `tokens` and the document's line breaks. Each parser span is
/// cut at line breaks; each nonempty piece is one LSP token.
pub fn lsp_semantic_tokens(
    tokens: &[WithSpan<IsographSemanticToken>],
    page_content: &str,
) -> Result<Vec<lsp_types::SemanticToken>, EncodeError> {
    let index = LineIndex::new(page_content);
    let mut cursor = index.cursor();
    // Start of the previous emitted piece. (0, 0) before the first.
    let mut last = Position { line: 0, col: 0 };
    let mut last_span_end = 0u32;
    let mut encoded: Vec<lsp_types::SemanticToken> = Vec::new();
    for token in tokens {
        index.check_span(token.location, last_span_end)?;
        last_span_end = token.location.end;
        let mut piece_start = token.location.start;
        while piece_start < token.location.end {
            let piece_position = cursor.position(piece_start);
            // Column of this piece on the line it starts on.
            let offset_on_line = piece_position.col;
            let piece_end = match cursor.break_before(token.location.end) {
                Some(line_break) => line_break.start,
                None => token.location.end,
            };
            if piece_end > piece_start {
                let length = utf16_units(
                    &page_content[(piece_start as usize)..(piece_end as usize)],
                );
                let token_type = lsp_type_index(token.item);
                // Same line as the previous piece: `delta_start` is start-to-start.
                // Later line: `delta_start` is offset_on_line (column on the line we landed on).
                let relative_to_previous = match piece_position.line - last.line {
                    0 => RelativeToPrevious::SameLine(SameLine(
                        offset_on_line - last.col,
                    )),
                    delta_line => RelativeToPrevious::MultiLine(MultiLine {
                        delta_line,
                        offset_on_line,
                    }),
                };
                let lsp_semantic_token = LspSemanticToken {
                    relative_to_previous,
                    length,
                    token_type,
                };
                encoded.push(lsp_semantic_token.to());
                // Next delta is from this start, not from this end.
                last = piece_position;
            }
            match cursor.break_before(token.location.end) {
                Some(line_break) => {
                    piece_start = line_break.after;
                    // Consume even when the piece was empty (a blank line in the span).
                    cursor.advance_break();
                }
                None => break,
            }
        }
    }
    encoded.wrap_ok()
}

struct LspSemanticToken {
    relative_to_previous: RelativeToPrevious,
    length: u32,
    token_type: u32,
}

/// Relative to the previous piece. `SameLine` is start-to-start on this line.
/// `MultiLine` is the previous piece on an earlier line; `offset_on_line` is this
/// piece's UTF-16 column on the line we landed on (`delta_start`).
enum RelativeToPrevious {
    SameLine(SameLine),
    MultiLine(MultiLine),
}

/// UTF-16 from the previous piece's start to this piece's start.
struct SameLine(u32);

struct MultiLine {
    delta_line: u32,
    /// UTF-16 from column 0 of this piece's line to this piece's start.
    offset_on_line: u32,
}

impl From<LspSemanticToken> for lsp_types::SemanticToken {
    fn from(token: LspSemanticToken) -> Self {
        let (delta_line, delta_start) = match token.relative_to_previous {
            RelativeToPrevious::SameLine(SameLine(delta_start)) => (0, delta_start),
            RelativeToPrevious::MultiLine(MultiLine {
                delta_line,
                offset_on_line,
            }) => {
                // After a line change `delta_start` is the column on the new line, not a delta from the previous start.
                (delta_line, offset_on_line)
            }
        };
        lsp_types::SemanticToken {
            delta_line,
            delta_start,
            length: token.length,
            token_type: token.token_type,
            token_modifiers_bitset: 0,
        }
    }
}

/// Why `lsp_semantic_tokens` refused a span. Ordered and exclusive is not enough:
/// a boundary strictly inside `\r\n` is still invalid.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Error)]
pub enum EncodeError {
    #[error("semantic token spans must be mutually exclusive and ordered; got {}..{} after end {}", .0.start, .0.end, .0.previous_end)]
    Overlap(Overlap),
    #[error("semantic token span {}..{} is inverted", .0.start, .0.end)]
    Inverted(SpanEnds),
    #[error("byte {} is out of range for a document of length {}", .0.offset, .0.len)]
    OutOfRange(OutOfRange),
    #[error("byte {} is not a char boundary", .0.offset)]
    NotCharBoundary(NotCharBoundary),
    #[error("byte {} is strictly inside a line break {}..{}", .0.offset, .0.start, .0.after)]
    InsideLineBreak(InsideLineBreak),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Overlap {
    pub start: u32,
    pub end: u32,
    pub previous_end: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SpanEnds {
    pub start: u32,
    pub end: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct OutOfRange {
    pub offset: u32,
    pub len: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct NotCharBoundary {
    pub offset: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct InsideLineBreak {
    pub offset: u32,
    pub start: u32,
    pub after: u32,
}

/// Every line-break index in `text`, from one scan.
struct LineIndex<'a> {
    text: &'a str,
    breaks: Vec<LineBreak>,
}

/// Two-pointer into `LineIndex.breaks`. `break_index` is the current line.
///
/// Offsets passed to [`LineCursor::position`] must not decrease.
struct LineCursor<'a> {
    text: &'a str,
    breaks: &'a [LineBreak],
    break_index: usize,
}

/// LSP position: zero-based line, UTF-16 column on that line.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct Position {
    line: u32,
    col: u32,
}

/// One line terminator. `start` is the first byte of `\n`, `\r`, or `\r\n`.
/// `after` is the first byte of the next line.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct LineBreak {
    start: u32,
    after: u32,
}

impl<'a> LineIndex<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            breaks: line_breaks(text),
        }
    }

    fn cursor(&self) -> LineCursor<'_> {
        LineCursor {
            text: self.text,
            breaks: &self.breaks,
            break_index: 0,
        }
    }

    /// Ordered, exclusive, in range, on a char boundary, and not strictly inside a line break.
    fn check_span(&self, span: Span, last_span_end: u32) -> Result<(), EncodeError> {
        if span.end < span.start {
            return EncodeError::Inverted(SpanEnds {
                start: span.start,
                end: span.end,
            })
            .wrap_err();
        }
        if span.start < last_span_end {
            return EncodeError::Overlap(Overlap {
                start: span.start,
                end: span.end,
                previous_end: last_span_end,
            })
            .wrap_err();
        }
        self.check_offset(span.start)?;
        self.check_offset(span.end)?;
        ().wrap_ok()
    }

    fn check_offset(&self, offset: u32) -> Result<(), EncodeError> {
        let len = self.text.len() as u32;
        if offset > len {
            return EncodeError::OutOfRange(OutOfRange { offset, len }).wrap_err();
        }
        if !self.text.is_char_boundary(offset as usize) {
            return EncodeError::NotCharBoundary(NotCharBoundary { offset }).wrap_err();
        }
        for line_break in &self.breaks {
            // Strict: a span may start or end at `start` or `after`, not between `\r` and `\n`.
            if line_break.start < offset && offset < line_break.after {
                return EncodeError::InsideLineBreak(InsideLineBreak {
                    offset,
                    start: line_break.start,
                    after: line_break.after,
                })
                .wrap_err();
            }
        }
        ().wrap_ok()
    }
}

impl LineCursor<'_> {
    /// Line and UTF-16 column of `offset`. Advances `break_index` past breaks that end at or before `offset`.
    fn position(&mut self, offset: u32) -> Position {
        while self.break_index < self.breaks.len()
            && self.breaks[self.break_index].after <= offset
        {
            self.break_index += 1;
        }
        let line_start = match self.break_index {
            0 => 0,
            n => self.breaks[n - 1].after,
        };
        Position {
            line: self.break_index as u32,
            col: utf16_units(&self.text[(line_start as usize)..(offset as usize)]),
        }
    }

    /// The current break, if it starts before `span_end`. Does not consume it.
    fn break_before(&self, span_end: u32) -> Option<LineBreak> {
        self.breaks
            .get(self.break_index)
            .filter(|line_break| line_break.start < span_end)
            .copied()
    }

    fn advance_break(&mut self) {
        self.break_index += 1;
    }
}

/// `\n`, `\r`, and `\r\n` (one break). Same set as `IsographLangTokenKind::LineBreak`.
fn line_breaks(text: &str) -> Vec<LineBreak> {
    let bytes = text.as_bytes();
    let mut index = 0usize;
    let mut breaks = Vec::new();
    while index < bytes.len() {
        match bytes[index] {
            b'\n' => {
                let start = index as u32;
                index += 1;
                breaks.push(LineBreak {
                    start,
                    after: index as u32,
                });
            }
            b'\r' => {
                let start = index as u32;
                index += 1;
                if matches!(bytes.get(index), Some(&b'\n')) {
                    index += 1;
                }
                breaks.push(LineBreak {
                    start,
                    after: index as u32,
                });
            }
            _ => index += 1,
        }
    }
    breaks
}

/// UTF-16 length. For ASCII that is `text.len()`.
fn utf16_units(text: &str) -> u32 {
    if text.is_ascii() {
        text.len() as u32
    } else {
        text.encode_utf16().count() as u32
    }
}
```

`LineIndex` holds every break index from one scan of `page_content`. `LineCursor` walks that vec: `break_index` is the first break not yet passed. `position(offset)` advances while `after <= offset` (each such advance is one line). `break_before` peeks the current break if it starts before `span.end`. `advance_break` consumes it and moves to the next line. Tokens are in file order, so the cursor only moves forward. `break_index` is the line number after those advances.

`last` is the previous piece's start `Position`. `offset_on_line` is this piece's UTF-16 column on the line it starts on. `SameLine` subtracts `last.col` (start-to-start). `MultiLine` keeps `offset_on_line` as-is: that is the offset on the line we landed on, and `From` writes it to `delta_start`. `length` and `token_type` live on `LspSemanticToken`. `From` fills `delta_line: 0` on `SameLine` and `token_modifiers_bitset: 0`. A blank line inside a span is `piece_end == piece_start`; nothing is emitted, then `advance_break` still runs, so the next `position` is on the following line.

`check_span` runs before any slice. Ordered exclusive spans are not enough: `page = "a\r\nb"` with tokens `[0..1, 2..3]` is ordered and exclusive, and offset 2 sits strictly inside the break `{ start: 1, after: 3 }`. Parser leftover skips `LineBreak` tokens, so this is a caller concat bug. `EncodeError` names it instead of panicking on `page_content[3..2]`.

`position` is only called on a piece start after `check_span`, so `line_start..offset` is in range and on a char boundary. Offsets passed to the cursor are non-decreasing.

`utf16_units` is `text.len()` when `is_ascii`, else `encode_utf16().count()`. Almost every iso lexeme is ASCII.

## Origin

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs (upstream)
fn absolutize_relative_token<'a>(
    page_content: &'a str,
    iso_literal_extraction_span: Span,
    relative_token: &'a WithEmbeddedLocation<IsographSemanticToken>,
) -> impl Iterator<Item = AbsoluteIsographSemanticToken> + 'a {
    let span_content = &page_content[(iso_literal_extraction_span.start as usize
        + relative_token.location.span.start as usize)
        ..(iso_literal_extraction_span.start as usize + relative_token.location.span.end as usize)];

    span_content
        .split_inclusive('\n')
        .scan(0, move |iterated_so_far_within_token, line_text| {
            let token = AbsoluteIsographSemanticToken {
                absolute_char_start: iso_literal_extraction_span.start
                    + relative_token.location.span.start
                    + *iterated_so_far_within_token,
                len: line_text.len() as u32,
                semantic_token: relative_token.item,
            };
            *iterated_so_far_within_token += line_text.len() as u32;
            token.wrap_some()
        })
}

fn convert_absolute_token_to_lsp_token<'a>(
    absolute_tokens: impl Iterator<Item = AbsoluteIsographSemanticToken> + 'a,
    page_content: &'a str,
) -> impl Iterator<Item = LspSemanticToken> + 'a {
    absolute_tokens.scan(0, |last_token_start, absolute_token| {
        let new_token_start = absolute_token.absolute_char_start;
        let in_between_content =
            &page_content[(*last_token_start as usize)..(new_token_start as usize)];
        let (delta_line, delta_start) = delta_line_delta_start(in_between_content);
        let token = LspSemanticToken {
            delta_line,
            delta_start,
            length: absolute_token.len,
            token_type: absolute_token.semantic_token.lsp_semantic_token.0,
            token_modifiers_bitset: 0,
        };
        *last_token_start = absolute_token.absolute_char_start;
        token.wrap_some()
    })
}

pub fn delta_line_delta_start(text: &str) -> (u32, u32) {
    let mut last_line_break_index = 0;
    let mut line_break_count = 0;
    for (index, char) in text.chars().enumerate() {
        if char == '\n' {
            line_break_count += 1;
            last_line_break_index = index as u32 + 1;
        }
    }
    (line_break_count, text.len() as u32 - last_line_break_index)
}
```

Checked against the LSP 3.17 encoding: five integers per token; `deltaLine` is lines since the previous token start; if `deltaLine` is 0, `deltaStart` is UTF-16 units since the previous token start, otherwise UTF-16 units from column 0; `length` is UTF-16 units on that line. A client discards a token that extends past the end of the line.

Deltas from that extract:

- `LineIndex` holds every break. `LineCursor` walks it forward (`break_index` is the line). Origin materializes `AbsoluteIsographSemanticToken` then delta-encodes from `last_token_start`. There is no `AbsoluteToken`, no `partition_point` per piece.
- `token` is `&WithSpan<IsographSemanticToken>`. Span is file-absolute. The caller applied `with_offset(extraction_span.start)` when the parse was literal-relative.
- `token_type` is `lsp_type_index(token.item)`, not a field on the parser token. `lsp_type_index` is private.
- Origin `split_inclusive('\n')` kept the line break in `line_text`, so `len` included `\n` and the client may discard the token. Origin also treated only `\n` as a break. `line_breaks` records `\r\n` then `\n` then `\r`; `length` is the text before the break.
- Empty pieces emit nothing. `advance_break` still runs, so the next `position` is on the following line.
- Origin `line_text.len()` is UTF-8 bytes. `length` and `col` are UTF-16: `utf16_units`, ASCII `len()` otherwise `encode_utf16().count()`.
- Origin `delta_line_delta_start` used `chars().enumerate()` for `\n` and `text.len()` (bytes) for the last-line width. `Position` is `(line, utf16_col)` from `LineIndex::position`.
- Unordered, inverted, out-of-range, non-char-boundary, and CRLF-interior spans are `EncodeError`. Origin sliced `page_content[last_token_start..new_start]` and panics on a backwards range. The server logs the error and returns empty tokens.

## Legend and `lsp_type_index`

Origin legend, same order, so the indices match isograph's `LspSemanticToken(n)` constants.

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
use lsp_types::{
    SemanticTokenModifier, SemanticTokenType, SemanticTokensLegend,
};

/// Sole list of legend token types. Index in this slice is `token_type`.
/// Unused slots keep the origin numbering.
const LEGEND_TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::NAMESPACE,
    SemanticTokenType::TYPE,
    SemanticTokenType::CLASS,
    SemanticTokenType::ENUM,
    SemanticTokenType::INTERFACE,
    SemanticTokenType::STRUCT,
    SemanticTokenType::TYPE_PARAMETER,
    SemanticTokenType::PARAMETER,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::PROPERTY,
    SemanticTokenType::ENUM_MEMBER,
    SemanticTokenType::EVENT,
    SemanticTokenType::FUNCTION,
    SemanticTokenType::METHOD,
    SemanticTokenType::MACRO,
    SemanticTokenType::KEYWORD,
    SemanticTokenType::MODIFIER,
    SemanticTokenType::COMMENT,
    SemanticTokenType::STRING,
    SemanticTokenType::NUMBER,
    SemanticTokenType::REGEXP,
    SemanticTokenType::OPERATOR,
    SemanticTokenType::DECORATOR,
];

const LEGEND_TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION,
    SemanticTokenModifier::DEFINITION,
    SemanticTokenModifier::READONLY,
    SemanticTokenModifier::STATIC,
    SemanticTokenModifier::DEPRECATED,
    SemanticTokenModifier::ABSTRACT,
    SemanticTokenModifier::ASYNC,
];

pub fn semantic_token_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: LEGEND_TOKEN_TYPES.to_vec(),
        token_modifiers: LEGEND_TOKEN_MODIFIERS.to_vec(),
    }
}

fn legend_index(ty: SemanticTokenType) -> u32 {
    LEGEND_TOKEN_TYPES
        .iter()
        .position(|listed| listed == &ty)
        .expect("LEGEND_TOKEN_TYPES lists every type lsp_type_index maps") as u32
}

fn lsp_type_index(token: IsographSemanticToken) -> u32 {
    match token {
        IsographSemanticToken::Keyword => legend_index(SemanticTokenType::KEYWORD),
        IsographSemanticToken::Type => legend_index(SemanticTokenType::CLASS),
        IsographSemanticToken::FieldName | IsographSemanticToken::ObjectKey => {
            legend_index(SemanticTokenType::PROPERTY)
        }
        IsographSemanticToken::GraphQLTypeName => legend_index(SemanticTokenType::TYPE),
        IsographSemanticToken::DirectiveName => legend_index(SemanticTokenType::DECORATOR),
        IsographSemanticToken::Variable | IsographSemanticToken::BooleanOrNull => {
            legend_index(SemanticTokenType::VARIABLE)
        }
        IsographSemanticToken::Argument => legend_index(SemanticTokenType::PARAMETER),
        IsographSemanticToken::Integer => legend_index(SemanticTokenType::NUMBER),
        IsographSemanticToken::String => legend_index(SemanticTokenType::STRING),
        IsographSemanticToken::Period
        | IsographSemanticToken::Colon
        | IsographSemanticToken::Equals
        | IsographSemanticToken::Parenthesis
        | IsographSemanticToken::Brace
        | IsographSemanticToken::Content
        | IsographSemanticToken::Bracket => legend_index(SemanticTokenType::OPERATOR),
        IsographSemanticToken::Error => legend_index(SemanticTokenType::COMMENT),
    }
}
```

Origin maps each `ST_*` constant's `lsp_semantic_token` field. i2 has one enum. `FieldName` is PROPERTY (origin uses METHOD for `Type.name` and PROPERTY for selections). `Error` is COMMENT (`ST_COMMENT`). `Content` is OPERATOR.

## Crate

```toml
# from crates/isograph_lsp/Cargo.toml
[package]
name = "isograph_lsp"
version = { workspace = true }
edition = { workspace = true }
license = { workspace = true }

[dependencies]
isograph_parser = { path = "../isograph_parser" }
lsp-types = { workspace = true }
prelude = { path = "../prelude" }
span = { path = "../span" }
thiserror = { workspace = true }

[lints]
workspace = true
```

Workspace member via `./crates/*`.

```rust
// from crates/isograph_lsp/src/lib.rs
mod semantic_tokens;

pub use semantic_tokens::{
    EncodeError, lsp_semantic_tokens, semantic_token_legend,
};
```

`LspSemanticToken`, `RelativeToPrevious`, `SameLine`, `MultiLine`, `LineIndex`, `LineCursor`, `Position`, `LineBreak`, `line_breaks`, `utf16_units`, `lsp_type_index`, `legend_index` stay in the module. Tests in that module call them. `legend_index` `expect`s that `LEGEND_TOKEN_TYPES` lists every type the match names.

## Tests

Test helpers live in the test module. `encoded` parses then encodes a source that is the whole file. `rebased` applies `with_offset`. `of_type` picks LSP tokens by legend index.

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
#[cfg(test)]
mod tests {
    use isograph_parser::{IsographSemanticToken, parse_iso_literal};
    use lsp_types::{SemanticToken as LspSemanticToken, SemanticTokenType};
    use prelude::Postfix;
    use span::{Span, WithSpan, WithSpanPostfix};

    use super::{
        EncodeError, InsideLineBreak, LineBreak, LineIndex, NotCharBoundary, OutOfRange,
        Overlap, Position, SpanEnds, line_breaks, lsp_semantic_tokens, lsp_type_index,
        semantic_token_legend,
    };

    const STRING: u32 = 18;
    const KEYWORD: u32 = 15;
    const CLASS: u32 = 2;
    const PROPERTY: u32 = 9;
    const OPERATOR: u32 = 21;

    fn encoded(source: &str) -> Vec<LspSemanticToken> {
        let parsed = parse_iso_literal(source);
        encode(&parsed.tokens, source)
    }

    fn rebased(
        tokens: &[WithSpan<IsographSemanticToken>],
        offset: u32,
    ) -> Vec<WithSpan<IsographSemanticToken>> {
        tokens
            .iter()
            .map(|token| token.item.with_span(token.location.with_offset(offset)))
            .collect()
    }

    fn encode_rebased(
        tokens: &[WithSpan<IsographSemanticToken>],
        offset: u32,
        page_content: &str,
    ) -> Vec<LspSemanticToken> {
        encode(&rebased(tokens, offset), page_content)
    }

    fn of_type(lsp: &[LspSemanticToken], token_type: u32) -> Vec<&LspSemanticToken> {
        lsp.iter()
            .filter(|token| token.token_type == token_type)
            .collect()
    }

    fn encode(
        tokens: &[WithSpan<IsographSemanticToken>],
        page_content: &str,
    ) -> Vec<LspSemanticToken> {
        lsp_semantic_tokens(tokens, page_content).expect(
            "the fixture's spans are mutually exclusive, ordered, in range, on char boundaries, and not inside a line break",
        )
    }

    #[test]
    fn line_breaks_records_crlf_as_one_break() {
        assert_eq!(
            line_breaks("a\r\nb\nc\rd"),
            vec![
                LineBreak {
                    start: 1,
                    after: 3,
                },
                LineBreak {
                    start: 4,
                    after: 5,
                },
                LineBreak {
                    start: 6,
                    after: 7,
                },
            ],
        );
    }

    #[test]
    fn position_same_line() {
        let index = LineIndex::new("   abc");
        assert_eq!(index.cursor().position(3), Position { line: 0, col: 3 });
    }

    #[test]
    fn position_after_newline() {
        let index = LineIndex::new("\n  x");
        assert_eq!(index.cursor().position(3), Position { line: 1, col: 2 });
    }

    #[test]
    fn position_after_crlf_and_cr() {
        let index = LineIndex::new("\r\n  ");
        assert_eq!(index.cursor().position(4), Position { line: 1, col: 2 });
        let index = LineIndex::new("\r  ");
        assert_eq!(index.cursor().position(3), Position { line: 1, col: 2 });
        let index = LineIndex::new("a\r\nb");
        assert_eq!(index.cursor().position(3), Position { line: 1, col: 0 });
        let index = LineIndex::new("x\n\ny");
        assert_eq!(index.cursor().position(4), Position { line: 2, col: 1 });
        let index = LineIndex::new("é\r\n  ");
        assert_eq!(index.cursor().position(6), Position { line: 1, col: 2 });
        let index = LineIndex::new("é\r  ");
        assert_eq!(index.cursor().position(5), Position { line: 1, col: 2 });
    }

    #[test]
    fn position_counts_utf16_on_the_line() {
        let index = LineIndex::new("é\n  ");
        assert_eq!(index.cursor().position(5), Position { line: 1, col: 2 });
        let index = LineIndex::new("aé");
        assert_eq!(index.cursor().position(3), Position { line: 0, col: 2 });
        let index = LineIndex::new("a😀");
        assert_eq!(index.cursor().position(5), Position { line: 0, col: 3 });
    }

    #[test]
    fn entrypoint_encodes_as_keyword_class_operator_property() {
        let lsp = encoded("entrypoint Query.foo");
        assert_eq!(lsp.len(), 4);
        assert_eq!(lsp[0].delta_line, 0);
        assert_eq!(lsp[0].delta_start, 0);
        assert_eq!(lsp[0].length, 11);
        assert_eq!(lsp[0].token_type, KEYWORD);
        assert_eq!(lsp[0].token_modifiers_bitset, 0);
        assert_eq!(lsp[1].delta_line, 0);
        assert_eq!(lsp[1].delta_start, 12);
        assert_eq!(lsp[1].length, 5);
        assert_eq!(lsp[1].token_type, CLASS);
        assert_eq!(lsp[2].delta_line, 0);
        assert_eq!(lsp[2].delta_start, 5);
        assert_eq!(lsp[2].length, 1);
        assert_eq!(lsp[2].token_type, OPERATOR);
        assert_eq!(lsp[3].delta_line, 0);
        assert_eq!(lsp[3].delta_start, 1);
        assert_eq!(lsp[3].length, 3);
        assert_eq!(lsp[3].token_type, PROPERTY);
    }

    #[test]
    fn extraction_offset_shifts_the_first_delta_start() {
        let prefix = "const x = iso(`";
        let literal = "field Pet.fullName { id }";
        let source = format!("{prefix}{literal}`)");
        let parsed = parse_iso_literal(literal);
        let lsp = encode_rebased(&parsed.tokens, prefix.len() as u32, source.reference());
        assert_eq!(lsp[0].delta_line, 0);
        assert_eq!(lsp[0].delta_start, prefix.len() as u32);
        assert_eq!(lsp[0].token_type, KEYWORD);
        assert_eq!(lsp[1].token_type, CLASS);
        assert_eq!(&source[prefix.len()..prefix.len() + 5], "field");
    }

    #[test]
    fn extraction_offset_after_a_newline_sets_delta_line() {
        let prefix = "const x = iso(`\n  ";
        let literal = "field Pet.fullName { id }";
        let source = format!("{prefix}{literal}`)");
        let parsed = parse_iso_literal(literal);
        let lsp = encode_rebased(&parsed.tokens, prefix.len() as u32, source.reference());
        assert_eq!(lsp[0].delta_line, 1);
        assert_eq!(lsp[0].delta_start, 2);
        assert_eq!(lsp[0].token_type, KEYWORD);
        assert_eq!(lsp[0].length, 5);
    }

    #[test]
    fn extraction_offset_counts_utf16_in_the_prefix() {
        let prefix = "const é = iso(`";
        let literal = "field Pet.fullName { id }";
        let source = format!("{prefix}{literal}`)");
        let parsed = parse_iso_literal(literal);
        let lsp = encode_rebased(&parsed.tokens, prefix.len() as u32, source.reference());
        assert_eq!(prefix.len(), 16);
        assert_eq!(lsp[0].delta_line, 0);
        assert_eq!(lsp[0].delta_start, 15);
        assert_eq!(lsp[0].token_type, KEYWORD);
    }

    #[test]
    fn two_literals_are_file_absolute_and_in_order() {
        let source = "iso(`entrypoint Query.A`)\niso(`entrypoint Query.B`)";
        let first = "entrypoint Query.A";
        let second = "entrypoint Query.B";
        let first_start = source.find(first).expect("the first literal is in the file");
        let second_start = source.find(second).expect("the second literal is in the file");
        let parsed_a = parse_iso_literal(first);
        let parsed_b = parse_iso_literal(second);
        let mut tokens = rebased(&parsed_a.tokens, first_start as u32);
        tokens.extend(rebased(&parsed_b.tokens, second_start as u32));
        let lsp = encode(&tokens, source);
        assert_eq!(lsp.len(), 8);
        assert_eq!(lsp[4].delta_line, 1);
        assert_eq!(lsp[4].delta_start, 5);
        assert_eq!(lsp[4].length, 11);
        assert_eq!(lsp[4].token_type, KEYWORD);
        assert_eq!(lsp[7].delta_line, 0);
        assert_eq!(lsp[7].delta_start, 1);
        assert_eq!(lsp[7].token_type, PROPERTY);
    }

    #[test]
    fn two_literals_on_the_same_line() {
        let source = "iso(`entrypoint Query.A`) iso(`entrypoint Query.B`)";
        let first = "entrypoint Query.A";
        let second = "entrypoint Query.B";
        let first_start = source.find(first).expect("the first literal is in the file");
        let second_start = source.find(second).expect("the second literal is in the file");
        let parsed_a = parse_iso_literal(first);
        let parsed_b = parse_iso_literal(second);
        let mut tokens = rebased(&parsed_a.tokens, first_start as u32);
        tokens.extend(rebased(&parsed_b.tokens, second_start as u32));
        let lsp = encode(&tokens, source);
        assert_eq!(lsp.len(), 8);
        assert_eq!(lsp[4].delta_line, 0);
        assert_eq!(lsp[4].delta_start, 9);
        assert_eq!(lsp[4].token_type, KEYWORD);
    }

    #[test]
    fn tokens_on_successive_lines_set_delta_line() {
        let lsp = encoded("field Query.Foo {\n  bar\n}");
        assert_eq!(lsp.len(), 7);
        assert_eq!(lsp[4].token_type, OPERATOR);
        assert_eq!(lsp[4].length, 1);
        assert_eq!(lsp[5].delta_line, 1);
        assert_eq!(lsp[5].delta_start, 2);
        assert_eq!(lsp[5].length, 3);
        assert_eq!(lsp[5].token_type, PROPERTY);
        assert_eq!(lsp[6].delta_line, 1);
        assert_eq!(lsp[6].delta_start, 0);
        assert_eq!(lsp[6].length, 1);
        assert_eq!(lsp[6].token_type, OPERATOR);
    }

    #[test]
    fn a_blank_line_between_parser_tokens_increments_delta_line() {
        let lsp = encoded("field Query.Foo {\n\n  bar\n}");
        assert_eq!(lsp.len(), 7);
        assert_eq!(lsp[5].delta_line, 2);
        assert_eq!(lsp[5].delta_start, 2);
        assert_eq!(lsp[5].token_type, PROPERTY);
    }

    #[test]
    fn a_quoted_string_is_one_lsp_token() {
        let source = "field Query.Foo \"the home route\" { bar }";
        let parsed = parse_iso_literal(source);
        assert_eq!(
            parsed
                .tokens
                .iter()
                .filter(|token| token.item == IsographSemanticToken::String)
                .count(),
            1,
        );
        let lsp = encoded(source);
        let strings = of_type(&lsp, STRING);
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].delta_line, 0);
        assert_eq!(strings[0].delta_start, 4);
        assert_eq!(strings[0].length, 16);
        assert_eq!(lsp[5].token_type, OPERATOR);
        assert_eq!(lsp[5].delta_line, 0);
        assert_eq!(lsp[5].delta_start, 17);
    }

    #[test]
    fn a_quoted_string_with_an_escaped_newline_is_one_lsp_token() {
        let source = "field Query.Foo \"hi\\n\" { bar }";
        let parsed = parse_iso_literal(source);
        assert_eq!(
            parsed
                .tokens
                .iter()
                .filter(|token| token.item == IsographSemanticToken::String)
                .count(),
            1,
        );
        let strings = of_type(&encoded(source), STRING);
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].length, 6);
        assert_eq!(strings[0].delta_line, 0);
    }

    #[test]
    fn an_empty_quoted_string_is_one_lsp_token() {
        let strings = of_type(&encoded("field Query.Foo \"\" { bar }"), STRING);
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].length, 2);
    }

    #[test]
    fn a_one_line_block_string_is_one_lsp_token() {
        let source = "field Query.Foo \"\"\"hi\"\"\" { bar }";
        let parsed = parse_iso_literal(source);
        assert_eq!(
            parsed
                .tokens
                .iter()
                .filter(|token| token.item == IsographSemanticToken::String)
                .count(),
            1,
        );
        let strings = of_type(&encoded(source), STRING);
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].length, 8);
    }

    #[test]
    fn an_empty_block_string_is_one_lsp_token() {
        let strings = of_type(&encoded("field Query.Foo \"\"\"\"\"\" { bar }"), STRING);
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].length, 6);
    }

    #[test]
    fn a_multiline_string_becomes_one_lsp_token_per_line_without_the_newline() {
        let source = "field Query.Foo \"\"\"\n  the home\n  route\n\"\"\" { bar }";
        let parsed = parse_iso_literal(source);
        assert_eq!(
            parsed
                .tokens
                .iter()
                .filter(|token| token.item == IsographSemanticToken::String)
                .count(),
            1,
        );
        let lsp = encoded(source);
        let strings = of_type(&lsp, STRING);
        assert_eq!(strings.len(), 4);
        assert_eq!(strings[0].length, 3);
        assert_eq!(strings[1].delta_line, 1);
        assert_eq!(strings[1].delta_start, 0);
        assert_eq!(strings[1].length, 10);
        assert_eq!(strings[2].delta_line, 1);
        assert_eq!(strings[2].delta_start, 0);
        assert_eq!(strings[2].length, 7);
        assert_eq!(strings[3].delta_line, 1);
        assert_eq!(strings[3].delta_start, 0);
        assert_eq!(strings[3].length, 3);
        assert_eq!(lsp[8].delta_line, 0);
        assert_eq!(lsp[8].delta_start, 4);
        assert_eq!(lsp[8].length, 1);
        assert_eq!(lsp[8].token_type, OPERATOR);
    }

    #[test]
    fn a_block_string_with_content_on_the_opening_line() {
        let source = "field Query.Foo \"\"\"the home\n  route\n\"\"\" { bar }";
        let strings = of_type(&encoded(source), STRING);
        assert_eq!(strings.len(), 3);
        assert_eq!(strings[0].length, 11);
        assert_eq!(strings[1].delta_line, 1);
        assert_eq!(strings[1].delta_start, 0);
        assert_eq!(strings[1].length, 7);
        assert_eq!(strings[2].delta_line, 1);
        assert_eq!(strings[2].delta_start, 0);
        assert_eq!(strings[2].length, 3);
    }

    #[test]
    fn a_block_string_with_closing_quotes_on_the_content_line() {
        let source = "field Query.Foo \"\"\"\n  route\"\"\" { bar }";
        let lsp = encoded(source);
        let strings = of_type(&lsp, STRING);
        assert_eq!(strings.len(), 2);
        assert_eq!(strings[0].length, 3);
        assert_eq!(strings[1].delta_line, 1);
        assert_eq!(strings[1].delta_start, 0);
        assert_eq!(strings[1].length, 10);
        assert_eq!(lsp[6].delta_line, 0);
        assert_eq!(lsp[6].delta_start, 11);
        assert_eq!(lsp[6].token_type, OPERATOR);
    }

    #[test]
    fn a_blank_line_in_a_block_string_does_not_emit_a_token() {
        let strings = of_type(
            &encoded("field Query.Foo \"\"\"\n\n  x\n\"\"\" { bar }"),
            STRING,
        );
        assert_eq!(strings.len(), 3);
        assert_eq!(strings[0].length, 3);
        assert_eq!(strings[1].delta_line, 2);
        assert_eq!(strings[1].delta_start, 0);
        assert_eq!(strings[1].length, 3);
        assert_eq!(strings[2].delta_line, 1);
        assert_eq!(strings[2].length, 3);
    }

    #[test]
    fn a_crlf_block_string_splits_without_including_the_break() {
        let strings = of_type(
            &encoded("field Query.Foo \"\"\"\r\n  the home\r\n  route\r\n\"\"\" { bar }"),
            STRING,
        );
        assert_eq!(strings.len(), 4);
        assert_eq!(strings[0].length, 3);
        assert_eq!(strings[1].delta_line, 1);
        assert_eq!(strings[1].delta_start, 0);
        assert_eq!(strings[1].length, 10);
        assert_eq!(strings[2].delta_line, 1);
        assert_eq!(strings[2].length, 7);
        assert_eq!(strings[3].delta_line, 1);
        assert_eq!(strings[3].length, 3);
    }

    #[test]
    fn a_cr_block_string_splits_without_including_the_break() {
        let strings = of_type(
            &encoded("field Query.Foo \"\"\"\r  x\r\"\"\" { bar }"),
            STRING,
        );
        assert_eq!(strings.len(), 3);
        assert_eq!(strings[0].length, 3);
        assert_eq!(strings[1].delta_line, 1);
        assert_eq!(strings[1].delta_start, 0);
        assert_eq!(strings[1].length, 3);
        assert_eq!(strings[2].delta_line, 1);
        assert_eq!(strings[2].length, 3);
    }

    #[test]
    fn a_non_ascii_continuation_line_of_a_block_string_is_utf16_length() {
        let strings = of_type(
            &encoded("field Query.Foo \"\"\"\n  café\n\"\"\" { bar }"),
            STRING,
        );
        assert_eq!(strings.len(), 3);
        assert_eq!(strings[1].delta_line, 1);
        assert_eq!(strings[1].delta_start, 0);
        assert_eq!(strings[1].length, 6);
    }

    #[test]
    fn a_multiline_leftover_token_splits_the_same_way() {
        let source = "\"\"\"\n  x";
        let tokens = IsographSemanticToken::Content
            .with_span(Span::from_usize(0, source.len()))
            .wrap_vec();
        let lsp = encode(&tokens, source);
        assert_eq!(lsp.len(), 2);
        assert_eq!(lsp[0].token_type, OPERATOR);
        assert_eq!(lsp[0].length, 3);
        assert_eq!(lsp[1].delta_line, 1);
        assert_eq!(lsp[1].delta_start, 0);
        assert_eq!(lsp[1].length, 3);
        assert_eq!(lsp[1].token_type, OPERATOR);
    }

    #[test]
    fn utf16_length_of_a_non_ascii_token() {
        let source = "aéb";
        let tokens = IsographSemanticToken::String
            .with_span(Span::from_usize(1, 3))
            .wrap_vec();
        let lsp = encode(&tokens, source);
        assert_eq!(lsp.len(), 1);
        assert_eq!(lsp[0].delta_start, 1);
        assert_eq!(lsp[0].length, 1);
    }

    #[test]
    fn utf16_length_of_a_surrogate_pair() {
        let source = "a😀b";
        let tokens = IsographSemanticToken::String
            .with_span(Span::from_usize(1, 5))
            .wrap_vec();
        let lsp = encode(&tokens, source);
        assert_eq!(lsp.len(), 1);
        assert_eq!(lsp[0].delta_start, 1);
        assert_eq!(lsp[0].length, 2);
    }

    #[test]
    fn adjacent_spans_encode() {
        let source = "ab";
        let tokens = vec![
            IsographSemanticToken::Keyword.with_span(Span::from_usize(0, 1)),
            IsographSemanticToken::Type.with_span(Span::from_usize(1, 2)),
        ];
        let lsp = encode(&tokens, source);
        assert_eq!(lsp.len(), 2);
        assert_eq!(lsp[0].delta_start, 0);
        assert_eq!(lsp[0].length, 1);
        assert_eq!(lsp[1].delta_line, 0);
        assert_eq!(lsp[1].delta_start, 1);
        assert_eq!(lsp[1].length, 1);
    }

    #[test]
    fn an_empty_span_emits_nothing() {
        let source = "a";
        let tokens = vec![
            IsographSemanticToken::Keyword.with_span(Span::from_usize(0, 0)),
            IsographSemanticToken::Type.with_span(Span::from_usize(0, 1)),
        ];
        let lsp = encode(&tokens, source);
        assert_eq!(lsp.len(), 1);
        assert_eq!(lsp[0].delta_start, 0);
        assert_eq!(lsp[0].length, 1);
        assert_eq!(lsp[0].token_type, CLASS);
    }

    #[test]
    fn overlapping_spans_are_an_error() {
        let source = "abcd";
        let tokens = vec![
            IsographSemanticToken::Keyword.with_span(Span::from_usize(0, 2)),
            IsographSemanticToken::Type.with_span(Span::from_usize(1, 3)),
        ];
        assert_eq!(
            lsp_semantic_tokens(&tokens, source),
            EncodeError::Overlap(Overlap {
                start: 1,
                end: 3,
                previous_end: 2,
            })
            .wrap_err(),
        );
    }

    #[test]
    fn out_of_order_spans_are_an_error() {
        let source = "abcd";
        let tokens = vec![
            IsographSemanticToken::Keyword.with_span(Span::from_usize(2, 4)),
            IsographSemanticToken::Type.with_span(Span::from_usize(0, 1)),
        ];
        assert_eq!(
            lsp_semantic_tokens(&tokens, source),
            EncodeError::Overlap(Overlap {
                start: 0,
                end: 1,
                previous_end: 4,
            })
            .wrap_err(),
        );
    }

    #[test]
    fn inverted_span_is_an_error() {
        let source = "abcd";
        let tokens = IsographSemanticToken::Keyword
            .with_span(Span { start: 3, end: 1 })
            .wrap_vec();
        assert_eq!(
            lsp_semantic_tokens(&tokens, source),
            EncodeError::Inverted(SpanEnds { start: 3, end: 1 }).wrap_err(),
        );
    }

    #[test]
    fn out_of_range_span_is_an_error() {
        let source = "ab";
        let tokens = IsographSemanticToken::Keyword
            .with_span(Span::from_usize(0, 5))
            .wrap_vec();
        assert_eq!(
            lsp_semantic_tokens(&tokens, source),
            EncodeError::OutOfRange(OutOfRange { offset: 5, len: 2 }).wrap_err(),
        );
    }

    #[test]
    fn a_span_inside_a_multibyte_char_is_an_error() {
        let source = "aéb";
        let tokens = IsographSemanticToken::Keyword
            .with_span(Span::from_usize(2, 3))
            .wrap_vec();
        assert_eq!(
            lsp_semantic_tokens(&tokens, source),
            EncodeError::NotCharBoundary(NotCharBoundary { offset: 2 }).wrap_err(),
        );
    }

    #[test]
    fn a_span_strictly_inside_crlf_is_an_error() {
        let source = "a\r\nb";
        let tokens = vec![
            IsographSemanticToken::Keyword.with_span(Span::from_usize(0, 1)),
            IsographSemanticToken::Type.with_span(Span::from_usize(2, 3)),
        ];
        assert_eq!(
            lsp_semantic_tokens(&tokens, source),
            EncodeError::InsideLineBreak(InsideLineBreak {
                offset: 2,
                start: 1,
                after: 3,
            })
            .wrap_err(),
        );
    }

    #[test]
    fn reversed_literals_are_an_error() {
        let source = "iso(`entrypoint Query.A`)\niso(`entrypoint Query.B`)";
        let first = "entrypoint Query.A";
        let second = "entrypoint Query.B";
        let first_start = source.find(first).expect("the first literal is in the file");
        let second_start = source.find(second).expect("the second literal is in the file");
        let parsed_a = parse_iso_literal(first);
        let parsed_b = parse_iso_literal(second);
        let mut tokens = rebased(&parsed_b.tokens, second_start as u32);
        tokens.extend(rebased(&parsed_a.tokens, first_start as u32));
        assert!(matches!(
            lsp_semantic_tokens(&tokens, source),
            Err(EncodeError::Overlap(_)),
        ));
    }

    #[test]
    fn lsp_type_index_matches_the_legend() {
        let types = semantic_token_legend().token_types;
        assert_eq!(types.len(), 23);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::Keyword) as usize], SemanticTokenType::KEYWORD);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::Type) as usize], SemanticTokenType::CLASS);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::FieldName) as usize], SemanticTokenType::PROPERTY);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::ObjectKey) as usize], SemanticTokenType::PROPERTY);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::GraphQLTypeName) as usize], SemanticTokenType::TYPE);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::DirectiveName) as usize], SemanticTokenType::DECORATOR);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::Variable) as usize], SemanticTokenType::VARIABLE);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::Argument) as usize], SemanticTokenType::PARAMETER);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::Integer) as usize], SemanticTokenType::NUMBER);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::String) as usize], SemanticTokenType::STRING);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::BooleanOrNull) as usize], SemanticTokenType::VARIABLE);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::Period) as usize], SemanticTokenType::OPERATOR);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::Colon) as usize], SemanticTokenType::OPERATOR);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::Equals) as usize], SemanticTokenType::OPERATOR);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::Parenthesis) as usize], SemanticTokenType::OPERATOR);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::Brace) as usize], SemanticTokenType::OPERATOR);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::Content) as usize], SemanticTokenType::OPERATOR);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::Bracket) as usize], SemanticTokenType::OPERATOR);
        assert_eq!(types[lsp_type_index(IsographSemanticToken::Error) as usize], SemanticTokenType::COMMENT);
    }
}
```

`extraction_offset_after_a_newline_sets_delta_line`: the template body starts after `` ` ``, then a newline and two spaces. `field` is `delta_line` 1, column 2.

`extraction_offset_counts_utf16_in_the_prefix`: `const é = iso(\`` is 16 UTF-8 bytes and 15 UTF-16 units. `delta_start` is 15.

`two_literals_are_file_absolute_and_in_order`: eight tokens. Index 4 is the second `entrypoint`. Previous piece is `A` on the previous line; `delta_line` 1, `delta_start` 5 (column of `entrypoint` after `iso(\``). Index 7 is `B` on the same line as that literal's `.`.

`two_literals_on_the_same_line`: same line as `A`; `delta_start` 9 is the column of the second `entrypoint`.

`a_quoted_string_is_one_lsp_token`: `"the home route"` is 16 UTF-16 units. `{` is one space after that lexeme, `delta_start` 17.

`a_quoted_string_with_an_escaped_newline_is_one_lsp_token`: source `"hi\n"` is quote, `h`, `i`, backslash, `n`, quote. Length 6. Not split.

`a_block_string_with_content_on_the_opening_line`: pieces `"""the home` (11), `  route` (7), `"""`.

`a_block_string_with_closing_quotes_on_the_content_line`: pieces `"""`, `  route"""` (10). `{` is one space after that piece, `delta_start` 11.

`a_non_ascii_continuation_line_of_a_block_string_is_utf16_length`: `  café` is 7 UTF-8 bytes, 6 UTF-16 units.

`a_multiline_leftover_token_splits_the_same_way`: unterminated `"""` is leftover `Content`. The encoder still splits. Pieces `"""`, `  x`.

`utf16_length_of_a_surrogate_pair`: `😀` is bytes 1..5 of `a😀b`, two UTF-16 units.

`an_empty_span_emits_nothing`: `0..0` records no piece; the next token at `0..1` still encodes.

`position_counts_utf16_on_the_line`: `a😀` is 3 UTF-16 units (1 + 2).

`a_span_strictly_inside_crlf_is_an_error`: tokens `[0..1, 2..3]` on `"a\r\nb"`; offset 2 is inside `{ start: 1, after: 3 }`.

`lsp_type_index_matches_the_legend`: `legend.token_types[lsp_type_index(token)]` is the `SemanticTokenType` that variant maps to. Inserting a type at the front of `LEGEND_TOKEN_TYPES` fails this test.

`expect` in tests names an invariant the fixture established.
