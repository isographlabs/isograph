# LSP semantic token encoding

`Vec<WithSpan<SemanticToken>>` is literal-relative byte spans. LSP `textDocument/semanticTokens/full` wants `lsp_types::SemanticToken`: `delta_line`, `delta_start`, `length`, `token_type` index into a legend, `token_modifiers_bitset`. `delta_start` and `length` are UTF-16 code units. A token does not include a line break.

The encoder is one walk: for each parser token in iterator order, add `extraction_span.start`, split that span on line breaks, encode each nonempty piece. Parser tokens (and the literals iterator) are mutually exclusive and ordered in file-absolute bytes. A violation is a compiler bug; the walk panics.

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
use span::{Span, WithSpan};

pub fn lsp_semantic_tokens(
    tokens: &[WithSpan<SemanticToken>],
    page_content: &str,
    extraction_span: Span,
) -> Vec<LspSemanticToken> {
    lsp_semantic_tokens_for_literals([(tokens, extraction_span)], page_content)
}

pub fn lsp_semantic_tokens_for_literals<'a>(
    literals: impl IntoIterator<Item = (&'a [WithSpan<SemanticToken>], Span)>,
    page_content: &'a str,
) -> Vec<LspSemanticToken> {
    let mut last_token_start = 0u32;
    let mut last_span_end = 0u32;
    let mut encoded = Vec::new();
    for (tokens, extraction_span) in literals {
        for relative_token in tokens {
            let span = relative_token.location.with_offset(extraction_span.start);
            assert!(
                span.start >= last_span_end && span.end >= span.start,
                "semantic token spans must be mutually exclusive and ordered; got {}..{} after end {last_span_end}",
                span.start,
                span.end,
            );
            last_span_end = span.end;
            let mut remaining = &page_content[(span.start as usize)..(span.end as usize)];
            let mut piece_start = span.start;
            loop {
                let (line_text, next) = match next_line_break(remaining) {
                    Some((break_at, after)) => (
                        &remaining[..break_at],
                        (piece_start + after as u32, &remaining[after..]).wrap_some(),
                    ),
                    None => (remaining, None),
                };
                if !line_text.is_empty() {
                    let in_between = &page_content
                        [(last_token_start as usize)..(piece_start as usize)];
                    let (delta_line, delta_start) = delta_line_delta_start(in_between);
                    encoded.push(LspSemanticToken {
                        delta_line,
                        delta_start,
                        length: utf16_units(line_text),
                        token_type: lsp_type_index(relative_token.item),
                        token_modifiers_bitset: 0,
                    });
                    last_token_start = piece_start;
                }
                match next {
                    Some((next_start, next_remaining)) => {
                        piece_start = next_start;
                        remaining = next_remaining;
                    }
                    None => break,
                }
            }
        }
    }
    encoded
}

fn next_line_break(text: &str) -> Option<(usize, usize)> {
    text.find(['\r', '\n']).map(|i| {
        let after = if text.as_bytes()[i] == b'\r'
            && matches!(text.as_bytes().get(i + 1), Some(&b'\n'))
        {
            i + 2
        } else {
            i + 1
        };
        (i, after)
    })
}

fn utf16_units(text: &str) -> u32 {
    text.encode_utf16().count() as u32
}

