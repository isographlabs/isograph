# Parsing standards

The rules every parser in the grammar stage follows. The feature docs (parse-entrypoint.md through no-final-comma.md) define what parses; this doc defines how parsing code is written, what it may and may not do, and which of those constraints the types enforce versus which are discipline. Each feature implementation is reviewed against this doc when it lands, and a deviation is resolved by amending this doc first or fixing the code, never by shipping the deviation silently.

## The three function shapes

Every parser function is one of three shapes, and its prefix states its contract. A function that fits none of these does not belong in the stage.

```rust
// A required piece of grammar. Errors on the found item, unconsumed, or at an empty
// span at `missing_at` when the chunk ran out. Consumes exactly the accepted items.
fn expect_x(items: &mut ChunkContents<'_>, ..., missing_at: u32) -> Result<X, WithSpan<ParseError>>

// An optional piece of grammar. Consumes and returns it when the next item opens it;
// consumes nothing and returns None otherwise. Never errors on absence.
fn consume_x(items: &mut ChunkContents<'_>, ...) -> Option<X>

// A composite production, built from expect_* and consume_* calls. Errors propagate
// from the first failing piece.
fn parse_x(...) -> Result<X, WithSpan<ParseError>>
```

The primitive set is closed: `expect_token`, `consume_token_if`, `expect_chunk_end`, `token_text`, `empty_chunk_comma_span`, `boundary_comma`, and `parse_level_items`. A new primitive is an amendment to this doc, not a local helper.

## Consumption discipline

- Parsers read a chunk's items through `SafePeekable` (`ChunkContents`), and only through it. `SafePeekable` has no rewind, and that is the enforcement: a committed item can never be un-consumed, so a backtracking parser cannot be written against it. Re-creating an iterator over items already walked is banned.
- One peek decides. Alternatives (is this value a variable, a string, an object?) are distinguished by the single next unconsumed item. An item is committed only when every continuation uses it: the selection parser may commit an identifier before knowing whether it is the alias or the name, because both continuations use it; it may not commit an item that some continuation would need to give back.
- Failure leaves the offender in place. `expect_*` on a wrong item errors without consuming it, so the error's span and any caller both see the same item.

## Level walks

- A list level (a selection set, a paren list, an object literal) is walked only by `parse_level_items`. It is the single place that turns chunks into items, turns a failed chunk into an unparsed item holding the reason and a clone of the chunk, and turns an empty chunk into the missing-item error at its comma. A second walk of a list level may not be written; a list behavior change is a change to `parse_level_items`.
- A one-item context (the root level, a `[...]` interior) is walked by its own function (`declaration_chunk`, `parse_bracket_interior_type`), which enforces exactly-one-contentful-chunk and, per no-final-comma.md, no boundary comma.
- These walk functions are the only code that touches `ChunkedLevel`'s chunks. This is discipline, not a type guarantee; if it is ever violated, the fix is to newtype the access, and that change lands through this doc.

## Boundaries and text

- The grammar stage never inspects a contentful chunk's boundary, with one sanctioned exception: `boundary_comma` in one-item contexts. `empty_chunk_comma_span` reads only an empty chunk's boundary. No other code touches `ChunkSeparator`.
- The literal's text is read only through `token_text`, and only for: the declaration keywords (`entrypoint`, `field`, `pointer`), the `to` keyword, the value words (`true`, `false`, `null`), and the `i64` conversion of an integer literal. Every other decision is made on token kinds and structure. Adding a text read is an amendment.
- Names are spans. The tree stores no strings and interns nothing; the one derived scalar is the converted `i64`, kept because deferring the conversion moves the overflow failure away from its source.

## Errors

- An error is a `WithSpan<ParseError>`, and the workhorse is `Expected(ExpectedFound { expected, found })`. The span covers the offending item, or is empty at the position a missing item was expected (the end of the previous accepted span, threaded as `missing_at`).
- Errors live in the tree (`UnparsedLiteral`, `UnparsedItem`), and `errors()` derives the list from the tree in source order. There is no error list beside the tree.
- Degradation is as local as the grammar allows: a failed list chunk degrades alone and its siblings parse; a failed declaration header degrades the literal. One error per degraded region; nothing inside a degraded region reports separately.
- The parser carries no prose. Messages are `Display` impls on the error types; contextual suggestions belong to the rendering stage, keyed off the `(expected, found)` pair.

## Totality

- The stage never panics, on any input. `unwrap`, `expect`, `unreachable!`, and type-level infallibility claims are banned in production code; where an invariant is real but unprovable to the compiler, the code takes the graceful fallback (`empty_chunk_comma_span` falls back to the chunk's span) and the invariant is stated in the doc comment.
- Every input yields a tree, and every position in the literal resolves to some node: parsed regions to grammar leaves, degraded regions through their retained chunks, uncovered whitespace to the nearest container.

## Trees, spans, and resolution

- Whether a type is span-carrying is decided at the type: a tree enum is wrapped in `WithSpan` once at its slot, variant payloads are bare, and every struct field carries its own `WithSpan`. Each wrapper's coverage is stated on the type (a selection set's span covers its braces; an item's span covers its chunk's contents, not its boundary).
- Names are fieldless marker structs, one per role: an alias is not a name, an argument name is not an object key. Punctuation and keywords that never resolve on their own (`Dot`, `Dollar`, `Exclamation`, the keyword markers) are unmarked fields and answer their container.
- `ResolvePosition` is derive-only; a manual impl means a missing feature in the `resolve_position` crate and becomes a prefactor there. A type's parent is a direct path alias while it has one parent and becomes an enum at the second; unparsed nodes keep chunk-stage data reachable, so the chunk-stage variants of `IsographResolutionNode` stay alive inside degraded regions only.

## Performance

- One pass. Each chunk's items are walked once, by reference; the output tree copies only spans and `Copy` tokens. Cloning happens only when a region degrades, so allocation beyond the output vecs is proportional to the error count, and an error-free parse allocates nothing but the tree.
- No backtracking, by the consumption discipline above; parse time is linear in the token count with no reparse of any region.
- The stage stays pico-free and interning-free: plain functions over `&str` and the chunk tree, per the crate's standing assumption that parsing one literal is trivially cheap.

## Amending

When a feature doc lands, its implementation is reviewed against this doc. An implementation need that this doc forbids is a decision point: either the code bends to the standard, or the standard is amended here, explicitly, in the same review. The closed lists above (the primitives, the text reads, the boundary inspections, the level walks) are the places amendments are expected to touch.
