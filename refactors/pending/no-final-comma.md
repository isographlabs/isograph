# no-final-comma: one-item contexts reject a trailing comma

A doc of the series parsing-plan.md orders, after the grammar feature docs; written against parsing-standards.md's world. It lands the last piece of the comma rule: a comma is meaningful only inside a list, between two items or after the last one, so a context that holds exactly one item is not a list and admits no comma at all. Chunking already emits the leading and doubled comma errors itself (no-empty-chunks.md); the one case it cannot distinguish is a final comma, which sits in a contentful chunk's trailing boundary like any legal list-trailing comma and is wrong only because of where its level sits in the grammar. This doc adds the one boundary inspection that catches it, in the two one-item walkers.

The two one-item contexts and what becomes an error:

```
entrypoint Query.foo,        <- error at the comma
field Query.Foo { bar },     <- error at the comma
pointer Pet.B to Pet { x },  <- error at the comma
field Query.Foo($x: [Pet,])  <- error at the comma
```

Trailing commas inside lists stay legal, wherever the comma sits among the boundary's line breaks: `{ bar, }`, `{ bar,\n }`, and pathologically `{ bar\n, }` all parse, as do `(a: 1,)` and `{ id: 4, }`.

## The method

`Chunk`'s fields are private (parsing-standards.md), so the read is a method on its narrowed surface, landing here with its first callers:

```rust
// from crates/isograph_parser/src/chunk.rs
impl Chunk {
    /// The comma in the trailing boundary, when one exists. Only a list gives a
    /// boundary comma meaning; the one-item walkers call this to reject it.
    pub fn boundary_comma(&self) -> Option<Span> {
        let separator = self.trailing_separator.as_ref()?;
        separator
            .0
            .iter()
            .find(|token| token.item == SeparatorToken::Comma)
            .map(|token| token.location)
    }
}
```

By one-comma-per-boundary.md there is at most one comma for `find` to find. `require_end` cannot subsume this check: the stream reads only a chunk's contents, and the boundary is not in it.

## Site 1: the root level

`try_parse` checks the declaration chunk's boundary after parsing it, so an incomplete declaration still reports its own error (`entrypoint,` reports the missing identifier, not the comma), and before the extra-chunk check, so `entrypoint A.b, field X.y` reports the comma, the earlier offense. Before:

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
    if let Some(comma) = declaration.item.boundary_comma() {
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

## Site 2: the `[...]` interior

`parse_bracket_interior_type` in variables.rs checks its one chunk the same way, after the type parses and the walker's end check passes. Before:

```rust
// from crates/isograph_parser/src/variables.rs
        let mut stream = chunk.item.stream();
        let parsed = parse_type_annotation(&mut stream)?;
        stream.require_end(Expectation::EndOfType)?;
        annotation = Some(parsed);
```

After:

```rust
// from crates/isograph_parser/src/variables.rs
        let mut stream = chunk.item.stream();
        let parsed = parse_type_annotation(&mut stream)?;
        stream.require_end(Expectation::EndOfType)?;
        if let Some(comma) = chunk.item.boundary_comma() {
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

The before-snippets track the feature docs as revised onto parsing-standards.md; this doc's changes are the `boundary_comma` insertions alone.

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

1. The `boundary_comma` method, the two site insertions, the test moves, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
