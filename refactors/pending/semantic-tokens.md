# Semantic tokens

The grammar stage records a semantic token as it consumes each token or bracket. Recording is a side effect of `consume_token_if` / `require_token` / `consume_group_if` / `require_group`. `require_token` takes a token kind, not a legend class. The tree does not mention tokens; they are a sibling of the tree, not a field on it.

Two shippable changes. The first always constructs tokens into a `Vec`. The second makes the collector a type parameter so the parse can be constructed with a noop or a non-noop.

## What a token is

First pass classifies by the kind just consumed. `require_token(NonBracketTokenKind::Identifier)` records `SemanticToken::Identifier`. An identifier that is a keyword, a type name, or a field name is still `Identifier` until the reclassify change.

```rust
// from crates/isograph_parser/src/semantic_token.rs
use span::{Span, WithSpan};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum SemanticToken {
    Identifier,
    Integer,
    String,
    BlockString,
    Period,
    Colon,
    Dollar,
    Equals,
    Exclamation,
    At,
    OpenParenthesis,
    CloseParenthesis,
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
}

impl SemanticToken {
    pub fn from_non_bracket(kind: NonBracketTokenKind) -> Option<SemanticToken> {
        match kind {
            NonBracketTokenKind::Identifier => SemanticToken::Identifier.wrap_some(),
            NonBracketTokenKind::IntegerLiteral => SemanticToken::Integer.wrap_some(),
            NonBracketTokenKind::StringLiteral => SemanticToken::String.wrap_some(),
            NonBracketTokenKind::BlockStringLiteral => SemanticToken::BlockString.wrap_some(),
            NonBracketTokenKind::Period => SemanticToken::Period.wrap_some(),
            NonBracketTokenKind::Colon => SemanticToken::Colon.wrap_some(),
            NonBracketTokenKind::Dollar => SemanticToken::Dollar.wrap_some(),
            NonBracketTokenKind::Equals => SemanticToken::Equals.wrap_some(),
            NonBracketTokenKind::Exclamation => SemanticToken::Exclamation.wrap_some(),
            NonBracketTokenKind::At => SemanticToken::At.wrap_some(),
            NonBracketTokenKind::Comma
            | NonBracketTokenKind::LineBreak
            | NonBracketTokenKind::EndOfFile
            | NonBracketTokenKind::Error
            | NonBracketTokenKind::ErrorUnterminatedString
            | NonBracketTokenKind::ErrorUnsupportedStringCharacter
            | NonBracketTokenKind::ErrorUnterminatedBlockString
            | NonBracketTokenKind::ErrorNumberLiteralLeadingZero
            | NonBracketTokenKind::ErrorNumberLiteralTrailingInvalid
            | NonBracketTokenKind::ErrorFloatLiteralMissingZero => None,
        }
    }

    pub fn for_open(kind: BracketKind) -> SemanticToken {
        match kind {
            BracketKind::Parenthesis => SemanticToken::OpenParenthesis,
            BracketKind::Brace => SemanticToken::OpenBrace,
            BracketKind::Bracket => SemanticToken::OpenBracket,
        }
    }

    pub fn for_close(kind: BracketKind) -> SemanticToken {
        match kind {
            BracketKind::Parenthesis => SemanticToken::CloseParenthesis,
            BracketKind::Brace => SemanticToken::CloseBrace,
            BracketKind::Bracket => SemanticToken::CloseBracket,
        }
    }
}
```

`from_non_bracket` is `None` for kinds the grammar stage does not consume (`Comma` and `LineBreak` live on chunk boundaries; `Error*` and `EndOfFile` are not `require_token` targets). Those spans get tokens only in the leftover fill-in change.

No `line_behavior` / `indent_change` on this type. Upstream puts both on `IsographSemanticToken` and the formatter walks that vec. Formatter metadata is a later change on this same type.

Tokens stay off the tree. Upstream stores `semantic_tokens: Vec<WithEmbeddedLocation<IsographSemanticToken>>` on each declaration (`entrypoint_declaration.rs`, `client_selectable_declaration.rs`). We do not.

## Origin: how isograph records during parse

Extracted from `crates/isograph_lang_parser/src/peekable_lexer.rs`. Every advance takes a legend constant and pushes it.

