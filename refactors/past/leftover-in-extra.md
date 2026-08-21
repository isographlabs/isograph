# Leftover in extra

Trailing separators of a singleton chunk, and unread remainder of a failed chunk, live in `Slot.extra`.

This change does not depend on four-trees.md, type-annotation-null.md, parse-iso-literal-entry.md, or leftover-semantic-tokens.md. Highlighting leftover is leftover-semantic-tokens.md.

## What the user sees

`entrypoint Query.foo,` parses as an entrypoint. The comma is an `AstError` (`Expected the end of the declaration, found comma (',').`). After this change the comma is also in `Slot.extra`, so resolve at the comma hits the leftover token.

`entrypoint $ $` does not parse an item. Extra is `$ $`.

`entrypoint Query.foo bar,` extra is `bar` then the comma.

## Trailing separators go in `Slot.extra`

A comma or line break that ends a singleton chunk is not a content item. It sits on `Chunk.trailing_separator`. Resolve on that separator currently hits the slot unmatched span, because extra does not contain it.

After a successful `parse_one_chunk`, when leftover is not `Expectation::Separator(_)` and `trailing_separator` is `Some`, this change appends those separator tokens to extra as `ChunkContentItem::NonBracket`. Root leftover is `EndOfDeclaration`. A `[...]` type leftover is `EndOfType`. List `parse_each_chunk` leftover is `Separator(_)`; a list trailing comma stays on the chunk and is not copied into extra.

`parse_singleton` still pushes the `boundary_comma()` error. That error does not move.

```rust
// from crates/isograph_parser/src/chunk.rs
fn separator_as_content(token: WithSpan<SeparatorToken>) -> WithSpan<ChunkContentItem> {
    let kind = match token.item {
        SeparatorToken::Comma => NonBracketTokenKind::Comma,
        SeparatorToken::LineBreak => NonBracketTokenKind::LineBreak,
    };
    ChunkContentItem::NonBracket(NonBracketToken(kind)).with_span(token.location)
}

fn extra_plus_trailing_separator(
    extra: Option<WithSpan<UnparsedChunkItems>>,
    trailing_separator: Option<&WithSpan<ChunkSeparator>>,
) -> Option<WithSpan<UnparsedChunkItems>> {
    let Some(separator) = trailing_separator else {
        return extra;
    };
    let added = separator.item.0.iter().copied().map(separator_as_content);
    match extra {
        Some(extra) => {
            let mut items = extra.item.0;
            items.extend(added);
            let location = Span::join(extra.location, separator.location);
            UnparsedChunkItems(items).with_span(location).wrap_some()
        }
        None => {
            let NonEmpty { head, tail } = separator.item.0.clone();
            let items = NonEmpty {
                head: separator_as_content(head),
                tail: tail.into_iter().map(separator_as_content).collect(),
            };
            UnparsedChunkItems(items)
                .with_span(separator.location)
                .wrap_some()
        }
    }
}
```

`WithSpan<SeparatorToken>` is `Copy`. `NonEmpty` is not; the `None` extra arm clones the separator's tokens.

Before, `parse_one_chunk` on success:

```rust
// from crates/isograph_parser/src/chunk.rs
        Ok(item) => match stream.remaining_contents() {
            None => {
                let location = item.location;
                Slot {
                    item: item.wrap_some(),
                    extra: None,
                }
                .with_span(location)
            }
            Some(remaining) => {
                stream.cursor().report_error(
                    AstError::expected(leftover, Found::from(remaining.first().item.reference()))
                        .with_span(remaining.first().location),
                );
                let leftover_span =
                    Span::join(remaining.first().location, remaining.last().location);
                let location = Span::join(item.location, leftover_span);
                Slot {
                    item: item.wrap_some(),
                    extra: UnparsedChunkItems(remaining)
                        .with_span(leftover_span)
                        .wrap_some(),
                }
                .with_span(location)
            }
        },
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
        Ok(item) => {
            let extra = match stream.remaining_contents() {
                None => None,
                Some(remaining) => {
                    stream.cursor().report_error(
                        AstError::expected(
                            leftover,
                            Found::from(remaining.first().item.reference()),
                        )
                        .with_span(remaining.first().location),
                    );
                    let leftover_span =
                        Span::join(remaining.first().location, remaining.last().location);
                    UnparsedChunkItems(remaining)
                        .with_span(leftover_span)
                        .wrap_some()
                }
            };
            let extra = match leftover {
                Expectation::Separator(_) => extra,
                _ => extra_plus_trailing_separator(
                    extra,
                    chunk.item.trailing_separator.as_ref(),
                ),
            };
            let location = match extra.as_ref() {
                None => item.location,
                Some(extra) => Span::join(item.location, extra.location),
            };
            Slot {
                item: item.wrap_some(),
                extra,
            }
            .with_span(location)
        }
```

