# report-error: the error sink lives on the cursor

`push_error: impl FnMut(WithSpan<ParseError>)` is a parameter of `parse_iso_literal`, `parse_one_chunk`, `parse_singleton`, and (in the feature docs) every grammar function that parses a nested list. Grammar functions do not report; they return `Result`. The helpers report. The callback is threaded so a nested `parse_each_chunk` / `parse_singleton` can report.

This doc puts `errors` on `ItemCursor` next to `text` and `tokens`. Reporting is `self.report_error`. Grammar functions already take `&mut ItemCursor`; that does not change. Nested lists take that same cursor. `report_error` is `&mut self`. That is expected: it writes the error vec. `text` and `token_text` stay `&self`.

Inner `parse_*` stays `Result`. The first `Err` of a form is returned. `parse_one_chunk` reports it. Grammar functions never call `report_error`.

## Change 1: `errors` on `ItemCursor`

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
pub(crate) struct ItemCursor<'a> {
    items: SafePeekable<nonempty::Iter<'a, WithSpan<ChunkContentItem>>>,
    previous_end: u32,
    text: &'a str,
    tokens: &'a mut Vec<WithSpan<SemanticToken>>,
}

pub(crate) struct ChunkStream<'a>(ItemCursor<'a>);

impl<'a> ChunkStream<'a> {
    pub(crate) fn new(
        contents: &'a NonEmpty<WithSpan<ChunkContentItem>>,
        text: &'a str,
        tokens: &'a mut Vec<WithSpan<SemanticToken>>,
    ) -> Self {
        ChunkStream(ItemCursor {
            previous_end: contents.first().location.start,
            items: contents.iter().safe_peekable(),
            text,
            tokens,
        })
    }
}
```

```rust
// from crates/isograph_parser/src/chunk.rs
    pub(crate) fn stream<'a>(
        &'a self,
        text: &'a str,
        tokens: &'a mut Vec<WithSpan<SemanticToken>>,
    ) -> ChunkStream<'a> {
        ChunkStream::new(self.contents.reference(), text, tokens)
    }
```

After:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
pub(crate) struct ItemCursor<'a> {
    items: SafePeekable<nonempty::Iter<'a, WithSpan<ChunkContentItem>>>,
    previous_end: u32,
    text: &'a str,
    tokens: &'a mut Vec<WithSpan<SemanticToken>>,
    errors: &'a mut Vec<WithSpan<ParseError>>,
}

pub(crate) struct ChunkStream<'a>(ItemCursor<'a>);

impl<'a> ChunkStream<'a> {
    pub(crate) fn new(
        contents: &'a NonEmpty<WithSpan<ChunkContentItem>>,
        text: &'a str,
        tokens: &'a mut Vec<WithSpan<SemanticToken>>,
        errors: &'a mut Vec<WithSpan<ParseError>>,
    ) -> Self {
        ChunkStream(ItemCursor {
            previous_end: contents.first().location.start,
            items: contents.iter().safe_peekable(),
            text,
            tokens,
            errors,
        })
    }
}

impl<'a> ItemCursor<'a> {
    pub(crate) fn report_error(&mut self, error: WithSpan<ParseError>) {
        self.errors.push(error);
    }

    pub(crate) fn stream_chunk<'c>(&'c mut self, chunk: &'c Chunk) -> ChunkStream<'c> {
        chunk.stream(self.text, self.tokens, self.errors)
    }

    pub(crate) fn text(&self) -> &'a str {
        self.text
    }

    pub(crate) fn token_text(&self, span: Span) -> &'a str {
        &self.text[span.as_usize_range()]
    }

    fn record(&mut self, token: SemanticToken, span: Span) {
        self.tokens.push(token.with_span(span));
    }
}
```

```rust
// from crates/isograph_parser/src/chunk.rs
    pub(crate) fn stream<'a>(
        &'a self,
        text: &'a str,
        tokens: &'a mut Vec<WithSpan<SemanticToken>>,
        errors: &'a mut Vec<WithSpan<ParseError>>,
    ) -> ChunkStream<'a> {
        ChunkStream::new(self.contents.reference(), text, tokens, errors)
    }
```

Fields stay private. `stream_chunk` is the split-borrow onto a child stream: it lives on `ItemCursor`, so `chunk.rs` never names the fields. `text()` stays for call sites that only read the source.

`CursorPeek` still holds `tokens: &'c mut Vec<WithSpan<SemanticToken>>`, taken from `self.tokens` in `peek`. It does not report.

`parse_*` is still `fn parse_entrypoint(cursor: &mut ItemCursor<'_>)`. No `F`.

