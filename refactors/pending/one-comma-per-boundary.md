# one-comma-per-boundary: a comma must follow an item

A prefactor to the series parsing-plan.md orders. Two changes to the chunking pass establish one invariant: an empty chunk exists exactly when a comma has no item before it, and its boundary starts with that comma. An empty chunk is therefore unambiguously a parse error to the grammar stage, with no boundary inspection and no first-chunk special case.

1. A level's leading line breaks move out of the chunk vec into a slot on the level. `foo {\n}` and a literal opening with a line break produce no empty chunk.
2. The boundary phase stops before a second comma, so every `ChunkSeparator` holds any number of line breaks and at most one comma: the boundary grammar is `line-break+` or `line-break* comma line-break*`. The refused comma opens the next chunk, which is empty when nothing sits between the commas.

A leading comma (`{, bar }`, `{,}`) also opens an empty chunk: the leading slot absorbs only line breaks, so a comma with no item before it always lands as the first boundary token of an empty chunk, whether at a level's start or between two commas.

Chunking stays infallible and `ChunkSeparator`'s shape stays `Vec<WithSpan<SeparatorToken>>`; both invariants are established by construction in the two functions that absorb separators.

## The level's shape

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
/// One level of the chunk tree: the whole literal at the root, a group's interior
/// below — the same role `MatchedBrackets` has on the bracket tree. Its chunks, in
/// order, and nothing else. A level that opens with separators holds them in a first
/// chunk with no contents: an admittedly suboptimal encoding, accepted as the price of
/// a level being a plain vec of one uniform chunk shape.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkedLevelParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ChunkedLevel(#[resolve_field] pub Vec<WithSpan<Chunk>>);
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
/// One level of the chunk tree: the whole literal at the root, a group's interior
/// below — the same role `MatchedBrackets` has on the bracket tree. The line breaks
/// before the first chunk sit in their own slot; a comma never lands there, so a chunk
/// with no contents is always a comma no item precedes, holding that comma as its
/// boundary's first token.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkedLevelParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ChunkedLevel {
    #[resolve_field(parent_variant = Level)]
    pub leading_line_breaks: Option<WithSpan<ChunkSeparator>>,
    #[resolve_field]
    pub chunks: Vec<WithSpan<Chunk>>,
}
```

`ChunkSeparator` now has two parents, so its direct parent alias becomes an enum, and `Chunk`'s separator field names its variant. Before:

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct Chunk {
    #[resolve_field]
    pub contents: Vec<WithSpan<ChunkContentItem>>,
    #[resolve_field]
    pub trailing_separator: Option<WithSpan<ChunkSeparator>>,
}

pub type ChunkSeparatorPath<'a> = PositionResolutionPath<&'a ChunkSeparator, ChunkPath<'a>>;
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct Chunk {
    #[resolve_field]
    pub contents: Vec<WithSpan<ChunkContentItem>>,
    #[resolve_field(parent_variant = Chunk)]
    pub trailing_separator: Option<WithSpan<ChunkSeparator>>,
}

#[derive(Debug)]
pub enum ChunkSeparatorParent<'a> {
    Chunk(ChunkPath<'a>),
    Level(ChunkedLevelPath<'a>),
}

pub type ChunkSeparatorPath<'a> = PositionResolutionPath<&'a ChunkSeparator, ChunkSeparatorParent<'a>>;
```

Every `.0` on a `ChunkedLevel` respells to `.chunks` — `chunk_level`, and each test that indexes a level.

## The absorption changes

`chunk_level` absorbs the leading slot first:

```rust
// from crates/isograph_parser/src/chunk.rs
fn chunk_level(level: &MatchedBrackets) -> ChunkedLevel {
    let mut items = level.0.iter().safe_peekable();
    let leading_line_breaks = absorb_leading_line_breaks(&mut items);
    let mut chunks = Vec::new();
    while items.peek().is_some() {
        chunks.push(absorb_chunk(&mut items));
    }
    ChunkedLevel {
        leading_line_breaks,
        chunks,
    }
}

/// The line breaks before a level's first chunk. Content opens the first chunk, and a
/// comma with no item before it opens an empty chunk, so a comma never lands here.
fn absorb_leading_line_breaks(items: &mut LevelItems<'_>) -> Option<WithSpan<ChunkSeparator>> {
    let mut line_breaks = Vec::new();
    while let Some(peek) = items.peek() {
        if separator_of(peek.view()) != Some(SeparatorToken::LineBreak) {
            break;
        }
        let item = peek.commit();
        line_breaks.push(WithSpan::new(SeparatorToken::LineBreak, item.location));
    }
    let location = line_breaks.iter().map(|token| token.location).reduce(Span::join)?;
    Some(WithSpan::new(ChunkSeparator(line_breaks), location))
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
/// A separator run: a chunk's ending boundary, or a level's leading line breaks. A
/// boundary holds its line breaks and at most one comma; a second comma is never
/// absorbed and opens the next chunk's boundary. Its tokens are not resolution leaves;
/// a position on any of them answers the separator.
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

## Generated code

The level's new expansion descends into the leading slot with the wrapped parent, then the chunks with the container's path:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for ChunkedLevel {
    type Parent<'a> = ChunkedLevelParent<'a>;
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        for item in self.leading_line_breaks.iter() {
            if item.location.contains(position) {
                let new_parent = <ChunkSeparator as ::resolve_position::ResolvePosition>::Parent::Level(self.path(parent).into());
                return item.item.resolve(new_parent, position);
            }
        }
        for item in self.chunks.iter() {
            if item.location.contains(position) {
                let new_parent = self.path(parent);
                return item.item.resolve(new_parent, position);
            }
        }
        return Self::ResolvedNode::ChunkedLevel(self.path(parent).into());
    }
}
```

