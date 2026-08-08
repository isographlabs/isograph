# Chunking

Requires raw-items.md. The pass after bracket matching: a bespoke recursive walk over each `MatchedBrackets` level. There is no `TreeContents` and no `map`. The output is a chunk tree of its own shape, not a re-labeled bracket tree. Chunking is infallible. It validates nothing and emits no errors; every raw token of every level lands in a chunk or a separator. A chunk is parsed independently by the chunk-parsing pass later. Bracket matching's unmatched tokens ride inside chunks as content, and the chunk-parsing pass reports leftover bracket tokens it finds there. The bracket tree's `errors()` stays available on the input; the chunk tree does not re-derive them.

A chunk holds its non-separator tokens and its trailing group, when one follows. In `bar { baz }`, one chunk carries the tokens `bar` and the brace group; the selection is not an adjacency the later pass has to reassemble from siblings. A raw unmatched open or close is content inside a chunk, not a tree item of its own kind.

## Behavior

- Commas and line breaks are the separators, equivalent. Any nonempty mix of consecutive separators is one boundary, held as one `Separator`. A level never produces an empty chunk: separators at a level's start or end are just boundary nodes there.
- Separators are required between fields, so `baz watttt` is one chunk, and it is that chunk's own parse that later fails ("expected a comma or line break"). Chunking never splits on token shape.
- A separator splits only its own level. Groups nest inside the chunk that owns them, so a chunk never spans a sibling group, and the separators inside a group's interior never affect the level outside the group.
- A chunk's span runs from its first token's start to the end of its trailing group when it has one, or to its last token's end otherwise; a separator's span likewise. Whitespace between tokens belongs to no node.
- A `BracketItem::Bracketed` attaches to the chunk in progress as that chunk's trailing group. A level that opens with a group (no preceding non-separator tokens) starts a chunk whose token list is empty and whose trailing group is that group.
- A `BracketItem::Raw` token — non-bracket, unmatched open, or unmatched close — is content of the chunk in progress. Unmatched brackets are not structure at this stage; the chunk-parsing pass is the one that reports them when they survive inside a chunk.

```
foo { bar, baz
qux }
```

chunks as: one top-level chunk whose tokens are `foo` and whose trailing group is the brace; the brace group's interior is chunk `bar`, separator `,`, chunk `baz`, separator (the line break), chunk `qux`.

```
a(
) {
}
```

has one top-level chunk, `a`, whose trailing group is the parenthesis (empty interior: a lone separator), and a following chunk whose tokens are empty and whose trailing group is the brace (empty interior: a lone separator). `foo\n{ bar }` chunks as chunk `foo`, then a separator, then a chunk with empty tokens and the brace as trailing group — that separator between the two chunks is what the chunk-parsing pass rejects when it assembles selections, so a selection set's brace still has to open on its field's line.

`foo { bar(a: }) }` chunks whatever the bracket pass produced: the paren open that never closed is a raw item on the brace level, so it rides inside a chunk as content; the leftover closes that raw-items left at the top ride inside top-level chunks the same way.

## The shape

New module `crates/isograph_parser/src/chunk.rs`, registered in lib.rs alongside the existing modules:

```rust
// from crates/isograph_parser/src/lib.rs
mod chunk;
mod matched_brackets;
...
pub use chunk::*;
pub use matched_brackets::*;
```

```rust
// from crates/isograph_parser/src/chunk.rs
use span::{Span, WithSpan};

use crate::{
    Bracketed, BracketItem, MatchedBrackets, NonBracketTokenKind, RawToken,
};

/// One level of the chunk tree: the whole literal at the root, a group's interior below.
#[derive(Debug, PartialEq, Eq)]
pub struct ChunkedLevel(pub Vec<WithSpan<ChunkedLevelItem>>);

#[derive(Debug, PartialEq, Eq)]
pub enum ChunkedLevelItem {
    Chunk(Chunk),
    Separator(Separator),
}

/// A maximal separator-free run of raw tokens, plus the group that follows it when one
/// does. The chunk-parsing pass consumes it as one unit. The wrapping `WithSpan`'s span
/// runs from the first token's start (or the group's start when there are no tokens) to
/// the end of the trailing group when present, otherwise the last token's end.
#[derive(Debug, PartialEq, Eq)]
pub struct Chunk {
    pub tokens: Vec<WithSpan<ChunkToken>>,
    pub trailing_group: Option<WithSpan<ChunkedGroup>>,
}

/// A matched group re-chunked: the bracket tree's opening and closing are kept, and the
/// interior is a `ChunkedLevel`.
#[derive(Debug, PartialEq, Eq)]
pub struct ChunkedGroup {
    pub opening: WithSpan<crate::OpenBracket>,
    pub children: WithSpan<ChunkedLevel>,
    pub closing: WithSpan<crate::CloseBracket>,
}

/// What a chunk can hold as flat content: every non-separator raw token from the bracket
/// tree, unmatched brackets included.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ChunkToken {
    NonBracket(NonBracketTokenKind),
    UnmatchedOpen(crate::BracketKind),
    UnmatchedClose(crate::BracketKind),
}

/// One boundary between chunks, holding every comma and line-break token it absorbed, in order.
#[derive(Debug, PartialEq, Eq)]
pub struct Separator(pub Vec<WithSpan<SeparatorToken>>);

/// The two token kinds a separator boundary can hold.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SeparatorToken {
    Comma,
    LineBreak,
}
```

## The pass

`chunk` walks each level left to right. The level is a flat mix of raw tokens and groups; the walk folds non-separator raws and the next group into one chunk, and folds consecutive separators into one boundary. Each outer iteration of `chunk_level` consumes at least the item `next` returned, so the helpers never see an empty stream.

