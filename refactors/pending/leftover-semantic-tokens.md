# Leftover semantic tokens

Leftover that the grammar did not consume is highlighted by walking the tree fields that hold it. `Slot.extra` holds unread remainder of a chunk and, after leftover-in-extra.md, a singleton's trailing separator. `Singleton.extra_chunks` holds whole extra chunks after the first. After the grammar parse of a chunk returns, this change records a semantic token for each leftover token in those fields.

When the grammar consumes a token, it records the role the call site names (`Keyword`, `Type`, `FieldName`, and the rest). Leftover uses `leftover_token`, which maps a token kind to `Content`, `Integer`, `String`, `Error`, or `Bracket`.

Unmatched brackets that the matcher cut never enter a chunk. They remain `BracketError` on the pipeline error list. They get a semantic token when semantic-tokens.md leftover fill-in walks `tokenize`.

## What the user sees

`entrypoint Query.foo bar` highlights `entrypoint` as Keyword, `Query` as Type, `.` as Period, `foo` as FieldName, and `bar` as Content.

`entrypoint $ $` highlights `entrypoint` as Keyword and each `$` as Content.

`entrypoint Query.foo,` highlights the comma as Content.

`entrypoint\nasdf` highlights `asdf` as Content.

`fieldd Query.foo { bar }` highlights `fieldd` as Keyword, `Query` / `.` / `foo` / `bar` as Content, and `{` `}` as Bracket.

`$` is Content. `asdf` is Content. A leftover `{ bar }` records Bracket on `{` and `}`, Content on `bar`. Leftover never records `Type`.

## Dependencies

This change depends on leftover-in-extra.md, which puts trailing separators and unread remainder in `Slot.extra`. Tests call `parse_iso_literal` (parse-iso-literal-entry.md). `parse_singleton` already builds `extra_chunks`. This change does not depend on four-trees.md or type-annotation-null.md.

## Extraction

Extracted from semantic-tokens.md leftover fill-in: `leftover_token` and the facts that leftover identifiers and punctuation are `Content`, leftover integers are `Integer`, leftover strings are `String`, leftover error tokens are `Error`, leftover brackets are `Bracket`, and line breaks and EOF record nothing.

The delta from that extraction is that leftover fill-in for extra and extra_chunks walks `Slot.extra` and `Singleton.extra_chunks`, not `tokenize`. `leftover_token` is `pub(crate)` so `chunk.rs` can call it. The leftover fill-in that remains in semantic-tokens.md is still a walk of `tokenize`, covering the matcher's cut.

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

The function is `pub(crate)`.

```rust
// from crates/isograph_parser/src/lib.rs
pub(crate) use semantic_token::leftover_token;
```

Facts:

- `leftover_token` on `Identifier`, `At`, `Exclamation`, `Dollar`, `Period`, `Colon`, `Equals`, and `Comma` is `Content`.
- `leftover_token` on `IntegerLiteral` is `Integer`. On `StringLiteral` and `BlockStringLiteral` is `String`. On `Error` is `Error`.
- `leftover_token` on each `BracketToken` is `Bracket`.
- `leftover_token` on `LineBreak` and `EndOfFile` is `None`.

## Record leftover

The walkers push a leftover role for each span that is not already in `tokens`. A failed chunk may clone contents that the prefix already committed. `entrypoint Query.` consumes every content item, then fails; leftover-in-extra.md puts the whole contents in extra, including `entrypoint` / `Query` / `.` which the grammar already recorded as `Keyword` / `Type` / `Period`. `record_leftover_span` skips a span that is already in `tokens`.

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

`record_leftover_chunk` walks `chunk.contents`. leftover-in-extra.md folds a singleton trailing separator into extra as `ChunkContentItem::NonBracket` items, so walking extra records that comma. Extra chunks keep their trailing separator on the chunk; `record_leftover_chunk` walks those chunks' `contents`. A trailing line break maps to `None` from `leftover_token`.

