# Chunking

Requires generic-resolution.md, which is what makes `MatchedBrackets<Chunked>` resolvable at the tree level; the resolution section below adds the run-interior step on top of it.

The pass after bracket matching. Chunking maps `MatchedBrackets<BracketsMatched>` to `MatchedBrackets<Chunked>` through `map`: the tree keeps its shape — the same groups, nesting, and strays — and every token run is replaced by its chunked form, an alternation of chunks (separator-free token runs) and separators (the comma and line-break tokens between them). Chunking is infallible. It validates nothing and emits no errors; every token of every run lands in a chunk or a separator. A chunk is parsed independently by the chunk-parsing pass later, and the bracket errors stay derivable from the chunked tree unchanged.

A group is not part of any chunk. In `foo { bar }` the top level is a chunked run holding the chunk `foo`, followed by the brace group as its sibling item; that a selection is a chunk plus the group after it is an adjacency the chunk-parsing pass reads off the level when it assembles selections.

## Behavior

- Commas and line breaks are the separators, equivalent. Any nonempty mix of consecutive separators is one boundary, held as one `Separator`. A run never produces an empty chunk: separators at a run's start or end are just boundary nodes there.
- Separators are required between fields, so `baz watttt` is one chunk, and it is that chunk's own parse that later fails ("expected a comma or line break"). Chunking never splits on token shape.
- A separator splits only its own run. Runs end at brackets, so a chunk never spans a group, and the separators inside a group's interior runs never affect the level outside the group.
- A chunk's span runs from its first token's start to its last token's end; a separator's span likewise. Whitespace between tokens belongs to no node.

```
foo { bar, baz
qux }
```

chunks as: the top-level run becomes the one chunk `foo`; the brace group's interior run becomes chunk `bar`, separator `,`, chunk `baz`, separator (the line break), chunk `qux`.

```
a(
) {
}
```

has one chunk, `a`: each group's interior run is a lone separator, and the top level has no separator anywhere. `foo\n{ bar }` chunks as chunk `foo`, then a separator, then the brace group — that separator between the chunk and the group is what the chunk-parsing pass rejects when it assembles selections, so a selection set's brace still has to open on its field's line.

`foo { bar(a: }) }` chunks whatever the bracket pass produced, strays included: the top level keeps the strays `)` and `}` as items, untouched, and `a :` becomes the one chunk of the parenthesis group's interior.

## The change

New module `crates/isograph_parser/src/chunk.rs`, registered in lib.rs alongside the existing modules:

```rust
// from crates/isograph_parser/src/lib.rs
mod chunk;
mod matched_brackets;
...
pub use chunk::*;
pub use matched_brackets::*;
```

The stage and its run type:

```rust
// from crates/isograph_parser/src/chunk.rs
use span::{Span, WithSpan};

use crate::{
    BracketsMatched, CloseBracket, Inner, MatchedBrackets, NonBracketTokenKind, TreeContents,
};

/// The stage `chunk` produces. The tree keeps the bracket tree's shape, with every run chunked.
#[derive(Debug, PartialEq, Eq)]
pub struct Chunked;

impl TreeContents for Chunked {
    type Inner = ChunkedRun;
    type StrayClose = CloseBracket;
}

/// One run, chunked into alternating chunks and separators, built with no two adjacent
/// separators (one boundary absorbs consecutive separator tokens) and no empty chunks.
#[derive(Debug, PartialEq, Eq)]
pub struct ChunkedRun(pub Vec<WithSpan<ChunkedRunItem>>);

#[derive(Debug, PartialEq, Eq)]
pub enum ChunkedRunItem {
    Chunk(Chunk),
    Separator(Separator),
}

/// A maximal separator-free run of tokens; the chunk-parsing pass consumes it as one
/// unit. The wrapping `WithSpan`'s span runs from the first token's start to the last
/// token's end.
#[derive(Debug, PartialEq, Eq)]
pub struct Chunk(pub Vec<WithSpan<NonBracketTokenKind>>);

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

The pass. `chunk_run` consumes each run through a peekable, the discipline the matcher already uses: each outer iteration emits exactly one complete node, so the alternation invariant holds by loop structure, with no accumulators crossing function boundaries.

```rust
// from crates/isograph_parser/src/chunk.rs
/// Chunk every run of a matched-brackets tree. The pass is infallible. Every token
/// lands in a chunk or a separator, and no grammar is checked.
pub fn chunk(tree: MatchedBrackets<BracketsMatched>) -> MatchedBrackets<Chunked> {
    tree.map(&mut |run| chunk_run(run.item), &mut |stray| stray.item)
}

