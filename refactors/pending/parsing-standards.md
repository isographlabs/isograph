# Parsing standards

Rules for all grammar-stage code. The feature docs define the grammar; this doc defines how parsers are written. An implementation need this doc forbids is resolved by amending this doc or fixing the code, in the same review, never by shipping the deviation.

This doc assumes cut-at-unmatched.md and no-empty-chunks.md: grouping returns `(WithSpan<MatchedBrackets>, Vec<BracketError>)` with unmatched brackets and their levels' tails absent from the tree, and chunking returns `(WithSpan<ChunkedLevel>, Vec<CommaWithoutItem>)` with empty chunks absent. No type downstream represents a bracket problem or an empty chunk; every chunk a parser sees has contents.

## Input shape

- The unit of parsing is the chunk: a vec of tokens and matched groups, read by exactly one `SafePeekable`, behind that chunk's `ChunkStream`. The literal is the root level's one chunk; a group's interior is levels of further chunks; each chunk parses independently. No cursor spans two chunks.
- A group is one item, consumed whole, always really closed. Its interior re-enters parsing only as fresh levels.
- Items arrive pre-spanned. Parsers compute a span only for a multi-item composite, via `spanning`.
- Separators were absorbed into boundaries by chunking: "a separator comes next" is `take_next()` returning `None`. Brackets were resolved by the matcher: no bracket state reaches a parser.

## Function shapes

- `require_*`: required grammar. Errors on the found item, unconsumed, or at an empty span where the missing item belonged. Consumes exactly the accepted items.
- `consume_*`: optional grammar. Consumes and returns the item when the next item opens it; consumes nothing and returns `None` otherwise. Never errors.
- `parse_*`: a composite production built from the other two. Errors propagate from the first failing piece.
- Shared structure is a higher-order function taking the item parser (`parse_level_items`); a hand-duplicated walk or wrapper is banned.

## Enforcement structures

Every operation a parser can perform is a method on one of four types. A new operation is a new method here, never a local helper.

### `ChunkStream`

```rust
// from crates/isograph_parser/src/chunk_stream.rs
/// The only reader of a chunk's contents. No rewind and no raw peek exist: a committed
/// item is committed, and a decision is made on at most the next item.
pub(crate) struct ChunkStream<'a> {
    items: SafePeekable<std::slice::Iter<'a, WithSpan<ChunkContentItem>>>,
    /// The end of the last accepted item (the chunk's start before any): where an
    /// `Expected(_, EndOfChunk)` error points.
    previous_end: u32,
}

impl<'a> ChunkStream<'a> {
    /// The next item's span when it is a non-bracket token of `kind`; the error
    /// otherwise, on the found item, unconsumed, or empty at `previous_end` when the
    /// chunk ran out.
    pub(crate) fn require_token(
        &mut self,
        kind: NonBracketTokenKind,
        expected: Expectation,
    ) -> Result<Span, WithSpan<ParseError>>;

    /// The next item's span, consumed, when it is a non-bracket token of `kind`;
    /// `None`, nothing consumed, otherwise.
    pub(crate) fn consume_token_if(&mut self, kind: NonBracketTokenKind) -> Option<Span>;

    /// The next item, consumed, when it is a group opened by `kind`; `None`, nothing
    /// consumed, otherwise.
    pub(crate) fn consume_group_if(
        &mut self,
        kind: BracketKind,
    ) -> Option<WithSpan<&'a ChunkedGroup>>;

    /// The next item, committed, or `None` at the chunk's end. Total; errors are the
    /// caller's to construct.
    pub(crate) fn take_next(&mut self) -> Option<&'a WithSpan<ChunkContentItem>>;

    /// The empty span at `previous_end`, for error arms that found `EndOfChunk`.
    pub(crate) fn end_span(&self) -> Span;

    /// Runs a sub-parse and wraps its result in the span it consumed: from the start
    /// of the first item the closure accepts to the end of its last, empty at
    /// `previous_end` when it accepts none.
    pub(crate) fn spanning<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, WithSpan<ParseError>>,
    ) -> Result<WithSpan<T>, WithSpan<ParseError>>;

    /// Nothing further may exist. The first leftover item is the error.
    pub(crate) fn require_end(&mut self, expected: Expectation) -> Result<(), WithSpan<ParseError>>;
}
```

- Peek-commit, unconsumed offenders, and `previous_end` anchoring live here once, not per parser.
- Span sources, exhaustively: a leaf's span is what `require_token` returned; an item's span is what its parse consumed (a degraded slot's is its chunk's `contents_span`); a composite's span comes from `spanning`. `Span::join` in a parser is banned; a span no source provides is a missing method here.
- Construction is `WithSpan::new` and plain `Ok`/`Some`; the upstream postfix helpers (`wrap_ok`, `with_span`) are not used.

### `ChunkedLevel`

```rust
// from crates/isograph_parser/src/chunk.rs
impl ChunkedLevel {
    /// The only access to a level's chunks, each with contents (no-empty-chunks.md).
    pub fn chunks(&self) -> impl Iterator<Item = &WithSpan<Chunk>>;
}
```

