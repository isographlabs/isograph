# Parsing standards

The rules every parser in the grammar stage follows, and the data structures that enforce them. The feature docs define what parses; this doc defines how parsing code is written, what it may and may not do, and which types make violations unwritable rather than merely disallowed. The intent is to concentrate review here: the enforcement structures each have one impl block, reviewed once, and a feature parser built from them has few ways to be wrong. Each feature implementation is reviewed against this doc when it lands, and a deviation is resolved by amending this doc first or fixing the code, never by shipping the deviation silently.

This doc assumes synthetic-closing.md's world: the bracket matcher returns its tree beside its errors, every open bracket forms a group (really or synthetically closed), and stray closes are extracted, so no unmatched-bracket state exists anywhere downstream of the matcher.

## The input is already structure

By the time the grammar stage runs, three passes have shaped the input: tokens, groups, chunks. A parser here never sees characters, brackets, or separators; it sees a chunk's items, and that changes how everything below works.

The unit of parsing is the chunk, recursively. A chunk is a vec of chunked and matched items, its tokens and its groups, and each such vec is read by exactly one `SafePeekable`, behind that chunk's `ChunkStream`. The literal is itself one chunk, the root level's only one, and a chunk's groups hold levels of further chunks, each parsed independently of every other. Nothing in the stage ever holds a cursor over more than one vec; the recursion is "a chunk parses, and hands each interior chunk to its own parse."

- A group is one item. Consuming it consumes its whole extent in a single step, interior included, and the interior re-enters parsing only as fresh levels, each chunk behind its own stream. No cursor ever stands inside a group it did not open. Whether the group's close was real or synthetic is invisible here; a group is a group.
- Structured items arrive pre-spanned. A group's span was computed when the matcher closed it; a token's span came from the lexer. Parse code therefore computes spans only for multi-item composites, which is all `spanning` exists for; everything else carries the span it already has.
- Brackets are not a parsing concern, in any form. The matcher resolved every one: matched and synthetic opens are groups, stray closes were extracted with their errors riding beside the tree. No item kind for a bracket problem exists for a parser to meet.
- Separators do not exist here. Chunking absorbed them into boundaries, so "a separator comes next" is the chunk simply ending: `take_next` returning `None`.

## The function shapes

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

    /// The next item, consumed, when it is a group opened by `kind`; `None`, nothing
    /// consumed, otherwise.
    pub(crate) fn consume_group_if(
        &mut self,
        kind: BracketKind,
    ) -> Option<WithSpan<&'a ChunkedGroup>>;

    /// The next item, committed, or `None` at the chunk's end: the value a dispatch
    /// position matches on. Total: no error case exists here; errors are the caller's
    /// to construct. The exhaustive match forces the `None` arm, so handling the end
    /// cannot be forgotten.
    pub(crate) fn take_next(&mut self) -> Option<&'a WithSpan<ChunkContentItem>>;

    /// The empty span at `previous_end`: where a missing item belongs, for error arms
    /// that found `EndOfChunk`.
    pub(crate) fn end_span(&self) -> Span;

    /// Runs a sub-parse and wraps its result in the span it consumed: from the start
    /// of the first item the closure accepts to the end of its last, empty at
    /// `previous_end` when it accepts none. The source of a composite node's span; a
    /// parser never joins spans by hand.
    pub(crate) fn spanning<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, WithSpan<ParseError>>,
    ) -> Result<WithSpan<T>, WithSpan<ParseError>>;

    /// Nothing further may exist. The first leftover item is the error.
    pub(crate) fn require_end(&mut self, expected: Expectation) -> Result<(), WithSpan<ParseError>>;
}
```

What this discharges: consumption discipline (peek-commit, failure leaves the offender in place, errors carry the right span) is written once here instead of once per parser, and `previous_end` anchors every missing-item error, so a wrong anchor cannot be written. `SafePeekable` underneath has no rewind, and `ChunkStream` exposes no raw peek, so one-peek-decides holds structurally: an item commits only when a method accepted it. `take_next` yields the chunk's own item type; no shadow taxonomy stands between the tree and the parser.

Spans follow the same rule as positions: a leaf's span is what `require_token` returned, an item's span is the extent its parse consumed (a degraded slot's is its chunk's `contents_span`), and a composite's span comes from `spanning`, upstream's `with_embedded_location_result` reborn without the location baggage. A hand-written `Span::join` in a parser is the anti-pattern; if a node's span cannot come from one of those sources, that is a missing `ChunkStream` capability and an amendment here. Construction stays the landed passes' style, `WithSpan::new` and plain `Ok`/`Some`; upstream's postfix sugar (`wrap_ok`, `with_span`) is not adopted.

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

What this discharges: the empty-chunk-is-an-error rule cannot be skipped, because a walker cannot see chunks without matching `CommaWithoutItem`, and the comma's span is computed once in chunk.rs rather than re-derived per site. `parse_level_items` and the one-item walkers are the only intended callers; a new walker is at least forced through the classification.

### `Chunk`'s narrowed surface

`Chunk`'s fields become private to chunk.rs, with exactly the methods parsing needs:

```rust
// from crates/isograph_parser/src/chunk.rs
impl Chunk {
    /// The stream a parser reads this chunk through.
    pub fn stream(&self) -> ChunkStream<'_>;

