# one-comma-per-boundary: chunk boundaries hold at most one comma

A prefactor to the series parsing-plan.md orders. The chunking pass's boundary phase stops before a second comma, so every `ChunkSeparator` holds any number of line breaks and at most one comma: the boundary grammar is `line-break+` or `line-break* comma line-break*`. A second comma opens the next chunk's boundary, and the chunk between the two commas is empty, so a doubled comma surfaces structurally and the grammar stage can report it as a missing item without ever inspecting a boundary's tokens.

Chunking stays infallible and `ChunkSeparator`'s shape stays `Vec<WithSpan<SeparatorToken>>`; the invariant is established by construction in the one function that builds separators. Nothing downstream reads a boundary's tokens to parse (the empty chunk carries the signal), so the vec stays the representation.

An empty chunk therefore appears in two places instead of one: as a level's first chunk holding leading separators (as today), and mid-level marking a doubled comma (new).

## The absorption change

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
    let mut separators = Vec::new();
    while let Some(peek) = items.peek() {
        let Some(separator) = separator_of(peek.view()) else {
            break;
        };
        let item = peek.commit();
        separators.push(WithSpan::new(separator, item.location));
    }
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
    let mut separators: Vec<WithSpan<SeparatorToken>> = Vec::new();
    while let Some(peek) = items.peek() {
        let Some(separator) = separator_of(peek.view()) else {
            break;
        };
        if separator == SeparatorToken::Comma
            && separators.iter().any(|absorbed| absorbed.item == SeparatorToken::Comma)
        {
            // The second comma opens the next chunk's boundary.
            break;
        }
        let item = peek.commit();
        separators.push(WithSpan::new(separator, item.location));
    }
```

The loop in `chunk_level` still always advances: a chunk that starts at the stopped-before comma has an empty separator list, so its boundary phase absorbs that comma as its first.

## Doc comments

`ChunkedLevel`, before:

```rust
// from crates/isograph_parser/src/chunk.rs
/// One level of the chunk tree: the whole literal at the root, a group's interior
/// below — the same role `MatchedBrackets` has on the bracket tree. Its chunks, in
/// order, and nothing else. A level that opens with separators holds them in a first
/// chunk with no contents: an admittedly suboptimal encoding, accepted as the price of
/// a level being a plain vec of one uniform chunk shape.
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
/// One level of the chunk tree: the whole literal at the root, a group's interior
/// below — the same role `MatchedBrackets` has on the bracket tree. Its chunks, in
/// order, and nothing else. A chunk with no contents holds a boundary no content
/// preceded: a level that opens with separators holds them in an empty first chunk, and
/// a second comma in one boundary opens an empty chunk mid-level. An admittedly
/// suboptimal encoding, accepted as the price of a level being a plain vec of one
/// uniform chunk shape.
```

`Chunk`, before:

```rust
// from crates/isograph_parser/src/chunk.rs
/// A maximal separator-free run of a level's items — tokens, groups, unmatched
/// brackets, anything — plus the separator run that ended it when one did. The
/// chunk-parsing pass consumes it as one unit. Every chunk but a level's last has a
/// trailing separator by construction; the last's is the optional trailing delimiter.
/// The wrapping `WithSpan`'s span runs from the first part's start to the last part's
/// end.
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
/// A maximal separator-free run of a level's items — tokens, groups, unmatched
/// brackets, anything — plus the boundary that ended it when one did: line breaks and
/// at most one comma, a second comma ending the boundary as well. The chunk-parsing
/// pass consumes it as one unit. Every chunk but a level's last has a trailing
/// separator by construction; the last's is the optional trailing delimiter.
/// The wrapping `WithSpan`'s span runs from the first part's start to the last part's
/// end.
```

`ChunkSeparator`, before:

```rust
// from crates/isograph_parser/src/chunk.rs
/// The boundary that ended its chunk, holding every comma and line-break token it
/// absorbed, in order. Its tokens are not resolution leaves; a position on any of them
/// answers the separator.
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
/// The boundary that ended its chunk: its line-break tokens and at most one comma, in
/// order. A second comma is never absorbed; it opens the next chunk's boundary. Its
/// tokens are not resolution leaves; a position on any of them answers the separator.
```

`absorb_chunk`, before:

```rust
// from crates/isograph_parser/src/chunk.rs
/// One chunk from a nonempty stream: the content phase, then the boundary phase.
/// Whichever phase matches the first item consumes it — every item is either content
/// or a separator — so the chunk has at least one part and its span exists. Group
/// interiors recurse in the content phase's `Bracketed` arm.
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
/// One chunk from a nonempty stream: the content phase, then the boundary phase.
/// Whichever phase matches the first item consumes it — every item is either content
/// or a separator — so the chunk has at least one part and its span exists; the
/// boundary phase stops before a second comma, and absorbs at least the comma it
/// starts at when the chunk opens on one. Group interiors recurse in the content
/// phase's `Bracketed` arm.
```

## Tests

The multi-separator tail of `commas_and_line_breaks_are_equivalent_separators` asserted that `a,\n\n,b` produces two chunks with a four-token boundary; that input now produces three. The test keeps its equivalence half and drops the tail:

```rust
// from crates/isograph_parser/src/chunk.rs
    #[test]
    fn commas_and_line_breaks_are_equivalent_separators() {
        let comma = chunked("a, b");
        let linebreak = chunked("a\nb");
        assert_eq!(comma.item.0.len(), 2);
        assert_eq!(linebreak.item.0.len(), 2);
        assert_eq!(
            separator_kinds(&comma.item.0[0].item.trailing_separator.as_ref().unwrap().item),
            vec![SeparatorToken::Comma]
        );
        assert_eq!(
            separator_kinds(
                &linebreak.item.0[0].item.trailing_separator.as_ref().unwrap().item
            ),
            vec![SeparatorToken::LineBreak]
        );
        assert!(comma.item.0[1].item.trailing_separator.is_none());
        assert!(linebreak.item.0[1].item.trailing_separator.is_none());
    }