- `ChunkedLevel`'s field is private to chunk.rs. Callers are `parse_level_items` and the one-item walkers, and no others.

### `Chunk`

```rust
// from crates/isograph_parser/src/chunk.rs
impl Chunk {
    /// The stream a parser reads this chunk through.
    pub fn stream(&self) -> ChunkStream<'_>;

    /// The span of the contents, without the boundary: a degraded slot's span. Total,
    /// because every chunk has contents.
    pub fn contents_span(&self) -> Span;

    /// The comma in the trailing boundary, when one exists; one-item contexts reject
    /// it (no-final-comma.md).
    pub fn boundary_comma(&self) -> Option<Span>;
}
```

- `Chunk`'s fields are private to chunk.rs. These methods are the only item access and the only boundary reads.

### `LiteralText`

```rust
// from crates/isograph_parser/src/literal_text.rs
/// The literal's text, admitting only the reads the grammar performs. Raw slicing is
/// unavailable outside this impl.
pub(crate) struct LiteralText<'a>(&'a str);

impl<'a> LiteralText<'a> {
    /// The text of an identifier token, for keyword dispatch by string match. The span
    /// is one an identifier-accepting stream method returned.
    pub(crate) fn identifier(&self, span: Span) -> &'a str;
}
```

- The complete set of text reads is this impl block: identifier text (keywords, `to`, `true`/`false`/`null`, all matched as strings at their one dispatch site each) and, when parse-arguments.md amends it, `integer` (`None` on out of range). String-literal contents and every other span stay unreadable.
- Anticipated `ChunkStream` amendments, each landing with its first caller: `spanning_from(start, parse)` for opener-anchored composites; a plainly-returning `spanning` sibling. An `Option`-returning sibling has no possible caller: a single optional item carries its own span, and an opener-marked composite is require-flow past its opener.

## Dispatch

A position allowing several forms is one exhaustive `match` on `take_next()`. The discriminating item commits up front; accepting arms continue from it, the rejecting arm reports it as the `found`.

```rust
// from crates/isograph_parser/src/arguments.rs (shape, not the final listing)
match stream.take_next() {
    Some(item) => match &item.item {
        ChunkContentItem::NonBracket(token) => match token.0 {
            NonBracketTokenKind::Dollar => { /* the variable's name follows */ }
            NonBracketTokenKind::StringLiteral => { /* done; item.location is the span */ }
            NonBracketTokenKind::IntegerLiteral => { /* convert via LiteralText */ }
            NonBracketTokenKind::Identifier => { /* match LiteralText::value_word */ }
            kind => return Err(WithSpan::new(
                ParseError::expected(Expectation::Value, Found::Token(kind)),
                item.location,
            )),
        },
        ChunkContentItem::Group(group) => { /* brace: object literal; other kinds: the same error shape */ }
    },
    None => return Err(WithSpan::new(
        ParseError::expected(Expectation::Value, Found::EndOfChunk),
        stream.end_span(),
    )),
}
```