```rust
// from crates/isograph_lang_parser/src/peekable_lexer.rs (upstream)
pub(crate) struct PeekableLexer<'source> {
    current: WithSpan<IsographLangTokenKind>,
    lexer: logos::Lexer<'source, IsographLangTokenKind>,
    source: &'source str,
    end_index_of_last_parsed_token: u32,
    offset: u32,
    semantic_tokens: Vec<WithEmbeddedLocation<IsographSemanticToken>>,
    pub text_source: TextSource,
}

impl<'source> PeekableLexer<'source> {
    fn parse_token(
        &mut self,
        isograph_semantic_token: IsographSemanticToken,
    ) -> WithEmbeddedLocation<IsographLangTokenKind> {
        let kind = self
            .lexer
            .next()
            .unwrap_or(IsographLangTokenKind::EndOfFile);
        self.end_index_of_last_parsed_token = self.current.span.end;
        let span = self.lexer_span();
        let parsed_token = std::mem::replace(&mut self.current, kind.with_span(span))
            .to_with_embedded_location(self.text_source);
        self.semantic_tokens
            .push(isograph_semantic_token.with_location(parsed_token.location));
        parsed_token
    }

    pub fn parse_token_of_kind(
        &mut self,
        expected_kind: IsographLangTokenKind,
        isograph_semantic_token: IsographSemanticToken,
    ) -> DiagnosticResult<WithEmbeddedLocation<IsographLangTokenKind>> {
        let found = self.peek();
        if found.item == expected_kind {
            self.parse_token(isograph_semantic_token).wrap_ok()
        } else {
            parse_token_kind_diagnostic(expected_kind, found.item, found.location).wrap_err()
        }
    }
}
```

Call sites pass the class:

```rust
// from crates/isograph_lang_parser/src/parse_iso_literal.rs (upstream)
let parent_type = tokens
    .parse_string_key_type(
        IsographLangTokenKind::Identifier,
        semantic_token_legend::ST_SERVER_OBJECT_TYPE,
    )?;
let dot = tokens
    .parse_token_of_kind(IsographLangTokenKind::Period, semantic_token_legend::ST_DOT)?;
```

Delta, common to both changes below:

- `require_token` / `consume_token_if` take only the token kind. The recorded class is `SemanticToken::from_non_bracket(kind)`. The call site does not name a legend entry.
- The constructor does not push a dummy token and pop it. Upstream `PeekableLexer::new` does `parse_token(ST_COMMENT)` then `semantic_tokens.pop()`.
- A failed `require_token` records nothing. A successful consume that a later `?` discards stays recorded. There is no corrective pop.
- The declaration types do not grow a `semantic_tokens` field.

## 1. Always record into a `Vec`

No trait. No type parameter. The cursor holds `&mut Vec<WithSpan<SemanticToken>>`. Every parse constructs tokens.

Grammar functions stay `fn parse_entrypoint(cursor: &mut ItemCursor<'_>)`. They do not mention the vec.

### The cursor

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

impl Chunk {
    pub(crate) fn stream<'a>(
        &'a self,
        text: &'a str,
        tokens: &'a mut Vec<WithSpan<SemanticToken>>,
    ) -> ChunkStream<'a> {
        ChunkStream::new(self.contents.reference(), text, tokens)
    }
}
```

Before (`consume_token_if` / `consume_group_if`):

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    pub(crate) fn consume_token_if(&mut self, kind: NonBracketTokenKind) -> Option<Span> {
        let peek = self.items.peek()?;
        match peek.view().item.reference() {
            ChunkContentItem::NonBracket(token) if token.0 == kind => {}
            _ => return None,
        }
        let item = peek.commit();
        self.previous_end = item.location.end;
        item.location.wrap_some()
    }

    pub(crate) fn consume_group_if(
        &mut self,
        kind: BracketKind,
    ) -> Option<WithSpan<&'a ChunkedGroup>> {
        let peek = self.items.peek()?;
        let item = *peek.view();
        match item.item.reference() {
            ChunkContentItem::Group(group) if group.opening.item.0 == kind => {
                peek.commit();
                self.previous_end = item.location.end;
                group.with_span(item.location).wrap_some()
            }
            _ => None,
        }
    }
```

After:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    pub(crate) fn consume_token_if(&mut self, kind: NonBracketTokenKind) -> Option<Span> {
        let peek = self.items.peek()?;
        match peek.view().item.reference() {
            ChunkContentItem::NonBracket(token) if token.0 == kind => {}
            _ => return None,
        }
        let item = peek.commit();
        self.previous_end = item.location.end;
        if let Some(token) = SemanticToken::from_non_bracket(kind) {
            self.record(token, item.location);
        }
        item.location.wrap_some()
    }

    pub(crate) fn consume_group_if(
        &mut self,
        kind: BracketKind,
    ) -> Option<WithSpan<&'a ChunkedGroup>> {
        let peek = self.items.peek()?;
        let item = *peek.view();
        match item.item.reference() {
            ChunkContentItem::Group(group) if group.opening.item.0 == kind => {
                peek.commit();
                self.previous_end = item.location.end;
                self.record(SemanticToken::for_open(kind), group.opening.location);
                group.with_span(item.location).wrap_some()
            }
            _ => None,
        }
    }

    pub(crate) fn record_group_close(&mut self, group: &ChunkedGroup) {
        self.record(
            SemanticToken::for_close(group.closing.item.0),
            group.closing.location,
        );
    }

    fn record(&mut self, token: SemanticToken, span: Span) {
        self.tokens.push(token.with_span(span));
    }
```

`require_token` / `require_group` stay `consume_*` or `Err(())`. They inherit the side effect. `expected` only peeks and records nothing.

A consume that does not match records nothing. `from_non_bracket` returning `None` also records nothing.

`parse_iso_literal_item` and `parse_entrypoint` are unchanged: they already call `require_token(Identifier)` / `require_token(Period)`.

### Source order for groups

`consume_group_if` records the opening and returns the group. The interior is a new cursor over `group.children`. The closing must be recorded after that interior, or the vec is `open, close, interior...`.

The group-plus-interior pattern in parsing-standards.md becomes one method on the cursor. The method reborrows `self.tokens` for the child cursors, then records the close. The parent cursor is not used for anything else during the reborrow.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
impl<'a> ItemCursor<'a> {
    pub(crate) fn parse_group_items<P, F>(
        &mut self,
        group: &ChunkedGroup,
        parse_item: impl Fn(&mut ItemCursor<'_>, &mut F) -> Result<P, WithSpan<ParseError>>,
        push_error: &mut F,
    ) -> Vec<WithSpan<Slot<P, UnparsedChunkItems>>>
    where
        F: FnMut(WithSpan<ParseError>),
    {
        let text = self.text;
        let items = group
            .children
            .item
            .parse_items(text, self.tokens, parse_item, push_error);
        self.record_group_close(group);
        items
    }

    pub(crate) fn parse_group_singleton<T, F>(
        &mut self,
        group: &ChunkedGroup,
        end: Expectation,
        extra_chunks: impl FnOnce(&WithSpan<Chunk>) -> WithSpan<ParseError>,
        parse: impl FnOnce(&mut ItemCursor<'_>, &mut F) -> Result<T, WithSpan<ParseError>>,
        push_error: &mut F,
    ) -> Singleton<Slot<T, UnparsedChunkItems>, ExtraChunks>
    where
        F: FnMut(WithSpan<ParseError>),
    {
        let text = self.text;
        let parsed = parse_singleton(
            group.children.reference(),
            text,
            self.tokens,
            end,
            extra_chunks,
            parse,
            push_error,
        );
        self.record_group_close(group);
        parsed
    }
}
```

`parse_group_items_with_trailing` is the same wrapper around `parse_items_with_trailing`, and lands with that function (parse-fields.md).

Before (group interior):

```rust
// from crates/isograph_parser/src/selections.rs (parse-fields.md)
    let group = cursor
        .require_group(BracketKind::Brace)
        .map_err(|()| cursor.expected(Expectation::SelectionSet))?;
    SelectionSet(
        group
            .item
            .children
            .item
            .parse_items_with_trailing(cursor.text(), parse_selection)
            .into_iter()
            .map(WithSpan::<SelectionSlot>::from)
            .collect(),
    )
    .with_span(group.location)
    .wrap_ok()
```

