# String and description values drop the outer quotes

A successful `StringLiteral` or `BlockStringLiteral` interned as a value or a description is the interior, not the lexeme with quotes. The token span still includes the quotes.

Does not depend on type-annotation-null.md, parse-iso-literal-entry.md, or variable-declaration-or-usage.md.

The consume site already distinguished the token kind. Quoted strings intern the interior span. Block strings run `clean_block_string` on the interior, then intern. No `starts_with("\"\"\"")`. No `\"\"\"` replace: GraphQL escaped triple quotes are not interned as `"""`; cutting the wrapping `"""` is enough.

Relay quoted strings intern `source[1..len-1]` with no escape processing. Quoted strings here intern that same interior via `TokenText::interned` on a span that already excludes the quotes. Block strings use Relay's indent algorithm (`clean_block_string_literal` after its inner slice).

## `TokenText` interior span

```rust
// from crates/isograph_parser/src/chunk_stream.rs
impl<'a> TokenText<'a> {
    pub(crate) fn exclude_ends(self, n: u32) -> TokenText<'a> {
        TokenText {
            location: Span::new(self.location.start + n, self.location.end - n),
            text: self.text,
        }
    }
}
```

`exclude_ends(1)` is the `"..."` interior. `exclude_ends(3)` is the `"""..."""` interior. `interned` interns `text()` of that span, a substring of the literal, not a slice of a slice of a copied lexeme.

The tree node's span stays the full token (`span.location`). `interned().location` is the interior; discard it.

## Block strings

```rust
// from crates/isograph_parser/src/string_value.rs
use intern::string_key::Intern;
use prelude::Postfix;

pub(crate) fn intern_block_string_value<T: From<intern::string_key::StringKey>>(
    interior: &str,
) -> T {
    clean_block_string(interior).intern().to()
}
```

`interior` is already without the wrapping `"""`. `clean_block_string` is `relay-crates/graphql-syntax/src/relay_parser.rs` `clean_block_string_literal` after `&source[3..len-3]`. Do not slice quotes again.

## Parse

```rust
// from crates/isograph_parser/src/arguments.rs
fn parse_string_literal(
    cursor: &mut ItemCursor<'_>,
) -> Result<StringLiteralValueWrapper, WithSpan<AstError>> {
    if let Some(span) = cursor
        .consume_token_if(NonBracketTokenKind::StringLiteral, SemanticToken::String)
    {
        return StringLiteralValueWrapper(span.exclude_ends(1).interned().item).wrap_ok();
    }
    if let Some(span) = cursor.consume_token_if(
        NonBracketTokenKind::BlockStringLiteral,
        SemanticToken::String,
    ) {
        return StringLiteralValueWrapper(intern_block_string_value(span.exclude_ends(3).text()))
            .wrap_ok();
    }
    cursor
        .expected(Expectation::Token(NonBracketTokenKind::StringLiteral))
        .wrap_err()
}
```

Before: `span.interned().map(StringLiteralValueWrapper).item` after `StringLiteral` or_else `BlockStringLiteral`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub(crate) fn consume_description(cursor: &mut ItemCursor<'_>) -> Option<WithSpan<Description>> {
    if let Some(span) = cursor
        .consume_token_if(NonBracketTokenKind::StringLiteral, SemanticToken::String)
    {
        return Description(span.exclude_ends(1).interned().item)
            .with_span(span.location)
            .wrap_some();
    }
    cursor
        .consume_token_if(
            NonBracketTokenKind::BlockStringLiteral,
            SemanticToken::String,
        )
        .map(|span| {
            Description(intern_block_string_value(span.exclude_ends(3).text()))
                .with_span(span.location)
        })
}
```

Before: `span.interned().map(Description).wrap_some()` after the same or_else.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
/// The interned interior of a description, quotes excluded.
pub struct Description(pub common_lang_types::DescriptionValue);
```

Before: "The interned source slice of a description, quotes included."

Who calls: `parse_string_literal`, `consume_description`. `TokenText::interned` stays for names.

## Tests

Span still covers the quotes. Interned payload is the interior (quoted) or `clean_block_string` of the interior (block).

```rust
// from crates/isograph_parser/src/arguments.rs
    #[test]
    fn a_quoted_string_value_drops_the_quotes() {
        let text = r#"a: "hi""#;
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        match as_argument(items[0].item.reference()).value.item.reference() {
            NonConstantValue::String(value) => {
                assert_eq!(value.0, "hi".intern().to());
            }
            value => panic!("expected a string, got {value:?}"),
        }
        assert_eq!(
            as_argument(items[0].item.reference()).value.location,
            span_of(text, r#""hi""#),
        );
    }

    #[test]
    fn a_quoted_string_does_not_process_escapes() {
        let text = r#"a: "hi\n""#;
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        match as_argument(items[0].item.reference()).value.item.reference() {
            NonConstantValue::String(value) => {
                assert_eq!(value.0, r#"hi\n"#.intern().to());
            }
            value => panic!("expected a string, got {value:?}"),
        }
    }
```

`an_empty_string_is_a_value` already only checks the variant. Keep it. `a_block_string_is_a_value` and `an_empty_block_string_is_a_value` assert the cleaned payload (`"hi"` and `""`).

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn a_single_line_description_drops_the_quotes() {
        let text = "\"the home route\"";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let description =
            consume_description(stream.cursor()).expect("the fixture is a string description");
        assert_eq!(description.location, span_of(text, "\"the home route\""));
        assert_eq!(
            description.item,
            Description("the home route".intern().to())
        );
    }
```

Before: interned `"\"the home route\""`. Same for `an_empty_string_is_a_description` (`""` payload empty), `an_empty_block_string_is_a_description`, `a_block_string_description_is_one_token_including_line_breaks` (payload is the dedented interior).

```rust
// from crates/isograph_parser/src/string_value.rs
    #[test]
    fn a_block_string_dedents() {
        assert_eq!(clean_block_string("hi"), "hi");
        assert_eq!(
            clean_block_string("\n  hello\n  world\n"),
            "hello\nworld",
        );
    }
```

Degenerate: a one-line block interior `"   hi"` keeps the leading spaces (first line is not dedented). Quoted `"\"hi\""` interned payload is `"hi"` including the inner quote characters.
