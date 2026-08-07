# Chunking selection sets

The pass after bracket matching. It takes `MatchedBrackets<BracketsMatched>`, assumes the literal is a selection set, and regroups every selection-set level (the root, and each brace group's interior, recursively) into an alternation of chunks (one per selection) and separator boundaries. Paren and square groups are not chunked at this stage: they are carried through unchanged as `Bracketed`, and this stage is cleanly separable from whatever chunks them later. Chunking is infallible: it validates nothing, emits no errors, and every token that survived the bracket pass lands in some chunk or separator. Each chunk is parsed independently by later passes.

The chunked tree has its own path enum, `ResolvedChunkedSelectionSetNode`, fully separate from `ResolvedBracketNode`. Resolving a position against the bracket tree answers at token granularity (a run of `Vec<WithSpan<NonBracketTokenKind>>`, an opening, a close); resolving the same position against the chunked tree answers at chunk granularity (which chunk, which separator, which selection set). Two trees, two independent queries; nothing in `matched_brackets.rs`'s path family changes.

## Behavior

- Commas and line breaks are the separators, equivalent. Any nonempty mix of consecutive separators is one boundary, held as one `Separator` node in the tree. Separators at the start or end of a level produce no empty chunks; they are just boundary nodes there.
- Separators are required between fields, so `baz watttt` is one chunk, and it is that chunk's own parse that later fails ("expected a comma or line break"). Chunking never splits on token shape.
- A separator only separates at its own level. A group is one opaque item at the level it appears in, so the separators inside a paren or square group never split a selection-set level.
- A chunk is a maximal separator-free sequence of a level's items. Runs split at separator tokens; a group or a stray close joins the chunk that is open where it appears. A group right after a boundary opens a chunk of its own.
- A chunk's span runs from its first item's start to its last item's end; a separator's span runs from its first token's start to its last token's end. Whitespace between tokens belongs to no node and resolves to the nearest enclosing node, as at the bracket stage.

```
foo { bar, baz
qux }
```

is four chunks: one at the top level (the run `foo` plus the brace group), containing a selection set of three chunks `bar`, `baz`, `qux`, with a separator boundary between each pair.

```
a(
) {
}
```

is one chunk: both line breaks sit inside groups, so the top level has no separator anywhere. Consequence in the other direction: `foo\n{ bar }` is two chunks; a selection set's brace has to open on its field's line.

`foo { bar(a: }) }` chunks whatever the bracket pass produced: the top level is one chunk (no separators) holding `foo`, the brace group, and the two strays `)` and `}`. Bracket errors keep their bracket-pass positions; chunking neither fixes nor widens them.

### Resolution answers, worked example

```
foo {
  bar,
  baz
}
```

The chunk query:

- on `foo`: the chunk `foo { ... }`
- on the space between `foo` and `{`: inside that chunk's span but in no item, so the chunk
- on `{`: the selection set
- on the line break after `{`: the leading `Separator` inside the selection set
- on the indent spaces before `bar`: the selection set (whitespace belongs to the parent)
- on `bar`: `bar`'s chunk
- on the comma and on the line break after it: the one `Separator` between `bar` and `baz`
- on `baz`: `baz`'s chunk
- on the line break after `baz`: the trailing `Separator`
- on `}`: the selection set

The bracket query on the same text answers `Inner` on `foo`, `bar`, `baz`, the comma, and the line breaks (they are run tokens there), `OpenBracket` on `{`, and `CloseBracket` on `}` (close-bracket-node.md).

## Ordered changes

1. The mixed-enum derive in resolve-option-like-enums.md. `ChunkItem`'s derive needs it; it ships first, in that doc.
2. Change 1 below: extract `collect_group_errors` in matched_brackets.rs. Independently shippable.
3. Change 2 below: the chunk module and its tests.

## Change 1 (prefactor): extract `collect_group_errors`

The group arm of the existing error collector becomes a `pub(crate)` function, so the chunk stage's collector can reuse it for carried-through groups. In `matched_brackets.rs`:

Before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
            BracketItem::Bracketed(bracketed) => {
                if bracketed.closing.is_none() {
                    errors.push(BracketError::Unclosed(WithSpan::new(
                        UnclosedGroup(bracketed.opening),
                        item.location,
                    )));
                }
                collect_errors(&bracketed.children, errors);
            }
```

After:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
            BracketItem::Bracketed(bracketed) => {
                collect_group_errors(bracketed, item.location, errors);
            }
```

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// The errors of one group and its subtree: its own synthetic closing, if any, then its
/// children's errors.
pub(crate) fn collect_group_errors<TContents>(
    bracketed: &Bracketed<TContents>,
    group_span: Span,
    errors: &mut Vec<BracketError>,
) where
    TContents: TreeContents<StrayClose = CloseBracket>,
{
    if bracketed.closing.is_none() {
        errors.push(BracketError::Unclosed(WithSpan::new(
            UnclosedGroup(bracketed.opening),
            group_span,
        )));
    }
    collect_errors(&bracketed.children, errors);
}
```

`collect_errors` also becomes `pub(crate)`.

## Change 2: the chunk stage

New module `crates/isograph_parser/src/chunk.rs`, registered in lib.rs alongside the existing modules:

```rust
mod chunk;
mod matched_brackets;
...
pub use chunk::*;
pub use matched_brackets::*;
```

There is no new stage marker. The chunked tree holds the same contents as the bracket tree (`Inner` runs, `CloseBracket` strays, optional closings); what changes is the shape, so the chunk types are generic over the same `TreeContents` and their derives instantiate at `BracketsMatched` via `self_type_generics`. `Bracketed` moves into the chunk tree wholesale for paren and square groups, and closings move over untouched.

### Tree types

```rust
/// One selection-set level of the chunked tree: the whole literal at the root, a brace
/// group's interior below.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = (),
    resolved_node = ResolvedChunkedSelectionSetNode<'a>,
    self_type_generics = <BracketsMatched>
)]
pub struct Chunks<TContents: TreeContents>(
    #[resolve_field] pub Vec<WithSpan<SelectionSetItem<TContents>>>,
);