After:

```rust
// from crates/isograph_parser/src/selections.rs
    let group = cursor
        .require_group(BracketKind::Brace)
        .map_err(|()| cursor.expected(Expectation::SelectionSet))?;
    SelectionSet(
        cursor
            .parse_group_items_with_trailing(group.item, parse_selection)
            .into_iter()
            .map(WithSpan::<SelectionSlot>::from)
            .collect(),
    )
    .with_span(group.location)
    .wrap_ok()
```

`parse_value`'s brace arm, `consume_argument_list`, `consume_variable_definitions`, and `[...]` via `parse_group_singleton` are the same substitution. Those functions still take only the cursor.

`parse_group_*` is not landed until `parse_items` is (parse-fields.md). This change lands `record_group_close` and the open-on-consume. parse-fields.md and parse-arguments.md / parse-variables.md use the helper in the snippets above.

### Threading through the list helpers

`parse_chunk`, `parse_one_item`, `parse_singleton`, and (when it lands) `parse_items` take `&mut Vec<WithSpan<SemanticToken>>` and pass it to `stream`. Grammar closures still receive `&mut ItemCursor`.

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
fn parse_chunk<'a, P>(
    chunk: &'a WithSpan<Chunk>,
    text: &'a str,
    parse_item: impl FnOnce(&mut ItemCursor<'a>) -> Result<P, WithSpan<ParseError>>,
) -> (ChunkStream<'a>, Result<WithSpan<P>, WithSpan<ParseError>>) {
    let mut stream = chunk.item.stream(text);
    let result = stream.cursor().spanning(parse_item);
    (stream, result)
}

fn parse_one_item<'a, P, F>(
    chunk: &'a WithSpan<Chunk>,
    text: &'a str,
    leftover: Expectation,
    parse: impl FnOnce(&mut ItemCursor<'a>, &mut F) -> Result<P, WithSpan<ParseError>>,
    push_error: &mut F,
) -> WithSpan<Slot<P, UnparsedChunkItems>>
where
    F: FnMut(WithSpan<ParseError>),
{
    let (mut stream, result) = parse_chunk(chunk, text, |cursor| parse(cursor, push_error));
    /* unchanged match */
}

