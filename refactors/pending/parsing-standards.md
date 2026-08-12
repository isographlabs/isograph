# Parsing standards

The rules every parser in the grammar stage follows, and the data structures that enforce them. The feature docs (parse-entrypoint.md through no-final-comma.md) define what parses; this doc defines how parsing code is written, what it may and may not do, and which types make violations unwritable rather than merely disallowed. The intent is to concentrate review here: the enforcement structures below each have one impl block, reviewed once, and a feature parser built from them has few ways to be wrong. Each feature implementation is reviewed against this doc when it lands, and a deviation is resolved by amending this doc first or fixing the code, never by shipping the deviation silently.

## The three function shapes

Every parser function is one of three shapes, and its prefix states its contract:

- `require_*`: a required piece of grammar. Errors on the found item, unconsumed, or at an empty span where the missing item belonged. Consumes exactly the accepted items.
- `consume_*`: an optional piece of grammar. Consumes and returns it when the next item opens it; consumes nothing and returns `None` otherwise. Never errors on absence.
- `parse_*`: a composite production, built from the other two. Errors propagate from the first failing piece.

Shared parsing structure is expressed as higher-order functions parameterized by the item parser, in the style upstream's `parse_delimited_list` set: `parse_level_items` takes `parse_item` and the unparsed-variant constructor, and every list reuses the one walk rather than restating it. When two productions share a shape, the shape becomes a higher-order function and the productions become its arguments; duplicating a walk or a wrapper by hand is the anti-pattern.

## The enforcement structures

Four types carry the invariants. Everything a parser can do is a method on one of them, so their impl blocks are the complete, reviewable surface, and extending the language's mechanics means amending them, visibly, rather than writing a local helper.

### `ChunkStream`: the only reader of a chunk's contents

```rust
// from crates/isograph_parser/src/chunk_stream.rs
/// The only reader of a chunk's contents. It owns the position, tracks where the last
/// accepted item ended so a missing-item error positions itself, and exposes no rewind:
/// a committed item is committed, so a backtracking parser cannot be written. The
/// methods here are the complete set of operations a parser can perform on a chunk.
pub(crate) struct ChunkStream<'a> {
    items: SafePeekable<std::slice::Iter<'a, WithSpan<ChunkContentItem>>>,
    /// The end of the last accepted item (the chunk's start before any), which is where
    /// an `Expected(_, EndOfChunk)` error points.
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

    /// The next item, consumed, when it is a matched group opened by `kind`; `None`,
    /// nothing consumed, otherwise.
    pub(crate) fn consume_group_if(
        &mut self,
        kind: BracketKind,
    ) -> Option<WithSpan<&'a ChunkedGroup>>;

    /// The next item, committed, or the end of the chunk: the value a dispatch
    /// position matches on. Total, so every match handles the end; the end's span is
    /// empty at `previous_end`.
    pub(crate) fn take_next(&mut self) -> WithSpan<Taken<'a>>;

    /// Nothing further may exist. The first leftover item is the error.
    pub(crate) fn require_end(&mut self, expected: Expectation) -> Result<(), WithSpan<ParseError>>;
}

/// What a dispatch position sees: the committed next item, its payload carried into
/// the match arm, or the chunk's end. `Found` converts from it for error arms.
pub(crate) enum Taken<'a> {
    Token(NonBracketTokenKind),
    Group(&'a ChunkedGroup),
    UnmatchedOpen(BracketKind),
    UnmatchedClose(BracketKind),
    EndOfChunk,
}
```

What this discharges: consumption discipline (peek-commit, failure leaves the offender in place, errors carry the right span) is written once here instead of once per parser; the `missing_at` threading that every `require_*` call previously carried by hand disappears into `previous_end`, so a wrong anchor cannot be written. `SafePeekable` underneath has no rewind, and `ChunkStream` exposes no raw peek, so one-peek-decides holds structurally: an item commits only when a method accepted it.

### Dispatch is a match

A position where the grammar allows one of several forms is written as one `match` on `take_next()`, never as a chain of `consume_*_if` attempts. The discriminating item is committed up front, which is sound because every arm uses it: the accepting arms continue from it, and the rejecting arm reports it as the `found`. The match is exhaustive over `Taken`, so handling the chunk's end cannot be forgotten, and a separator can never appear in an arm, because chunks are separator-free: "a separator comes next" is the `EndOfChunk` variant. Sketched on a value:

```rust
// from crates/isograph_parser/src/arguments.rs (shape, not the final listing)
let taken = stream.take_next();
match taken.item {
    Taken::Token(NonBracketTokenKind::Dollar) => { /* the variable's name follows */ }
    Taken::Token(NonBracketTokenKind::StringLiteral) => { /* done */ }
    Taken::Token(NonBracketTokenKind::IntegerLiteral) => { /* convert via LiteralText */ }
    Taken::Token(NonBracketTokenKind::Identifier) => { /* match LiteralText::value_word */ }
    Taken::Group(group) if /* brace */ => { /* object literal */ }
    taken => return Err(WithSpan::new(
        ParseError::expected(Expectation::Value, Found::from(&taken)),
        /* taken's span */,
    )),
}
```