    /// The span of the contents, without the boundary: a degraded slot's span.
    pub fn contents_span(&self) -> Option<Span>;

    /// The comma in the trailing boundary, when one exists. Only a list gives a
    /// boundary comma meaning; one-item contexts call this to reject it
    /// (no-final-comma.md).
    pub fn boundary_comma(&self) -> Option<Span>;
}
```

What this discharges: no code outside chunk.rs can index a chunk's items, read its separator tokens, or invent a boundary inspection beyond the sanctioned two, which are these methods' bodies.

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
}
```

What this discharges: the exhaustive list of text reads is one impl block, keyword dispatch becomes a match on `Keyword` instead of string comparison at call sites, and a future parser cannot quietly start reading text. parse-arguments.md amends this block with `value_word` (`true`/`false`/`null`) and `integer` (`None` on out of range); `spanning`'s siblings (`spanning_from` for opener-anchored composites, an optional or plainly-returning form) wait for their first callers, and under the first-item-decides law the optional form has none: a single optional item carries its own span, and an opener-marked composite is require-flow past its opener. Anticipated amendments live here in the doc, never as comments in the code.

## Dispatch is a match

A position where the grammar allows one of several forms is written as one `match` on `take_next()`, never as a chain of `consume_*_if` attempts. The discriminating item is committed up front, which is sound because every arm uses it: the accepting arms continue from it, and the rejecting arm reports it as the `found`. The match is exhaustive, so handling the chunk's end cannot be forgotten, and a separator can never appear in an arm, because chunks are separator-free: "a separator comes next" is the `None`. Sketched on a value:

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

Dispatch on an identifier's text is the same shape one level down: `require_token(Identifier, ...)` then a `match` on `LiteralText::keyword` (or `value_word`), so `entrypoint` versus `field` versus `pointer` is a match on the `Keyword` enum, not string comparisons scattered through arms.

The `consume_*_if` shape is not a dispatch tool. It exists for one case: a composition boundary, where a sub-parser meets an item that is not its own and must decline without consuming what belongs to its caller (the optional `!` after a type name, whose absence might be the caller's `=` or the chunk's end). Inside a production that owns all the alternatives at a position, reaching for a `consume_*_if` chain instead of a `take_next` match is the anti-pattern.

An opener-marked composite, one that is optional as a whole but required once its opener appears, is a dispatch arm, never a consume chain: the opener commits in the match and the remainder is `require_*`. The variable use is the existing instance (`$` commits, the name is required), and a future directive (`@ name (args)`) is the same shape, looping its position's match for repetition. Such a composite's span starts at the already-committed opener, which plain `spanning` cannot cover; that is `spanning_from`'s waiting caller.

The law under these shapes is no-rewind stated as grammar design: an optional construct is decided by its first item. A single optional item (a token, a group) is a `consume_*_if`, infallible; a multi-item optional commits its opener and is fallible from its second item on (`@@` errors at the second `@`). The one other legal shape is commit-and-reinterpret, the alias's colon deciding what the committed identifier was, and it is legal only because every continuation uses everything committed. Consume-and-decline does not exist, so a future grammar addition whose optionality needs more than one item of lookahead is not writable here, by construction.

## Level walks