```rust
// from crates/isograph_parser/src/chunk.rs
use std::iter::Peekable;
use std::slice::Iter;

type LevelItems<'a> = Peekable<Iter<'a, WithSpan<BracketItem>>>;

/// Chunk a matched-brackets tree. The pass is infallible. Every raw token lands in a
/// chunk or a separator, and no grammar is checked.
pub fn chunk(tree: WithSpan<MatchedBrackets>) -> WithSpan<ChunkedLevel> {
    tree.map(|level| chunk_level(&level))
}

fn chunk_level(level: &MatchedBrackets) -> ChunkedLevel {
    let mut items = level.0.iter().peekable();
    let mut out = Vec::new();
    while let Some(first) = items.next() {
        match separator_of(first) {
            Some(first_separator) => out.push(absorb_separator(first_separator, first.location, &mut items)),
            None => out.push(absorb_chunk(first, &mut items)),
        }
    }
    ChunkedLevel(out)
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

fn chunk_token(raw: RawToken) -> Option<ChunkToken> {
    match raw {
        RawToken::NonBracket(token) if separator_token(token.0).is_some() => None,
        RawToken::NonBracket(token) => Some(ChunkToken::NonBracket(token.0)),
        RawToken::Open(open) => Some(ChunkToken::UnmatchedOpen(open.0)),
        RawToken::Close(close) => Some(ChunkToken::UnmatchedClose(close.0)),
    }
}

fn absorb_separator(
    first: SeparatorToken,
    first_location: Span,
    items: &mut LevelItems<'_>,
) -> WithSpan<ChunkedLevelItem> {
    let mut span = first_location;
    let mut boundary = vec![WithSpan::new(first, first_location)];
    while let Some(item) = items.peek() {
        let Some(separator) = separator_of(item) else {
            break;
        };
        let Some(item) = items.next() else {
            break;
        };
        span = Span::join(span, item.location);
        boundary.push(WithSpan::new(separator, item.location));
    }
    WithSpan::new(ChunkedLevelItem::Separator(Separator(boundary)), span)
}

fn absorb_chunk(
    first: &WithSpan<BracketItem>,
    items: &mut LevelItems<'_>,
) -> WithSpan<ChunkedLevelItem> {
    match &first.item {
        BracketItem::Bracketed(group) => WithSpan::new(
            ChunkedLevelItem::Chunk(Chunk {
                tokens: Vec::new(),
                trailing_group: Some(WithSpan::new(chunk_group(group), first.location)),
            }),
            first.location,
        ),
        BracketItem::Raw(raw) => {
            let mut tokens = Vec::new();
            let mut span = first.location;
            if let Some(token) = chunk_token(*raw) {
                tokens.push(WithSpan::new(token, first.location));
            }
            let mut trailing_group = None;
            while let Some(item) = items.peek() {
                match &item.item {
                    BracketItem::Raw(raw) => {
                        let Some(token) = chunk_token(*raw) else {
                            break;
                        };
                        let Some(item) = items.next() else {
                            break;
                        };
                        span = Span::join(span, item.location);
                        tokens.push(WithSpan::new(token, item.location));
                    }
                    BracketItem::Bracketed(group) => {
                        let Some(item) = items.next() else {
                            break;
                        };
                        span = Span::join(span, item.location);
                        trailing_group = Some(WithSpan::new(chunk_group(group), item.location));
                        break;
                    }
                }
            }
            WithSpan::new(
                ChunkedLevelItem::Chunk(Chunk {
                    tokens,
                    trailing_group,
                }),
                span,
            )
        }
    }
}

fn chunk_group(group: &Bracketed) -> ChunkedGroup {
    ChunkedGroup {
        opening: group.opening,
        children: group.children.as_ref().map(|level| chunk_level(level)),
        closing: group.closing,
    }
}
```

`WithSpan::map` and `as_ref` already exist on the span crate.

## Resolution

The chunk tree derives `ResolvePosition` on its own types. A position on a bracket-tree unmatched token that rides inside a chunk answers the chunk (or, once chunk tokens become leaves, the token); a position on a group's opening or closing answers those leaves through `ChunkedGroup`. Parent enums and the resolved-node enum are written out when this pass is implemented; they are not a second stage of the bracket tree's `ResolvedBracketNode`.

## Tests

Structural facts only: which chunks and separators a level holds, which tokens and trailing group a chunk holds, that unmatched brackets land inside chunks, that separators collapse, that spans are tight. No snapshot of the whole tree. Written out fully when the pass is implemented; the cases below are the ones the suite must cover.

- `foo { bar, baz\nqux }` — top is one chunk (`foo` + brace); interior is five items alternating chunks and separators.
- `a, b` and `a\nb` separate identically; `a,\n\n,b` is one separator holding four separator tokens.
- `\n, a, b,\n` has separators first and last, no empty chunks.
- `bar, baz watttt, qux` keeps `baz watttt` as one chunk.
- `foo\n{ bar }` is chunk, separator, chunk(empty tokens + brace).
- `a ) b` puts the unmatched close inside a chunk between `a` and `b` (or as its own chunk of one token when separators bound it); `errors()` on the input bracket tree still reports the unmatched close.
- `foo { bar` — the brace never closed, so raw-items demoted the open; chunking sees a raw unmatched open and the following tokens at the top level, not an unbalanced group.

## Landing checklist

1. Add chunk.rs with the types, the pass, and the structural tests; register `mod chunk;` and `pub use chunk::*;` in lib.rs.
2. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
3. Move this doc to refactors/past.
