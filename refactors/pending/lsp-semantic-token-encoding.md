# LSP semantic token encoding

`Vec<WithSpan<SemanticToken>>` is literal-relative byte spans. LSP `textDocument/semanticTokens/full` wants `lsp_types::SemanticToken`: `delta_line`, `delta_start`, `length`, `token_type` index into a legend, `token_modifiers_bitset`. `delta_start` and `length` are UTF-16 code units. A token does not include a line break.

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
use span::{Span, WithSpan};

pub struct AbsoluteToken {
    pub absolute_char_start: u32,
    pub len: u32,
    pub semantic_token: SemanticToken,
}
```

Origin: `AbsoluteIsographSemanticToken`. Delta: `IsographSemanticToken` is i2's `SemanticToken`. `absolute_char_start` is a UTF-8 byte offset into `page_content`. `len` is UTF-16 code units of that token's text on one line.

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
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
    let absolute = literals.into_iter().flat_map(|(tokens, extraction_span)| {
        tokens.iter().flat_map(move |relative_token| {
            absolutize_relative_token(page_content, extraction_span, relative_token)
        })
    });
    convert_absolute_token_to_lsp_token(absolute, page_content).collect()
}
```

`extraction_span` is the literal's range in `page_content`. Relative token spans add to `extraction_span.start`. Literals are concatenated in iterator order.

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

- `relative_token` is `&WithSpan<SemanticToken>`. Span is `relative_token.location`.
- `token_type` is `lsp_type_index(absolute_token.semantic_token)`, not a field on the parser token.
- `split_inclusive('\n')` kept the line break in `line_text`, so `len` included `\n` and the client may discard the token. `split_inclusive` still walks, then `strip_suffix('\n')`. Empty pieces (a blank line inside a multiline token) emit nothing. The byte cursor still advances by the inclusive piece, including the `\n`.
- `line_text.len()` is UTF-8 bytes. `len` and `delta_start` are UTF-16 code units: `utf16_units`.
- `delta_line_delta_start` used `chars().enumerate()` for the newline index and `text.len()` (bytes) for the last-line width. Those units disagree on any non-ASCII last line. Newline search is bytes (`b'\n'`). Last-line width is `utf16_units`.

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
fn utf16_units(text: &str) -> u32 {
    text.encode_utf16().count() as u32
}

fn absolutize_relative_token<'a>(
    page_content: &'a str,
    iso_literal_extraction_span: Span,
    relative_token: &'a WithSpan<SemanticToken>,
) -> impl Iterator<Item = AbsoluteToken> + 'a {
    let start = iso_literal_extraction_span.start + relative_token.location.start;
    let end = iso_literal_extraction_span.start + relative_token.location.end;
    let span_content = &page_content[(start as usize)..(end as usize)];
    span_content
        .split_inclusive('\n')
        .scan(start, move |absolute_char_start, piece| {
            let line_text = piece.strip_suffix('\n').unwrap_or(piece);
            let token_start = *absolute_char_start;
            *absolute_char_start += piece.len() as u32;
            let token = if line_text.is_empty() {
                None
            } else {
                AbsoluteToken {
                    absolute_char_start: token_start,
                    len: utf16_units(line_text),
                    semantic_token: relative_token.item,
                }
                .wrap_some()
            };
            token.wrap_some()
        })
        .flatten()
}

fn convert_absolute_token_to_lsp_token<'a>(
    absolute_tokens: impl Iterator<Item = AbsoluteToken> + 'a,
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
            token_type: lsp_type_index(absolute_token.semantic_token),
            token_modifiers_bitset: 0,
        };
        *last_token_start = absolute_token.absolute_char_start;
        token.wrap_some()
    })
}

pub fn delta_line_delta_start(text: &str) -> (u32, u32) {
    let mut last_line_break_byte = 0usize;
    let mut line_break_count = 0u32;
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            line_break_count += 1;
            last_line_break_byte = index + 1;
        }
    }
    (
        line_break_count,
        utf16_units(&text[last_line_break_byte..]),
    )
}
```

`scan` yields `Option<AbsoluteToken>` on every piece (`token.wrap_some()`), so a blank line does not stop the iterator. `flatten` drops the inner `None`.

`strip_suffix` returns `Option<&str>`. `unwrap_or(piece)` is the piece when it has no trailing `\n` (the last segment).

`convert_absolute_token_to_lsp_token` is origin otherwise. `last_token_start` is the previous token's byte start, not its end: same-line `delta_start` is start-to-start.

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
    AbsoluteToken, delta_line_delta_start, lsp_semantic_tokens,
    lsp_semantic_tokens_for_literals, lsp_type_index, semantic_token_legend,
};
```

`absolutize_relative_token` and `convert_absolute_token_to_lsp_token` stay in the module. Tests in that module call them.

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
        assert_eq!(string_lsp[1].length, 10);
        assert_eq!(string_lsp[2].delta_line, 1);
        assert_eq!(string_lsp[2].length, 7);
        assert_eq!(string_lsp[3].delta_line, 1);
        assert_eq!(string_lsp[3].length, 3);
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

`\"\"\"\n  the home\n  route\n\"\"\"` splits into `\"\"\"`, `  the home` (10 UTF-16 units), `  route` (7), `\"\"\"`. The first string token is not the first token in the literal (`field` is), so `string_lsp[0].delta_line` is not asserted as 0. `string_lsp[1].delta_line` is 1 because the next piece is the next line of the same parser token.

`utf16_length_of_a_non_ascii_token`: `é` is bytes 1..3, one UTF-16 unit, column 1 after `a`.

`extraction_offset_shifts_the_first_delta_start`: `last_token_start` starts at 0, first token byte start is `prefix.len()`, between-text is the prefix, no newline, `delta_start` is UTF-16 units of the prefix. The prefix is ASCII, so that equals `prefix.len()`.

`two_literals`: second literal is on the next line, so some token after the first literal has `delta_line >= 1`. Last token is `B`, PROPERTY.

`expect` in tests names an invariant the fixture established.