- A list level is walked only by `parse_level_items`. It consumes `entries()`, turns `CommaWithoutItem` into the missing-item unparsed item, runs `parse_item` on a fresh `stream()`, and itself calls `require_end(Expectation::Separator)` after a successful item, so an item parser cannot forget the leftover check: item parsers parse their production and stop, and exhaustion is the walker's job.
- Trailing junk does not void a completed item. When `parse_item` succeeds and `require_end` then fails, the walker keeps the parsed item and records the junk as the slot's trailing error: `foo bar` is the selection `foo` plus an error at `bar`, and the LSP's go-to-definition, find-references, and completion see `foo` exactly as if the junk were absent. The item's span covers only what its parse consumed, so the junk's positions answer the containing level. A chunk degrades to unparsed only when its production fails before completing.
- Junk is always the suffix. Nothing after the first leftover is reattached, since nothing is ever skipped: `foo bar { baz }` is the scalar `foo` with junk from `bar` to the chunk's end, never an object selection, and `baz` is not in the grammar tree.
- A one-item context (the root level, a `[...]` interior) has its own walker (`declaration_chunk`, `parse_bracket_interior_type`), which enforces exactly one `Item`, errors on `CommaWithoutItem`, and rejects `boundary_comma` per no-final-comma.md. The end-of-chunk check is likewise the walker's, with its context's expectation (`EndOfDeclaration`, `EndOfType`).

## Failure isolation

One chunk to one item, and the item is a result: every chunk parses in its entirety (the walker requires exhaustion), always independently (its own stream), to exactly one output slot, holding the parsed item, beside any trailing-junk error, or the unparsed reason. A chunk therefore fails without affecting any other chunk, held by three mechanisms:

- A `ChunkStream` is built from one chunk and cannot read past it: separators and sibling chunks are not in it. There is no shared cursor to leave in a bad state, which is upstream's resynchronization problem (one `PeekableLexer` over the whole literal, so a failed production leaves the lexer wherever it stopped and everything after is suspect). Chunking pre-cut the input, so the recovery points are structural, not searched for.
- `parse_level_items` returns `Vec<WithSpan<T>>`, not `Result`. The signature is the enforcement: an item parser's `Err` has nowhere to go but the walker's `unparsed` conversion, so a `?` cannot leak one chunk's failure into its siblings or its level. Errors escape only the one-item walkers, where the failed item is the whole context, and the stated granularity applies: the literal at the root, the containing item for a `[...]` inside a variable declaration.
- Every `LevelEntry` yields exactly one output item: `Item` parses (possibly beside a trailing-junk error) or degrades to unparsed, `CommaWithoutItem` degrades to unparsed. The output length equals the entry count, so a failure cannot shift a sibling's position, and a degraded slot still resolves through its retained chunk.

## Errors

- An error is a `WithSpan<ParseError>`, and the workhorse is `Expected(ExpectedFound { expected, found })`. The span covers the offending item, or is empty at the position the missing item belonged, which `ChunkStream` computes.
- Errors live in the tree (`UnparsedLiteral`, `UnparsedItem`, the trailing-junk slot), and `errors()` derives the list from the tree in source order. There is no error list beside the grammar tree.
- Bracket errors are not the grammar's errors at all. The matcher returned them beside its tree (synthetic-closing.md), the one report each; no `ParseError` variant names a bracket. The combined diagnostics, the matcher's vec plus the grammar's `errors()`, are the final sweep: an error-free literal is one where both are empty.
- Degradation is as local as the grammar allows: a failed list chunk degrades alone and its siblings parse; a failed declaration header degrades the literal. One error per degraded region; nothing inside a degraded region reports separately.
- The parser carries no prose. Messages are `Display` impls on the error types; contextual suggestions belong to the rendering stage, keyed off the `(expected, found)` pair.

## Totality