```

The tail's input moves to a new test with the new expectations, anchored on the second comma's span:

```rust
// from crates/isograph_parser/src/chunk.rs
    #[test]
    fn a_second_comma_ends_the_boundary_and_leaves_an_empty_chunk() {
        let text = "a,\n\n,b";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 3);
        let first = &tree.item.0[0].item;
        assert_eq!(render_chunk(text, first), "a,\n\n");
        assert_eq!(
            separator_kinds(&first.trailing_separator.as_ref().unwrap().item),
            vec![
                SeparatorToken::Comma,
                SeparatorToken::LineBreak,
                SeparatorToken::LineBreak,
            ]
        );
        let middle = &tree.item.0[1];
        assert!(middle.item.contents.is_empty());
        assert_eq!(
            separator_kinds(&middle.item.trailing_separator.as_ref().unwrap().item),
            vec![SeparatorToken::Comma]
        );
        let second_comma = span_of(text, ",b");
        assert_eq!(middle.location, Span::new(second_comma.start, second_comma.start + 1));
        assert_eq!(render_chunk(text, &tree.item.0[2].item), "b");
    }

    #[test]
    fn doubled_leading_commas_leave_two_empty_chunks() {
        let text = ",,a";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 3);
        assert!(tree.item.0[0].item.contents.is_empty());
        assert!(tree.item.0[1].item.contents.is_empty());
        assert_eq!(render_chunk(text, &tree.item.0[2].item), "a");
    }
```

`leading_separators_make_an_empty_first_chunk` (`\n, a, b,\n`) holds one comma per boundary and is untouched, as is every other chunk test.

## Landing checklist

1. The absorption change, the comment updates, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past. parse-entrypoint.md builds on the invariant this doc lands.

## Notes

- In later stages, empty chunks become parse errors, and this should be unambiguous
