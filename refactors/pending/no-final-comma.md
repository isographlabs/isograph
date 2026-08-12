# no-final-comma: one-item contexts reject a trailing comma

A doc of the series parsing-plan.md orders, after the grammar feature docs. It lands the last piece of the comma rule: a comma is meaningful only inside a list, between two items or after the last one, so a context that holds exactly one item is not a list and admits no comma at all. Chunking already emits the leading and doubled comma errors itself (no-empty-chunks.md); the one case it cannot distinguish is a final comma, which sits in a contentful chunk's trailing boundary like any legal list delimiter. This doc adds the one inspection that catches it.

The two one-item contexts and what becomes an error:

```
entrypoint Query.foo,        <- error at the comma
field Query.Foo { bar },     <- error at the comma
pointer Pet.B to Pet { x },  <- error at the comma
field Query.Foo($x: [Pet,])  <- error at the comma
```

Trailing commas inside lists stay legal, wherever the comma sits among the boundary's line breaks: `{ bar, }`, `{ bar,\n }`, and pathologically `{ bar\n, }` all parse, as do `(a: 1,)` and `{ id: 4, }`.

## The helper

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
/// The comma in a chunk's trailing boundary, when one exists. Only a list level gives a
/// boundary comma meaning; the callers sit in one-item contexts, where it is an error.
pub(crate) fn boundary_comma(chunk: &WithSpan<Chunk>) -> Option<Span> {
    let separator = chunk.item.trailing_separator.as_ref()?;
    separator
        .item
        .0
        .iter()
        .find(|token| token.item == SeparatorToken::Comma)
        .map(|token| token.location)
}
```

`SeparatorToken` joins parse_iso_literal.rs's crate imports.

## Site 1: the root level

`try_parse` checks the declaration chunk's boundary after parsing it, so an incomplete declaration still reports its own error (`entrypoint,` reports the missing identifier, not the comma). Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn try_parse(text: &str, root: &WithSpan<ChunkedLevel>) -> Result<IsoLiteralParse, WithSpan<ParseError>> {
    let (declaration, extra) = declaration_chunk(root)?;
    let parse = parse_declaration_chunk(text, declaration)?;
    if let Some(extra) = extra {
        return Err(WithSpan::new(ParseError::MultipleDeclarations, extra));
    }
    Ok(parse)
}
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn try_parse(text: &str, root: &WithSpan<ChunkedLevel>) -> Result<IsoLiteralParse, WithSpan<ParseError>> {
    let (declaration, extra) = declaration_chunk(root)?;
    let parse = parse_declaration_chunk(text, declaration)?;
    if let Some(comma) = boundary_comma(declaration) {
        return Err(WithSpan::new(
            ParseError::expected(Expectation::EndOfDeclaration, Found::Token(NonBracketTokenKind::Comma)),
            comma,
        ));
    }
    if let Some(extra) = extra {
        return Err(WithSpan::new(ParseError::MultipleDeclarations, extra));
    }
    Ok(parse)
}
```

The comma check precedes the extra-chunk check, so `entrypoint A.b, field X.y` reports the comma, which is the earlier offense.

## Site 2: the `[...]` interior

`parse_bracket_interior_type` in variables.rs checks its one chunk's boundary the same way, after the type parses. Before:

```rust
// from crates/isograph_parser/src/variables.rs
        let mut items = chunk.item.contents.iter().safe_peekable();
        let parsed = parse_type_annotation(&mut items, chunk.location.start)?;
        expect_chunk_end(&mut items, Expectation::EndOfType)?;
        annotation = Some(parsed);
```

After:

```rust
// from crates/isograph_parser/src/variables.rs
        let mut items = chunk.item.contents.iter().safe_peekable();
        let parsed = parse_type_annotation(&mut items, chunk.location.start)?;
        expect_chunk_end(&mut items, Expectation::EndOfType)?;
        if let Some(comma) = boundary_comma(chunk) {
            return Err(WithSpan::new(
                ParseError::expected(
                    Expectation::EndOfType,
                    Found::Token(NonBracketTokenKind::Comma),
                ),
                comma,
            ));
        }
        annotation = Some(parsed);
```

`boundary_comma` joins variables.rs's crate imports, and `parse_bracket_interior_type`'s doc comment gains the sentence "No comma is valid in its boundary: the interior is a one-item context."

## Tests

The parse-entrypoint.md test `surrounding_line_breaks_and_interior_spaces_are_insignificant` loses its trailing-comma cases (`"entrypoint Query.foo,"`, `"\nentrypoint Query.foo,\n"`), which move here as errors:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs (test module)
    #[test]
    fn a_final_comma_after_the_declaration_is_an_error() {
        for text in [
            "entrypoint Query.foo,",
            "\nentrypoint Query.foo,\n",
            "field Query.Foo { bar },",
            "pointer Pet.BestFriend to Pet { id },",
        ] {
            assert_unparsed(
                text,
                expected(Expectation::EndOfDeclaration, Found::Token(Comma)),
                span_of(text, ","),
            );
        }
    }

    #[test]
    fn a_comma_before_a_second_declaration_reports_the_comma() {
        let text = "entrypoint Query.foo, field User.name";
        assert_unparsed(
            text,
            expected(Expectation::EndOfDeclaration, Found::Token(Comma)),
            span_of(text, ","),
        );
    }

    #[test]
    fn a_final_comma_inside_a_list_type_degrades_that_declaration() {
        let text = "field Query.Foo($pets: [Pet,]) { bar }";
        let parse = parsed(text);
        let unparsed = match &variables_of(&parse).item.0[0].item {
            VariableDeclaration::Unparsed(unparsed) => unparsed.reason,
            declaration => panic!("expected an unparsed declaration, got {declaration:?}"),
        };
        assert_eq!(
            unparsed.item,
            expected(Expectation::EndOfType, Found::Token(NonBracketTokenKind::Comma))
        );
        assert_eq!(unparsed.location, span_of(text, ","));
    }

    #[test]
    fn trailing_commas_inside_lists_stay_legal() {
        for text in [
            "field Query.Foo { bar, }",
            "field Query.Foo { bar,\n}",
            "field Query.Foo { bar\n, }",
            "field Query.Foo { bar(a: 1,) }",
            "field Query.Foo { bar(input: { id: 4, }) }",
            "field Query.Foo($x: Int,) { bar }",
        ] {
            let parse = parsed(text);
            assert_eq!(parse.item.errors(), vec![], "for literal {text:?}");
        }
    }
```

The `pointer` and `field` cases require their docs to have landed, which the ordering guarantees.

## Landing checklist

1. The helper, the two site checks, the test moves, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
