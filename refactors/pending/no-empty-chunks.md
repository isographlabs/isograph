# no-empty-chunks: chunking returns its tree beside its errors

A prefactor to the parsing series, after cut-at-unmatched.md. A comma no item precedes (a leading or doubled comma) is an error in every grammar context, so chunking emits it and drops the empty chunk it would have opened: no empty chunk reaches the parsing stage.

```rust
// from crates/isograph_parser/src/chunk.rs
pub fn chunk(tree: &WithSpan<MatchedBrackets>) -> (WithSpan<ChunkedLevel>, Vec<CommaWithoutItem>)
```

By example:

```
{ a,, b }    ->  { a, b }    [CommaWithoutItem at the second comma]
{, a }       ->  { a }       [CommaWithoutItem at the comma]
,,a          ->  a           [CommaWithoutItem twice]
```

Sibling chunks are untouched: a doubled comma is a local typo, not a poison, so unlike the matcher's cut nothing after it is dropped. Positions on a dropped comma resolve to the level that dropped it. Errors come out in source order by construction, since levels are chunked in item order and group interiors recurse at their positions.

## The error type

```rust
// from crates/isograph_parser/src/chunk.rs
/// A chunking error: a comma no item precedes, at the comma's span. The empty chunk it
/// would have opened is not in the tree.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CommaWithoutItem(pub Span);
```

## The pass

`chunk` and `chunk_level`, before:

```rust
// from crates/isograph_parser/src/chunk.rs
pub fn chunk(tree: &WithSpan<MatchedBrackets>) -> WithSpan<ChunkedLevel> {
    WithSpan::new(chunk_level(&tree.item), tree.location)
}

fn chunk_level(level: &MatchedBrackets) -> ChunkedLevel {
    let mut items = level.0.iter().safe_peekable();
    let mut out = Vec::new();
    while items.peek().is_some() {
        out.push(absorb_chunk(&mut items));
    }
    ChunkedLevel(out)
}
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
pub fn chunk(tree: &WithSpan<MatchedBrackets>) -> (WithSpan<ChunkedLevel>, Vec<CommaWithoutItem>) {
    let mut errors = Vec::new();
    let level = chunk_level(&tree.item, &mut errors);
    (WithSpan::new(level, tree.location), errors)
}

fn chunk_level(level: &MatchedBrackets, errors: &mut Vec<CommaWithoutItem>) -> ChunkedLevel {
    let mut items = level.0.iter().safe_peekable();
    let mut out = Vec::new();
    while items.peek().is_some() {
        let chunk = absorb_chunk(&mut items, errors);
        if chunk.item.contents.is_empty() {
            // An empty chunk is a comma no item precedes, its boundary's first token
            // (one-comma-per-boundary.md). The fallback span cannot occur: a chunk has
            // at least one part.
            let comma = chunk
                .item
                .trailing_separator
                .as_ref()
                .and_then(|separator| separator.item.0.first())
                .map(|token| token.location)
                .unwrap_or(chunk.location);
            errors.push(CommaWithoutItem(comma));
        } else {
            out.push(chunk);
        }
    }
    ChunkedLevel(out)
}
```

`absorb_chunk` and `chunk_group` change only by threading the vec, for the group arm's recursion. Before:

```rust
// from crates/isograph_parser/src/chunk.rs
fn absorb_chunk(items: &mut LevelItems<'_>) -> WithSpan<Chunk> {
```

```rust
// from crates/isograph_parser/src/chunk.rs
            BracketItem::Bracketed(group) => ChunkContentItem::Group(chunk_group(group)),
```

```rust
// from crates/isograph_parser/src/chunk.rs
fn chunk_group(group: &Bracketed) -> ChunkedGroup {
    ChunkedGroup {
        opening: group.opening,
        children: WithSpan::new(chunk_level(&group.children.item), group.children.location),
        closing: group.closing,
    }
}
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
fn absorb_chunk(
    items: &mut LevelItems<'_>,
    errors: &mut Vec<CommaWithoutItem>,
) -> WithSpan<Chunk> {
```

```rust
// from crates/isograph_parser/src/chunk.rs
            BracketItem::Bracketed(group) => ChunkContentItem::Group(chunk_group(group, errors)),
```

```rust
// from crates/isograph_parser/src/chunk.rs
fn chunk_group(group: &Bracketed, errors: &mut Vec<CommaWithoutItem>) -> ChunkedGroup {
    ChunkedGroup {
        opening: group.opening,
        children: WithSpan::new(
            chunk_level(&group.children.item, errors),
            group.children.location,
        ),
        closing: group.closing,
    }
}
```

## Doc comments

`ChunkedLevel`, before:

```rust
// from crates/isograph_parser/src/chunk.rs
/// One level of the chunk tree: the whole literal at the root, a group's interior
/// below — the same role `MatchedBrackets` has on the bracket tree. Its chunks, in
/// order, and nothing else. Line breaks at the level's start are captured by the
/// opening bracket (or the literal's start) and appear nowhere, so a chunk with no
/// contents is always a comma no item precedes, holding that comma as its boundary's
/// first token.
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
/// One level of the chunk tree: the whole literal at the root, a group's interior
/// below — the same role `MatchedBrackets` has on the bracket tree. Its chunks, in
/// order, each with contents, and nothing else: line breaks at the level's start were
/// captured by the opening bracket (or the literal's start), and a comma no item
/// precedes is a `CommaWithoutItem` error beside the tree, its empty chunk dropped.
```

## Consequences for the series docs

- No empty chunk exists, so the grammar stage never classifies one: parsing-standards.md's `LevelEntry` and its `CommaWithoutItem` arm are deleted, the level accessor yields chunks plainly, and the `Expected(<the level's item>, found ',')` error family never arises in the grammar.
- Every chunk has contents, so `Chunk::contents_span` returns `Span`, not `Option<Span>`.
- The final sweep is three lists: the matcher's `Vec<BracketError>`, chunking's `Vec<CommaWithoutItem>`, and the grammar's `errors()`.

## Tests

Chunking joins the passes whose errors are output: `chunked` asserts both upstream vecs empty for well-formed fixtures, and error fixtures destructure.

```rust
// from crates/isograph_parser/src/chunk.rs (test module)
    fn chunked(literal: &str) -> WithSpan<ChunkedLevel> {
        let (chunked, comma_errors) = chunk(&well_formed(literal));
        assert_eq!(comma_errors, vec![]);
        chunked
    }
```

`a_comma_before_the_first_item_opens_an_empty_chunk` becomes `\n, a` -> the one chunk `a` with `[CommaWithoutItem]` at the comma; `a_second_comma_ends_the_boundary_and_leaves_an_empty_chunk` becomes `a,\n\n,b` -> the two chunks `a,\n\n` and `b` with `[CommaWithoutItem]` at the second comma; `doubled_leading_commas_leave_two_empty_chunks` becomes `,,a` -> the one chunk `a` with two errors, in order. A new resolution assertion pins that a position on a dropped comma answers its level. Every other chunk test asserts through `chunked` and so pins an empty error vec.

## Landing checklist

1. The signature and threading changes, the comment updates, and the test rewrites; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past. Downstream of chunking, no empty chunk exists.