/// The separator kind of a token, or `None` for a token that belongs in a chunk.
fn separator_token(kind: NonBracketTokenKind) -> Option<SeparatorToken> {
    match kind {
        NonBracketTokenKind::Comma => Some(SeparatorToken::Comma),
        NonBracketTokenKind::LineBreak => Some(SeparatorToken::LineBreak),
        _ => None,
    }
}

/// Split one token run at separator tokens; consecutive separator tokens collapse into
/// one boundary.
fn chunk_run(Inner(tokens): Inner) -> ChunkedRun {
    let mut tokens = tokens.into_iter().peekable();
    let mut items = Vec::new();
    while let Some(&first) = tokens.peek() {
        if separator_token(first.item).is_some() {
            let mut span = first.location;
            let mut boundary = Vec::new();
            while let Some(&token) = tokens.peek() {
                let Some(separator) = separator_token(token.item) else {
                    break;
                };
                tokens.next();
                span = Span::join(span, token.location);
                boundary.push(WithSpan::new(separator, token.location));
            }
            items.push(WithSpan::new(
                ChunkedRunItem::Separator(Separator(boundary)),
                span,
            ));
        } else {
            let mut span = first.location;
            let mut chunk = Vec::new();
            while let Some(&token) = tokens.peek() {
                if separator_token(token.item).is_some() {
                    break;
                }
                tokens.next();
                span = Span::join(span, token.location);
                chunk.push(token);
            }
            items.push(WithSpan::new(ChunkedRunItem::Chunk(Chunk(chunk)), span));
        }
    }
    ChunkedRun(items)
}
```

`errors()` needs nothing: its bound is `TreeContents<StrayClose = CloseBracket>`, which `Chunked` satisfies, so `MatchedBrackets<Chunked>` answers the errors query as-is.

### Resolution

The tree query needs nothing from this doc: generic-resolution.md makes `MatchedBrackets<Chunked>` resolve as-is, with a position in a run answering `ResolvedBracketNode::Inner(InnerPath<'a, Chunked>)` — a path to the `ChunkedRun` with its full ancestry. The chunk-level question is the second step: `ChunkedRun` resolves its own interior, parented by that `InnerPath`, so a chunk's path chains run, group, root.

```rust
// from crates/isograph_parser/src/chunk.rs
/// Every node a position can resolve to inside one chunked run: the run itself (a
/// position on whitespace between its items), a chunk, or a separator.
#[derive(Debug)]
#[cfg_attr(test, derive(derive_more::Unwrap))]
pub enum ResolvedChunkedRunNode<'a> {
    ChunkedRun(ChunkedRunPath<'a>),
    Chunk(ChunkPath<'a>),
    Separator(SeparatorPath<'a>),
}

pub type ChunkedRunPath<'a> =
    PositionResolutionPath<&'a ChunkedRun, InnerPath<'a, Chunked>>;

/// The one place a `ChunkedRunItem` can sit: its run.
#[derive(Debug)]
pub enum ChunkedRunItemParent<'a> {
    ChunkedRun(ChunkedRunPath<'a>),
}

pub type ChunkPath<'a> = PositionResolutionPath<&'a Chunk, ChunkedRunItemParent<'a>>;
pub type SeparatorPath<'a> = PositionResolutionPath<&'a Separator, ChunkedRunItemParent<'a>>;
```

The derives, replacing the plain derive lines shown above — `ChunkedRun` delegates into its items, and the items are leaves; `Chunk` and `Separator` implement nothing:

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = InnerPath<'a, Chunked>,
    resolved_node = ResolvedChunkedRunNode<'a>
)]
pub struct ChunkedRun(#[resolve_field] pub Vec<WithSpan<ChunkedRunItem>>);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = ChunkedRunItemParent<'a>,
    resolved_node = ResolvedChunkedRunNode<'a>
)]
pub enum ChunkedRunItem {
    #[resolve_field(leaf = Chunk)]
    Chunk(Chunk),
    #[resolve_field(leaf = Separator)]
    Separator(Separator),
}
```

with the one `From` the delegation needs:

```rust
// from crates/isograph_parser/src/chunk.rs
impl<'a> From<ChunkedRunPath<'a>> for ChunkedRunItemParent<'a> {
    fn from(path: ChunkedRunPath<'a>) -> Self {
        ChunkedRunItemParent::ChunkedRun(path)
    }
}
```

The two-step query, as a consumer writes it: resolve the tree, and when the answer is `Inner`, resolve the run with that path as the parent.

```rust
// from crates/isograph_parser/src/chunk.rs (tests)
let run_path = tree.resolve((), position).unwrap_inner();
let node = run_path.inner.resolve(run_path, position);
```

### Tests

`#[cfg(test)]` in chunk.rs, written out in full. `span_of` is the same anchor helper the bracket tests use; the extractors panic with the mismatch when a walk hits the wrong item kind, as the bracket tests' extractors do.