`chunk.rs` adds `leftover_token`, `SplitToken`, and `BracketToken` to its `use crate::{...}` list. `SplitToken` and `BracketToken` are already `pub(crate)` from `lib.rs`.

## `parse_one_chunk` records extra

leftover-in-extra.md's `parse_one_chunk` returns a `Slot`. `extra` is unread remainder, and, when leftover is not `Expectation::Separator(_)`, the trailing separator tokens. That function returns the `Slot` from its `match result`. This change binds the match to `slot`, records leftover on `extra`, and returns `slot`.

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

`parse_singleton` already builds `extra_chunks` from chunks after the first. After that value is built, this change walks each extra chunk and records leftover on its contents.

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

This change renames `leftover_after_an_entrypoint_is_not_recorded` to `leftover_after_an_entrypoint_is_content`. The committed prefix stays `Keyword` / `Type` / `Period` / `FieldName`. `bar` is `Content`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn leftover_after_an_entrypoint_is_content() {
        let text = "entrypoint Query.foo bar";
        let parsed = parse_iso_literal(text);
        let parse = parsed.item.expect("the fixture is not an empty literal");
        as_entrypoint(parse.reference());
        assert_eq!(
            parsed.errors,
            expected(EndOfDeclaration, Found::Token(Identifier))
                .to::<ParseError>()
                .with_span(span_of(text, "bar"))
                .wrap_vec(),
        );
        assert_eq!(
            parsed.tokens,
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
        let parsed = parse_iso_literal(text);
        let parse = parsed.item.expect("the fixture is not an empty literal");
        assert!(parsed_item(parse.reference()).is_none());
        let dollars: Vec<_> = parsed.tokens
            .iter()
            .filter(|token| token.item == SemanticToken::Content)
            .collect();
        assert_eq!(dollars.len(), 2);
        assert!(parsed.tokens.iter().any(|token| {
            token.item == SemanticToken::Keyword
                && token.location == span_of(text, "entrypoint")
        }));
    }

    #[test]
    fn an_extra_chunk_is_recorded_as_content() {
        let text = "entrypoint\nasdf";
        let parsed = parse_iso_literal(text);
        let parse = parsed.item.expect("the fixture is not an empty literal");
        assert!(parse.item.extra_chunks.as_ref().is_some());
        assert!(parsed.tokens.iter().any(|token| {
            token.item == SemanticToken::Content && token.location == span_of(text, "asdf")
        }));
    }

    #[test]
    fn leftover_at_after_an_entrypoint_is_content() {
        let text = "entrypoint Query.foo @lazy";
        let parsed = parse_iso_literal(text);
        assert!(parsed.tokens.iter().any(|token| {
            token.item == SemanticToken::Content && token.location == span_of(text, "@")
        }));
        assert!(parsed.tokens.iter().any(|token| {
            token.item == SemanticToken::Content && token.location == span_of(text, "lazy")
        }));
    }

    #[test]
    fn a_trailing_comma_after_an_entrypoint_is_content() {
        let text = "entrypoint Query.foo,";
        let parsed = parse_iso_literal(text);
        assert!(parsed.tokens.iter().any(|token| {
            token.item == SemanticToken::Content && token.location == span_of(text, ",")
        }));
    }
```

The token vecs on `a_failed_prefix_keeps_the_tokens_it_committed` and the unknown-keyword test gain leftover roles. `entrypoint Foo.$ asdf` keeps `Keyword` / `Type` / `Period` on the consumed prefix and records `Content` on `$` and `asdf`. `fieldd Query.foo { bar }` still records `Keyword` at `fieldd`. The rest of that chunk is leftover: `Query`, `.`, `foo`, and `bar` are `Content`; `{` and `}` are `Bracket`.

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

List leftover is extra on the slot `parse_one_chunk` already built. `foo bar` keeps `FieldName` on `foo` and records `Content` on `bar`. `.\nfoo` records `Content` on the failed `.` and `FieldName` on `foo`.

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