pub(crate) fn parse_singleton<'a, T, F>(
    level: &'a WithSpan<ChunkedLevel>,
    text: &'a str,
    end: Expectation,
    extra_chunks: impl FnOnce(&'a WithSpan<Chunk>) -> WithSpan<ParseError>,
    parse: impl FnOnce(&mut ItemCursor<'a>, &mut F) -> Result<T, WithSpan<ParseError>>,
    push_error: &mut F,
) -> Singleton<Slot<T, UnparsedChunkItems>, ExtraChunks>
where
    F: FnMut(WithSpan<ParseError>),
{
    let item = parse_one_item(&level.item.0[0], text, end, parse, push_error);
    /* unchanged extra-chunks / comma */
}
```

After:

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

fn parse_one_item<'a, P, F>(
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
    let (mut stream, result) =
        parse_chunk(chunk, text, tokens, |cursor| parse(cursor, push_error));
    /* unchanged match */
}

pub(crate) fn parse_singleton<'a, T, F>(
    level: &'a WithSpan<ChunkedLevel>,
    text: &'a str,
    tokens: &mut Vec<WithSpan<SemanticToken>>,
    end: Expectation,
    extra_chunks: impl FnOnce(&'a WithSpan<Chunk>) -> WithSpan<ParseError>,
    parse: impl FnOnce(&mut ItemCursor<'a>, &mut F) -> Result<T, WithSpan<ParseError>>,
    push_error: &mut F,
) -> Singleton<Slot<T, UnparsedChunkItems>, ExtraChunks>
where
    F: FnMut(WithSpan<ParseError>),
{
    let item = parse_one_item(&level.item.0[0], text, tokens, end, parse, push_error);
    /* unchanged extra-chunks / comma */
}
```

`parse_items` (parsing-standards.md, lands in parse-fields.md) takes `tokens: &mut Vec<WithSpan<SemanticToken>>` and passes it to each `parse_one_item`. Sequential chunks: the previous `ChunkStream` is dropped before the next `stream` reborrows `tokens`.

### Entry point

`parse_iso_literal` takes the vec. Every call constructs tokens.

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

```
entrypoint Query.foo
```

records, in source order:

- `SemanticToken::Identifier` at `entrypoint`
- `SemanticToken::Identifier` at `Query`
- `SemanticToken::Period` at `.`
- `SemanticToken::Identifier` at `foo`

`entrypoint Foo.$ asdf` records `entrypoint`, `Foo`, `.` and then fails at `$`. Those three tokens stay. There is no pop.

Leftover items (`asdf`) and separator commas are not consumed, so they are not recorded. Positions in leftover still resolve through `UnparsedChunkItems`. Highlighting them is the leftover fill-in change.

### Layering

Every byte of the literal is classified by at most one of these, and errors are a third channel:

1. Recorded: a token or bracket the grammar consumed. First pass: the kind. After reclassify: the role.
2. Lexical fill-in (later): a token no consume covered (leftover, a separator comma, text in the matcher's cut). Classification is the token kind from `tokenize`.
3. Errors are diagnostics: the matcher's vec, chunking's `CommaWithoutItem` vec, and `push_error`. No `SemanticToken` variant is an error.

So `foo ( asfd`: `foo` is recorded as `Identifier`; `(` is an unmatched-open diagnostic and is not in the tree; `asfd` sits in the cut and, after fill-in, highlights as an identifier.

### Tests

No snapshots. Facts:

- `require_token(Identifier)` on `foo` yields `[Identifier @ foo]`.
- `require_token(Period)` when the next item is an identifier records nothing and returns `Err(())`.
- `consume_token_if` that does not match records nothing.
- `consume_group_if(Brace)` on `{ bar }` records `OpenBrace` at `{`. `record_group_close` then records `CloseBrace` at `}`.
- `parse_iso_literal` on `entrypoint Query.foo` fills the four tokens above and the same tree as today.
- A failed first chunk that consumed a prefix (`entrypoint Foo.$`) still has the prefix tokens.
- `expected` does not record.

`stream_of` in `chunk_stream.rs` tests takes the vec:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    fn stream_of<'a>(
        tree: &'a WithSpan<ChunkedLevel>,
        text: &'a str,
        tokens: &'a mut Vec<WithSpan<SemanticToken>>,
    ) -> ChunkStream<'a> {
        first_chunk(tree).stream(text, tokens)
    }
```

Existing consume tests pass `&mut Vec::new()` and do not assert on it. Collecting tests pass a vec and assert its contents.

### Docs this change amends

- parsing-standards.md: `ItemCursor` gains `tokens: &'a mut Vec<WithSpan<SemanticToken>>`. `Chunk::stream`, `parse_chunk`, `parse_one_item`, `parse_singleton`, `parse_items` take that vec. Catalog adds `record`, `record_group_close`, `parse_group_items` / `parse_group_singleton`. The group-plus-interior listing becomes the helper.
- parse-entrypoint.md: `parse_iso_literal` takes `tokens: &mut Vec<WithSpan<SemanticToken>>`.
- parse-fields.md, parse-arguments.md, parse-variables.md: group interiors go through the helper.
- parsing-plan.md: tokens are recorded during parse into a vec.

## 2. Construct with a noop or a non-noop

The collector becomes a type parameter. Two implementors, by design; this seam is the whole of what the trait exists for. The parse is constructed with one or the other.

```rust
// from crates/isograph_parser/src/semantic_token.rs
pub trait SemanticTokens {
    fn record(&mut self, token: SemanticToken, span: Span);
}

pub struct CollectedSemanticTokens(pub Vec<WithSpan<SemanticToken>>);

pub struct NoSemanticTokens;

impl CollectedSemanticTokens {
    pub fn new() -> CollectedSemanticTokens {
        CollectedSemanticTokens(Vec::new())
    }
}

impl NoSemanticTokens {
    pub fn new() -> NoSemanticTokens {
        NoSemanticTokens
    }
}

impl SemanticTokens for CollectedSemanticTokens {
    fn record(&mut self, token: SemanticToken, span: Span) {
        self.0.push(token.with_span(span));
    }
}

impl SemanticTokens for NoSemanticTokens {
    fn record(&mut self, _token: SemanticToken, _span: Span) {}
}
```