## Change 2: helpers drop `F`; nested lists take the parent cursor

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
fn parse_chunk<'a, P>(
    chunk: &'a WithSpan<Chunk>,
    text: &'a str,
    tokens: &'a mut Vec<WithSpan<SemanticToken>>,
    parse_item: impl FnOnce(&mut ItemCursor<'a>) -> Result<P, WithSpan<ParseError>>,
) -> (ChunkStream<'a>, Result<WithSpan<P>, WithSpan<ParseError>>) {
    let mut stream = chunk.item.stream(text, tokens);
    let result = stream.cursor().spanning(parse_item);
    (stream, result)
}

fn parse_one_chunk<'a, P, F>(
    chunk: &'a WithSpan<Chunk>,
    text: &'a str,
    tokens: &'a mut Vec<WithSpan<SemanticToken>>,
    leftover: Expectation,
    parse: impl FnOnce(&mut ItemCursor<'a>, &mut F) -> Result<P, WithSpan<ParseError>>,
    push_error: &mut F,
) -> WithSpan<Slot<P, UnparsedChunkItems>>
where
    F: FnMut(WithSpan<ParseError>),
{
    let (mut stream, result) = parse_chunk(chunk, text, tokens, |cursor| parse(cursor, push_error));
    match result {
        Ok(item) => match stream.remaining_contents() {
            None => {
                let location = item.location;
                Slot {
                    item: item.wrap_some(),
                    extra_tokens: None,
                }
                .with_span(location)
            }
            Some(remaining) => {
                push_error(
                    ParseError::expected(leftover, Found::from(remaining.first().item.reference()))
                        .with_span(remaining.first().location),
                );
                let leftover_span =
                    Span::join(remaining.first().location, remaining.last().location);
                let location = Span::join(item.location, leftover_span);
                Slot {
                    item: item.wrap_some(),
                    extra_tokens: UnparsedChunkItems(remaining)
                        .with_span(leftover_span)
                        .wrap_some(),
                }
                .with_span(location)
            }
        },
        Err(reason) => {
            push_error(reason);
            let location = chunk.item.contents_span();
            Slot {
                item: None,
                extra_tokens: UnparsedChunkItems(chunk.item.contents.clone())
                    .with_span(location)
                    .wrap_some(),
            }
            .with_span(location)
        }
    }
}

