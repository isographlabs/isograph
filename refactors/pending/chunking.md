# Chunking

Requires isograph-resolution-node.md. The pass after bracket matching: each level's item list is repartitioned at separators, and the partition recurses into every group's interior through the one algorithm — the same chunking serves selection sets, argument lists, and every other bracketed interior. The skeleton is preserved: `foo { bar }`'s level goes from the items `foo`, group to one chunk holding both, and the group's interior goes from `[bar]` to `[Chunk([bar])]`. Chunking is infallible. It validates nothing and emits no errors; every raw token of every level lands in a chunk, as content or inside its trailing separator. A chunk is parsed independently by the chunk-parsing pass later. Bracket matching's unmatched tokens ride inside chunks as content, and the chunk-parsing pass reports leftover bracket tokens it finds there. The pass borrows the bracket tree and moves nothing out of it, so `errors()` stays available on the input; the chunk tree does not re-derive them.

Resolution moves with the newest tree: after this refactor, positions resolve against the chunk tree, `IsographResolutionNode`'s variants become the chunk tree's leaves, and the bracket tree parses and reports errors but answers no positions (the same intermediate state raw-items.md shipped through). The token leaf types are shared between the trees and keep their one `ResolvePosition` impl each, re-pointed at chunk-tree parents.

A chunk holds every non-separator item between two boundaries — tokens, groups, unmatched brackets, anything, in source order — plus the separator run that ended it. A chunk can contain multiple groups: `foo { } { }` is one chunk, and `a() { }` is one chunk whose contents are `a`, the argument list, and the selection set, which is exactly the unit the chunk-parsing pass wants to consume as one field. There is no distinction between the first n chunks and the final one: every chunk's trailing separator is simply optional, so a trailing delimiter at a level's end is nothing special.

## Behavior

- Commas and line breaks are the separators, equivalent. Any nonempty run of consecutive separators is one boundary, held as one `ChunkSeparator` in the trailing slot of the chunk it ends, so `foo,,,, bar` is two chunks, the first of which holds four commas in its trailing separator.
- Separators are the only thing chunking splits on. Everything between two boundaries is one chunk, whatever it is: `baz watttt` is one chunk, and it is that chunk's own parse that later fails ("expected a comma or line break").
- Every chunk except a level's last carries a trailing separator, by construction: a new chunk only ever starts after a boundary ends. The last chunk's separator is present exactly when the level ends in separators.
- A level that starts with separators gets a leading first chunk with no contents, holding only that boundary. That is the one shape of chunk with empty contents.
- A separator ends only its own level's chunk. Group interiors are chunked recursively, and the separators inside a group's interior never affect the level outside the group.
- A chunk's span runs from its first part's start to its last part's end, the parts being its contents and then its trailing separator. Spaces the tokenizer skipped sit in whichever node whose span covers them: a gap between a chunk's own parts answers that chunk; a gap no chunk covers falls through the derive to the nearest container — the root `ChunkedLevel` at the top of the literal, the group when the gap sits between that group's interior chunks (group interiors are a bare vec of chunks, not a nested level node, so the group is what answers).
- A `BracketItem::Raw` token — non-bracket, unmatched open, or unmatched close — is content of the chunk in progress. Unmatched brackets are not structure at this stage; the chunk-parsing pass is the one that reports them when they survive inside a chunk.

```
foo { bar, baz
qux }
```

chunks as: one top-level chunk whose contents are `foo` and the brace group; the brace group's interior is chunk `bar` (a comma trailing), chunk `baz` (a line break trailing), chunk `qux` (nothing trailing).

```
a(
) {
}
```

is one top-level chunk whose contents are `a`, the parenthesis group, and the brace group — no separator sits between `)` and `{`, so nothing splits them; each group's interior is a single chunk holding only a separator. `foo\n{ bar }` chunks differently: chunk `foo` with the line break as its trailing separator, then a chunk whose only content is the brace group — the brace riding in a chunk of its own is what the chunk-parsing pass rejects when it assembles selections, so a selection set's brace still has to open on its field's line.

`foo { bar(a: }) }` chunks whatever the bracket pass produced: the paren open that never closed is a raw item on the brace level, so it rides inside a chunk as content; the leftover closes that raw-items left at the top ride inside top-level chunks the same way.

## The shape