`record` takes `(SemanticToken, Span)`, not a pre-built `WithSpan`. `CollectedSemanticTokens` constructs the `WithSpan` inside `record`. `NoSemanticTokens::record` is empty, so that monomorphization does not build a `WithSpan`.

### Cursor and helpers

`ItemCursor` / `ChunkStream` gain `TTokens`. The bound lives on the `impl`, not the struct. Every `&mut Vec<WithSpan<SemanticToken>>` from change 1 becomes `&mut TTokens`. Grammar functions become generic; their bodies do not mention `TTokens`.

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
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_entrypoint(
    cursor: &mut ItemCursor<'_>,
) -> Result<EntrypointDeclaration, WithSpan<ParseError>>
```

After:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
pub(crate) struct ItemCursor<'a, TTokens> {
    items: SafePeekable<nonempty::Iter<'a, WithSpan<ChunkContentItem>>>,
    previous_end: u32,
    text: &'a str,
    tokens: &'a mut TTokens,
}

pub(crate) struct ChunkStream<'a, TTokens>(ItemCursor<'a, TTokens>);

impl<'a, TTokens: SemanticTokens> ItemCursor<'a, TTokens> {
    fn record(&mut self, token: SemanticToken, span: Span) {
        self.tokens.record(token, span);
    }
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_entrypoint<TTokens: SemanticTokens>(
    cursor: &mut ItemCursor<'_, TTokens>,
) -> Result<EntrypointDeclaration, WithSpan<ParseError>>
```

`parse_iso_literal_item`, `parse_selection`, `parse_value`, and the rest of the grammar functions are the same substitution.

`Chunk::stream`, `parse_chunk`, `parse_one_item`, `parse_singleton`, `parse_items`, and `parse_group_*` are generic over `TTokens: SemanticTokens` and take `&mut TTokens`. Closures receive `&mut ItemCursor<'_, TTokens>`.

```rust
// from crates/isograph_parser/src/chunk.rs
fn parse_chunk<'a, P, TTokens: SemanticTokens>(
    chunk: &'a WithSpan<Chunk>,
    text: &'a str,
    tokens: &'a mut TTokens,
    parse_item: impl FnOnce(&mut ItemCursor<'a, TTokens>) -> Result<P, WithSpan<ParseError>>,
) -> (
    ChunkStream<'a, TTokens>,
    Result<WithSpan<P>, WithSpan<ParseError>>,
) {
    let mut stream = chunk.item.stream(text, tokens);
    let result = stream.cursor().spanning(parse_item);
    (stream, result)
}
```

### Entry point

`parse_iso_literal` is generic. The caller constructs the collector.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal<TTokens: SemanticTokens>(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    mut push_error: impl FnMut(WithSpan<ParseError>),
    tokens: &mut TTokens,
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

Non-noop:

```rust
let mut tokens = CollectedSemanticTokens::new();
let tree = parse_iso_literal(text, root, push_error, &mut tokens);
// tokens.0 is the vec
```

Noop:

```rust
let tree = parse_iso_literal(text, root, push_error, &mut NoSemanticTokens::new());
```

The tree is the same either way.

### Tests

The change-1 facts still hold against `CollectedSemanticTokens`. Added:

- `parse_iso_literal` with `NoSemanticTokens::new()` returns the same tree as with `CollectedSemanticTokens::new()`.
- `stream_of` is generic. Existing consume tests that do not assert tokens pass `&mut NoSemanticTokens::new()`. Collecting tests pass `&mut CollectedSemanticTokens::new()` and assert on `.0`.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    fn stream_of<'a, TTokens: SemanticTokens>(
        tree: &'a WithSpan<ChunkedLevel>,
        text: &'a str,
        tokens: &'a mut TTokens,
    ) -> ChunkStream<'a, TTokens> {
        first_chunk(tree).stream(text, tokens)
    }