fn delta_line_delta_start(text: &str) -> (u32, u32) {
    let mut rest = text;
    let mut line_break_count = 0u32;
    while let Some((_, after)) = next_line_break(rest) {
        line_break_count += 1;
        rest = &rest[after..];
    }
    (line_break_count, utf16_units(rest))
}
```

`extraction_span` is the literal's range in `page_content`. Relative token spans add to `extraction_span.start` via `Span::with_offset`. Literals are concatenated in iterator order; that order must be file order.

`assert!` is a panic. The input is `Vec<WithSpan<SemanticToken>>`; disjoint ordered spans are a parser invariant that type cannot hold. Adjacent spans (`end` of one equals `start` of the next) pass. `last_span_end` is the previous parser token's absolute end. `last_token_start` is the previous *emitted piece's* byte start: same-line `delta_start` is start-to-start.

`next_line_break` returns `(start_of_break, after_break)` in bytes into `text`. A break is `\r\n` (one break), `\n`, or `\r`, the same set as `IsographLangTokenKind::LineBreak` and as VS Code / LSP line offsets. The piece's `length` is UTF-16 units of the text before the break. A piece that is only a break (a blank line inside a multiline token) emits nothing; the byte cursor still advances by `after_break`. `delta_line_delta_start` uses the same function, so a skipped blank line shows up in the next piece's `delta_line`.

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
- `relative_token` is `&WithSpan<SemanticToken>`. Span is `relative_token.location.with_offset(extraction_span.start)`.
- `token_type` is `lsp_type_index(relative_token.item)`, not a field on the parser token.
- Origin `split_inclusive('\n')` kept the line break in `line_text`, so `len` included `\n` and the client may discard the token. Origin also treated only `\n` as a break, so `\r\n` left `\r` in `len` and a bare `\r` did not split. `next_line_break` is `\r\n` then `\n` then `\r`; `length` is the text before the break.
- Empty pieces emit nothing. `last_span_end` still advances by the whole parser token.
- Origin `line_text.len()` is UTF-8 bytes. `length` and `delta_start` are UTF-16: `utf16_units`.
- Origin `delta_line_delta_start` used `chars().enumerate()` for the newline index and `text.len()` (bytes) for the last-line width. Those units disagree on any non-ASCII last line. It also counted only `\n`. Both encode and delta use `next_line_break`.
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

pub use semantic_tokens::{
    lsp_semantic_tokens, lsp_semantic_tokens_for_literals, lsp_type_index,
    semantic_token_legend,
};
```

`next_line_break`, `utf16_units`, `delta_line_delta_start` stay in the module. Tests in that module call them.

## Tests

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
#[cfg(test)]
mod tests {
    use isograph_parser::{SemanticToken, parse_iso_literal};
    use prelude::Postfix;
    use span::{Span, WithSpanPostfix};

    use super::{
        delta_line_delta_start, lsp_semantic_tokens, lsp_semantic_tokens_for_literals,
        lsp_type_index, semantic_token_legend,
    };

    #[test]
    fn delta_line_delta_start_same_line() {
        assert_eq!(delta_line_delta_start("   "), (0, 3));
    }

    #[test]
    fn delta_line_delta_start_newline() {
        assert_eq!(delta_line_delta_start("\n  "), (1, 2));
    }

    #[test]
    fn delta_line_delta_start_crlf_and_cr_are_one_break_each() {
        assert_eq!(delta_line_delta_start("\r\n  "), (1, 2));
        assert_eq!(delta_line_delta_start("\r  "), (1, 2));
        assert_eq!(delta_line_delta_start("a\r\nb"), (1, 1));
        assert_eq!(delta_line_delta_start("x\n\ny"), (2, 1));
    }

    #[test]
    fn delta_line_delta_start_counts_utf16_on_the_last_line() {
        assert_eq!(delta_line_delta_start("é\n  "), (1, 2));
        assert_eq!(delta_line_delta_start("aé"), (0, 2));
    }

    #[test]
    fn entrypoint_encodes_as_keyword_class_operator_property() {
        let source = "entrypoint Query.foo";
        let parsed = parse_iso_literal(source);
        let lsp = lsp_semantic_tokens(
            &parsed.tokens,
            source,
            Span::from_usize(0, source.len()),
        );
        assert_eq!(lsp.len(), 4);
        assert_eq!(lsp[0].delta_line, 0);
        assert_eq!(lsp[0].delta_start, 0);
        assert_eq!(lsp[0].length, 11);
        assert_eq!(lsp[0].token_type, 15);
        assert_eq!(lsp[0].token_modifiers_bitset, 0);
        assert_eq!(lsp[1].delta_line, 0);
        assert_eq!(lsp[1].delta_start, 12);
        assert_eq!(lsp[1].length, 5);
        assert_eq!(lsp[1].token_type, 2);
        assert_eq!(lsp[2].delta_line, 0);
        assert_eq!(lsp[2].delta_start, 5);
        assert_eq!(lsp[2].length, 1);
        assert_eq!(lsp[2].token_type, 21);
        assert_eq!(lsp[3].delta_line, 0);
        assert_eq!(lsp[3].delta_start, 1);
        assert_eq!(lsp[3].length, 3);
        assert_eq!(lsp[3].token_type, 9);
    }