/// What a selection-set level holds: an alternation of chunks and separator boundaries.
/// Built with no two adjacent separators (one boundary node absorbs a run of them) and no
/// empty chunks.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = SelectionSetItemParent<'a>,
    resolved_node = ResolvedChunkedSelectionSetNode<'a>,
    self_type_generics = <BracketsMatched>
)]
pub enum SelectionSetItem<TContents: TreeContents> {
    Chunk(Chunk<TContents>),
    Separator(Separator),
}

/// A maximal separator-free sequence of a level's items: one selection. The wrapping
/// `WithSpan`'s span runs from the first item's start to the last item's end.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = SelectionSetItemParent<'a>,
    resolved_node = ResolvedChunkedSelectionSetNode<'a>,
    self_type_generics = <BracketsMatched>
)]
pub struct Chunk<TContents: TreeContents>(
    #[resolve_field] pub Vec<WithSpan<ChunkItem<TContents>>>,
);

/// One boundary between chunks: every comma and line break token it absorbed, in order.
/// The wrapping `WithSpan`'s span runs from the first token's start to the last token's
/// end.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = SelectionSetItemParent<'a>,
    resolved_node = ResolvedChunkedSelectionSetNode<'a>
)]
pub struct Separator(pub Vec<WithSpan<SeparatorToken>>);

/// The two token kinds a separator boundary can hold.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SeparatorToken {
    Comma,
    LineBreak,
}

/// What a chunk holds: runs split at separators, a brace group with its interior chunked,
/// a paren or square group carried through unchunked, or a stray close riding along. Only
/// the selection set resolves deeper in the chunk query; the other variants answer the
/// enclosing chunk.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = ChunkItemParent<'a>,
    resolved_node = ResolvedChunkedSelectionSetNode<'a>,
    self_type_generics = <BracketsMatched>,
    fallback = Chunk
)]
pub enum ChunkItem<TContents: TreeContents> {
    Inner(TContents::Inner),
    #[resolve_field]
    SelectionSet(SelectionSet<TContents>),
    Bracketed(Bracketed<TContents>),
    StrayClose(TContents::StrayClose),
}

