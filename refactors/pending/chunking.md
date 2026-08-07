# Chunking

The pass after bracket matching. Chunking maps `MatchedBrackets<BracketsMatched>` to `MatchedBrackets<Chunked>`: the tree keeps its shape — the same groups, nesting, and strays — and every token run is replaced by its chunked form, an alternation of chunks (separator-free token runs) and separators (the comma and line-break tokens between them). Chunking is infallible. It validates nothing and emits no errors; every token of every run lands in a chunk or a separator. A chunk is parsed independently by the chunk-parsing pass later, and the bracket errors stay derivable from the chunked tree unchanged.

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

## Change 1 (prefactor): `map`

`MatchedBrackets` gains the infallible counterpart of `try_map`, for passes that cannot refuse; without it, an infallible pass through `try_map` would unwrap a `Result` that cannot fail. In `matched_brackets.rs`, beside `try_map`:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
impl<TFrom: TreeContents> MatchedBrackets<TFrom> {
    /// Cross the tree to another stage, mapping every slot. The shape is unchanged.
    pub fn map<TTo: TreeContents>(
        self,
        map_inner: &mut impl FnMut(WithSpan<TFrom::Inner>) -> TTo::Inner,
        map_stray_close: &mut impl FnMut(WithSpan<TFrom::StrayClose>) -> TTo::StrayClose,
    ) -> MatchedBrackets<TTo> {
        MatchedBrackets(map_items(self.0, map_inner, map_stray_close))
    }
}

fn map_items<TFrom, TTo>(
    items: Vec<WithSpan<BracketItem<TFrom>>>,
    map_inner: &mut impl FnMut(WithSpan<TFrom::Inner>) -> TTo::Inner,
    map_stray_close: &mut impl FnMut(WithSpan<TFrom::StrayClose>) -> TTo::StrayClose,
) -> Vec<WithSpan<BracketItem<TTo>>>
where
    TFrom: TreeContents,
    TTo: TreeContents,
{
    items
        .into_iter()
        .map(|with_span| {
            let WithSpan {
                item,
                location: span,
            } = with_span;
            let item = match item {
                BracketItem::Inner(inner) => {
                    BracketItem::Inner(map_inner(WithSpan::new(inner, span)))
                }
                BracketItem::StrayClose(stray) => {
                    BracketItem::StrayClose(map_stray_close(WithSpan::new(stray, span)))
                }
                BracketItem::Bracketed(Bracketed {
                    opening,
                    children,
                    closing,
                }) => BracketItem::Bracketed(Bracketed {
                    opening,
                    children: map_items(children, map_inner, map_stray_close),
                    closing,
                }),
            };
            WithSpan::new(item, span)
        })
        .collect()
}
```

Independently shippable; `try_map` is untouched.

## Change 2: the chunk stage

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

The pass:

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
    let mut items = Vec::new();
    let mut chunk: Vec<WithSpan<NonBracketTokenKind>> = Vec::new();
    let mut boundary: Vec<WithSpan<SeparatorToken>> = Vec::new();
    for token in tokens {
        match separator_token(token.item) {
            Some(separator) => {
                flush_chunk(&mut items, &mut chunk);
                boundary.push(WithSpan::new(separator, token.location));
            }
            None => {
                flush_separator(&mut items, &mut boundary);
                chunk.push(token);
            }
        }
    }
    flush_chunk(&mut items, &mut chunk);
    flush_separator(&mut items, &mut boundary);
    ChunkedRun(items)
}

/// End the chunk in progress, if any, spanning its first token's start to its last
/// token's end.
fn flush_chunk(
    items: &mut Vec<WithSpan<ChunkedRunItem>>,
    chunk: &mut Vec<WithSpan<NonBracketTokenKind>>,
) {
    let span = match (chunk.first(), chunk.last()) {
        (Some(first), Some(last)) => Span::join(first.location, last.location),
        _ => return,
    };
    items.push(WithSpan::new(
        ChunkedRunItem::Chunk(Chunk(std::mem::take(chunk))),
        span,
    ));
}

/// End the boundary in progress, if any, spanning its first token's start to its last
/// token's end.
fn flush_separator(
    items: &mut Vec<WithSpan<ChunkedRunItem>>,
    boundary: &mut Vec<WithSpan<SeparatorToken>>,
) {
    let span = match (boundary.first(), boundary.last()) {
        (Some(first), Some(last)) => Span::join(first.location, last.location),
        _ => return,
    };
    items.push(WithSpan::new(
        ChunkedRunItem::Separator(Separator(std::mem::take(boundary))),
        span,
    ));
}
```

A chunk in progress flushes when a separator token arrives and a boundary in progress flushes when a chunk token arrives, so a run comes out as chunks and separators alternating, with a boundary allowed first and last.

`errors()` needs nothing: its bound is `TreeContents<StrayClose = CloseBracket>`, which `Chunked` satisfies, so `MatchedBrackets<Chunked>` answers the errors query as-is.

Resolution over the chunked tree is the resolve phase, after chunking lands: the derives must emit impls generic over `TContents` (today `self_type_generics` pins them to `<BracketsMatched>`), so that resolving `MatchedBrackets<Chunked>` monomorphizes the same walk with `ChunkedRun`'s own resolve at the run slot and chunk and separator leaves. That work gets its own doc; nothing in this one blocks on it.

### Tests

`#[cfg(test)]` in chunk.rs, in the style of the bracket tests: inline literals, `span_of` anchors, assertions by walking the mapped tree. The suite:

- `foo { bar, baz\nqux }`: the top-level run is one chunk `foo` of one identifier token; the brace group's interior run is chunk, separator, chunk, separator, chunk, and the separator after `baz` holds one line-break token.
- `a, b` and `a\nb` and `a,\n\n,b` all chunk to chunk-separator-chunk; the third's separator holds the four tokens comma, line break, line break, comma.
- `\n, a, b,\n`: separator, chunk, separator, chunk, separator — boundaries first and last, no empty chunks.
- `bar, baz watttt, qux`: three chunks; the middle one holds two identifier tokens.
- `bar(abc), qux`: the top level is chunk `bar`, then the parenthesis group (its interior one chunk `abc`), then a run holding separator, chunk `qux`.
- `foo\n{ bar }`: chunk `foo` and a separator, then the brace group as the next item.
- `a(\n) {\n}`: each group's interior run is one lone separator; the top level's runs hold one chunk `a` and nothing else.
- `foo {}`: the brace group has no children. `""`: no items. `" \n , \n "`: one run holding one `Separator` with all three separator tokens.
- `foo , bar`: the chunk spans equal `span_of(text, "foo")` and `span_of(text, "bar")`, and the separator's span equals `span_of(text, ",")`.
- `a ) b`: the stray `)` carries through as `BracketItem::StrayClose(CloseBracket(Parenthesis))` between two one-chunk runs; `errors()` on the chunked tree reports the one stray close at `span_of(text, ")")`.
- `foo { bar`: the brace group's closing is `None` in the chunked tree; `errors()` reports the one unclosed brace.