New module `crates/isograph_parser/src/chunk.rs`, registered in lib.rs alongside the existing modules:

```rust
// from crates/isograph_parser/src/lib.rs
mod chunk;
mod isograph_resolution_node;
mod matched_brackets;
...
pub use chunk::*;
pub use isograph_resolution_node::*;
pub use matched_brackets::*;
```

```rust
// from crates/isograph_parser/src/chunk.rs
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan};

use crate::{
    BracketItem, Bracketed, CloseBracket, IsographResolutionNode, MatchedBrackets,
    NonBracketToken, NonBracketTokenKind, OpenBracket, RawToken,
};

/// The root of the chunk tree: the literal's top-level chunks, in order. Group
/// interiors use the same chunking algorithm but store the resulting vec directly on
/// `ChunkedGroup::children`, not a nested `ChunkedLevel` — a nested level would own
/// inter-chunk gaps and empty interiors as itself, and the derive has no way to hand
/// those positions to the group. A level that opens with separators holds them in a
/// first chunk with no contents: an admittedly suboptimal encoding, accepted as the
/// price of a level being a plain vec of one uniform chunk shape.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkedLevelParent, resolved_node = IsographResolutionNode<'a>)]
pub struct ChunkedLevel(#[resolve_field(parent_variant = Root)] pub Vec<WithSpan<Chunk>>);

/// A maximal separator-free run of a level's items — tokens, groups, unmatched
/// brackets, anything — plus the separator run that ended it when one did. The
/// chunk-parsing pass consumes it as one unit. Every chunk but a level's last has a
/// trailing separator by construction; the last's is the optional trailing delimiter.
/// The wrapping `WithSpan`'s span runs from the first part's start to the last part's
/// end.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct Chunk {
    #[resolve_field]
    pub contents: Vec<WithSpan<ChunkContentItem>>,
    #[resolve_field]
    pub trailing_separator: Option<WithSpan<ChunkSeparator>>,
}

/// What a chunk holds: every non-separator item of its level, groups included.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum ChunkContentItem {
    NonBracket(NonBracketToken),
    UnmatchedOpen(#[resolve_field(parent_variant = Unmatched)] OpenBracket),
    UnmatchedClose(#[resolve_field(parent_variant = Unmatched)] CloseBracket),
    Group(ChunkedGroup),
}

/// A matched group re-chunked: the bracket tree's opening and closing are kept, and the
/// interior is the same vec of chunks the root would hold for that level — not a nested
/// `ChunkedLevel`, so positions no interior chunk covers answer the group.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ChunkedGroup {
    #[resolve_field(parent_variant = Matched)]
    pub opening: WithSpan<OpenBracket>,
    #[resolve_field(parent_variant = Interior)]
    pub children: Vec<WithSpan<Chunk>>,
    #[resolve_field(parent_variant = Matched)]
    pub closing: WithSpan<CloseBracket>,
}

/// The boundary that ended its chunk, holding every comma and line-break token it
/// absorbed, in order. Its tokens are not resolution leaves; a position on any of them
/// answers the separator.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ChunkSeparator(pub Vec<WithSpan<SeparatorToken>>);

/// The two token kinds a separator boundary can hold.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SeparatorToken {
    Comma,
    LineBreak,
}
```

## Resolution

Positions resolve against the chunk tree, from `tree.resolve(ChunkedLevelParent::Root, position)`. The parent and path types live beside the chunk types. Every child of a chunk — its contents and its trailing separator — has the chunk's path as its parent directly, per one-variant-parent-enums.md. A chunk's parent is `ChunkParent`: `Root` when the chunk sits on the root `ChunkedLevel`, `Interior` when it sits on a group's `children`. A group's parent is the chunk that holds it. Every `ResolvePosition` impl is derived.

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug)]
pub enum ChunkedLevelParent {
    Root,
}

pub type ChunkedLevelPath<'a> =
    PositionResolutionPath<&'a ChunkedLevel, ChunkedLevelParent>;

