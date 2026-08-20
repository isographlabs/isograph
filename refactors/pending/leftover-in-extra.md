# Leftover in extra, and highlighted

Trailing separators of a singleton chunk, unread remainder of a failed chunk, and extra root chunks are in the tree (`Slot.extra` / `Singleton.extra_chunks`) and get leftover semantic tokens.

Does not depend on four-trees.md, type-annotation-null.md, or parse-iso-literal-entry.md. Uses `leftover_token` from semantic-tokens.md; that doc's leftover fill-in for extra and extra_chunks is this work. Cut unmatched brackets stay `BracketError` until leftover fill-in walks `tokenize`.

## `leftover_token`

```rust
// from crates/isograph_parser/src/semantic_token.rs
fn leftover_token(kind: SplitToken) -> Option<SemanticToken> {
    match kind {
        SplitToken::NonBracket(NonBracketTokenKind::IntegerLiteral) => {
            SemanticToken::Integer.wrap_some()
        }
        SplitToken::NonBracket(
            NonBracketTokenKind::StringLiteral | NonBracketTokenKind::BlockStringLiteral,
        ) => SemanticToken::String.wrap_some(),
        SplitToken::NonBracket(NonBracketTokenKind::Error) => SemanticToken::Error.wrap_some(),
        SplitToken::NonBracket(NonBracketTokenKind::LineBreak | NonBracketTokenKind::EndOfFile) => {
            None
        }
        SplitToken::NonBracket(_) => SemanticToken::Content.wrap_some(),
        SplitToken::Bracket(_) => SemanticToken::Bracket.wrap_some(),
    }
}
```

`$` is `Content`. `asdf` is `Content`. A leftover `{ bar }` records `Bracket` on `{` and `}`, `Content` on `bar`.

Record only spans not already in `tokens` (a failed chunk may clone contents that the prefix already committed as `Keyword`).

```rust
// from crates/isograph_parser/src/chunk.rs
fn record_leftover_item(
    tokens: &mut Vec<WithSpan<SemanticToken>>,
    item: &WithSpan<ChunkContentItem>,
) {
    match item.item.reference() {
        ChunkContentItem::NonBracket(token) => {
            if let Some(role) = leftover_token(SplitToken::from(IsographLangTokenKind::from(token.0)))
                && tokens.iter().all(|recorded| recorded.location != item.location)
            {
                tokens.push(role.with_span(item.location));
            }
        }
        ChunkContentItem::Group(group) => {
            record_leftover_span(
                tokens,
                leftover_token(SplitToken::Bracket(BracketToken::Open(group.opening.item.0))),
                group.opening.location,
            );
            for chunk in group.children.item.0.iter() {
                record_leftover_chunk(tokens, chunk.item.reference());
            }
            record_leftover_span(
                tokens,
                leftover_token(SplitToken::Bracket(BracketToken::Close(group.closing.item.0))),
                group.closing.location,
            );
        }
    }
}
```

`record_leftover_chunk` walks `contents` and, when the trailing separator is in extra, those tokens too. `record_leftover_span` is the already-recorded check plus push.

## Trailing separators go in `Slot.extra`

`entrypoint Query.foo,` parses. The comma is a `ParseError` and is not a content item; it sits on `Chunk.trailing_separator`. Resolve hits the slot unmatched span.

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

Then record leftover on that extra.

## Extra chunks highlighted

`parse_singleton` already puts chunks after the first in `extra_chunks`. After that `Some`, walk each extra chunk's contents (and groups) with `record_leftover_item`. `entrypoint\nasdf`: chunk 0 is `entrypoint` (fails), chunk 1 is `asdf` in `extra_chunks`, `asdf` is `Content`.

## Tests

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn a_trailing_comma_after_an_entrypoint_is_extra() {
        let text = "entrypoint Query.foo,";
        let (parse, errors, _, _, tokens) = parsed_with_tokens(text);
        let parse = parse.expect("the fixture is not an empty literal");
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
        assert!(tokens.iter().any(|token| {
            token.item == SemanticToken::Content && token.location == span_of(text, ",")
        }));
    }

    #[test]
    fn leftover_dollars_after_entrypoint_are_extra_and_content() {
        let text = "entrypoint $ $";
        let (parse, errors, _, _, tokens) = parsed_with_tokens(text);
        let parse = parse.expect("the fixture is not an empty literal");
        assert!(parsed_item(parse.reference()).is_none());
        let extra = first_slot(parse.reference())
            .extra
            .as_ref()
            .expect("$ $ is extra");
        assert_eq!(extra.location, span_of(text, "$ $"));
        assert!(errors.iter().any(|error| {
            error.item == expected(token(Identifier), Found::Token(Dollar))
        }));
        let dollars: Vec<_> = tokens
            .iter()
            .filter(|token| token.item == SemanticToken::Content)
            .collect();
        assert_eq!(dollars.len(), 2);
        assert!(tokens.iter().any(|token| {
            token.item == SemanticToken::Keyword
                && token.location == span_of(text, "entrypoint")
        }));
    }

    #[test]
    fn an_extra_chunk_is_recorded_as_content() {
        let text = "entrypoint\nasdf";
        let (parse, _, _, _, tokens) = parsed_with_tokens(text);
        let parse = parse.expect("the fixture is not an empty literal");
        assert!(parse.item.extra_chunks.as_ref().is_some());
        assert!(tokens.iter().any(|token| {
            token.item == SemanticToken::Content && token.location == span_of(text, "asdf")
        }));
    }
```

`leftover_after_an_entrypoint_is_not_recorded` (`entrypoint Query.foo bar`) becomes recorded: four consumed tokens plus `Content` at `bar`. Rename to `leftover_after_an_entrypoint_is_content`. `an_unknown_keyword_records_keyword_at_that_identifier` (`fieldd Query.foo { bar }`) still records `Keyword` at `fieldd`; remainder is extra and gets leftover roles (`Type` is not used; `Query` is `Content`, `{` `}` `Bracket`, `foo` `bar` `Content`).
