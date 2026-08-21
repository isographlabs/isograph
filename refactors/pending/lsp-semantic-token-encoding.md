# LSP semantic token encoding

`Vec<WithSpan<SemanticToken>>` is literal-relative byte spans. LSP `textDocument/semanticTokens/full` wants `lsp_types::SemanticToken`: `delta_line`, `delta_start`, `length`, `token_type` index into a legend, `token_modifiers_bitset`. `delta_start` and `length` are UTF-16 code units. A token does not include a line break.

One scan of `page_content` records every line break. The encoder then walks a file-absolute `tokens` slice: split each span on those breaks, encode each nonempty piece. A piece contains no line break. The next delta therefore uses only the gap after the previous piece's end, and same-line `delta_start` is the previous piece's UTF-16 length plus the gap's last-line width. Spans are mutually exclusive and ordered. A violation is a compiler bug; the walk panics. The caller rebases a literal-relative span with `with_offset` before concatenating literals.

Origin: `crates/isograph_lsp/src/semantic_tokens.rs` and `crates/isograph_lang_types/src/semantic_token_legend/mod.rs` in isograph. This crate is `crates/isograph_lsp`. It does not start the server. lsp-semantic-tokens.md adds `file_literals` and the stdio loop, and calls the functions here.

The parser type stays `SemanticToken`. `lsp_types::SemanticToken` is `LspSemanticToken` at the use site.

One shippable change.

## What the user does

No user-facing change until lsp-semantic-tokens.md. Tests construct a source string, parse a literal, encode, and assert the LSP integers.

## Types

Most important first.

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
use isograph_parser::SemanticToken;
use lsp_types::SemanticToken as LspSemanticToken;
use prelude::Postfix;
use span::WithSpan;

