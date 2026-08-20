# String and description values are GraphQL string values

A successful `StringLiteral` or `BlockStringLiteral` interned as a value or a description is the GraphQL string value, not the lexeme. The token span still includes the quotes.

Does not depend on type-annotation-null.md, parse-iso-literal-entry.md, or variable-declaration-or-usage.md.

Mental model `ArgumentValue::String(String)` is the decoded string. Descriptions use the same StringValue.

Relay strips quotes on `"..."` and does not process escapes (`"\\n"` stays backslash-n). Block strings get the spec indent algorithm, not `\"\"\"` unescape. This stage does the spec.

The lexer has already accepted the token. Decode is a total function over that lexeme.

## Decode

```rust
// from crates/isograph_parser/src/string_value.rs
pub(crate) fn string_value(lexeme: &str) -> String {
    decode_quoted_string(&lexeme[1..lexeme.len() - 1])
}

pub(crate) fn block_string_value(lexeme: &str) -> String {
    let inner = lexeme[3..lexeme.len() - 3].replace("\\\"\"\"", "\"\"\"");
    clean_block_string(&inner)
}
```

`string_value` is GraphQL `"..."` StringValue: `\"` `\\` `\/` `\b` `\f` `\n` `\r` `\t` and `\uXXXX`. `block_string_value` is GraphQL BlockStringValue: replace escaped `"""`, then the June 2018 indent algorithm (common indent of lines after the first, strip leading/trailing whitespace-only lines, join with `\n`). First line is not dedented.

`clean_block_string` is the algorithm in `relay-crates/graphql-syntax/src/relay_parser.rs` `clean_block_string_literal` after the inner slice, plus the `\"\"\"` replace before it.

Callers intern the returned `String`.

## Parse

```rust
// from crates/isograph_parser/src/string_value.rs
pub(crate) fn intern_string_lexeme<T: From<intern::string_key::StringKey>>(
    lexeme: &str,
) -> T {
    let value = if lexeme.starts_with("\"\"\"") {
        block_string_value(lexeme)
    } else {
        string_value(lexeme)
    };
    value.intern().to()
}
```

A quoted string cannot start with three quotes: the lexer already classified it. Slices in `string_value` / `block_string_value` are of a token the lexer produced (`""` is at least two characters, `""""""` at least six).

```rust
// from crates/isograph_parser/src/arguments.rs
    StringLiteralValueWrapper(intern_string_lexeme(span.text())).wrap_ok()
```

Before: `span.interned().map(StringLiteralValueWrapper).item`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    Description(intern_string_lexeme(span.text()))
        .with_span(span.location)
        .wrap_some()
```

Before: `span.interned().map(Description).wrap_some()`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
/// The interned GraphQL string value of a description.
pub struct Description(pub common_lang_types::DescriptionValue);
```

Before: "The interned source slice of a description, quotes included."

Who calls: `parse_string_literal`, `consume_description`. `TokenText::interned` stays for names.

The quoted-vs-block choice is duplicated. A `fn intern_string_token(span: TokenText<'_>) -> String` that reads `span.text()` and branches is the one helper both call.

## Tests

Span still covers the quotes. Interned payload is the value.

```rust
// from crates/isograph_parser/src/arguments.rs
    #[test]
    fn a_quoted_string_value_is_decoded() {
        let text = r#"a: "hi\n\"""#;
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        match as_argument(items[0].item.reference()).value.item.reference() {
            NonConstantValue::String(value) => {
                assert_eq!(value.0, "hi\n\"".intern().to());
            }
            value => panic!("expected a string, got {value:?}"),
        }
        assert_eq!(
            as_argument(items[0].item.reference()).value.location,
            span_of(text, r#""hi\n\"""#),
        );
    }

    #[test]
    fn an_empty_string_value_is_empty() {
        let text = r#"a: """#;
        let (items, errors, _) = parsed_pairs(text);
        assert_eq!(errors, vec![]);
        match as_argument(items[0].item.reference()).value.item.reference() {
            NonConstantValue::String(value) => {
                assert_eq!(value.0, "".intern().to());
            }
            value => panic!("expected a string, got {value:?}"),
        }
    }
```

`an_empty_string_is_a_value` already only checks the variant. Keep it. `a_block_string_is_a_value` and `an_empty_block_string_is_a_value` assert the decoded payload (`"hi"` and `""`).

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn a_single_line_description_is_the_decoded_string() {
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
    fn a_quoted_string_processes_escapes() {
        assert_eq!(string_value(r#""a\nb""#), "a\nb");
        assert_eq!(string_value(r#""\"""#), "\"");
        assert_eq!(string_value(r#""\\""#), "\\");
        assert_eq!(string_value(r#""\u0041""#), "A");
        assert_eq!(string_value("\"\""), "");
    }

    #[test]
    fn a_block_string_dedents_and_unescapes_triple_quotes() {
        assert_eq!(block_string_value("\"\"\"hi\"\"\""), "hi");
        assert_eq!(
            block_string_value("\"\"\"\n  hello\n  world\n\"\"\""),
            "hello\nworld",
        );
        assert_eq!(
            block_string_value("\"\"\"\\\"\"\"\"\"\""),
            "\"\"\"",
        );
    }
```

Degenerate: `"\\/"` is `/`. `"\\b"` is U+0008. A one-line block string `"\"\"\"   hi\"\"\""` keeps the leading spaces (first line is not dedented).