Dispatch on an identifier's text is the same shape one level down: `require_token(Identifier, ...)` then a `match` on `LiteralText::keyword` (or `value_word`), so `entrypoint` versus `field` versus `pointer` is a match on the `Keyword` enum, not string comparisons scattered through arms.

The `consume_*_if` shape is not a dispatch tool. It exists for one case: a composition boundary, where a sub-parser meets an item that is not its own and must decline without consuming what belongs to its caller (the optional `!` after a type name, whose absence might be the caller's `=` or the chunk's end). Inside a production that owns all the alternatives at a position, reaching for a `consume_*_if` chain instead of a `take_next` match is the anti-pattern.

### `LevelEntry`: the only access to a level's chunks

`ChunkedLevel`'s field becomes private to chunk.rs, and the one accessor classifies:

```rust
// from crates/isograph_parser/src/chunk.rs
impl ChunkedLevel {
    /// The only access to a level's chunks. Classification forces every walker to
    /// handle both cases: an item to parse, or a comma no item precedes, already
    /// positioned for its error.
    pub fn entries(&self) -> impl Iterator<Item = LevelEntry<'_>>;
}

pub enum LevelEntry<'a> {
    /// A chunk with contents: one grammar item.
    Item(&'a WithSpan<Chunk>),
    /// An empty chunk: a comma no item precedes, always an error at `comma`.
    CommaWithoutItem {
        comma: Span,
        chunk: &'a WithSpan<Chunk>,
    },
}
```

What this discharges: the empty-chunk-is-an-error rule cannot be skipped, because a walker cannot see chunks without matching `CommaWithoutItem`, and the comma's span is computed once in chunk.rs rather than re-derived per site (`empty_chunk_comma_span` ceases to exist). `parse_level_items` and the one-item walkers are the only intended callers; a new walker is at least forced through the classification.

### `Chunk`'s narrowed surface

`Chunk`'s fields become private to chunk.rs, with exactly the methods parsing needs:

```rust
// from crates/isograph_parser/src/chunk.rs
impl Chunk {
    /// The stream a parser reads this chunk through.
    pub fn stream(&self) -> ChunkStream<'_>;

    /// The span of the contents, without the boundary: an item's span.
    pub fn contents_span(&self) -> Option<Span>;

    /// The comma in the trailing boundary, when one exists. Only a list gives a
    /// boundary comma meaning; one-item contexts call this to reject it
    /// (no-final-comma.md).
    pub fn boundary_comma(&self) -> Option<Span>;
}
```

What this discharges: no code outside chunk.rs can index a chunk's items, read its separator tokens, or invent a third boundary inspection. The two sanctioned boundary reads are these methods' bodies.

### `LiteralText`: the only reader of the text

```rust
// from crates/isograph_parser/src/literal_text.rs
/// The literal's text, admitting only the reads the grammar performs: one method per
/// legal read, so the complete set is this impl block. Raw slicing is unavailable
/// outside it.
pub(crate) struct LiteralText<'a>(&'a str);

/// The identifiers the grammar gives keyword meaning.
pub(crate) enum Keyword {
    Entrypoint,
    Field,
    Pointer,
    To,
}

impl<'a> LiteralText<'a> {
    pub(crate) fn keyword(&self, span: Span) -> Option<Keyword>;

    // parse-arguments.md adds, as amendments here:
    // fn value_word(&self, span: Span) -> Option<ValueWord>;   // true / false / null
    // fn integer(&self, span: Span) -> Option<i64>;            // None: out of range
}
```

What this discharges: the exhaustive list of text reads is one impl block, keyword dispatch becomes a match on `Keyword` instead of string comparison at call sites, and a future parser cannot quietly start reading text.

## Level walks

- A list level is walked only by `parse_level_items`. It consumes `entries()`, turns `CommaWithoutItem` into the missing-item unparsed item, runs `parse_item` on a fresh `stream()`, and itself calls `require_end(Expectation::Separator)` after a successful item, so an item parser cannot forget the leftover check: item parsers parse their production and stop, and exhaustion is the walker's job.
- A one-item context (the root level, a `[...]` interior) has its own walker (`declaration_chunk`, `parse_bracket_interior_type`), which enforces exactly one `Item`, errors on `CommaWithoutItem`, and rejects `boundary_comma` per no-final-comma.md. The end-of-chunk check is likewise the walker's, with its context's expectation (`EndOfDeclaration`, `EndOfType`).

## Errors

- An error is a `WithSpan<ParseError>`, and the workhorse is `Expected(ExpectedFound { expected, found })`. The span covers the offending item, or is empty at the position the missing item belonged, which `ChunkStream` computes.
- Errors live in the tree (`UnparsedLiteral`, `UnparsedItem`), and `errors()` derives the list from the tree in source order. There is no error list beside the tree.
- Degradation is as local as the grammar allows: a failed list chunk degrades alone and its siblings parse; a failed declaration header degrades the literal. One error per degraded region; nothing inside a degraded region reports separately.
- The parser carries no prose. Messages are `Display` impls on the error types; contextual suggestions belong to the rendering stage, keyed off the `(expected, found)` pair.