pub fn lsp_semantic_tokens(
    tokens: &[WithSpan<SemanticToken>],
    page_content: &str,
) -> Vec<LspSemanticToken> {
    let breaks = line_breaks(page_content);
    let mut last_end = 0u32;
    let mut last_len = 0u32;
    let mut last_span_end = 0u32;
    let mut break_index = 0usize;
    let mut encoded = Vec::new();
    for token in tokens {
        let span = token.location;
        assert!(
            span.start >= last_span_end && span.end >= span.start,
            "semantic token spans must be mutually exclusive and ordered; got {}..{} after end {last_span_end}",
            span.start,
            span.end,
        );
        last_span_end = span.end;
        let mut piece_start = span.start;
        while piece_start < span.end {
            let split_index = first_break_at_or_after(&breaks, break_index, piece_start);
            let split = if split_index < breaks.len() && breaks[split_index].start < span.end
            {
                breaks[split_index].wrap_some()
            } else {
                None
            };
            let line_end = match split {
                Some(line_break) => line_break.start,
                None => span.end,
            };
            let line_text = &page_content[(piece_start as usize)..(line_end as usize)];
            if !line_text.is_empty() {
                let gap = gap_delta(
                    &breaks[break_index..],
                    page_content,
                    last_end,
                    piece_start,
                );
                let delta_start = match gap.delta_line {
                    0 => last_len + gap.tail_utf16,
                    _ => gap.tail_utf16,
                };
                let length = utf16_units(line_text);
                encoded.push(LspSemanticToken {
                    delta_line: gap.delta_line,
                    delta_start,
                    length,
                    token_type: lsp_type_index(token.item),
                    token_modifiers_bitset: 0,
                });
                last_end = piece_start + line_text.len() as u32;
                last_len = length;
                break_index = first_break_at_or_after(&breaks, break_index, last_end);
            }
            match split {
                Some(line_break) => piece_start = line_break.after,
                None => break,
            }
        }
    }
    encoded
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct LineBreak {
    start: u32,
    after: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct GapDelta {
    delta_line: u32,
    tail_utf16: u32,
}

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

fn first_break_at_or_after(breaks: &[LineBreak], mut index: usize, at: u32) -> usize {
    while index < breaks.len() && breaks[index].start < at {
        index += 1;
    }
    index
}

fn gap_delta(
    breaks: &[LineBreak],
    page_content: &str,
    from: u32,
    to: u32,
) -> GapDelta {
    let mut last_after = from;
    let mut delta_line = 0u32;
    for line_break in breaks {
        if line_break.start >= to {
            break;
        }
        if line_break.start >= from {
            delta_line += 1;
            last_after = line_break.after;
        }
    }
    GapDelta {
        delta_line,
        tail_utf16: utf16_units(&page_content[(last_after as usize)..(to as usize)]),
    }
}

fn utf16_units(text: &str) -> u32 {
    if text.is_ascii() {
        text.len() as u32
    } else {
        text.encode_utf16().count() as u32
    }
}
```

`assert!` is a panic. The input is `&[WithSpan<SemanticToken>]` with file-absolute spans; disjoint ordered spans are a parser-plus-caller invariant that type cannot hold. Adjacent spans (`end` of one equals `start` of the next) pass. `last_span_end` is the previous parser token's absolute end.

`LineBreak.start` is the first byte of `\r\n`, `\n`, or `\r`. `after` is the first byte of the next line. Same set as `IsographLangTokenKind::LineBreak` and as VS Code / LSP line offsets. `line_breaks` walks `page_content` once. The token walk keeps `break_index` as the first break with `start >= last_end`.

`last_end` is the previous emitted piece's byte end. `last_len` is that piece's UTF-16 length. `GapDelta` is the gap `page_content[last_end..piece_start]`: `delta_line` is how many recorded breaks fall in that range; `tail_utf16` is UTF-16 units after the last such break, or of the whole gap if there is none. A piece contains no line break, so `delta_line` cannot change inside it. Same-line `delta_start` is `last_len + tail_utf16`. After a line break, `delta_start` is `tail_utf16`. Adjacent tokens have an empty gap; `delta_start` is `last_len`.

A piece's `length` is UTF-16 units of the text before the break. `utf16_units` is `text.len()` when `is_ascii`, else `encode_utf16().count()`. Almost every iso lexeme and gap is ASCII. The first token's gap is the prefix before the first literal: a file with no newline before that literal would otherwise `encode_utf16` the whole prefix.

A piece that is only a break (a blank line inside a multiline token) emits nothing; `piece_start` becomes `after`. `last_end` stays at the previous emit, so those newlines sit in the next nonempty piece's gap.

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

- One walk. Origin materializes `AbsoluteIsographSemanticToken` then delta-encodes. There is no `AbsoluteToken`.
- `token` is `&WithSpan<SemanticToken>`. Span is file-absolute. The caller applied `with_offset(extraction_span.start)` when the parse was literal-relative.
- `token_type` is `lsp_type_index(relative_token.item)`, not a field on the parser token.
- Origin `split_inclusive('\n')` kept the line break in `line_text`, so `len` included `\n` and the client may discard the token. Origin also treated only `\n` as a break, so `\r\n` left `\r` in `len` and a bare `\r` did not split. `line_breaks` records `\r\n` then `\n` then `\r`; `length` is the text before the break.
- Empty pieces emit nothing. `last_span_end` still advances by the whole parser token. `last_end` / `last_len` do not; the skipped break is in the next gap.
- Origin `line_text.len()` is UTF-8 bytes. `length` and `delta_start` are UTF-16: `utf16_units`, ASCII `len()` otherwise `encode_utf16().count()`.
- Origin `delta_line_delta_start` used `chars().enumerate()` for the newline index and `text.len()` (bytes) for the last-line width. Those units disagree on any non-ASCII last line. It also counted only `\n`. Split and delta read the same `LineBreak` list; last-line width is `utf16_units`.
- Origin `last_token_start` was the previous piece's start. `in_between` was previous piece plus gap, so each piece was scanned again on the next emit to recover `length`. `last_end` / `last_len` walk only the gap.
- Unordered or overlapping parser tokens panic. Origin sliced `page_content[last_token_start..new_start]`, which panics only when a later start is numerically before the previous start, and accepts overlap when starts still increase.

## Legend and `lsp_type_index`

Origin legend, same order, so the indices match isograph's `LspSemanticToken(n)` constants.

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
use lsp_types::{
    SemanticTokenModifier, SemanticTokenType, SemanticTokensLegend,
};

pub fn semantic_token_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
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
        ],
        token_modifiers: vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::DEFINITION,
            SemanticTokenModifier::READONLY,
            SemanticTokenModifier::STATIC,
            SemanticTokenModifier::DEPRECATED,
            SemanticTokenModifier::ABSTRACT,
            SemanticTokenModifier::ASYNC,
        ],
    }
}

const LSP_ST_TYPE: u32 = 1;
const LSP_ST_CLASS: u32 = 2;
const LSP_ST_PARAMETER: u32 = 7;
const LSP_ST_VARIABLE: u32 = 8;
const LSP_ST_PROPERTY: u32 = 9;
const LSP_ST_KEYWORD: u32 = 15;
const LSP_ST_COMMENT: u32 = 17;
const LSP_ST_STRING: u32 = 18;
const LSP_ST_NUMBER: u32 = 19;
const LSP_ST_OPERATOR: u32 = 21;
const LSP_ST_DECORATOR: u32 = 22;

pub fn lsp_type_index(token: SemanticToken) -> u32 {
    match token {
        SemanticToken::Keyword => LSP_ST_KEYWORD,
        SemanticToken::Type => LSP_ST_CLASS,
        SemanticToken::FieldName => LSP_ST_PROPERTY,
        SemanticToken::ObjectKey => LSP_ST_PROPERTY,
        SemanticToken::GraphQLTypeName => LSP_ST_TYPE,
        SemanticToken::DirectiveName => LSP_ST_DECORATOR,
        SemanticToken::Variable => LSP_ST_VARIABLE,
        SemanticToken::Argument => LSP_ST_PARAMETER,
        SemanticToken::Integer => LSP_ST_NUMBER,
        SemanticToken::String => LSP_ST_STRING,
        SemanticToken::BooleanOrNull => LSP_ST_VARIABLE,
        SemanticToken::Period
        | SemanticToken::Colon
        | SemanticToken::Equals
        | SemanticToken::Parenthesis
        | SemanticToken::Brace
        | SemanticToken::Content
        | SemanticToken::Bracket => LSP_ST_OPERATOR,
        SemanticToken::Error => LSP_ST_COMMENT,
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

[lints]
workspace = true
```

Workspace member via `./crates/*`.

```rust
// from crates/isograph_lsp/src/lib.rs
mod semantic_tokens;

pub use semantic_tokens::{lsp_semantic_tokens, semantic_token_legend};
```

`LineBreak`, `GapDelta`, `line_breaks`, `first_break_at_or_after`, `gap_delta`, `utf16_units` stay in the module. Tests in that module call them.

## Tests

Test helpers live in the test module. `encoded` parses then encodes a source that is the whole file. `rebased` applies `with_offset`. `of_type` picks LSP tokens by legend index.

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
#[cfg(test)]
mod tests {
    use isograph_parser::{SemanticToken, parse_iso_literal};
    use lsp_types::SemanticToken as LspSemanticToken;
    use prelude::Postfix;
    use span::{Span, WithSpan, WithSpanPostfix};

    use super::{
        GapDelta, LineBreak, gap_delta, line_breaks, lsp_semantic_tokens, lsp_type_index,
        semantic_token_legend,
    };

    const STRING: u32 = 18;
    const KEYWORD: u32 = 15;
    const CLASS: u32 = 2;
    const PROPERTY: u32 = 9;
    const OPERATOR: u32 = 21;

    fn encoded(source: &str) -> Vec<LspSemanticToken> {
        let parsed = parse_iso_literal(source);
        lsp_semantic_tokens(&parsed.tokens, source)
    }

    fn rebased(
        tokens: &[WithSpan<SemanticToken>],
        offset: u32,
    ) -> Vec<WithSpan<SemanticToken>> {
        tokens
            .iter()
            .map(|token| token.item.with_span(token.location.with_offset(offset)))
            .collect()
    }

    fn encode_rebased(
        tokens: &[WithSpan<SemanticToken>],
        offset: u32,
        page_content: &str,
    ) -> Vec<LspSemanticToken> {
        lsp_semantic_tokens(&rebased(tokens, offset), page_content)
    }

    fn of_type(lsp: &[LspSemanticToken], token_type: u32) -> Vec<&LspSemanticToken> {
        lsp.iter()
            .filter(|token| token.token_type == token_type)
            .collect()
    }

    fn gap(text: &str) -> GapDelta {
        gap_delta(&line_breaks(text), text, 0, text.len() as u32)
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
    fn gap_delta_same_line() {
        assert_eq!(
            gap("   "),
            GapDelta {
                delta_line: 0,
                tail_utf16: 3,
            },
        );
    }

    #[test]
    fn gap_delta_newline() {
        assert_eq!(
            gap("\n  "),
            GapDelta {
                delta_line: 1,
                tail_utf16: 2,
            },
        );
    }

    #[test]
    fn gap_delta_crlf_and_cr_are_one_break_each() {
        assert_eq!(
            gap("\r\n  "),
            GapDelta {
                delta_line: 1,
                tail_utf16: 2,
            },
        );
        assert_eq!(
            gap("\r  "),
            GapDelta {
                delta_line: 1,
                tail_utf16: 2,
            },
        );
        assert_eq!(
            gap("a\r\nb"),
            GapDelta {
                delta_line: 1,
                tail_utf16: 1,
            },
        );
        assert_eq!(
            gap("x\n\ny"),
            GapDelta {
                delta_line: 2,
                tail_utf16: 1,
            },
        );
        assert_eq!(
            gap("é\r\n  "),
            GapDelta {
                delta_line: 1,
                tail_utf16: 2,
            },
        );
        assert_eq!(
            gap("é\r  "),
            GapDelta {
                delta_line: 1,
                tail_utf16: 2,
            },
        );
    }

    #[test]
    fn gap_delta_counts_utf16_on_the_last_line() {
        assert_eq!(
            gap("é\n  "),
            GapDelta {
                delta_line: 1,
                tail_utf16: 2,
            },
        );
        assert_eq!(
            gap("aé"),
            GapDelta {
                delta_line: 0,
                tail_utf16: 2,
            },
        );
        assert_eq!(
            gap("a😀"),
            GapDelta {
                delta_line: 0,
                tail_utf16: 3,
            },
        );
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
        let lsp = lsp_semantic_tokens(&tokens, source);
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
        let lsp = lsp_semantic_tokens(&tokens, source);
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
                .filter(|token| token.item == SemanticToken::String)
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
                .filter(|token| token.item == SemanticToken::String)
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
                .filter(|token| token.item == SemanticToken::String)
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
                .filter(|token| token.item == SemanticToken::String)
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
        let tokens = SemanticToken::Content
            .with_span(Span::from_usize(0, source.len()))
            .wrap_vec();
        let lsp = lsp_semantic_tokens(&tokens, source);
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
        let tokens = SemanticToken::String
            .with_span(Span::from_usize(1, 3))
            .wrap_vec();
        let lsp = lsp_semantic_tokens(&tokens, source);
        assert_eq!(lsp.len(), 1);
        assert_eq!(lsp[0].delta_start, 1);
        assert_eq!(lsp[0].length, 1);
    }

    #[test]
    fn utf16_length_of_a_surrogate_pair() {
        let source = "a😀b";
        let tokens = SemanticToken::String
            .with_span(Span::from_usize(1, 5))
            .wrap_vec();
        let lsp = lsp_semantic_tokens(&tokens, source);
        assert_eq!(lsp.len(), 1);
        assert_eq!(lsp[0].delta_start, 1);
        assert_eq!(lsp[0].length, 2);
    }

    #[test]
    fn adjacent_spans_encode() {
        let source = "ab";
        let tokens = vec![
            SemanticToken::Keyword.with_span(Span::from_usize(0, 1)),
            SemanticToken::Type.with_span(Span::from_usize(1, 2)),
        ];
        let lsp = lsp_semantic_tokens(&tokens, source);
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
            SemanticToken::Keyword.with_span(Span::from_usize(0, 0)),
            SemanticToken::Type.with_span(Span::from_usize(0, 1)),
        ];
        let lsp = lsp_semantic_tokens(&tokens, source);
        assert_eq!(lsp.len(), 1);
        assert_eq!(lsp[0].delta_start, 0);
        assert_eq!(lsp[0].length, 1);
        assert_eq!(lsp[0].token_type, CLASS);
    }

    #[test]
    #[should_panic(expected = "mutually exclusive and ordered")]
    fn overlapping_spans_panic() {
        let source = "abcd";
        let tokens = vec![
            SemanticToken::Keyword.with_span(Span::from_usize(0, 2)),
            SemanticToken::Type.with_span(Span::from_usize(1, 3)),
        ];
        let _ = lsp_semantic_tokens(&tokens, source);
    }

    #[test]
    #[should_panic(expected = "mutually exclusive and ordered")]
    fn out_of_order_spans_panic() {
        let source = "abcd";
        let tokens = vec![
            SemanticToken::Keyword.with_span(Span::from_usize(2, 4)),
            SemanticToken::Type.with_span(Span::from_usize(0, 1)),
        ];
        let _ = lsp_semantic_tokens(&tokens, source);
    }

    #[test]
    #[should_panic(expected = "mutually exclusive and ordered")]
    fn reversed_literals_panic() {
        let source = "iso(`entrypoint Query.A`)\niso(`entrypoint Query.B`)";
        let first = "entrypoint Query.A";
        let second = "entrypoint Query.B";
        let first_start = source.find(first).expect("the first literal is in the file");
        let second_start = source.find(second).expect("the second literal is in the file");
        let parsed_a = parse_iso_literal(first);
        let parsed_b = parse_iso_literal(second);
        let mut tokens = rebased(&parsed_b.tokens, second_start as u32);
        tokens.extend(rebased(&parsed_a.tokens, first_start as u32));
        let _ = lsp_semantic_tokens(&tokens, source);
    }

    #[test]
    fn lsp_type_index_matches_the_legend() {
        assert_eq!(semantic_token_legend().token_types.len(), 23);
        assert_eq!(lsp_type_index(SemanticToken::Keyword), KEYWORD);
        assert_eq!(lsp_type_index(SemanticToken::Type), CLASS);
        assert_eq!(lsp_type_index(SemanticToken::FieldName), PROPERTY);
        assert_eq!(lsp_type_index(SemanticToken::ObjectKey), PROPERTY);
        assert_eq!(lsp_type_index(SemanticToken::GraphQLTypeName), 1);
        assert_eq!(lsp_type_index(SemanticToken::DirectiveName), 22);
        assert_eq!(lsp_type_index(SemanticToken::Variable), 8);
        assert_eq!(lsp_type_index(SemanticToken::Argument), 7);
        assert_eq!(lsp_type_index(SemanticToken::Integer), 19);
        assert_eq!(lsp_type_index(SemanticToken::String), STRING);
        assert_eq!(lsp_type_index(SemanticToken::BooleanOrNull), 8);
        assert_eq!(lsp_type_index(SemanticToken::Period), OPERATOR);
        assert_eq!(lsp_type_index(SemanticToken::Colon), OPERATOR);
        assert_eq!(lsp_type_index(SemanticToken::Equals), OPERATOR);
        assert_eq!(lsp_type_index(SemanticToken::Parenthesis), OPERATOR);
        assert_eq!(lsp_type_index(SemanticToken::Brace), OPERATOR);
        assert_eq!(lsp_type_index(SemanticToken::Content), OPERATOR);
        assert_eq!(lsp_type_index(SemanticToken::Bracket), OPERATOR);
        assert_eq!(lsp_type_index(SemanticToken::Error), 17);
    }
}
```

`extraction_offset_after_a_newline_sets_delta_line`: the template body starts after `` ` ``, then a newline and two spaces. `field` is `delta_line` 1, column 2.

`extraction_offset_counts_utf16_in_the_prefix`: `const é = iso(\`` is 16 UTF-8 bytes and 15 UTF-16 units. `delta_start` is 15.

`two_literals_are_file_absolute_and_in_order`: eight tokens. Index 4 is the second `entrypoint`. Previous piece is `A`; the gap is `` `)\niso(` ``, so `delta_line` 1, `delta_start` 5. Index 7 is `B` on the same line as that literal's `.`.

`two_literals_on_the_same_line`: gap after `A` is `` `) iso(` ``, `last_len` 1, `delta_start` 9.

`a_quoted_string_is_one_lsp_token`: `"the home route"` is 16 UTF-16 units. `{` is `last_len` 16 plus one space, `delta_start` 17.

`a_quoted_string_with_an_escaped_newline_is_one_lsp_token`: source `"hi\n"` is quote, `h`, `i`, backslash, `n`, quote. Length 6. Not split.

`a_block_string_with_content_on_the_opening_line`: pieces `"""the home` (11), `  route` (7), `"""`.

`a_block_string_with_closing_quotes_on_the_content_line`: pieces `"""`, `  route"""` (10). `{` is `last_len` 10 plus one space, `delta_start` 11.

`a_non_ascii_continuation_line_of_a_block_string_is_utf16_length`: `  café` is 7 UTF-8 bytes, 6 UTF-16 units.

`a_multiline_leftover_token_splits_the_same_way`: unterminated `"""` is leftover `Content`. The encoder still splits. Pieces `"""`, `  x`.

`utf16_length_of_a_surrogate_pair`: `😀` is bytes 1..5 of `a😀b`, two UTF-16 units.

`an_empty_span_emits_nothing`: `0..0` records no piece; the next token at `0..1` still encodes.

`gap_delta_counts_utf16_on_the_last_line`: `a😀` is 3 UTF-16 units (1 + 2).

`expect` in tests names an invariant the fixture established.
