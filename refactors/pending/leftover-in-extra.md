# Leftover in extra

Trailing separators of a singleton chunk and unread remainder of a failed chunk are in `Slot.extra`.

Does not depend on four-trees.md, type-annotation-null.md, parse-iso-literal-entry.md, or leftover-semantic-tokens.md. Highlighting is leftover-semantic-tokens.md.

## Trailing separators go in `Slot.extra`

`entrypoint Query.foo,` parses. The comma is an `AstError` and is not a content item; it sits on `Chunk.trailing_separator`. Resolve hits the slot unmatched span.

After a successful `parse_one_chunk`, when `leftover` is not `Expectation::Separator(_)` (root / `[...]` type: `EndOfDeclaration`, `EndOfType`) and `trailing_separator` is `Some`, append those separator tokens to `extra` as `ChunkContentItem::NonBracket`.

```rust
// from crates/isograph_parser/src/chunk.rs
fn separator_as_content(token: WithSpan<SeparatorToken>) -> WithSpan<ChunkContentItem> {
    let kind = match token.item {
        SeparatorToken::Comma => NonBracketTokenKind::Comma,
        SeparatorToken::LineBreak => NonBracketTokenKind::LineBreak,
    };
    ChunkContentItem::NonBracket(NonBracketToken(kind)).with_span(token.location)
}
```

Before: `parse_singleton` only `errors.push` at `boundary_comma()`. After: that error stays, and `parse_one_chunk` folds the trailing separator into `extra` (after any unread contents). List `parse_each_chunk` leftover is `Separator(_)`; trailing commas stay on the chunk, not in `extra`.

`entrypoint Query.foo bar,` extra is `bar` then the comma.

## Failed remainder is `extra`, not the whole chunk

```rust
// from crates/isograph_parser/src/chunk.rs
        Err(reason) => {
            stream.cursor().report_error(reason);
            let extra = match stream.remaining_contents() {
                Some(remaining) => remaining,
                None => chunk.item.contents.clone(),
            };
            let location = Span::join(extra.first().location, extra.last().location);
            Slot {
                item: None,
                extra: UnparsedChunkItems(extra).with_span(location).wrap_some(),
            }
            .with_span(location)
        }
```

Before: `UnparsedChunkItems(chunk.item.contents.clone())` always. After: unread remainder. `entrypoint $ $` consumes `entrypoint`, fails at the first `$` without consuming it, extra is `$ $`. When the form fails after consuming every content item (`entrypoint Query.`), remaining is `None`; extra is the whole contents so `item: None, extra: None` is not built.

## Tests

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn a_trailing_comma_after_an_entrypoint_is_extra() {
        let text = "entrypoint Query.foo,";
        let (parse, errors) = parsed(text);
        as_entrypoint(parse.reference());
        let extra = first_slot(parse.reference())
            .extra
            .as_ref()
            .expect("the comma is extra");
        assert_eq!(extra.location, span_of(text, ","));
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Token(Comma))
                .with_span(span_of(text, ","))
                .wrap_vec(),
        );
    }

    #[test]
    fn leftover_dollars_after_entrypoint_are_extra() {
        let text = "entrypoint $ $";
        let (parse, errors) = parsed(text);
        assert!(parsed_item(parse.reference()).is_none());
        let extra = first_slot(parse.reference())
            .extra
            .as_ref()
            .expect("$ $ is extra");
        assert_eq!(extra.location, span_of(text, "$ $"));
        assert!(errors.iter().any(|error| {
            error.item == expected(token(Identifier), Found::Token(Dollar))
        }));
    }
```