`Chunk`'s expansion changes only in wrapping its separator's parent in `ChunkSeparatorParent::Chunk`, per the same pattern.

## Tests

The multi-separator tail of `commas_and_line_breaks_are_equivalent_separators` asserted that `a,\n\n,b` produces two chunks with a four-token boundary; that input now produces three. The test keeps its equivalence half and drops the tail (and respells `.0` to `.chunks`):

```rust
// from crates/isograph_parser/src/chunk.rs
    #[test]
    fn commas_and_line_breaks_are_equivalent_separators() {
        let comma = chunked("a, b");
        let linebreak = chunked("a\nb");
        assert_eq!(comma.item.chunks.len(), 2);
        assert_eq!(linebreak.item.chunks.len(), 2);
        assert_eq!(
            separator_kinds(&comma.item.chunks[0].item.trailing_separator.as_ref().unwrap().item),
            vec![SeparatorToken::Comma]
        );
        assert_eq!(
            separator_kinds(
                &linebreak.item.chunks[0].item.trailing_separator.as_ref().unwrap().item
            ),
            vec![SeparatorToken::LineBreak]
        );
        assert!(comma.item.chunks[1].item.trailing_separator.is_none());
        assert!(linebreak.item.chunks[1].item.trailing_separator.is_none());
    }
```

`leading_separators_make_an_empty_first_chunk` (`\n, a, b,\n`) described the old encoding and is replaced by one test per side of the new invariant:

```rust
// from crates/isograph_parser/src/chunk.rs
    #[test]
    fn leading_line_breaks_fill_the_level_slot_and_make_no_chunk() {
        let text = "\n\na, b\n";
        let tree = chunked(text);
        let leading = tree.item.leading_line_breaks.as_ref().unwrap();
        assert_eq!(
            separator_kinds(&leading.item),
            vec![SeparatorToken::LineBreak, SeparatorToken::LineBreak]
        );
        assert_eq!(leading.location, Span::new(0, 2));
        assert_eq!(tree.item.chunks.len(), 2);
        assert_eq!(render_chunk(text, &tree.item.chunks[0].item), "a,");
    }

    #[test]
    fn a_comma_before_the_first_item_opens_an_empty_chunk() {
        let text = "\n, a";
        let tree = chunked(text);
        let leading = tree.item.leading_line_breaks.as_ref().unwrap();
        assert_eq!(separator_kinds(&leading.item), vec![SeparatorToken::LineBreak]);
        assert_eq!(tree.item.chunks.len(), 2);
        let empty = &tree.item.chunks[0];
        assert!(empty.item.contents.is_empty());
        assert_eq!(
            separator_kinds(&empty.item.trailing_separator.as_ref().unwrap().item),
            vec![SeparatorToken::Comma]
        );
        assert_eq!(empty.location, span_of(text, ","));
        assert_eq!(render_chunk(text, &tree.item.chunks[1].item), "a");
    }

    #[test]
    fn a_second_comma_ends_the_boundary_and_leaves_an_empty_chunk() {
        let text = "a,\n\n,b";
        let tree = chunked(text);
        assert_eq!(tree.item.chunks.len(), 3);
        let first = &tree.item.chunks[0].item;
        assert_eq!(render_chunk(text, first), "a,\n\n");
        assert_eq!(
            separator_kinds(&first.trailing_separator.as_ref().unwrap().item),
            vec![
                SeparatorToken::Comma,
                SeparatorToken::LineBreak,
                SeparatorToken::LineBreak,
            ]
        );
        let middle = &tree.item.chunks[1];
        assert!(middle.item.contents.is_empty());
        assert_eq!(
            separator_kinds(&middle.item.trailing_separator.as_ref().unwrap().item),
            vec![SeparatorToken::Comma]
        );
        let second_comma = span_of(text, ",b");
        assert_eq!(middle.location, Span::new(second_comma.start, second_comma.start + 1));
        assert_eq!(render_chunk(text, &tree.item.chunks[2].item), "b");
    }

    #[test]
    fn doubled_leading_commas_leave_two_empty_chunks() {
        let text = ",,a";
        let tree = chunked(text);
        assert!(tree.item.leading_line_breaks.is_none());
        assert_eq!(tree.item.chunks.len(), 3);
        assert!(tree.item.chunks[0].item.contents.is_empty());
        assert!(tree.item.chunks[1].item.contents.is_empty());
        assert_eq!(render_chunk(text, &tree.item.chunks[2].item), "a");
    }

    #[test]
    fn leading_line_breaks_resolve_to_the_separator_with_the_level_as_parent() {
        let text = "\nfoo";
        let tree = chunked(text);
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "\n")) {
            IsographResolutionNode::ChunkSeparator(separator) => {
                assert!(matches!(separator.parent, ChunkSeparatorParent::Level(_)));
            }
            node => panic!("expected the separator leaf, got {node:?}"),
        }
    }
```

The remaining edits are mechanical: every other chunk.rs test respells `.item.0` on levels to `.item.chunks`, and `resolution_walks_ancestry_against_source_text`'s separator case matches through `ChunkSeparatorParent::Chunk`. The whitespace-only test additionally asserts `leading_line_breaks.is_none()` for `"   "` and `""`.

## Landing checklist

1. The level restructure, the absorption changes, the comment updates, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past. parse-entrypoint.md builds on the invariant this doc lands: an empty chunk is always an error, and its boundary's first token is its comma.