## Totality

- The stage never panics, on any input. `unwrap`, `expect`, `unreachable!`, and type-level infallibility claims are banned in production code; where an invariant is real but unprovable to the compiler, the code takes the graceful fallback and the invariant is stated in the doc comment.
- Every input yields a tree, and every position in the literal resolves to some node: parsed regions to grammar leaves, degraded regions through their retained chunks, uncovered whitespace to the nearest container.

## Trees, spans, and resolution

- Whether a type is span-carrying is decided at the type: a tree enum is wrapped in `WithSpan` once at its slot, variant payloads are bare, and every struct field carries its own `WithSpan`. Each wrapper's coverage is stated on the type (a selection set's span covers its braces; an item's span is its `contents_span`).
- Names are spans, held by fieldless marker structs, one per role: an alias is not a name, an argument name is not an object key. The tree stores no strings and interns nothing; the one derived scalar is the converted `i64`, kept because deferring the conversion moves the overflow failure away from its source. Punctuation and keywords that never resolve on their own (`Dot`, `Dollar`, `Exclamation`, the keyword markers) are unmarked fields and answer their container.
- `ResolvePosition` is derive-only; a manual impl means a missing feature in the `resolve_position` crate and becomes a prefactor there. A type's parent is a direct path alias while it has one parent and becomes an enum at the second; unparsed nodes keep chunk-stage data reachable, so the chunk-stage variants of `IsographResolutionNode` stay alive inside degraded regions only.

## Performance

- One pass. Each chunk's items are walked once, by reference; the output tree copies only spans and `Copy` tokens. Cloning happens only when a region degrades, so allocation beyond the output vecs is proportional to the error count, and an error-free parse allocates nothing but the tree.
- No backtracking, by construction (`ChunkStream` has no rewind); parse time is linear in the token count with no reparse of any region.
- The stage stays pico-free and interning-free: plain functions over `LiteralText` and the chunk tree, per the crate's standing assumption that parsing one literal is trivially cheap.

## Relation to the upstream parser

Each standard above has a counterpart in upstream isograph's `isograph_lang_parser` (`parse_iso_literal.rs`, `peekable_lexer.rs`), where the constraint exists as convention or comment; here it is a rule, held by a type where one can hold it.

- Function shapes. Upstream has one primitive, `parse_token_of_kind`, serving both the required and the optional case: callers write `...?` for the first and `if ....is_ok()` for the second, so the failure contract lives at each call site. The `require_*` / `consume_*` split puts that contract in the signature; `parse_delimited_list` is the ancestor of `parse_level_items`, and `PeekableLexer` the ancestor of `ChunkStream`.
- Backtracking. Upstream dispatches alternatives through `to_control_flow` chains that stay safe only while every alternative fails on its first, unconsumed token; the code cannot enforce that, and `parse_type_annotation` carries the comment admitting it: adding a case after the open bracket has been eaten "will leave the parser in an inconsistent state". Here `ChunkStream` offers no rewind and no raw peek, so that parser is unwritable.
- Totality. Upstream is fail-fast (`DiagnosticResult`, first error aborts the literal) and panics on inputs it did not expect: `number.parse().expect(...)` on integer overflow, `unreachable!()` in the block-string lexer. Here every input yields a tree, degradation is local, and panics are banned.
- Errors. Upstream errors are prose `String`s built inline throughout the parser, some with `Span::todo_generated()` where no span was threaded ("TODO get a span"). Here errors are structured (`Expected`/`Found`), a span is present by construction, and prose exists only in `Display`.
- Locations and inputs. Upstream threads `TextSource` and extraction context through the parse, with its own comment noting the cost ("we break memoization, due to this parameter"), and interns names as it goes. Here the parse is a function of `LiteralText` and the chunk tree, spans only, no interning, extraction elsewhere.
- Separators. Upstream's lexer skips line breaks as whitespace and recovers them by inspecting the skipped text (`parse_line_break` reads `white_space_span`), and each list re-implements its delimiter policy via `parse_comma_or_line_break`. Here line breaks are tokens, separator policy lives once in the chunking pass, and the grammar stage consumes structure through `LevelEntry`.
- Semantic tokens. Upstream accumulates them inside the lexer, one legend constant per parse call. Here they are absent from parsing and derivable from the finished tree.

## Amending

When a feature doc lands, its implementation is reviewed against this doc. An implementation need that this doc forbids is a decision point: either the code bends to the standard, or the standard is amended here, explicitly, in the same review. The expected amendment sites are the impl blocks of the enforcement structures: a new `ChunkStream` method, a new `LiteralText` read, a new `Chunk` accessor. A change that routes around a structure instead of extending it is the thing this doc exists to prevent.
