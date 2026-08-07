# Chunking selection sets

The pass after bracket matching. It takes `MatchedBrackets<BracketsMatched>`, assumes the literal is a selection set, and regroups every selection-set level (the root, and each brace group's interior, recursively) into chunks, one chunk per selection. Paren and square groups are not chunked at this stage: they are carried through unchanged as `Bracketed`, and this stage is cleanly separable from whatever chunks them later. Chunking is infallible: it validates nothing, emits no errors, and every token that survived the bracket pass lands in some chunk. Each chunk is parsed independently by later passes.

The chunked tree has its own path enum, `ResolvedChunkedSelectionSetNode`, fully separate from `ResolvedBracketNode`. Resolving a position against the bracket tree answers at token granularity (a run of `Vec<WithSpan<NonBracketTokenKind>>`, an opening, a stray close); resolving the same position against the chunked tree answers at chunk granularity (which chunk, which selection set). Two trees, two independent queries; nothing in `matched_brackets.rs`'s path family changes.

## Behavior

- Commas and line breaks are the separators, equivalent. Any nonempty mix of consecutive separators is one boundary. Separators at the start or end of a level produce no empty chunks.
- Separators are required between fields, so `baz watttt` is one chunk, and it is that chunk's own parse that later fails ("expected a comma or line break"). Chunking never splits on token shape.
- A separator only separates at its own level. A group is one opaque item at the level it appears in, so the separators inside a paren or square group never split a selection-set level.
- A chunk is a maximal separator-free sequence of a level's items. Runs split at separator tokens; a group or a stray close joins the chunk that is open where it appears. A group right after a boundary opens a chunk of its own.
- A chunk's span runs from its first item's start to its last item's end; separators sit outside every chunk.

```
foo { bar, baz
qux }
```

is four chunks: one at the top level (the run `foo` plus the brace group), containing a selection set of three chunks `bar`, `baz`, `qux`.

```
a(
) {
}
```

is one chunk: both line breaks sit inside groups, so the top level has no separator anywhere. Consequence in the other direction: `foo\n{ bar }` is two chunks; a selection set's brace has to open on its field's line.

`foo { bar(a: }) }` chunks whatever the bracket pass produced: the top level is one chunk (no separators) holding `foo`, the brace group, and the two strays `)` and `}`. Bracket errors keep their bracket-pass positions; chunking neither fixes nor widens them.

## Ordered changes

1. The mixed-enum derive in resolve-option-like-enums.md (its Change 1, revised there to let a marked payload variant delegate). `ChunkItem`'s derive needs it; it ships first, in that doc.
2. Change 1 below: extract `collect_group_errors` in matched_brackets.rs. Independently shippable.
3. Change 2 below: the chunk module and its tests.

## Change 1 (prefactor): extract `collect_group_errors`

The group arm of the existing error collector becomes a `pub(crate)` function, so the chunk stage's collector can reuse it for carried-through groups. In `matched_brackets.rs`:

Before:

```rust
            BracketItem::Bracketed(bracketed) => {
                if matches!(bracketed.closing.item, Closing::Synthetic(())) {
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
            BracketItem::Bracketed(bracketed) => {
                collect_group_errors(bracketed, item.location, errors);
            }
```

```rust
/// The errors of one group and its subtree: its own synthetic closing, if any, then its
/// children's errors.
pub(crate) fn collect_group_errors<TContents>(
    bracketed: &Bracketed<TContents>,
    group_span: Span,
    errors: &mut Vec<BracketError>,
) where
    TContents: TreeContents<Stray = UnmatchedClose, Unclosed = ()>,
{
    if matches!(bracketed.closing.item, Closing::Synthetic(())) {
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

There is no new stage marker. The chunked tree holds the same contents as the bracket tree (`Inner` runs, `UnmatchedClose` strays, `()` synthetic closings); what changes is the shape, so the chunk types are generic over the same `TreeContents` and their derives instantiate at `BracketsMatched` via `self_type_generics`. `Bracketed` moves into the chunk tree wholesale for paren and square groups, and closings move over untouched.

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
    #[resolve_field] pub Vec<WithSpan<Chunk<TContents>>>,
);

/// A maximal separator-free sequence of a level's items: one selection. The wrapping
/// `WithSpan`'s span runs from the first item's start to the last item's end.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = ChunkParent<'a>,
    resolved_node = ResolvedChunkedSelectionSetNode<'a>,
    self_type_generics = <BracketsMatched>
)]
pub struct Chunk<TContents: TreeContents>(
    #[resolve_field] pub Vec<WithSpan<ChunkItem<TContents>>>,
);

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
    StrayClose(TContents::Stray),
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
    /// A real closing's span is its close token; a synthetic closing's span is zero-width
    /// where the close should have been.
    pub closing: WithSpan<Closing<TContents>>,
    #[resolve_field]
    pub children: Vec<WithSpan<Chunk<TContents>>>,
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
    SelectionSet(SelectionSetPath<'a>),
}

pub type ChunksPath<'a> = PositionResolutionPath<&'a Chunks<BracketsMatched>, ()>;

/// Everything a `Chunk` can sit inside.
#[derive(Debug)]
pub enum ChunkParent<'a> {
    Chunks(ChunksPath<'a>),
    SelectionSet(Box<SelectionSetPath<'a>>),
}

pub type ChunkPath<'a> = PositionResolutionPath<&'a Chunk<BracketsMatched>, ChunkParent<'a>>;

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

`ChunkItem`'s derived impl, written out (the mixed-enum emission: the marked payload variant delegates, the unmarked variants answer the fallback):

```rust
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

So the chunk query answers: a position on a run, a stray close, or anywhere inside a carried-through paren or square group resolves to the enclosing `Chunk`; a position inside a brace group recurses into that selection set's chunks; a position on a separator or a brace, or on whitespace between chunks, resolves to the enclosing `SelectionSet` or to `Chunks` at the root.

### The pass

```rust
/// Chunk every selection-set level of a matched-brackets tree. Infallible: every token
/// that survived the bracket pass lands in some chunk, and no grammar is checked.
pub fn chunk(tree: MatchedBrackets<BracketsMatched>) -> Chunks<BracketsMatched> {
    Chunks(chunk_items(tree.0))
}

/// Commas and line breaks separate chunks, equivalently.
fn is_separator(kind: NonBracketTokenKind) -> bool {
    matches!(
        kind,
        NonBracketTokenKind::Comma | NonBracketTokenKind::LineBreak
    )
}

/// One selection-set level: split the runs at separator tokens, and let every
/// non-separator item join the chunk that is open where it appears. Consecutive separators
/// never produce an empty chunk. Brace groups' interiors go through this recursively;
/// paren and square groups move over unchanged.
fn chunk_items(
    items: Vec<WithSpan<BracketItem<BracketsMatched>>>,
) -> Vec<WithSpan<Chunk<BracketsMatched>>> {
    let mut chunks = Vec::new();
    let mut current: Vec<WithSpan<ChunkItem<BracketsMatched>>> = Vec::new();
    for item in items {
        let WithSpan { item, location } = item;
        match item {
            BracketItem::Inner(Inner(tokens)) => {
                let mut run: Vec<WithSpan<NonBracketTokenKind>> = Vec::new();
                for token in tokens {
                    if is_separator(token.item) {
                        flush_run(&mut current, &mut run);
                        flush_chunk(&mut chunks, &mut current);
                    } else {
                        run.push(token);
                    }
                }
                flush_run(&mut current, &mut run);
            }
            BracketItem::Bracketed(bracketed) => {
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
                current.push(WithSpan::new(ChunkItem::StrayClose(stray), location));
            }
        }
    }
    flush_chunk(&mut chunks, &mut current);
    chunks
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
    chunks: &mut Vec<WithSpan<Chunk<BracketsMatched>>>,
    current: &mut Vec<WithSpan<ChunkItem<BracketsMatched>>>,
) {
    let span = match (current.first(), current.last()) {
        (Some(first), Some(last)) => Span::join(first.location, last.location),
        _ => return,
    };
    chunks.push(WithSpan::new(Chunk(std::mem::take(current)), span));
}
```

### Errors

The pipeline tip still answers the errors query. A stray close rides in its chunk, an unclosed selection set keeps its synthetic closing, and a carried-through group's subtree is bracket-stage, so its errors come from the shared collector:

```rust
impl<TContents> Chunks<TContents>
where
    TContents: TreeContents<Stray = UnmatchedClose, Unclosed = ()>,
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
    chunks: &[WithSpan<Chunk<TContents>>],
    errors: &mut Vec<BracketError>,
) where
    TContents: TreeContents<Stray = UnmatchedClose, Unclosed = ()>,
{
    for chunk in chunks {
        for item in &chunk.item.0 {
            match &item.item {
                ChunkItem::Inner(_) => {}
                ChunkItem::StrayClose(stray) => {
                    errors.push(BracketError::UnexpectedClose(WithSpan::new(
                        *stray,
                        item.location,
                    )));
                }
                ChunkItem::SelectionSet(selection_set) => {
                    if matches!(selection_set.closing.item, Closing::Synthetic(())) {
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

`#[cfg(test)]` in chunk.rs, in the style of the bracket tests: inline literals, `span_of` anchors, structure asserted by direct walks, chunk-query paths asserted through the resolution helpers `chunk_leaf` (expects `ResolvedChunkedSelectionSetNode::Chunk`), `set_leaf` (expects `::SelectionSet`), `set_of` (expects `ChunkParent::SelectionSet`), `chunk_of` (expects `ChunkItemParent::Chunk`), and `assert_root` (expects `ChunkParent::Chunks`). The representative path test:

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
    assert!(matches!(selection_set.inner.closing.item, Closing::Real));
    let foo_chunk = chunk_of(selection_set.parent);
    assert_eq!(foo_chunk.inner.0.len(), 2);
    assert_root(foo_chunk.parent);
}
```

The full suite:

- `foo { bar, baz\nqux }`: one top-level chunk of two items; the selection set holds three chunks of one run each; no errors.
- `a, b` and `a\nb` and `a,\n\n,b` chunk identically: two chunks.
- `\n, a, b,\n`: two chunks; leading, trailing, and consecutive separators produce no empty chunks.
- `bar, baz watttt, qux`: three chunks; the middle one holds one run of two identifiers.
- `bar(abc), qux`: two chunks; the first holds the run `bar` and a `ChunkItem::Bracketed` paren group whose interior is the untouched bracket-stage items.
- `foo\n{ bar }`: two chunks; the second holds only the selection set.
- `a(\n) {\n}`: one chunk; the line breaks inside the groups separate nothing at the top level.
- `foo {}`: the selection set holds zero chunks. `""` and `" \n , \n "`: zero chunks at the root.
- `foo , bar`: chunk spans equal `span_of(text, "foo")` and `span_of(text, "bar")`.
- The path test above.
- `foo { bar, baz }`: the comma resolves to the `SelectionSet` leaf; in `foo, bar` the comma resolves to the `Chunks` root; the `{` resolves to the `SelectionSet` leaf.
- `a ) b`: one chunk; the `)` resolves to that `Chunk` (the chunk query does not descend into strays); `errors()` reports the one stray close at `span_of(text, ")")`.
- `foo { bar`: the selection set's closing is `Closing::Synthetic(())`; `errors()` reports the one unclosed brace.
- `foo(a`: the carried-through paren group's closing is synthetic; `errors()` reports the one unclosed paren, through the shared `collect_group_errors`.