    #[test]
    fn extraction_offset_shifts_the_first_delta_start() {
        let prefix = "const x = iso(`";
        let literal = "field Pet.fullName { id }";
        let source = format!("{prefix}{literal}`)");
        let parsed = parse_iso_literal(literal);
        let extraction = Span::from_usize(prefix.len(), prefix.len() + literal.len());
        let lsp = lsp_semantic_tokens(&parsed.tokens, source.reference(), extraction);
        assert_eq!(lsp[0].delta_line, 0);
        assert_eq!(lsp[0].delta_start, prefix.len() as u32);
        assert_eq!(lsp[0].token_type, 15);
        assert_eq!(lsp[1].token_type, 2);
        assert_eq!(
            &source[prefix.len()..prefix.len() + 5],
            "field"
        );
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
        let lsp = lsp_semantic_tokens_for_literals(
            [
                (
                    parsed_a.tokens.as_slice(),
                    Span::from_usize(first_start, first_start + first.len()),
                ),
                (
                    parsed_b.tokens.as_slice(),
                    Span::from_usize(second_start, second_start + second.len()),
                ),
            ],
            source,
        );
        let last = lsp.last().expect("two literals produce tokens");
        assert!(last.delta_line >= 1);
        assert_eq!(last.token_type, 9);
    }

    #[test]
    fn tokens_on_successive_lines_set_delta_line() {
        let source = "field Query.Foo {\n  bar\n}";
        let parsed = parse_iso_literal(source);
        let lsp = lsp_semantic_tokens(
            &parsed.tokens,
            source,
            Span::from_usize(0, source.len()),
        );
        assert_eq!(lsp.len(), 7);
        assert_eq!(lsp[4].token_type, 21);
        assert_eq!(lsp[4].length, 1);
        assert_eq!(lsp[5].delta_line, 1);
        assert_eq!(lsp[5].delta_start, 2);
        assert_eq!(lsp[5].length, 3);
        assert_eq!(lsp[5].token_type, 9);
        assert_eq!(lsp[6].delta_line, 1);
        assert_eq!(lsp[6].delta_start, 0);
        assert_eq!(lsp[6].length, 1);
        assert_eq!(lsp[6].token_type, 21);
    }

    #[test]
    fn a_multiline_string_becomes_one_lsp_token_per_line_without_the_newline() {
        let source = "field Query.Foo \"\"\"\n  the home\n  route\n\"\"\" { bar }";
        let parsed = parse_iso_literal(source);
        let string_tokens = parsed
            .tokens
            .iter()
            .filter(|token| token.item == SemanticToken::String)
            .count();
        assert_eq!(string_tokens, 1);
        let lsp = lsp_semantic_tokens(
            &parsed.tokens,
            source,
            Span::from_usize(0, source.len()),
        );
        let string_lsp: Vec<_> = lsp
            .iter()
            .filter(|token| token.token_type == 18)
            .collect();
        assert_eq!(string_lsp.len(), 4);
        assert_eq!(string_lsp[0].length, 3);
        assert_eq!(string_lsp[1].delta_line, 1);
        assert_eq!(string_lsp[1].delta_start, 0);
        assert_eq!(string_lsp[1].length, 10);
        assert_eq!(string_lsp[2].delta_line, 1);
        assert_eq!(string_lsp[2].delta_start, 0);
        assert_eq!(string_lsp[2].length, 7);
        assert_eq!(string_lsp[3].delta_line, 1);
        assert_eq!(string_lsp[3].delta_start, 0);
        assert_eq!(string_lsp[3].length, 3);
        assert_eq!(lsp[8].delta_line, 0);
        assert_eq!(lsp[8].delta_start, 4);
        assert_eq!(lsp[8].length, 1);
        assert_eq!(lsp[8].token_type, 21);
    }

