# Leftover semantic tokens

`Slot.extra` and `Singleton.extra_chunks` get `leftover_token`. Grammar consume still names the role. Cut unmatched brackets stay `BracketError` until semantic-tokens.md leftover fill-in walks `tokenize`.

Depends on leftover-in-extra.md. leftover-in-extra puts trailing separators and unread remainder in extra. `extra_chunks` is already in `parse_singleton`. Does not depend on four-trees.md, type-annotation-null.md, or parse-iso-literal-entry.md.

Extracted from semantic-tokens.md leftover fill-in (`leftover_token` and the Content / Integer / String / Error / Bracket facts). Delta: leftover fill-in for extra and extra_chunks walks `Slot.extra` and `Singleton.extra_chunks`, not `tokenize`. `leftover_token` is `pub(crate)` so `chunk.rs` can call it. semantic-tokens.md leftover fill-in remains a walk of `tokenize` for the matcher's cut.

`$` is `Content`. `asdf` is `Content`. A leftover `{ bar }` records `Bracket` on `{` and `}`, `Content` on `bar`. Record only spans not already in `tokens` (a failed chunk may clone contents that the prefix already committed as `Keyword`).

## `leftover_token`

```rust
// from crates/isograph_parser/src/semantic_token.rs
// Only leftover fill-in: extra, extra_chunks, the matcher's cut.
pub(crate) fn leftover_token(kind: SplitToken) -> Option<SemanticToken> {
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

Not on the crate surface.

Facts:

- `leftover_token` on `Identifier`, `At`, `Exclamation`, `Dollar`, `Period`, `Colon`, `Equals`, and `Comma` is `Content`.
- `leftover_token` on `IntegerLiteral` is `Integer`. On `StringLiteral` and `BlockStringLiteral` is `String`. On `Error` is `Error`.
- `leftover_token` on each `BracketToken` is `Bracket`.
- `leftover_token` on `LineBreak` and `EndOfFile` is `None`.

## Record leftover

```rust
// from crates/isograph_parser/src/chunk.rs
fn record_leftover_item(
    tokens: &mut Vec<WithSpan<SemanticToken>>,
    item: &WithSpan<ChunkContentItem>,
) {
    match item.item.reference() {
        ChunkContentItem::NonBracket(token) => {
            record_leftover_span(
                tokens,
                leftover_token(SplitToken::NonBracket(token.0)),
                item.location,
            );
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

fn record_leftover_chunk(tokens: &mut Vec<WithSpan<SemanticToken>>, chunk: &Chunk) {
    for item in chunk.contents.iter() {
        record_leftover_item(tokens, item);
    }
}

fn record_leftover_extra(
    tokens: &mut Vec<WithSpan<SemanticToken>>,
    extra: &Option<WithSpan<UnparsedChunkItems>>,
) {
    if let Some(extra) = extra {
        for item in extra.item.0.iter() {
            record_leftover_item(tokens, item);
        }
    }
}

fn record_leftover_span(
    tokens: &mut Vec<WithSpan<SemanticToken>>,
    role: Option<SemanticToken>,
    span: Span,
) {
    if let Some(role) = role
        && tokens.iter().all(|recorded| recorded.location != span)
    {
        tokens.push(role.with_span(span));
    }
}
```

`record_leftover_chunk` walks `contents`. leftover-in-extra.md folds a singleton trailing separator into extra as `NonBracket` items, so walking extra covers that comma. extra_chunks keep their trailing separator on the chunk; leftover fill-in does not walk it (`LineBreak` is `None` anyway).

`chunk.rs` imports `leftover_token`, `SplitToken`, and `BracketToken`.

## `parse_one_chunk` records extra

After leftover-in-extra.md's `parse_one_chunk` (extra is unread remainder, and on a leftover that is not `Separator(_)` the trailing separator tokens):

```rust
// from crates/isograph_parser/src/chunk.rs
    record_leftover_extra(stream.tokens(), &slot.item.extra);
    slot
```

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    pub(crate) fn tokens(&mut self) -> &mut Vec<WithSpan<SemanticToken>> {
        &mut self.0.tokens
    }
```

`parse_each_chunk` goes through `parse_one_chunk`. Leftover in a list item (`foo bar`, a failed `.` chunk) is extra and is recorded here.

## `parse_singleton` records extra_chunks

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
    let extra_chunks = (level.item.len() > 1).then(|| {
        errors.push(extra_chunks(&level.item.0[1]));
        let rest = NonEmpty {
            head: level.item.0[1].clone(),
            tail: level.item.0[2..].to_vec(),
        };
        let location = Span::join(rest.head.location, rest.last().location);
        ExtraChunks(rest).with_span(location)
    });
    Singleton { item, extra_chunks }
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
    let extra_chunks = (level.item.len() > 1).then(|| {
        errors.push(extra_chunks(&level.item.0[1]));
        let rest = NonEmpty {
            head: level.item.0[1].clone(),
            tail: level.item.0[2..].to_vec(),
        };
        let location = Span::join(rest.head.location, rest.last().location);
        ExtraChunks(rest).with_span(location)
    });
    if let Some(extra) = extra_chunks.as_ref() {
        for chunk in extra.item.0.iter() {
            record_leftover_chunk(tokens, chunk.item.reference());
        }
    }
    Singleton { item, extra_chunks }
```

`entrypoint\nasdf`: chunk 0 is `entrypoint` (fails), chunk 1 is `asdf` in `extra_chunks`, `asdf` is `Content`.

## Tests

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn leftover_after_an_entrypoint_is_content() {
        let text = "entrypoint Query.foo bar";
        let (parse, errors, bracket_errors, comma_errors, tokens) = parsed_with_tokens(text);
        assert!(bracket_errors.is_empty());
        assert_eq!(comma_errors, vec![]);
        let parse = parse.expect("the fixture is not an empty literal");
        as_entrypoint(parse.reference());
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Token(Identifier))
                .with_span(span_of(text, "bar"))
                .wrap_vec(),
        );
        assert_eq!(
            tokens,
            vec![
                SemanticToken::Keyword.with_span(span_of(text, "entrypoint")),
                SemanticToken::Type.with_span(span_of(text, "Query")),
                SemanticToken::Period.with_span(span_of(text, ".")),
                SemanticToken::FieldName.with_span(span_of(text, "foo")),
                SemanticToken::Content.with_span(span_of(text, "bar")),
            ],
        );
    }

    #[test]
    fn leftover_dollars_after_entrypoint_are_content() {
        let text = "entrypoint $ $";
        let (parse, _, _, _, tokens) = parsed_with_tokens(text);
        let parse = parse.expect("the fixture is not an empty literal");
        assert!(parsed_item(parse.reference()).is_none());
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

    #[test]
    fn leftover_at_after_an_entrypoint_is_content() {
        let text = "entrypoint Query.foo @lazy";
        let (_, _, _, _, tokens) = parsed_with_tokens(text);
        assert!(tokens.iter().any(|token| {
            token.item == SemanticToken::Content && token.location == span_of(text, "@")
        }));
        assert!(tokens.iter().any(|token| {
            token.item == SemanticToken::Content && token.location == span_of(text, "lazy")
        }));
    }

    #[test]
    fn a_trailing_comma_after_an_entrypoint_is_content() {
        let text = "entrypoint Query.foo,";
        let (_, _, _, _, tokens) = parsed_with_tokens(text);
        assert!(tokens.iter().any(|token| {
            token.item == SemanticToken::Content && token.location == span_of(text, ",")
        }));
    }
```

`leftover_after_an_entrypoint_is_not_recorded` becomes `leftover_after_an_entrypoint_is_content`.

Exact token vecs that currently stop at the committed prefix gain leftover roles:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn a_failed_prefix_keeps_the_tokens_it_committed() {
        let text = "entrypoint Foo.$ asdf";
        // ...
        assert_eq!(
            tokens,
            vec![
                SemanticToken::Keyword.with_span(span_of(text, "entrypoint")),
                SemanticToken::Type.with_span(span_of(text, "Foo")),
                SemanticToken::Period.with_span(span_of(text, ".")),
                SemanticToken::Content.with_span(span_of(text, "$")),
                SemanticToken::Content.with_span(span_of(text, "asdf")),
            ],
        );
    }

    fn an_unknown_keyword_records_keyword_at_that_identifier() {
        let text = "fieldd Query.foo { bar }";
        // ...
        assert!(tokens.iter().any(|token| {
            token.item == SemanticToken::Keyword && token.location == span_of(text, "fieldd")
        }));
        assert!(tokens.iter().any(|token| {
            token.item == SemanticToken::Content && token.location == span_of(text, "Query")
        }));
        assert!(tokens.iter().any(|token| {
            token.item == SemanticToken::Content && token.location == span_of(text, ".")
        }));
        assert!(tokens.iter().any(|token| {
            token.item == SemanticToken::Content && token.location == span_of(text, "foo")
        }));
        assert!(tokens.iter().any(|token| {
            token.item == SemanticToken::Bracket && token.location == span_of(text, "{")
        }));
        assert!(tokens.iter().any(|token| {
            token.item == SemanticToken::Content && token.location == span_of(text, "bar")
        }));
        assert!(tokens.iter().any(|token| {
            token.item == SemanticToken::Bracket && token.location == span_of(text, "}")
        }));
    }
```

`Type` is not used on leftover. `an_unknown_keyword_records_keyword_at_that_identifier` still records `Keyword` at `fieldd`.

```rust
// from crates/isograph_parser/src/chunk.rs
    fn leftover_after_a_list_item_keeps_the_item() {
        let text = "foo bar";
        // ...
        assert_eq!(
            tokens,
            vec![
                SemanticToken::FieldName.with_span(span_of(text, "foo")),
                SemanticToken::Content.with_span(span_of(text, "bar")),
            ],
        );
    }

    fn a_failed_list_chunk_is_none_and_the_next_chunk_still_parses() {
        let text = ".\nfoo";
        // ...
        assert_eq!(
            tokens,
            vec![
                SemanticToken::Content.with_span(span_of(text, ".")),
                SemanticToken::FieldName.with_span(span_of(text, "foo")),
            ],
        );
    }
```
