# one-comma-per-boundary: a comma must follow an item

A prefactor to the series parsing-plan.md orders. Two changes to the chunking pass establish one invariant: an empty chunk exists exactly when a comma has no item before it, and its boundary starts with that comma. An empty chunk is therefore unambiguously a parse error to the grammar stage, with no boundary inspection and no first-chunk special case.

1. An opening bracket captures the line breaks directly after it, and the literal's start does the same for the root level: `chunk_level` consumes them before absorbing chunks, emitting nothing. `foo {\n}` and a literal opening with a line break produce no empty chunk, and a position on a captured line break resolves to the interior level, exactly as a space there does. The opening's span stays the bare bracket.
2. The boundary phase stops before a second comma, so every `ChunkSeparator` holds any number of line breaks and at most one comma: the boundary grammar is `line-break+` or `line-break* comma line-break*`. The refused comma opens the next chunk, which is empty when nothing sits between the commas.

A leading comma (`{, bar }`, `{,}`) also opens an empty chunk: capture consumes only line breaks, so a comma with no item before it always lands as the first boundary token of an empty chunk, whether at a level's start or between two commas.

Chunking stays infallible, and `ChunkedLevel` and `ChunkSeparator` keep their shapes; both invariants are established by construction in the functions that consume separators.

## The absorption changes

`chunk_level` captures before the chunk loop:

```rust
// from crates/isograph_parser/src/chunk.rs
fn chunk_level(level: &MatchedBrackets) -> ChunkedLevel {
    let mut items = level.0.iter().safe_peekable();
    skip_captured_line_breaks(&mut items);
    let mut out = Vec::new();
    while items.peek().is_some() {
        out.push(absorb_chunk(&mut items));
    }
    ChunkedLevel(out)
}

/// Line breaks directly after an opening bracket, or at the literal's start for the
/// root, are captured: consumed as insignificant whitespace, never a boundary, so a
/// level's first boundary token is content or a comma. A comma is never captured; a
/// comma with no item before it opens an empty chunk.
fn skip_captured_line_breaks(items: &mut LevelItems<'_>) {
    while let Some(peek) = items.peek() {
        if separator_of(peek.view()) != Some(SeparatorToken::LineBreak) {
            break;
        }
        peek.commit();
    }
}
```

`absorb_chunk`'s boundary phase stops before a second comma. Before:

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

The loop in `chunk_level` still always advances: a chunk that starts at a refused or leading comma has an empty separator list, so its boundary phase absorbs that comma as its first token. That token is the chunk's first part, which is why an empty chunk's boundary always opens with its comma.

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
/// order, and nothing else. Line breaks at the level's start are captured by the
/// opening bracket (or the literal's start) and appear nowhere, so a chunk with no
/// contents is always a comma no item precedes, holding that comma as its boundary's
/// first token.
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

`leading_separators_make_an_empty_first_chunk` (`\n, a, b,\n`) described the old encoding and is replaced by one test per side of the new invariant:

```rust
// from crates/isograph_parser/src/chunk.rs
    #[test]
    fn line_breaks_at_a_levels_start_are_captured_and_make_no_chunk() {
        let text = "\n\na, b\n";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 2);
        assert_eq!(render_chunk(text, &tree.item.0[0].item), "a,");
        assert_eq!(tree.location, Span::from_usize(0, text.len()));

        let interior = "foo {\n bar\n}";
        let tree = chunked(interior);
        let brace = as_group(content_item(&tree.item.0[0].item, 1));
        assert_eq!(brace.children.item.0.len(), 1);
        assert_eq!(render_chunk(interior, &brace.children.item.0[0].item), "bar\n");
    }

    #[test]
    fn a_comma_before_the_first_item_opens_an_empty_chunk() {
        let text = "\n, a";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 2);
        let empty = &tree.item.0[0];
        assert!(empty.item.contents.is_empty());
        assert_eq!(
            separator_kinds(&empty.item.trailing_separator.as_ref().unwrap().item),
            vec![SeparatorToken::Comma]
        );
        assert_eq!(empty.location, span_of(text, ","));
        assert_eq!(render_chunk(text, &tree.item.0[1].item), "a");
    }

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

    #[test]
    fn a_captured_line_break_resolves_to_its_level() {
        let text = "\nfoo {\n bar }";
        let tree = chunked(text);
        match tree.resolve(ChunkedLevelParent::Root, Span::new(0, 1)) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Root));
            }
            node => panic!("expected the root level, got {node:?}"),
        }
        let opening = span_of(text, "{\n");
        match tree.resolve(ChunkedLevelParent::Root, Span::new(opening.start + 1, opening.end)) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Interior(_)));
            }
            node => panic!("expected the interior level, got {node:?}"),
        }
    }
```

`whitespace_only_and_empty_literals_are_empty_levels` gains `"\n\n"` beside `"   "` and `""`: all three produce zero chunks. Every other chunk test is untouched; a line break between items (`a_line_break_before_a_group_splits_the_field_from_its_selection_set`) is still a boundary, since capture applies only at a level's start.

## Landing checklist

1. The absorption changes, the comment updates, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past. parse-entrypoint.md builds on the invariant this doc lands: an empty chunk is always an error, and its boundary's first token is its comma.