pub(crate) fn parse_singleton<'a, T, F>(
    level: &'a WithSpan<ChunkedLevel>,
    text: &'a str,
    tokens: &'a mut Vec<WithSpan<SemanticToken>>,
    end: Expectation,
    extra_chunks: impl FnOnce(&'a WithSpan<Chunk>) -> WithSpan<ParseError>,
    parse: impl FnOnce(&mut ItemCursor<'a>, &mut F) -> Result<T, WithSpan<ParseError>>,
    push_error: &mut F,
) -> Singleton<Slot<T, UnparsedChunkItems>, ExtraChunks>
where
    F: FnMut(WithSpan<ParseError>),
{
    let item = parse_one_chunk(&level.item.0[0], text, tokens, end, parse, push_error);
    if let Some(comma) = level.item.0[0].item.boundary_comma() {
        push_error(
            ParseError::expected(end, Found::Token(NonBracketTokenKind::Comma)).with_span(comma),
        );
    }
    let extra_chunks = (level.item.len() > 1).then(|| {
        push_error(extra_chunks(&level.item.0[1]));
        let rest = NonEmpty {
            head: level.item.0[1].clone(),
            tail: level.item.0[2..].to_vec(),
        };
        let location = Span::join(rest.head.location, rest.last().location);
        ExtraChunks(rest).with_span(location)
    });
    Singleton { item, extra_chunks }
}
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
fn parse_one_chunk<'a, P>(
    chunk: &'a WithSpan<Chunk>,
    mut stream: ChunkStream<'a>,
    leftover: Expectation,
    parse: impl FnOnce(&mut ItemCursor<'_>) -> Result<P, WithSpan<ParseError>>,
) -> WithSpan<Slot<P, UnparsedChunkItems>> {
    let result = stream.cursor().spanning(parse);
    match result {
        Ok(item) => match stream.remaining_contents() {
            None => {
                let location = item.location;
                Slot {
                    item: item.wrap_some(),
                    extra_tokens: None,
                }
                .with_span(location)
            }
            Some(remaining) => {
                stream.cursor().report_error(
                    ParseError::expected(leftover, Found::from(remaining.first().item.reference()))
                        .with_span(remaining.first().location),
                );
                let leftover_span =
                    Span::join(remaining.first().location, remaining.last().location);
                let location = Span::join(item.location, leftover_span);
                Slot {
                    item: item.wrap_some(),
                    extra_tokens: UnparsedChunkItems(remaining)
                        .with_span(leftover_span)
                        .wrap_some(),
                }
                .with_span(location)
            }
        },
        Err(reason) => {
            stream.cursor().report_error(reason);
            let location = chunk.item.contents_span();
            Slot {
                item: None,
                extra_tokens: UnparsedChunkItems(chunk.item.contents.clone())
                    .with_span(location)
                    .wrap_some(),
            }
            .with_span(location)
        }
    }
}

pub(crate) fn parse_singleton<'a, T>(
    level: &'a WithSpan<ChunkedLevel>,
    text: &'a str,
    tokens: &'a mut Vec<WithSpan<SemanticToken>>,
    errors: &'a mut Vec<WithSpan<ParseError>>,
    end: Expectation,
    extra_chunks: impl FnOnce(&'a WithSpan<Chunk>) -> WithSpan<ParseError>,
    parse: impl FnOnce(&mut ItemCursor<'_>) -> Result<T, WithSpan<ParseError>>,
) -> Singleton<Slot<T, UnparsedChunkItems>, ExtraChunks> {
    let item = parse_one_chunk(
        &level.item.0[0],
        level.item.0[0].item.stream(text, tokens, errors),
        end,
        parse,
    );
    if let Some(comma) = level.item.0[0].item.boundary_comma() {
        errors.push(
            ParseError::expected(end, Found::Token(NonBracketTokenKind::Comma)).with_span(comma),
        );
    }
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
}
```

The root has no cursor yet. `parse_iso_literal` holds `text`, `tokens`, and `errors` and passes them into `parse_singleton`, which builds the first stream with `Chunk::stream`. A nested list builds the child stream with `parent.stream_chunk`. `parse_one_chunk` takes that stream. Leftover and failed-form diagnostics go through the child (`stream.cursor().report_error`). After the stream drops, `parse_singleton` pushes the boundary comma and extra chunks onto `errors` directly.

Who reports:

- `parse_iso_literal`: empty literal, `errors.push`
- `parse_one_chunk`: leftover after `Ok`, failed form, `cursor.report_error`
- `parse_singleton`: boundary comma, extra chunks, `errors.push`

`parse_each_chunk` (parse-arguments.md) takes the parent cursor. Each child stream is `parent.stream_chunk`. That function is not landed by this doc. The listing is the shape parse-arguments.md lands:

```rust
// from crates/isograph_parser/src/chunk.rs
impl ChunkedLevel {
    pub(crate) fn parse_each_chunk<'a, P>(
        &'a self,
        parent: &mut ItemCursor<'_>,
        leftover: Expectation,
        parse_item: impl Fn(&mut ItemCursor<'_>) -> Result<P, WithSpan<ParseError>>,
    ) -> Vec<WithSpan<Slot<P, UnparsedChunkItems>>> {
        self.0
            .iter()
            .map(|chunk| {
                parse_one_chunk(
                    chunk,
                    parent.stream_chunk(&chunk.item),
                    leftover,
                    &parse_item,
                )
            })
            .collect()
    }
}
```

A nested list from a grammar function is the cursor they already hold. That call is `&mut self` on the parent cursor. Expected: the child parse writes the same token vec and error vec.

```rust
    let group = cursor.consume_group_if(BracketKind::Brace, SemanticToken::Brace)?;
    group.item.children.item.parse_each_chunk(
        cursor,
        Expectation::Separator(BracketKind::Brace),
        parse_item,
    )
```

```rust
    let group = cursor
        .require_group(BracketKind::Brace, SemanticToken::Brace)
        .map_err(|()| cursor.expected(expectation))?;
    group.item.children.item.parse_each_chunk(
        cursor,
        Expectation::Separator(BracketKind::Brace),
        parse_item,
    )
```

A nested `parse_singleton` (`[...]` in parse-variables.md) takes the parent cursor the same way: `stream_chunk` for chunk 0, then `parent.report_error` for the boundary comma and extra chunks after that stream drops.

The nest site today is `cursor.text()` (`&self`) plus a separate `push_error`. After this it is `parse_each_chunk(cursor, ...)` (`&mut ItemCursor`). The grammar function already holds `&mut ItemCursor`; the nested list now uses that mutable borrow instead of a shared one. `ChunkedLevel::parse_each_chunk` stays `&self` on the level. The level is not the sink.

## Change 3: `parse_iso_literal` takes the error vec

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    mut push_error: impl FnMut(WithSpan<ParseError>),
    tokens: &mut Vec<WithSpan<SemanticToken>>,
) -> Option<WithSpan<IsoLiteralParse>> {
    let location = root.location;
    if root.item.len() == 0 {
        push_error(ParseError::EmptyLiteral.with_span(location));
        return None;
    }
    let singleton = parse_singleton(
        root.reference(),
        text,
        tokens,
        Expectation::EndOfDeclaration,
        |extra| ParseError::MultipleDeclarations.with_span(extra.location),
        |cursor, _| parse_iso_literal_item(cursor),
        &mut push_error,
    );
    singleton.with_span(location).wrap_some()
}
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    errors: &mut Vec<WithSpan<ParseError>>,
    tokens: &mut Vec<WithSpan<SemanticToken>>,
) -> Option<WithSpan<IsoLiteralParse>> {
    let location = root.location;
    if root.item.len() == 0 {
        errors.push(ParseError::EmptyLiteral.with_span(location));
        return None;
    }
    let singleton = parse_singleton(
        root.reference(),
        text,
        tokens,
        errors,
        Expectation::EndOfDeclaration,
        |extra| ParseError::MultipleDeclarations.with_span(extra.location),
        parse_iso_literal_item,
    );
    singleton.with_span(location).wrap_some()
}
```

The entry point takes `&mut Vec<WithSpan<ParseError>>`, same shape as `tokens`. Artifact generation is `errors.is_empty()` (and the earlier-stage lists empty). Tests read that vec.

`parse_iso_literal_item` is passed directly. It does not take a sink. `parse_entrypoint` is unchanged.

## Change 4: tests

`parsed_with_tokens` passes the vec:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        let mut errors = Vec::new();
        let mut tokens = Vec::new();
        let parse = parse_iso_literal(text, tree, &mut errors, &mut tokens);
```

`stream_of` takes the error vec. Every `chunk_stream.rs` test that builds a stream passes `&mut errors`. Consume tests do not assert on it.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    fn stream_of<'a>(
        tree: &'a WithSpan<ChunkedLevel>,
        text: &'a str,
        tokens: &'a mut Vec<WithSpan<SemanticToken>>,
        errors: &'a mut Vec<WithSpan<ParseError>>,
    ) -> ChunkStream<'a> {
        first_chunk(tree).stream(text, tokens, errors)
    }
```

Added:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    #[test]
    fn report_error_appends_to_the_vec() {
        let text = "foo";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let error = ParseError::EmptyLiteral.with_span(span_of(text, "foo"));
        stream.cursor().report_error(error);
        assert_eq!(errors, error.wrap_vec());
        assert_eq!(tokens, vec![]);
    }
```

The existing leftover / empty-literal / extra-chunk / boundary-comma tests in `parse_iso_literal.rs` already assert the vec contents. They cover the helper call sites. `report_error` does not record a semantic token.

parse-arguments.md tests feed a list interior and have no parent parse. They construct a stream from the first chunk of the subject when it has one and pass that cursor into `parse_each_chunk`. An empty interior never split-borrows the parent; the test helper still passes a cursor whose `text` / `tokens` / `errors` are the fixture's. That helper lives with those tests.

## Change 5: parsing-standards.md and parsing-plan.md

`parse_iso_literal`'s listing takes `errors: &mut Vec<WithSpan<ParseError>>` instead of `push_error: impl FnMut(...)`.

`ItemCursor` / `ChunkStream` listings gain `errors`, `report_error`, and `stream_chunk`. `ChunkStream::new` and `Chunk::stream` take `errors`.

The `parse_one_chunk` / `parse_each_chunk` / `parse_singleton` listings are Change 2. `parse_*` does not take a sink. A nested list is `parse_each_chunk(cursor, ...)`.

Prose that currently says diagnostics go through `push_error` says they go through `report_error` (on a cursor) or `errors.push` (at the root, where there is no cursor). Artifact generation is `errors.is_empty()`. Tests read the vec `parse_iso_literal` was passed.

Function shapes:

- `parse_*`: parameter is `&mut ItemCursor`. First `Err` is returned. Shared iteration is `parse_each_chunk`, `parse_singleton`, or `spanning`. A nested list takes the cursor.
- Diagnostic: `report_error` on the child cursor in `parse_one_chunk`; `errors.push` in `parse_singleton` and `parse_iso_literal`. Not stored on the tree.

Catalog: the `push_error` rows become `report_error` / `errors.push`. `ItemCursor::report_error` and `ItemCursor::stream_chunk` are catalog entries.

`parse_value` in the standards (and every grammar function in the feature docs, when next opened) drops `F` and `push_error`. Object / array / argument-list / selection-set / variable-list interiors pass `cursor`.

semantic-tokens.md's `TTokens` lands on `ItemCursor.tokens` (`&'a mut TTokens`) as today. Grammar functions stay free of a sink parameter.

## Landing checklist

1. `errors` on `ItemCursor`, `report_error`, `stream_chunk`, `Chunk::stream` / `ChunkStream::new` taking `errors`, `parse_one_chunk` taking a `ChunkStream`, `parse_singleton` without `F`, `parse_iso_literal` taking the error vec, the test helper and `stream_of` updates, the `report_error_appends_to_the_vec` test. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. parsing-standards.md and parsing-plan.md match Change 5.
3. Move this doc to refactors/past.