- Content dispatch is the same shape one level down: `require_token(Identifier, ...)`, then a `match` on `LiteralText::identifier`'s string (`"entrypoint"`, `"field"`, `"pointer"`; `"true"`/`"false"`/`"null"`; `"to"`), the `_` arm the one reject.
- `consume_*_if` is not a dispatch tool. It exists for the composition boundary: a sub-parser declining an item that belongs to its caller (the optional `!` after a type name, whose absence might be the caller's `=`). A `consume_*_if` chain where one production owns all the alternatives is banned.
- An opener-marked composite (optional as a whole, required past its opener: `$name`, a future `@ name (args)`) is a dispatch arm; the opener commits in the match, the remainder is `require_*`, repetition is the position's match in a loop.
- Optionality is decided by the first item, always. A single optional item is a `consume_*_if`, infallible. A multi-item optional commits its opener and is fallible from its second item on (`@@` errors at the second `@`). Commit-and-reinterpret (the alias's colon deciding what the committed identifier was) is legal only when every continuation uses everything committed. Consume-and-decline does not exist, so a grammar addition needing more than one item of lookahead for optionality is unwritable.

## Level walks

- A list level is walked only by `parse_level_items`: it runs `parse_item` on a fresh `stream()` per chunk and itself calls `require_end(Expectation::Separator)` after a successful item. Item parsers parse their production and stop; exhaustion is the walker's.
- Trailing junk does not void a completed item: on `require_end` failure after success, the walker keeps the item and records the junk as the slot's trailing error. `foo bar` is the selection `foo` plus an error at `bar`; go-to-definition, find-references, and completion see `foo` as if the junk were absent. The item's span excludes the junk, so junk positions answer the containing level.
- Junk is always the suffix; nothing after the first leftover is reattached. `foo bar { baz }` is the scalar `foo` with junk from `bar` on; `baz` is not in the tree.
- A one-item context (the root level, a `[...]` interior) has its own walker (`declaration_chunk`, `parse_bracket_interior_type`): exactly one chunk, `boundary_comma` rejected per no-final-comma.md, and the end check with its own expectation (`EndOfDeclaration`, `EndOfType`).

## Failure isolation

One chunk to one item, and the item is a result: each chunk parses in its entirety, independently, to exactly one output slot holding the parsed item (beside any trailing-junk error) or the unparsed reason.

- A `ChunkStream` cannot read past its chunk; there is no shared cursor to corrupt.
- `parse_level_items` returns `Vec<WithSpan<T>>`, not `Result`: an item parser's `Err` has nowhere to go but the `unparsed` conversion, so `?` cannot leak a chunk's failure to its siblings. Errors escape only the one-item walkers, whose failed item is the whole context.
- Output length equals chunk count; a failure shifts no sibling, and a degraded slot resolves through its retained chunk.

## Errors

- An error is `WithSpan<ParseError>`; the workhorse is `Expected(ExpectedFound { expected, found })`. The span covers the offending item or is empty where the missing item belonged.
- Errors live in the tree (`UnparsedLiteral`, `UnparsedItem`, the trailing-junk slot); `errors()` derives the list in source order. No error list exists beside the grammar tree.
- Bracket and comma errors are the earlier passes', returned beside their trees (cut-at-unmatched.md, no-empty-chunks.md); no `ParseError` variant names either. The final sweep is all three lists: an error-free literal has empty vecs from the matcher and chunking and an empty `errors()` from the grammar.
- Degradation is as local as the grammar allows: a failed list chunk degrades alone; a failed declaration header degrades the literal. One error per degraded region.
- No prose in the parser. Messages are `Display` impls; contextual suggestions belong to rendering, keyed off the `(expected, found)` pair.

## Totality

- No panics, on any input: `unwrap`, `expect`, `unreachable!`, and type-level infallibility claims are banned. A real but compiler-invisible invariant gets a graceful fallback and a doc comment stating the invariant.
- Every input yields a tree; every position resolves: parsed regions to grammar leaves, degraded regions through retained chunks, everything else (whitespace, the matcher's dropped regions) to the nearest container.

## Trees and spans

- Span placement is decided at the type: a tree enum is `WithSpan`-wrapped once at its slot, variant payloads are bare, struct fields each carry their own `WithSpan`, and each wrapper's coverage is stated on the type.
- Names are spans held by fieldless marker structs, one per role (an alias is not a name; an argument name is not an object key). No strings, no interning; the one derived scalar is the converted `i64`. Punctuation and keyword markers (`Dot`, `Dollar`, `Exclamation`, ...) are unmarked fields and answer their container.
- `ResolvePosition` is derive-only; a manual impl is a missing `resolve_position` feature and becomes a prefactor there. A parent is a direct path alias at one parent, an enum at the second. Chunk-stage `IsographResolutionNode` variants are reachable inside degraded regions only.

## Performance

- One pass, by reference; the output copies spans and `Copy` tokens. Cloning happens only when a region degrades: allocation beyond the output vecs is proportional to the error count.
- No backtracking exists (no rewind), so parse time is linear in the token count.
- No pico, no interning: plain functions over `LiteralText` and the chunk tree.

## Relation to the upstream parser

- `parse_token_of_kind` served required and optional cases, the contract at each call site (`...?` vs `.is_ok()`); here the contract is the `require_*`/`consume_*` split. `parse_delimited_list` -> `parse_level_items`; `PeekableLexer` -> `ChunkStream`; `with_embedded_location_result` -> `spanning`.
- Upstream's `to_control_flow` alternative chains are safe only while every alternative fails on its first unconsumed token, admitted in `parse_type_annotation`'s comment ("will leave the parser in an inconsistent state"); here no rewind exists, so that parser cannot be written.
- Upstream is fail-fast and panics on unexpected input (`number.parse().expect(...)`, `unreachable!()` in the block-string lexer); here every input yields a tree.
- Upstream errors are inline prose `String`s, some spanless (`Span::todo_generated()`); here they are structured with mandatory spans.
- Upstream threads `TextSource` and extraction context (its own comment: "we break memoization") and interns during the parse; here the inputs are `LiteralText` and the chunk tree.
- Upstream re-derives separator policy per list (`parse_comma_or_line_break`, `white_space_span` inspection) and meets bracket mistakes wherever a token check trips; here separators live in chunking, brackets in the matcher.
- Upstream accumulates semantic tokens during parsing; here they are derivable later from the tree.

## Shipping and amending

Nothing here ships on its own: each structure and each method lands with the feature doc of its first production caller (`ChunkStream`'s required-token core, `LiteralText::identifier`, and `ChunkedLevel`'s privacy with parse-entrypoint.md; `take_next`, the `consume_*` methods, `contents_span`, and `parse_level_items` with parse-fields.md; `spanning` and `integer` with parse-arguments.md; `boundary_comma` with no-final-comma.md). A method with no caller yet exists only in this doc.

A feature implementation is reviewed against this doc when it lands. The expected amendment sites are the four impl blocks; a change that routes around a structure instead of extending it is what this doc exists to prevent. This doc itself never moves to refactors/past: it is normative and stays current.