```

### Docs this change amends

- parsing-standards.md: `ItemCursor` / `ChunkStream` gain `TTokens`. List helpers are generic over `TTokens: SemanticTokens`. Grammar functions take `ItemCursor<'_, TTokens>`. Catalog replaces the vec with `SemanticTokens`, `CollectedSemanticTokens::new`, `NoSemanticTokens::new`.
- parse-entrypoint.md: `parse_iso_literal` is generic over `TTokens`.
- parse-fields.md, parse-arguments.md, parse-variables.md: each `parse_*` gains `TTokens: SemanticTokens`.
- spanless-parsing.md: the cheap pass is `NoSemanticTokens::new()` plus `NoSpan`.

## Later changes

Each is independently shippable. `require_token` still takes only a kind.

### Reclassify

After `token_text` names the role, the cursor overwrites the token it just recorded. The trait grows one method. `NoSemanticTokens` stays a no-op.

```rust
// from crates/isograph_parser/src/semantic_token.rs
pub trait SemanticTokens {
    fn record(&mut self, token: SemanticToken, span: Span);
    fn reclassify_last(&mut self, token: SemanticToken);
}

impl SemanticTokens for CollectedSemanticTokens {
    fn record(&mut self, token: SemanticToken, span: Span) {
        self.0.push(token.with_span(span));
    }

    fn reclassify_last(&mut self, token: SemanticToken) {
        if let Some(last) = self.0.last_mut() {
            last.item = token;
        }
    }
}

impl SemanticTokens for NoSemanticTokens {
    fn record(&mut self, _token: SemanticToken, _span: Span) {}

    fn reclassify_last(&mut self, _token: SemanticToken) {}
}

// from crates/isograph_parser/src/chunk_stream.rs
impl<TTokens: SemanticTokens> ItemCursor<'_, TTokens> {
    pub(crate) fn reclassify(&mut self, token: SemanticToken) {
        self.tokens.reclassify_last(token);
    }
}
```

`SemanticToken` grows the roles the grammar knows: `Keyword`, `Type`, `Field`, `Variable`, `BooleanOrNull`. `parse_iso_literal_item` after matching `"entrypoint"`:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let keyword = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::DeclarationKeyword))?;
    match cursor.token_text(keyword) {
        text if text == "entrypoint" => {
            cursor.reclassify(SemanticToken::Keyword);
            IsoLiteralItem::Entrypoint(parse_entrypoint(cursor)?).wrap_ok()
        }
        /* field / pointer / other unchanged */
    }
```

`parse_entrypoint` reclassifies `Query` to `Type` and `foo` to `Field`. Selection names, argument names, variable names, `to`, `true` / `false` / `null` are the same. `reclassify` is called immediately after the consume that recorded.

### Leftover fill-in

A walk over `tokenize(text)` that emits `from_non_bracket` / `for_open` / `for_close` for every token whose span is not already in the collected vec, in source order. Separators, leftover items, and the matcher's cut get lexical tokens. The collected vec stays sorted by span. This is the LSP layer, not the parser's consume path. It runs against `CollectedSemanticTokens`.

### Formatter metadata

`SemanticToken` gains `line_behavior` and `indent_change`, the fields on upstream `IsographSemanticToken`. Reclassify (or the first-pass mapping) fills them. The formatter walks `Vec<WithSpan<SemanticToken>>` the way `crates/isograph_lsp/src/format.rs` walks the upstream vec.

### Cheap pass

spanless-parsing.md's `TSpan = NoSpan` parse is constructed with `NoSemanticTokens::new()`. The compile path:

```rust
let tree = parse_iso_literal::<NoSpan>(
    text,
    root,
    push_error,
    &mut NoSemanticTokens::new(),
);
if /* push_error was called, or tree.has_errors() */ {
    let mut tokens = CollectedSemanticTokens::new();
    let spanned = parse_iso_literal::<Span>(text, root, push_error, &mut tokens);
    report(spanned);
}
```

The LSP path always constructs with `CollectedSemanticTokens::new()` and `TSpan = Span`. One function, two instantiations of each parameter, same consume-time recording. The two trees cannot disagree about structure.

`TSpan` is the tree's parameter. `TTokens` is the cursor's. They are chosen together at the entry point by which values are constructed. `require_token` names neither.