```rust
// from crates/isograph_parser/src/chunk.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BracketItem, Bracketed, match_brackets, tokenize};

    fn chunked(literal: &str) -> MatchedBrackets<Chunked> {
        chunk(match_brackets(tokenize(literal)))
    }

    /// The span of `pattern`, which must occur exactly once in `text`: an anchor an edit
    /// cannot silently shift, and one that fails loudly when it stops being unique.
    fn span_of(text: &str, pattern: &str) -> Span {
        let mut occurrences = text.match_indices(pattern);
        let (offset, _) = occurrences
            .next()
            .expect("the pattern the test anchors on occurs in the literal");
        assert!(
            occurrences.next().is_none(),
            "the pattern the test anchors on occurs exactly once in the literal"
        );
        Span::from_usize(offset, offset + pattern.len())
    }

    fn run(items: &[WithSpan<BracketItem<Chunked>>], index: usize) -> &ChunkedRun {
        match &items[index].item {
            BracketItem::Inner(run) => run,
            item => panic!("expected a chunked run at {index}, got {item:?}"),
        }
    }

    fn group(items: &[WithSpan<BracketItem<Chunked>>], index: usize) -> &Bracketed<Chunked> {
        match &items[index].item {
            BracketItem::Bracketed(group) => group,
            item => panic!("expected a group at {index}, got {item:?}"),
        }
    }

    fn chunk_at(run: &ChunkedRun, index: usize) -> &WithSpan<ChunkedRunItem> {
        let item = &run.0[index];
        assert!(
            matches!(item.item, ChunkedRunItem::Chunk(_)),
            "expected a chunk at {index}, got {item:?}"
        );
        item
    }

    fn separator_at(run: &ChunkedRun, index: usize) -> &WithSpan<ChunkedRunItem> {
        let item = &run.0[index];
        assert!(
            matches!(item.item, ChunkedRunItem::Separator(_)),
            "expected a separator at {index}, got {item:?}"
        );
        item
    }

    /// The token kinds of the chunk at `index`.
    fn chunk_kinds(run: &ChunkedRun, index: usize) -> Vec<NonBracketTokenKind> {
        match &chunk_at(run, index).item {
            ChunkedRunItem::Chunk(Chunk(tokens)) => {
                tokens.iter().map(|token| token.item).collect()
            }
            item => panic!("expected a chunk at {index}, got {item:?}"),
        }
    }

    /// The separator kinds of the boundary at `index`.
    fn separator_kinds(run: &ChunkedRun, index: usize) -> Vec<SeparatorToken> {
        match &separator_at(run, index).item {
            ChunkedRunItem::Separator(Separator(tokens)) => {
                tokens.iter().map(|token| token.item).collect()
            }
            item => panic!("expected a separator at {index}, got {item:?}"),
        }
    }

    #[test]
    fn a_selection_set_chunks_by_separator() {
        let tree = chunked("foo { bar, baz\nqux }");
        assert_eq!(tree.0.len(), 2);
        let top = run(&tree.0, 0);
        assert_eq!(top.0.len(), 1);
        assert_eq!(chunk_kinds(top, 0), vec![NonBracketTokenKind::Identifier]);
        let brace = group(&tree.0, 1);
        assert_eq!(brace.children.len(), 1);
        let interior = run(&brace.children, 0);
        assert_eq!(interior.0.len(), 5);
        assert_eq!(chunk_kinds(interior, 0), vec![NonBracketTokenKind::Identifier]);
        assert_eq!(separator_kinds(interior, 1), vec![SeparatorToken::Comma]);
        assert_eq!(chunk_kinds(interior, 2), vec![NonBracketTokenKind::Identifier]);
        assert_eq!(separator_kinds(interior, 3), vec![SeparatorToken::LineBreak]);
        assert_eq!(chunk_kinds(interior, 4), vec![NonBracketTokenKind::Identifier]);
        assert_eq!(tree.errors(), vec![]);
    }

    #[test]
    fn a_comma_a_line_break_and_a_mix_separate_identically() {
        for text in ["a, b", "a\nb"] {
            let tree = chunked(text);
            let top = run(&tree.0, 0);
            assert_eq!(top.0.len(), 3, "for {text:?}");
            chunk_at(top, 0);
            separator_at(top, 1);
            chunk_at(top, 2);
        }
        let tree = chunked("a,\n\n,b");
        let top = run(&tree.0, 0);
        assert_eq!(top.0.len(), 3);
        assert_eq!(
            separator_kinds(top, 1),
            vec![
                SeparatorToken::Comma,
                SeparatorToken::LineBreak,
                SeparatorToken::LineBreak,
                SeparatorToken::Comma,
            ]
        );
    }

    #[test]
    fn boundaries_sit_first_and_last_with_no_empty_chunks() {
        let tree = chunked("\n, a, b,\n");
        let top = run(&tree.0, 0);
        assert_eq!(top.0.len(), 5);
        separator_at(top, 0);
        chunk_at(top, 1);
        separator_at(top, 2);
        chunk_at(top, 3);
        separator_at(top, 4);
    }

    #[test]
    fn garbage_between_separators_is_one_chunk() {
        let tree = chunked("bar, baz watttt, qux");
        let top = run(&tree.0, 0);
        assert_eq!(top.0.len(), 5);
        assert_eq!(
            chunk_kinds(top, 2),
            vec![
                NonBracketTokenKind::Identifier,
                NonBracketTokenKind::Identifier,
            ]
        );
    }

    #[test]
    fn a_group_sits_between_the_runs_it_splits() {
        let tree = chunked("bar(abc), qux");
        assert_eq!(tree.0.len(), 3);
        let before = run(&tree.0, 0);
        assert_eq!(before.0.len(), 1);
        assert_eq!(chunk_kinds(before, 0), vec![NonBracketTokenKind::Identifier]);
        let parenthesis = group(&tree.0, 1);
        let interior = run(&parenthesis.children, 0);
        assert_eq!(interior.0.len(), 1);
        assert_eq!(chunk_kinds(interior, 0), vec![NonBracketTokenKind::Identifier]);
        let after = run(&tree.0, 2);
        assert_eq!(after.0.len(), 2);
        separator_at(after, 0);
        assert_eq!(chunk_kinds(after, 1), vec![NonBracketTokenKind::Identifier]);
    }

    #[test]
    fn a_line_break_before_the_brace_is_a_boundary() {
        let tree = chunked("foo\n{ bar }");
        let top = run(&tree.0, 0);
        assert_eq!(top.0.len(), 2);
        chunk_at(top, 0);
        separator_at(top, 1);
        group(&tree.0, 1);
    }

    #[test]
    fn separators_inside_groups_do_not_reach_the_outer_level() {
        let tree = chunked("a(\n) {\n}");
        assert_eq!(tree.0.len(), 3);
        let top = run(&tree.0, 0);
        assert_eq!(top.0.len(), 1);
        assert_eq!(chunk_kinds(top, 0), vec![NonBracketTokenKind::Identifier]);
        let parenthesis = group(&tree.0, 1);
        assert_eq!(separator_kinds(run(&parenthesis.children, 0), 0), vec![SeparatorToken::LineBreak]);
        let brace = group(&tree.0, 2);
        assert_eq!(separator_kinds(run(&brace.children, 0), 0), vec![SeparatorToken::LineBreak]);
    }

    #[test]
    fn empty_trees_stay_empty() {
        let tree = chunked("foo {}");
        assert!(group(&tree.0, 1).children.is_empty());
        assert!(chunked("").0.is_empty());
        let tree = chunked(" \n , \n ");
        let top = run(&tree.0, 0);
        assert_eq!(top.0.len(), 1);
        assert_eq!(
            separator_kinds(top, 0),
            vec![
                SeparatorToken::LineBreak,
                SeparatorToken::Comma,
                SeparatorToken::LineBreak,
            ]
        );
    }

    #[test]
    fn chunk_and_separator_spans_are_tight() {
        let text = "foo , bar";
        let tree = chunked(text);
        let top = run(&tree.0, 0);
        assert_eq!(chunk_at(top, 0).location, span_of(text, "foo"));
        assert_eq!(separator_at(top, 1).location, span_of(text, ","));
        assert_eq!(chunk_at(top, 2).location, span_of(text, "bar"));
    }

    #[test]
    fn a_stray_close_carries_through_between_runs() {
        let text = "a ) b";
        let tree = chunked(text);
        assert_eq!(tree.0.len(), 3);
        assert!(matches!(
            tree.0[1].item,
            BracketItem::StrayClose(CloseBracket(crate::BracketKind::Parenthesis))
        ));
        match tree.errors().as_slice() {
            [crate::BracketError::UnexpectedClose(stray)] => {
                assert_eq!(stray.location, span_of(text, ")"));
            }
            errors => panic!("expected exactly the stray close, got {errors:?}"),
        }
    }

    /// The two-step query: the tree answers the run, the run answers its interior.
    fn resolve_within_run(
        tree: &MatchedBrackets<Chunked>,
        position: Span,
    ) -> ResolvedChunkedRunNode<'_> {
        let run_path = tree.resolve((), position).unwrap_inner();
        run_path.inner.resolve(run_path, position)
    }

    #[test]
    fn a_chunk_resolves_with_its_full_ancestry() {
        let text = "foo { bar, baz }";
        let tree = chunked(text);
        let baz = resolve_within_run(&tree, span_of(text, "baz")).unwrap_chunk();
        let ChunkedRunItemParent::ChunkedRun(interior) = baz.parent;
        match interior.parent.parent {
            BracketItemParent::Bracketed(brace) => {
                assert!(brace.inner.closing.is_some());
                assert!(matches!(
                    brace.parent,
                    BracketItemParent::MatchedBrackets(_)
                ));
            }
            parent => panic!("expected the brace group, got {parent:?}"),
        }
    }

    #[test]
    fn a_separator_and_a_brace_resolve_to_their_nodes() {
        let text = "foo { bar, baz }";
        let tree = chunked(text);
        let comma = resolve_within_run(&tree, span_of(text, ",")).unwrap_separator();
        let ChunkedRunItemParent::ChunkedRun(interior) = comma.parent;
        assert!(matches!(
            interior.parent.parent,
            BracketItemParent::Bracketed(_)
        ));
        assert!(matches!(
            tree.resolve((), span_of(text, "{")),
            ResolvedBracketNode::OpenBracket(_)
        ));
    }

    #[test]
    fn an_unclosed_group_keeps_its_none_closing() {
        let text = "foo { bar";
        let tree = chunked(text);
        let brace = group(&tree.0, 1);
        assert!(brace.closing.is_none());
        assert_eq!(chunk_kinds(run(&brace.children, 0), 0), vec![NonBracketTokenKind::Identifier]);
        match tree.errors().as_slice() {
            [crate::BracketError::Unclosed(unclosed)] => {
                assert_eq!(unclosed.item.0.location, span_of(text, "{"));
            }
            errors => panic!("expected exactly the unclosed brace, got {errors:?}"),
        }
    }
}
```

### Landing checklist

1. Add chunk.rs with the types, the pass, and the test module above; register `mod chunk;` and `pub use chunk::*;` in lib.rs.
2. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
3. Move this doc to refactors/past.