- The stage never panics, on any input. `unwrap`, `expect`, `unreachable!`, and type-level infallibility claims are banned in production code; where an invariant is real but unprovable to the compiler, the code takes the graceful fallback and the invariant is stated in the doc comment.
- Every input yields a tree, and every position in the literal resolves to some node: parsed regions to grammar leaves, degraded regions through their retained chunks, uncovered whitespace (and an extracted stray close's position) to the nearest container.

## Trees, spans, and resolution

- Whether a type is span-carrying is decided at the type: a tree enum is wrapped in `WithSpan` once at its slot, variant payloads are bare, and every struct field carries its own `WithSpan`. Each wrapper's coverage is stated on the type (a selection set's span covers its braces; an item's span is what its parse consumed).
- Names are spans, held by fieldless marker structs, one per role: an alias is not a name, an argument name is not an object key. The tree stores no strings and interns nothing; the one derived scalar is the converted `i64`, kept because deferring the conversion moves the overflow failure away from its source. Punctuation and keywords that never resolve on their own (`Dot`, `Dollar`, `Exclamation`, the keyword markers) are unmarked fields and answer their container.
- `ResolvePosition` is derive-only; a manual impl means a missing feature in the `resolve_position` crate and becomes a prefactor there. A type's parent is a direct path alias while it has one parent and becomes an enum at the second; unparsed nodes keep chunk-stage data reachable, so the chunk-stage variants of `IsographResolutionNode` stay alive inside degraded regions only.

## Performance

- One pass. Each chunk's items are walked once, by reference; the output tree copies only spans and `Copy` tokens. Cloning happens only when a region degrades, so allocation beyond the output vecs is proportional to the error count, and an error-free parse allocates nothing but the tree.
- No backtracking, by construction (`ChunkStream` has no rewind); parse time is linear in the token count with no reparse of any region.
- The stage stays pico-free and interning-free: plain functions over `LiteralText` and the chunk tree, per the crate's standing assumption that parsing one literal is trivially cheap.

## Relation to the upstream parser

Each standard above has a counterpart in upstream isograph's `isograph_lang_parser` (`parse_iso_literal.rs`, `peekable_lexer.rs`), where the constraint exists as convention or comment; here it is a rule, held by a type where one can hold it.

- Function shapes. Upstream has one primitive, `parse_token_of_kind`, serving both the required and the optional case: callers write `...?` for the first and `if ....is_ok()` for the second, so the failure contract lives at each call site. The `require_*` / `consume_*` split puts that contract in the signature; `parse_delimited_list` is the ancestor of `parse_level_items`, `PeekableLexer` the ancestor of `ChunkStream`, and `with_embedded_location_result` the ancestor of `spanning`, minus the `TextSource` it had to thread.
- Backtracking. Upstream dispatches alternatives through `to_control_flow` chains that stay safe only while every alternative fails on its first, unconsumed token; the code cannot enforce that, and `parse_type_annotation` carries the comment admitting it: adding a case after the open bracket has been eaten "will leave the parser in an inconsistent state". Here `ChunkStream` offers no rewind and no raw peek, so that parser is unwritable.
- Totality. Upstream is fail-fast (`DiagnosticResult`, first error aborts the literal) and panics on inputs it did not expect: `number.parse().expect(...)` on integer overflow, `unreachable!()` in the block-string lexer. Here every input yields a tree, degradation is local, and panics are banned.
- Errors. Upstream errors are prose `String`s built inline throughout the parser, some with `Span::todo_generated()` where no span was threaded ("TODO get a span"). Here errors are structured (`Expected`/`Found`), a span is present by construction, and prose exists only in `Display`.
- Locations and inputs. Upstream threads `TextSource` and extraction context through the parse, with its own comment noting the cost ("we break memoization, due to this parameter"), and interns names as it goes. Here the parse is a function of `LiteralText` and the chunk tree, spans only, no interning, extraction elsewhere.
- Separators and brackets. Upstream's lexer skips line breaks as whitespace and recovers them by inspecting the skipped text (`parse_line_break` reads `white_space_span`), each list re-implements its delimiter policy via `parse_comma_or_line_break`, and a bracket mistake surfaces wherever a `parse_token_of_kind` happens to trip over it. Here line breaks are tokens, separator policy lives once in the chunking pass, bracket policy lives once in the matcher, and the grammar stage consumes structure through `LevelEntry`.
- Semantic tokens. Upstream accumulates them inside the lexer, one legend constant per parse call. Here they are absent from parsing and derivable from the finished tree.

## Amending

When a feature doc lands, its implementation is reviewed against this doc. An implementation need that this doc forbids is a decision point: either the code bends to the standard, or the standard is amended here, explicitly, in the same review. The expected amendment sites are the impl blocks of the enforcement structures: a new `ChunkStream` method, a new `LiteralText` read, a new `Chunk` accessor. A change that routes around a structure instead of extending it is the thing this doc exists to prevent.