`chunk.item.trailing_separator` is the private field on `Chunk`. `parse_one_chunk` is in the same module.

## Failed remainder is extra, not the whole chunk

Before, a failed form always clones the whole chunk into extra:

```rust
// from crates/isograph_parser/src/chunk.rs
        Err(reason) => {
            stream.cursor().report_error(reason);
            let location = chunk.item.contents_span();
            Slot {
                item: None,
                extra: UnparsedChunkItems(chunk.item.contents.clone())
                    .with_span(location)
                    .wrap_some(),
            }
            .with_span(location)
        }
```

After, extra is the unread remainder. When the form fails after consuming every content item, remaining is `None`; extra is the whole contents, so `item: None, extra: None` is not built.

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

`entrypoint $ $` consumes `entrypoint`, fails at the first `$` without consuming it, extra is `$ $`. `entrypoint Foo.$ asdf` consumes `entrypoint` / `Foo` / `.`, fails at `$` without consuming it, extra is `$ asdf`. `entrypoint Query.` consumes every content item and then fails; remaining is `None`; extra is the whole contents.

The failed arm does not fold `trailing_separator` into extra.

## Tests

This change renames `a_failed_form_keeps_the_whole_chunk_as_remaining` to `a_failed_form_puts_unread_remainder_in_extra`. Extra is `$ asdf`, not the consumed prefix.

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

    #[test]
    fn leftover_then_a_trailing_comma_are_both_extra() {
        let text = "entrypoint Query.foo bar,";
        let (parse, errors) = parsed(text);
        as_entrypoint(parse.reference());
        let extra = first_slot(parse.reference())
            .extra
            .as_ref()
            .expect("bar and the comma are extra");
        assert_eq!(extra.location, span_of(text, "bar,"));
        assert!(errors.iter().any(|error| {
            error.item == expected(EndOfDeclaration, Found::Token(Identifier))
                && error.location == span_of(text, "bar")
        }));
        assert!(errors.iter().any(|error| {
            error.item == expected(EndOfDeclaration, Found::Token(Comma))
                && error.location == span_of(text, ",")
        }));
    }

    #[test]
    fn a_failed_form_that_consumed_every_item_still_has_extra() {
        let text = "entrypoint Query.";
        let (parse, _) = parsed(text);
        assert!(parsed_item(parse.reference()).is_none());
        let extra = first_slot(parse.reference())
            .extra
            .as_ref()
            .expect("the whole contents are extra");
        assert_eq!(extra.location, span_of(text, "entrypoint Query."));
    }

    #[test]
    fn a_failed_form_puts_unread_remainder_in_extra() {
        let text = "entrypoint Foo.$ asdf";
        let (parse, errors) = parsed(text);
        assert!(parsed_item(parse.reference()).is_none());
        let extra = first_slot(parse.reference())
            .extra
            .as_ref()
            .expect("$ asdf is extra");
        assert_eq!(extra.location, span_of(text, "$ asdf"));
        assert!(errors.iter().any(|error| {
            error.item == expected(token(Identifier), Found::Token(Dollar))
                && error.location == span_of(text, "$")
        }));
    }

    #[test]
    fn a_trailing_comma_after_an_entrypoint_resolves_to_the_comma() {
        let text = "entrypoint Query.foo,";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, ",")) {
            IsographResolutionNode::NonBracketToken(_) => {}
            node => panic!("expected the leftover comma, got {node:?}"),
        }
    }
```

`a_list_trailing_comma_is_not_a_parse_each_chunk_diagnostic` in `chunk.rs` still has `items[0].item.extra.is_none()`. Leftover `Separator(_)` does not fold the comma into extra.