/// A chunk sits on the root level or in a group's interior.
#[derive(Debug)]
pub enum ChunkParent<'a> {
    Root(ChunkedLevelPath<'a>),
    Interior(ChunkedGroupPath<'a>),
}

pub type ChunkPath<'a> = PositionResolutionPath<&'a Chunk, ChunkParent<'a>>;

pub type ChunkedGroupPath<'a> = PositionResolutionPath<&'a ChunkedGroup, ChunkPath<'a>>;

pub type ChunkSeparatorPath<'a> =
    PositionResolutionPath<&'a ChunkSeparator, ChunkPath<'a>>;

pub type NonBracketTokenPath<'a> =
    PositionResolutionPath<&'a NonBracketToken, ChunkPath<'a>>;

/// Shared by `OpenBracket` and `CloseBracket`: a bracket token is a group's own
/// opening or closing, or unmatched content of a chunk.
#[derive(Debug)]
pub enum BracketTokenParent<'a> {
    Matched(ChunkedGroupPath<'a>),
    Unmatched(ChunkPath<'a>),
}

pub type OpenBracketPath<'a> =
    PositionResolutionPath<&'a OpenBracket, BracketTokenParent<'a>>;
pub type CloseBracketPath<'a> =
    PositionResolutionPath<&'a CloseBracket, BracketTokenParent<'a>>;
```

`IsographResolutionNode` is modified in place: `MatchedBrackets` and `Bracketed` come out, the chunk tree's four node kinds go in, and the three token variants stay under their names with chunk-hosted paths.

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
use crate::{
    ChunkPath, ChunkSeparatorPath, ChunkedGroupPath, ChunkedLevelPath, CloseBracketPath,
    NonBracketTokenPath, OpenBracketPath,
};

/// What a position resolves to: the leaves of the newest tree. Each parsing stage
/// modifies these variants in place; today they are the chunk tree's.
#[derive(Debug)]
pub enum IsographResolutionNode<'a> {
    ChunkedLevel(ChunkedLevelPath<'a>),
    /// This will be resolved for spans that contain one of the opening/closing brackets
    /// and part of the inside, e.g. "{ ba" in "foo { bar }". Single-character spans
    /// will never resolve to this.
    ChunkedGroup(ChunkedGroupPath<'a>),
    Chunk(ChunkPath<'a>),
    ChunkSeparator(ChunkSeparatorPath<'a>),
    NonBracketToken(NonBracketTokenPath<'a>),
    OpenBracket(OpenBracketPath<'a>),
    CloseBracket(CloseBracketPath<'a>),
}
```

matched_brackets.rs gives its resolution up: the `ResolvePosition` derives come off `MatchedBrackets`, `BracketItem`, `RawToken`, and `Bracketed` (plain derives remain), and `MatchedBracketsParent`, `MatchedBracketsPath`, `BracketedPath`, and the old `BracketTokenParent` and path aliases are deleted — chunk.rs defines the token parent and path types now. The token leaf types keep their definitions there and re-point their one impl each at the chunk tree:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NonBracketToken(pub NonBracketTokenKind);

/// An opening bracket; its parent says whether it is a group's own opening or
/// unmatched content of a chunk.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = BracketTokenParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct OpenBracket(pub BracketKind);

/// A closing bracket; its parent says whether it is a group's own closing or
/// unmatched content of a chunk.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = BracketTokenParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct CloseBracket(pub BracketKind);
```

No macro changes: unmarked delegation, `parent_variant` wrapping, and the reflexive and `Box` `From` conversions cover every site. No hand-written `ResolvePosition` impls.

## The pass

`chunk` walks each level left to right; each iteration absorbs exactly one chunk, in two phases: content — every item up to the next separator — and then the boundary, the separator run that follows. When the level's next item is a separator, the content phase absorbs nothing and the chunk is a leading boundary holder. Group interiors recurse as their groups are absorbed. The stream is a `SafePeekable` over the level's slice (refactors/past/safe-peekable.md): the phases peek and commit exactly the items they absorb, so an item a phase refuses stays as the next item. The iterator's items are `&WithSpan<BracketItem>` and `Copy`, so `*peek.view()` detaches the reference from the guard before the guard is committed or dropped.

```rust
// from crates/isograph_parser/src/chunk.rs
use safe_peekable::{IntoSafePeekable, SafePeekable};

type LevelItems<'a> = SafePeekable<std::slice::Iter<'a, WithSpan<BracketItem>>>;

/// Chunk a matched-brackets tree. The pass is infallible. Every raw token lands in a
/// chunk, and no grammar is checked.
pub fn chunk(tree: &WithSpan<MatchedBrackets>) -> WithSpan<ChunkedLevel> {
    WithSpan::new(ChunkedLevel(chunk_level(&tree.item)), tree.location)
}

fn chunk_level(level: &MatchedBrackets) -> Vec<WithSpan<Chunk>> {
    let mut items = level.0.iter().safe_peekable();
    let mut out = Vec::new();
    while let Some(chunk) = absorb_chunk(&mut items) {
        out.push(chunk);
    }
    out
}

fn separator_token(kind: NonBracketTokenKind) -> Option<SeparatorToken> {
    match kind {
        NonBracketTokenKind::Comma => Some(SeparatorToken::Comma),
        NonBracketTokenKind::LineBreak => Some(SeparatorToken::LineBreak),
        _ => None,
    }
}

fn separator_of(item: &WithSpan<BracketItem>) -> Option<SeparatorToken> {
    match &item.item {
        BracketItem::Raw(RawToken::NonBracket(token)) => separator_token(token.0),
        _ => None,
    }
}

/// The content item a level item becomes, or `None` for a separator. Group interiors
/// recurse here.
fn chunk_content_item_of(item: &BracketItem) -> Option<ChunkContentItem> {
    match item {
        BracketItem::Raw(RawToken::NonBracket(token))
            if separator_token(token.0).is_some() =>
        {
            None
        }
        BracketItem::Raw(RawToken::NonBracket(token)) => {
            Some(ChunkContentItem::NonBracket(*token))
        }
        BracketItem::Raw(RawToken::Open(open)) => Some(ChunkContentItem::UnmatchedOpen(*open)),
        BracketItem::Raw(RawToken::Close(close)) => Some(ChunkContentItem::UnmatchedClose(*close)),
        BracketItem::Bracketed(group) => Some(ChunkContentItem::Group(chunk_group(group))),
    }
}

/// One chunk: the content phase, then the boundary phase. `None` when the stream is
/// empty — every item is either content or a separator, so a nonempty stream always
/// produces a chunk. Whichever phase matches the first item consumes it.
fn absorb_chunk(items: &mut LevelItems<'_>) -> Option<WithSpan<Chunk>> {
    let mut span = None;
    let mut contents = Vec::new();
    while let Some(peek) = items.peek() {
        let item = *peek.view();
        let Some(content_item) = chunk_content_item_of(&item.item) else {
            // A separator ends the content phase.
            break;
        };
        peek.commit();
        span = Span::join_optional(span, item.location);
        contents.push(WithSpan::new(content_item, item.location));
    }

    let mut separator_span = None;
    let mut separators = Vec::new();
    while let Some(peek) = items.peek() {
        let item = *peek.view();
        let Some(separator) = separator_of(item) else {
            break;
        };
        peek.commit();
        separator_span = Span::join_optional(separator_span, item.location);
        separators.push(WithSpan::new(separator, item.location));
    }
    if let Some(separator_span) = separator_span {
        span = Span::join_optional(span, separator_span);
    }
    let trailing_separator =
        separator_span.map(|location| WithSpan::new(ChunkSeparator(separators), location));

    if let Some(span) = span {
        Some(WithSpan::new(
            Chunk {
                contents,
                trailing_separator,
            },
            span,
        ))
    } else {
        None
    }
}

fn chunk_group(group: &Bracketed) -> ChunkedGroup {
    ChunkedGroup {
        opening: group.opening,
        children: chunk_level(&group.children.item),
        closing: group.closing,
    }
}
```

`Span::join_optional` is new on the span crate, beside `join` and `between`:

```rust
// from crates/span/src/lib.rs
    /// The running-fold form of [`Span::join`]: the first span seeds the accumulator,
    /// each later one extends it.
    pub fn join_optional(joined: Option<Span>, next: Span) -> Option<Span> {
        Some(match joined {
            Some(joined) => Span::join(joined, next),
            None => next,
        })
    }
```

## Tests

### Structural

Structural facts only: which chunks a level holds, which contents and trailing separator a chunk holds, that unmatched brackets land inside chunks, that separators collapse into one trailing boundary, that spans are tight, and that every non-final chunk carries a trailing separator. No snapshot of the whole tree. Written out fully when the pass is implemented; the cases below are the ones the suite must cover.

- `foo { bar, baz\nqux }` — top is one chunk whose contents are `foo` and the brace group; the interior is three chunks, the first two with one separator token trailing each.
- `foo { } { }` — one chunk, three contents, two of them groups; each brace interior is an empty `children` vec (zero chunks), not a chunk with empty contents.
- `{}` — top is one chunk whose only content is the brace group; the group's `children` is empty.
- `a, b` and `a\nb` chunk identically apart from the separator token kind; `a,\n\n,b` is chunk `a` with four tokens in its trailing separator, then chunk `b`.
- `\n, a, b,\n` — the first chunk has no contents and holds the leading boundary; `b`'s trailing separator holds `,` and the line break; no chunk follows `b`.
- `bar, baz watttt, qux` keeps `baz watttt` as one chunk.
- `foo\n{ bar }` is chunk `foo` (line break trailing), then a chunk whose only content is the brace group.
- `a ) b` puts the unmatched close inside a chunk between `a` and `b`; `errors()` on the input bracket tree still reports the unmatched close.
- `foo { bar` — the brace never closed, so the matcher demoted the open; chunking sees a raw unmatched open and the following tokens at the top level, not an unbalanced group.

### Modifications to the existing unit tests

The bracket tree's structural tests and error tests are untouched. Its six resolution tests come out of matched_brackets.rs and re-land in chunk.rs against the chunk tree, with the answers the new tree gives:

- the unmatched-open test (`foo { ( }`): `(` answers `OpenBracket` with `BracketTokenParent::Unmatched`, and the host chunk's parent is `ChunkParent::Interior` for the brace group.
- the root unmatched-close test (`a }`): `}` answers `CloseBracket` with `Unmatched`, and the host chunk's parent is `ChunkParent::Root` over a level with `ChunkedLevelParent::Root`.
- the matched-pair test (`foo { bar }`): `{` and `}` answer `Matched`, and the group's parent is the chunk whose first content is `foo`.
- the straddling-span test: "{ ba" answers `ChunkedGroup`.
- the ordinary-token test: `bar` answers `NonBracketToken` hosted by its chunk.
- the whitespace test changes answer: the space between `foo` and `{` sits inside the top-level chunk's span, so it answers `Chunk`. A gap no chunk covers answers the nearest derived container: at the root that is `ChunkedLevel`; inside a group that is the group.

### Resolution

The assertions navigate the resolved path and check ancestry against source text, per the shape "the chunk this token is part of renders as ...". A test helper renders a node back to text by slicing the literal at its span (for a chunk, derived from its parts); whether that becomes a general token serializer is a test implementation detail. On `foo { bar, baz }`:

- the position of `bar` answers `NonBracketToken(Identifier)`; its host chunk renders as `bar,`; walking up, the chunk's parent is `Interior` for the brace group, and that group belongs to the chunk that renders as `foo { bar, baz }`.
- the position of `{` answers `OpenBracket` with `Matched`, and the group's holding chunk renders as `foo { bar, baz }`.
- the position of `,` answers `ChunkSeparator`, and its parent is the chunk whose content is `bar`.
- the space between `foo` and `{` answers `Chunk`, the chunk rendering as `foo { bar, baz }`.
- the space between the interior chunks (after `bar,`, before `baz`) answers `ChunkedGroup` — the brace group — because group interiors are a bare vec of chunks with no nested level node.
- the literal's leading whitespace answers `ChunkedLevel` with `Root` as parent.

On `foo {}`: a position in the empty interior (between `{` and `}`) answers `ChunkedGroup` — `children` is empty, so the group's derive falls through to itself. Same answer for a space inside `foo { }`.

And on `a ) b`: the position of `)` answers `CloseBracket` with `Unmatched`, and the host chunk renders as `a ) b`.

## Landing checklist

1. Add `Span::join_optional` to the span crate, and chunk.rs: the types, the parent and path types, the pass; register `mod chunk;` and `pub use chunk::*;` in lib.rs.
2. Modify isograph_resolution_node.rs to the chunk-stage variant set.
3. Strip matched_brackets.rs's resolution: derives off the four structure types, the old parent and path types deleted, the token leaf types re-pointed at chunk parents, the six resolution tests removed.
4. The structural and resolution tests in chunk.rs.
5. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
6. Move this doc to refactors/past.