/// A brace group whose interior is chunked. Built only from brace groups; paren and square
/// groups stay `ChunkItem::Bracketed`. `opening` and `closing` carry no `#[resolve_field]`:
/// they resolve in the bracket query, and in the chunk query a position on them answers
/// this selection set.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = ChunkItemParent<'a>,
    resolved_node = ResolvedChunkedSelectionSetNode<'a>,
    self_type_generics = <BracketsMatched>
)]
pub struct SelectionSet<TContents: TreeContents> {
    pub opening: WithSpan<OpenBracket>,
    #[resolve_field]
    pub children: Vec<WithSpan<SelectionSetItem<TContents>>>,
    /// The close the author typed, or `None` for a group that never got its close and
    /// was forced to end.
    pub closing: Option<WithSpan<CloseBracket>>,
}
```

### Path types

```rust
/// Every node the chunk query can resolve to. Token-granularity answers come from
/// resolving the bracket tree instead.
#[derive(Debug)]
pub enum ResolvedChunkedSelectionSetNode<'a> {
    Chunks(ChunksPath<'a>),
    Chunk(ChunkPath<'a>),
    Separator(SeparatorPath<'a>),
    SelectionSet(SelectionSetPath<'a>),
}

pub type ChunksPath<'a> = PositionResolutionPath<&'a Chunks<BracketsMatched>, ()>;

/// Everything a `SelectionSetItem` can sit inside; `Chunk` and `Separator` share it
/// through the enum's delegation.
#[derive(Debug)]
pub enum SelectionSetItemParent<'a> {
    Chunks(ChunksPath<'a>),
    SelectionSet(Box<SelectionSetPath<'a>>),
}

pub type ChunkPath<'a> =
    PositionResolutionPath<&'a Chunk<BracketsMatched>, SelectionSetItemParent<'a>>;
pub type SeparatorPath<'a> =
    PositionResolutionPath<&'a Separator, SelectionSetItemParent<'a>>;

/// The one place a `ChunkItem` can sit: its chunk.
#[derive(Debug)]
pub enum ChunkItemParent<'a> {
    Chunk(Box<ChunkPath<'a>>),
}

/// The unwrap the mixed-enum fallback arm converts through.
impl<'a> From<ChunkItemParent<'a>> for ChunkPath<'a> {
    fn from(parent: ChunkItemParent<'a>) -> Self {
        match parent {
            ChunkItemParent::Chunk(chunk) => *chunk,
        }
    }
}

pub type SelectionSetPath<'a> =
    PositionResolutionPath<&'a SelectionSet<BracketsMatched>, ChunkItemParent<'a>>;
```

`SelectionSetItem` is an all-delegate enum (both variants continue in the chunk family), so it uses today's enum emission. `ChunkItem` uses the mixed emission; its derived impl, written out:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for ChunkItem<BracketsMatched> {
    type Parent<'a> = ChunkItemParent<'a>;
    type ResolvedNode<'a> = ResolvedChunkedSelectionSetNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        match self {
            ChunkItem::SelectionSet(inner) => inner.resolve(parent, position),
            _ => Self::ResolvedNode::Chunk(parent.into()),
        }
    }
}
```

### The pass

```rust
/// Chunk every selection-set level of a matched-brackets tree. Infallible: every token
/// that survived the bracket pass lands in some chunk or separator, and no grammar is
/// checked.
pub fn chunk(tree: MatchedBrackets<BracketsMatched>) -> Chunks<BracketsMatched> {
    Chunks(chunk_items(tree.0))
}

/// The separator kind of a token, or `None` for a token that belongs in a chunk.
fn separator_token(kind: NonBracketTokenKind) -> Option<SeparatorToken> {
    match kind {
        NonBracketTokenKind::Comma => Some(SeparatorToken::Comma),
        NonBracketTokenKind::LineBreak => Some(SeparatorToken::LineBreak),
        _ => None,
    }
}

/// One selection-set level: split the runs at separator tokens, absorb each run of
/// separators into one boundary, and let every non-separator item join the chunk that is
/// open where it appears. Brace groups' interiors go through this recursively; paren and
/// square groups move over unchanged.
fn chunk_items(
    items: Vec<WithSpan<BracketItem<BracketsMatched>>>,
) -> Vec<WithSpan<SelectionSetItem<BracketsMatched>>> {
    let mut level = Vec::new();
    let mut boundary: Vec<WithSpan<SeparatorToken>> = Vec::new();
    let mut current: Vec<WithSpan<ChunkItem<BracketsMatched>>> = Vec::new();
    for item in items {
        let WithSpan { item, location } = item;
        match item {
            BracketItem::Inner(Inner(tokens)) => {
                let mut run: Vec<WithSpan<NonBracketTokenKind>> = Vec::new();
                for token in tokens {
                    match separator_token(token.item) {
                        Some(separator) => {
                            flush_run(&mut current, &mut run);
                            flush_chunk(&mut level, &mut current);
                            boundary.push(WithSpan::new(separator, token.location));
                        }
                        None => {
                            flush_separator(&mut level, &mut boundary);
                            run.push(token);
                        }
                    }
                }
                flush_run(&mut current, &mut run);
            }
            BracketItem::Bracketed(bracketed) => {
                flush_separator(&mut level, &mut boundary);
                let chunk_item = match bracketed.opening.item.0 {
                    BracketKind::Brace => ChunkItem::SelectionSet(SelectionSet {
                        opening: bracketed.opening,
                        closing: bracketed.closing,
                        children: chunk_items(bracketed.children),
                    }),
                    BracketKind::Paren | BracketKind::Bracket => {
                        ChunkItem::Bracketed(bracketed)
                    }
                };
                current.push(WithSpan::new(chunk_item, location));
            }
            BracketItem::StrayClose(stray) => {
                flush_separator(&mut level, &mut boundary);
                current.push(WithSpan::new(ChunkItem::StrayClose(stray), location));
            }
        }
    }
    flush_chunk(&mut level, &mut current);
    flush_separator(&mut level, &mut boundary);
    level
}

/// End the separator-free run in progress, if any, into one `Inner` item spanning its
/// first token's start to its last token's end.
fn flush_run(
    items: &mut Vec<WithSpan<ChunkItem<BracketsMatched>>>,
    run: &mut Vec<WithSpan<NonBracketTokenKind>>,
) {
    let span = match (run.first(), run.last()) {
        (Some(first), Some(last)) => Span::join(first.location, last.location),
        _ => return,
    };
    items.push(WithSpan::new(
        ChunkItem::Inner(Inner(std::mem::take(run))),
        span,
    ));
}

/// End the chunk in progress, if any, spanning its first item's start to its last item's
/// end.
fn flush_chunk(
    level: &mut Vec<WithSpan<SelectionSetItem<BracketsMatched>>>,
    current: &mut Vec<WithSpan<ChunkItem<BracketsMatched>>>,
) {
    let span = match (current.first(), current.last()) {
        (Some(first), Some(last)) => Span::join(first.location, last.location),
        _ => return,
    };
    level.push(WithSpan::new(
        SelectionSetItem::Chunk(Chunk(std::mem::take(current))),
        span,
    ));
}

/// End the boundary in progress, if any, into one `Separator` spanning its first token's
/// start to its last token's end.
fn flush_separator(
    level: &mut Vec<WithSpan<SelectionSetItem<BracketsMatched>>>,
    boundary: &mut Vec<WithSpan<SeparatorToken>>,
) {
    let span = match (boundary.first(), boundary.last()) {
        (Some(first), Some(last)) => Span::join(first.location, last.location),
        _ => return,
    };
    level.push(WithSpan::new(
        SelectionSetItem::Separator(Separator(std::mem::take(boundary))),
        span,
    ));
}
```

A chunk in progress flushes when a separator token arrives, and a boundary in progress flushes when chunk content arrives, so a level comes out as chunks and separators alternating, with a boundary allowed first and last.

### Errors

The pipeline tip still answers the errors query. A stray close rides in its chunk, an unclosed selection set keeps its synthetic closing, and a carried-through group's subtree is bracket-stage, so its errors come from the shared collector:

```rust
impl<TContents> Chunks<TContents>
where
    TContents: TreeContents<StrayClose = CloseBracket>,
{
    /// Every bracket error under this tree, in source order of the position each error
    /// starts at.
    pub fn errors(&self) -> Vec<BracketError> {
        let mut errors = Vec::new();
        collect_chunk_errors(&self.0, &mut errors);
        errors
    }
}

fn collect_chunk_errors<TContents>(
    level: &[WithSpan<SelectionSetItem<TContents>>],
    errors: &mut Vec<BracketError>,
) where
    TContents: TreeContents<StrayClose = CloseBracket>,
{
    for level_item in level {
        let SelectionSetItem::Chunk(chunk) = &level_item.item else {
            continue;
        };
        for item in &chunk.0 {
            match &item.item {
                ChunkItem::Inner(_) => {}
                ChunkItem::StrayClose(stray) => {
                    errors.push(BracketError::UnexpectedClose(WithSpan::new(
                        *stray,
                        item.location,
                    )));
                }
                ChunkItem::SelectionSet(selection_set) => {
                    if selection_set.closing.is_none() {
                        errors.push(BracketError::Unclosed(WithSpan::new(
                            UnclosedGroup(selection_set.opening),
                            item.location,
                        )));
                    }
                    collect_chunk_errors(&selection_set.children, errors);
                }
                ChunkItem::Bracketed(bracketed) => {
                    collect_group_errors(bracketed, item.location, errors);
                }
            }
        }
    }
}
```

### Tests

`#[cfg(test)]` in chunk.rs, in the style of the bracket tests: inline literals, `span_of` anchors, structure asserted by direct walks, chunk-query paths asserted through the resolution helpers `chunk_leaf` (expects `ResolvedChunkedSelectionSetNode::Chunk`), `separator_leaf` (expects `::Separator`), `set_leaf` (expects `::SelectionSet`), `set_of` (expects `SelectionSetItemParent::SelectionSet`), `chunk_of` (expects `ChunkItemParent::Chunk`), and `assert_root` (expects `SelectionSetItemParent::Chunks`). The representative path test:

```rust
#[test]
fn a_position_resolves_to_its_chunk() {
    let text = "foo { bar(asdf) }";
    let tree = chunk(match_brackets(tokenize(text)));
    // `asdf` sits inside the carried-through paren group, so the chunk query answers the
    // enclosing chunk `bar(asdf)`; the selection set above it rides in the chunk
    // `foo { ... }` at the root.
    let bar_chunk = chunk_leaf(tree.resolve((), span_of(text, "asdf")));
    assert_eq!(bar_chunk.inner.0.len(), 2);
    let selection_set = set_of(bar_chunk.parent);
    assert!(selection_set.inner.closing.is_some());
    let foo_chunk = chunk_of(selection_set.parent);
    assert_eq!(foo_chunk.inner.0.len(), 2);
    assert_root(foo_chunk.parent);
}
```

The full suite:

- `foo { bar, baz\nqux }`: one top-level chunk of two items; the selection set holds three chunks of one run each, with a separator between each pair; no errors.
- `a, b` and `a\nb` and `a,\n\n,b` all yield two chunks; the third yields exactly one `Separator` holding the four tokens comma, line break, line break, comma.
- `\n, a, b,\n`: two chunks, with boundary nodes leading and trailing and no empty chunks.
- `bar, baz watttt, qux`: three chunks; the middle one holds one run of two identifiers.
- `bar(abc), qux`: two chunks; the first holds the run `bar` and a `ChunkItem::Bracketed` paren group whose interior is the untouched bracket-stage items.
- `foo\n{ bar }`: two chunks; the second holds only the selection set.
- `a(\n) {\n}`: one chunk; the line breaks inside the groups separate nothing at the top level.
- `foo {}`: the selection set holds zero items. `""`: zero items at the root; `" \n , \n "`: one item, a single `Separator`.
- `foo , bar`: the chunk spans equal `span_of(text, "foo")` and `span_of(text, "bar")`, and the separator's span equals `span_of(text, ",")`.
- The path test above.
- The worked example from Behavior, position by position: `foo` answers its chunk, the space before `{` answers the same chunk, `{` and `}` answer the selection set, each line break and the comma answer their `Separator` (the comma and the line break after it the same one), the indent whitespace answers the selection set, `bar` and `baz` answer their chunks.
- `foo, bar`: the comma's `Separator` has the root as its parent.
- `a ) b`: one chunk; the `)` resolves to that `Chunk` (the chunk query does not descend into strays); `errors()` reports the one stray close at `span_of(text, ")")`.
- `foo { bar`: the selection set's closing is `None`; `errors()` reports the one unclosed brace.
- `foo(a`: the carried-through paren group's closing is `None`; `errors()` reports the one unclosed paren, through the shared `collect_group_errors`.