    #[test]
    fn a_blank_line_in_a_block_string_does_not_emit_a_token() {
        let source = "field Query.Foo \"\"\"\n\n  x\n\"\"\" { bar }";
        let parsed = parse_iso_literal(source);
        let lsp = lsp_semantic_tokens(
            &parsed.tokens,
            source,
            Span::from_usize(0, source.len()),
        );
        let string_lsp: Vec<_> = lsp
            .iter()
            .filter(|token| token.token_type == 18)
            .collect();
        assert_eq!(string_lsp.len(), 3);
        assert_eq!(string_lsp[0].length, 3);
        assert_eq!(string_lsp[1].delta_line, 2);
        assert_eq!(string_lsp[1].delta_start, 0);
        assert_eq!(string_lsp[1].length, 3);
        assert_eq!(string_lsp[2].delta_line, 1);
        assert_eq!(string_lsp[2].length, 3);
    }

    #[test]
    fn a_crlf_block_string_splits_without_including_the_break() {
        let source = "field Query.Foo \"\"\"\r\n  the home\r\n  route\r\n\"\"\" { bar }";
        let parsed = parse_iso_literal(source);
        let lsp = lsp_semantic_tokens(
            &parsed.tokens,
            source,
            Span::from_usize(0, source.len()),
        );
        let string_lsp: Vec<_> = lsp
            .iter()
            .filter(|token| token.token_type == 18)
            .collect();
        assert_eq!(string_lsp.len(), 4);
        assert_eq!(string_lsp[0].length, 3);
        assert_eq!(string_lsp[1].delta_line, 1);
        assert_eq!(string_lsp[1].delta_start, 0);
        assert_eq!(string_lsp[1].length, 10);
        assert_eq!(string_lsp[2].delta_line, 1);
        assert_eq!(string_lsp[2].length, 7);
        assert_eq!(string_lsp[3].delta_line, 1);
        assert_eq!(string_lsp[3].length, 3);
    }

    #[test]
    fn a_cr_block_string_splits_without_including_the_break() {
        let source = "field Query.Foo \"\"\"\r  x\r\"\"\" { bar }";
        let parsed = parse_iso_literal(source);
        let lsp = lsp_semantic_tokens(
            &parsed.tokens,
            source,
            Span::from_usize(0, source.len()),
        );
        let string_lsp: Vec<_> = lsp
            .iter()
            .filter(|token| token.token_type == 18)
            .collect();
        assert_eq!(string_lsp.len(), 3);
        assert_eq!(string_lsp[0].length, 3);
        assert_eq!(string_lsp[1].delta_line, 1);
        assert_eq!(string_lsp[1].delta_start, 0);
        assert_eq!(string_lsp[1].length, 3);
        assert_eq!(string_lsp[2].delta_line, 1);
        assert_eq!(string_lsp[2].length, 3);
    }

    #[test]
    fn utf16_length_of_a_non_ascii_token() {
        let source = "aéb";
        let tokens = SemanticToken::String
            .with_span(Span::from_usize(1, 3))
            .wrap_vec();
        let lsp = lsp_semantic_tokens(
            &tokens,
            source,
            Span::from_usize(0, source.len()),
        );
        assert_eq!(lsp.len(), 1);
        assert_eq!(lsp[0].delta_start, 1);
        assert_eq!(lsp[0].length, 1);
    }

    #[test]
    fn adjacent_spans_encode() {
        let source = "ab";
        let tokens = vec![
            SemanticToken::Keyword.with_span(Span::from_usize(0, 1)),
            SemanticToken::Type.with_span(Span::from_usize(1, 2)),
        ];
        let lsp = lsp_semantic_tokens(
            &tokens,
            source,
            Span::from_usize(0, source.len()),
        );
        assert_eq!(lsp.len(), 2);
        assert_eq!(lsp[0].delta_start, 0);
        assert_eq!(lsp[0].length, 1);
        assert_eq!(lsp[1].delta_line, 0);
        assert_eq!(lsp[1].delta_start, 1);
        assert_eq!(lsp[1].length, 1);
    }

    #[test]
    #[should_panic(expected = "mutually exclusive and ordered")]
    fn overlapping_spans_panic() {
        let source = "abcd";
        let tokens = vec![
            SemanticToken::Keyword.with_span(Span::from_usize(0, 2)),
            SemanticToken::Type.with_span(Span::from_usize(1, 3)),
        ];
        let _ = lsp_semantic_tokens(
            &tokens,
            source,
            Span::from_usize(0, source.len()),
        );
    }

