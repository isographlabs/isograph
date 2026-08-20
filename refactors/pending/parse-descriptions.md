# parse-descriptions: consume_description

A description is one `StringLiteral` token (`"..."`) or one `BlockStringLiteral` token (`"""..."""`). `consume_description` takes it when the next item is one of those.

A block string is a single token whatever it contains, line breaks included, so a multi-line description never splits the chunk. The value is the interned source slice, quotes included. Unquoting and block-string dedenting are derivations a consumer computes from that key.

Origin: `parse_optional_description` in `crates/isograph_lang_parser/src/description.rs` and `Description` in `crates/isograph_lang_types/src/string_key_wrappers.rs`. Delta: i2 does not unquote or dedent; isograph stores the cleaned inner text. i2 function name is `consume_description` (cursor `consume_*` convention).

## The types

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
/// The interned source slice of a description, quotes included.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Description(common_lang_types::DescriptionValue);
```

## The parser

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[cfg_attr(not(test), expect(dead_code))]
pub(crate) fn consume_description(cursor: &mut ItemCursor<'_>) -> Option<WithSpan<Description>> {
    let span = cursor
        .consume_token_if(NonBracketTokenKind::StringLiteral, SemanticToken::String)
        .or_else(|| {
            cursor.consume_token_if(
                NonBracketTokenKind::BlockStringLiteral,
                SemanticToken::String,
            )
        })?;
    span.interned().map(Description).wrap_some()
}
```

A token that is not a string or block string is not consumed. An unterminated `"` lexes as `Error` plus whatever follows, so it is not consumed here.

## Tests

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn chunked(text: &str) -> WithSpan<ChunkedLevel> {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        let (tree, comma_errors) = chunk(brackets.reference());
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        tree
    }

    fn stream_of<'a>(
        tree: &'a WithSpan<ChunkedLevel>,
        text: &'a str,
        tokens: &'a mut Vec<WithSpan<SemanticToken>>,
        errors: &'a mut Vec<WithSpan<ParseError>>,
    ) -> crate::chunk_stream::ChunkStream<'a> {
        tree.item.0[0].item.stream(text, tokens, errors)
    }

    #[test]
    fn a_single_line_description_is_the_source_slice_including_quotes() {
        let text = "\"the home route\"";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let description = consume_description(stream.cursor())
            .expect("the fixture is a string description");
        assert_eq!(description.location, span_of(text, "\"the home route\""));
        assert_eq!(
            description.item,
            Description("\"the home route\"".intern().to())
        );
        assert_eq!(errors, vec![]);
        assert_eq!(
            tokens,
            SemanticToken::String
                .with_span(span_of(text, "\"the home route\""))
                .wrap_vec(),
        );
    }

    #[test]
    fn an_empty_string_is_a_description() {
        let text = "\"\"";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let description = consume_description(stream.cursor())
            .expect("the fixture is a string description");
        assert_eq!(description.location, span_of(text, "\"\""));
        assert_eq!(description.item, Description("\"\"".intern().to()));
    }

    #[test]
    fn a_block_string_description_is_one_token_including_line_breaks() {
        let text = "\"\"\"\n  the home\n  route\n\"\"\"";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 1);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let description = consume_description(stream.cursor())
            .expect("the fixture is a block-string description");
        assert_eq!(
            description.location,
            span_of(text, "\"\"\"\n  the home\n  route\n\"\"\"")
        );
        assert_eq!(
            description.item,
            Description("\"\"\"\n  the home\n  route\n\"\"\"".intern().to())
        );
        assert_eq!(errors, vec![]);
        assert_eq!(
            tokens,
            SemanticToken::String
                .with_span(span_of(text, "\"\"\"\n  the home\n  route\n\"\"\""))
                .wrap_vec(),
        );
    }

    #[test]
    fn a_block_string_with_line_breaks_does_not_split_the_chunk() {
        let text = "Foo \"\"\"\n  the home\n  route\n\"\"\" Bar";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 1);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        assert_eq!(
            cursor
                .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
                .map(|token| token.location),
            span_of(text, "Foo").wrap_some(),
        );
        let description = consume_description(cursor)
            .expect("the fixture carries a block-string description");
        assert_eq!(
            description.location,
            span_of(text, "\"\"\"\n  the home\n  route\n\"\"\"")
        );
        assert_eq!(
            cursor
                .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
                .map(|token| token.location),
            span_of(text, "Bar").wrap_some(),
        );
    }

    #[test]
    fn a_description_does_not_consume_the_following_item() {
        let text = "\"hi\" Foo";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        let description = consume_description(cursor).expect("the fixture starts with a description");
        assert_eq!(description.location, span_of(text, "\"hi\""));
        assert_eq!(
            cursor
                .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
                .map(|token| token.location),
            span_of(text, "Foo").wrap_some(),
        );
    }

    #[test]
    fn a_non_string_is_not_a_description() {
        let text = "Foo";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        assert_eq!(consume_description(cursor), None);
        assert_eq!(tokens, vec![]);
        assert_eq!(errors, vec![]);
        assert_eq!(
            cursor
                .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
                .map(|token| token.location),
            span_of(text, "Foo").wrap_some(),
        );
    }

    #[test]
    fn a_brace_group_is_not_a_description() {
        let text = "{ bar }";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        assert_eq!(consume_description(cursor), None);
        assert_eq!(
            cursor
                .consume_group_if(BracketKind::Brace, SemanticToken::Brace, |_, _| ())
                .map(|group| group.location),
            span_of(text, "{ bar }").wrap_some(),
        );
    }

    #[test]
    fn an_unterminated_string_is_not_a_description() {
        let text = "\"unterminated";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        assert_eq!(consume_description(cursor), None);
        assert_eq!(
            cursor
                .consume_token_if(NonBracketTokenKind::Error, SemanticToken::Error)
                .map(|token| token.location),
            span_of(text, "\"").wrap_some(),
        );
    }

    #[test]
    fn an_empty_block_string_is_a_description() {
        let text = "\"\"\"\"\"\"";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let description = consume_description(stream.cursor())
            .expect("the fixture is a block-string description");
        assert_eq!(description.location, span_of(text, "\"\"\"\"\"\""));
        assert_eq!(description.item, Description("\"\"\"\"\"\"".intern().to()));
    }
```

## Landing checklist

1. The `Description` type, `consume_description`, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