    #[test]
    #[should_panic(expected = "mutually exclusive and ordered")]
    fn out_of_order_spans_panic() {
        let source = "abcd";
        let tokens = vec![
            SemanticToken::Keyword.with_span(Span::from_usize(2, 4)),
            SemanticToken::Type.with_span(Span::from_usize(0, 1)),
        ];
        let _ = lsp_semantic_tokens(
            &tokens,
            source,
            Span::from_usize(0, source.len()),
        );
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
        let _ = lsp_semantic_tokens_for_literals(
            [
                (
                    parsed_b.tokens.as_slice(),
                    Span::from_usize(second_start, second_start + second.len()),
                ),
                (
                    parsed_a.tokens.as_slice(),
                    Span::from_usize(first_start, first_start + first.len()),
                ),
            ],
            source,
        );
    }

    #[test]
    fn lsp_type_index_matches_the_legend() {
        assert_eq!(semantic_token_legend().token_types.len(), 23);
        assert_eq!(lsp_type_index(SemanticToken::Keyword), 15);
        assert_eq!(lsp_type_index(SemanticToken::Type), 2);
        assert_eq!(lsp_type_index(SemanticToken::FieldName), 9);
        assert_eq!(lsp_type_index(SemanticToken::ObjectKey), 9);
        assert_eq!(lsp_type_index(SemanticToken::GraphQLTypeName), 1);
        assert_eq!(lsp_type_index(SemanticToken::DirectiveName), 22);
        assert_eq!(lsp_type_index(SemanticToken::Variable), 8);
        assert_eq!(lsp_type_index(SemanticToken::Argument), 7);
        assert_eq!(lsp_type_index(SemanticToken::Integer), 19);
        assert_eq!(lsp_type_index(SemanticToken::String), 18);
        assert_eq!(lsp_type_index(SemanticToken::BooleanOrNull), 8);
        assert_eq!(lsp_type_index(SemanticToken::Period), 21);
        assert_eq!(lsp_type_index(SemanticToken::Colon), 21);
        assert_eq!(lsp_type_index(SemanticToken::Equals), 21);
        assert_eq!(lsp_type_index(SemanticToken::Parenthesis), 21);
        assert_eq!(lsp_type_index(SemanticToken::Brace), 21);
        assert_eq!(lsp_type_index(SemanticToken::Content), 21);
        assert_eq!(lsp_type_index(SemanticToken::Bracket), 21);
        assert_eq!(lsp_type_index(SemanticToken::Error), 17);
    }
}
```

`tokens_on_successive_lines_set_delta_line`: seven parser tokens, each one line. `{` is index 4. `bar` is the next line, column 2. `}` is the next line, column 0.

`a_multiline_string_becomes_one_lsp_token_per_line_without_the_newline`: parser still emits one `String`. Pieces are `"""`, `  the home` (10), `  route` (7), `"""`. Continuation `delta_start` is 0. `lsp[8]` is `{` on the same line as the closing `"""`, start-to-start 4.

`a_blank_line_in_a_block_string_does_not_emit_a_token`: pieces `"""`, skip, `  x`, `"""`. The skip is `delta_line == 2` on the next piece.

`a_crlf_block_string_splits_without_including_the_break` / `a_cr_block_string_splits_without_including_the_break`: same piece counts and lengths as LF; `length` is not 4 on the opening `"""`.

`adjacent_spans_encode`: `end == next.start` is ordered.

`overlapping_spans_panic` / `out_of_order_spans_panic` / `reversed_literals_panic`: the `assert!` message.

`extraction_offset_shifts_the_first_delta_start`: `last_token_start` starts at 0, first token byte start is `prefix.len()`, between-text is the prefix, no newline, `delta_start` is UTF-16 units of the prefix. The prefix is ASCII, so that equals `prefix.len()`.

`two_literals`: second literal is on the next line, so some token after the first literal has `delta_line >= 1`. Last token is `B`, PROPERTY.

`utf16_length_of_a_non_ascii_token`: `é` is bytes 1..3, one UTF-16 unit, column 1 after `a`.

`expect` in tests names an invariant the fixture established.
