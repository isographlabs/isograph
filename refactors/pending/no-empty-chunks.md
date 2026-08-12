# no-empty-chunks: chunking returns its tree beside its errors

A prefactor to the parsing series, after cut-at-unmatched.md. A comma no item precedes (a leading or doubled comma) is an error in every grammar context, so chunking emits it and never builds the empty chunk it would have opened: no empty chunk reaches the parsing stage, and `Chunk::contents` becomes a `NonEmptyVec`, so the state is unrepresentable rather than avoided.

```rust
// from crates/isograph_parser/src/chunk.rs
pub fn chunk(tree: &WithSpan<MatchedBrackets>) -> (WithSpan<ChunkedLevel>, Vec<CommaWithoutItem>)
```

By example:

```
{ a,, b }    ->  { a, b }    [CommaWithoutItem at the second comma]
{, a }       ->  { a }       [CommaWithoutItem at the comma]
{,}          ->  {}          [CommaWithoutItem at the comma]
,,a          ->  a           [CommaWithoutItem at each comma, in order]
a, ,         ->  a,          [CommaWithoutItem at the second comma]
a,,\nb       ->  a, b        [CommaWithoutItem at the second comma]
{ a, }       ->  { a, }      no error: a trailing comma on a contentful chunk is a list
                             concern (no-final-comma.md), not chunking's
```

Sibling chunks are untouched: a comma typo poisons nothing, so unlike the matcher's cut nothing else is dropped. What is dropped is the offending boundary itself, the comma and any line breaks absorbed after it, as the `a,,\nb` row shows: the newline is gone with its comma, and positions on either answer the level that dropped them. Errors come out in source order by construction, since levels absorb in item order and group interiors recurse at their positions.

## Change 1: the `non_empty_vec` crate

A vec that cannot be empty, stored as first-plus-rest so `first` and `last` are total. Two fields have the invariant and adopt it here: `Chunk::contents` and `ChunkSeparator`'s tokens (a boundary is only ever built from its first absorbed separator).

`crates/non_empty_vec/Cargo.toml`:

```toml
[package]
name = "non_empty_vec"
version = { workspace = true }
edition = { workspace = true }
license = { workspace = true }

[dependencies]

[lints]
workspace = true
```

```rust
// from crates/non_empty_vec/src/lib.rs
use std::iter::once;
use std::ops::Index;

/// A vec with at least one element, by representation: the first element is its own
/// field, so no emptying operation can exist and `first`/`last` are total.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonEmptyVec<T> {
    first: T,
    rest: Vec<T>,
}

impl<T> NonEmptyVec<T> {
    pub fn of(first: T) -> Self {
        NonEmptyVec { first, rest: Vec::new() }
    }

    pub fn push(&mut self, item: T) {
        self.rest.push(item);
    }

    pub fn first(&self) -> &T {
        &self.first
    }

    pub fn last(&self) -> &T {
        self.rest.last().unwrap_or(&self.first)
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        1 + self.rest.len()
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter(once(&self.first).chain(self.rest.iter()))
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        match index {
            0 => Some(&self.first),
            index => self.rest.get(index - 1),
        }
    }
}

/// Indexes like a slice, panicking out of bounds like one; `get` is the checked form.
impl<T> Index<usize> for NonEmptyVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        match index {
            0 => &self.first,
            index => &self.rest[index - 1],
        }
    }
}

/// The iterator over a `NonEmptyVec`, named so callers can store it: the first-plus-rest
/// representation has no slice to iterate.
pub struct Iter<'a, T>(std::iter::Chain<std::iter::Once<&'a T>, std::slice::Iter<'a, T>>);

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        self.0.next()
    }
}
```

The `unwrap_or` in `last` is total, not a fallback: both arms are valid states.

## Change 2: the derive accepts `NonEmptyVec`

`resolve_position_macros`' field grammar (`parse_resolve_field_type`) treats `NonEmptyVec` exactly as `Vec`: the container branch's condition

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        if (last_segment.ident == "Vec" || last_segment.ident == "Option")
```

becomes

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        if (last_segment.ident == "Vec"
            || last_segment.ident == "Option"
            || last_segment.ident == "NonEmptyVec")
```

and the error message's type list gains it. The emission already iterates via `.iter()`, which `NonEmptyVec` provides. No behavior change at any existing site; the macro's test suite passes untouched.

## The error type

```rust
// from crates/isograph_parser/src/chunk.rs
/// A chunking error: a comma no item precedes, at the comma's span. The comma and its
/// boundary have no chunk; positions on them answer their level.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CommaWithoutItem(pub Span);
```

## The types

`Chunk` and `ChunkSeparator`, before:

```rust
// from crates/isograph_parser/src/chunk.rs
/// A maximal separator-free run of a level's items — tokens and groups — plus the
/// boundary that ended it when one did: line breaks and at most one comma, a second
/// comma ending the boundary as well. The chunk-parsing pass consumes it as one unit.
/// Every chunk but a level's last has a trailing separator by construction; the last's
/// is the optional trailing delimiter.
/// The wrapping `WithSpan`'s span runs from the first part's start to the last part's
/// end.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkedLevelPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct Chunk {
    #[resolve_field]
    pub contents: Vec<WithSpan<ChunkContentItem>>,
    #[resolve_field]
    pub trailing_separator: Option<WithSpan<ChunkSeparator>>,
}
```

```rust
// from crates/isograph_parser/src/chunk.rs
/// The boundary that ended its chunk: its line-break tokens and at most one comma, in
/// order. A second comma is never absorbed; it opens the next chunk's boundary. Its
/// tokens are not resolution leaves; a position on any of them answers the separator.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ChunkSeparator(pub Vec<WithSpan<SeparatorToken>>);
```

After (struct comments unchanged):

```rust
// from crates/isograph_parser/src/chunk.rs
/// A maximal separator-free run of a level's items — tokens and groups — plus the
/// boundary that ended it when one did: line breaks and at most one comma, a second
/// comma ending the boundary as well. The chunk-parsing pass consumes it as one unit.
/// Every chunk but a level's last has a trailing separator by construction; the last's
/// is the optional trailing delimiter.
/// The wrapping `WithSpan`'s span runs from the first part's start to the last part's
/// end.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkedLevelPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct Chunk {
    #[resolve_field]
    pub contents: NonEmptyVec<WithSpan<ChunkContentItem>>,
    #[resolve_field]
    pub trailing_separator: Option<WithSpan<ChunkSeparator>>,
}
```

```rust
// from crates/isograph_parser/src/chunk.rs
/// The boundary that ended its chunk: its line-break tokens and at most one comma, in
/// order. A second comma is never absorbed; it opens the next chunk's boundary. Its
/// tokens are not resolution leaves; a position on any of them answers the separator.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ChunkSeparator(pub NonEmptyVec<WithSpan<SeparatorToken>>);
```

## The pass

An absorption returns the two cases it already knows, so no empty chunk is ever constructed, no span is fished out of a separator vec after the fact, and the landed `.expect("absorb_chunk requires a nonempty stream")` is deleted: `absorb_chunk` becomes total over the stream, returning `None` at its end, and every span is a join from a committed anchor.

```rust
// from crates/isograph_parser/src/chunk.rs
/// One absorption: a chunk, or a comma no item precedes, whose boundary is consumed
/// and dropped.
enum Absorbed {
    Chunk(WithSpan<Chunk>),
    CommaWithoutItem(Span),
}
```

`chunk` and `chunk_level`, before:

```rust
// from crates/isograph_parser/src/chunk.rs
/// Chunk a matched-brackets tree. The pass is infallible. Every raw token lands in a
/// chunk, and no grammar is checked.
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
/// Chunk a matched-brackets tree. Every non-separator token lands in a chunk; a comma
/// no item precedes is the pass's one error, returned beside the tree; no grammar is
/// checked.
pub fn chunk(tree: &WithSpan<MatchedBrackets>) -> (WithSpan<ChunkedLevel>, Vec<CommaWithoutItem>) {
    let mut errors = Vec::new();
    let level = chunk_level(&tree.item, &mut errors);
    (WithSpan::new(level, tree.location), errors)
}

fn chunk_level(level: &MatchedBrackets, errors: &mut Vec<CommaWithoutItem>) -> ChunkedLevel {
    let mut items = level.0.iter().safe_peekable();
    let mut out = Vec::new();
    while let Some(absorbed) = absorb_chunk(&mut items, errors) {
        match absorbed {
            Absorbed::Chunk(chunk) => out.push(chunk),
            Absorbed::CommaWithoutItem(comma) => errors.push(CommaWithoutItem(comma)),
        }
    }
    ChunkedLevel(out)
}
```

`absorb_chunk`, before:

```rust
// from crates/isograph_parser/src/chunk.rs
/// One chunk from a nonempty stream: the content phase, then the boundary phase.
/// Whichever phase matches the first item consumes it — every item is either content
/// or a separator — so the chunk has at least one part and its span exists; the
/// boundary phase stops before a second comma, and absorbs at least the comma it
/// starts at when the chunk opens on one. Group interiors recurse in the content
/// phase's `Bracketed` arm.
fn absorb_chunk(items: &mut LevelItems<'_>) -> WithSpan<Chunk> {
    let mut contents = Vec::new();
    while let Some(peek) = items.peek() {
        let content_item = match &peek.view().item {
            BracketItem::Raw(token) => {
                if separator_token(token.0).is_some() {
                    // A separator ends the content phase.
                    break;
                }
                ChunkContentItem::NonBracket(*token)
            }
            BracketItem::Bracketed(group) => ChunkContentItem::Group(chunk_group(group)),
        };
        let item = peek.commit();
        contents.push(WithSpan::new(content_item, item.location));
    }

    let mut separators: Vec<WithSpan<SeparatorToken>> = Vec::new();
    while let Some(peek) = items.peek() {
        let Some(separator) = separator_of(peek.view()) else {
            break;
        };
        if separator == SeparatorToken::Comma
            && separators
                .iter()
                .any(|absorbed| absorbed.item == SeparatorToken::Comma)
        {
            // The second comma opens the next chunk's boundary.
            break;
        }
        let item = peek.commit();
        separators.push(WithSpan::new(separator, item.location));
    }

    let separator_location = separators.iter().map(|s| s.location).reduce(Span::join);
    let span = contents
        .iter()
        .map(|c| c.location)
        .chain(separator_location)
        .reduce(Span::join)
        .expect("absorb_chunk requires a nonempty stream");
    let trailing_separator =
        separator_location.map(|location| WithSpan::new(ChunkSeparator(separators), location));
    WithSpan::new(
        Chunk {
            contents,
            trailing_separator,
        },
        span,
    )
}
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
/// One absorption, or `None` at the stream's end. The first item is classified by
/// kind: a comma is the error, its following line breaks drained; a line break is
/// dropped and classification retries (the matcher already captured a level's leading
/// line breaks; this arm names the other separator so a leak is not reported as
/// `CommaWithoutItem`); otherwise the content phase runs from that item, then the
/// boundary phase, which stops before a second comma. Group interiors recurse in the
/// content phase's `Bracketed` arm.
fn absorb_chunk(
    items: &mut LevelItems<'_>,
    errors: &mut Vec<CommaWithoutItem>,
) -> Option<Absorbed> {
    let peek = loop {
        let peek = items.peek()?;
        match separator_of(peek.view()) {
            Some(SeparatorToken::Comma) => {
                let comma = peek.commit().location;
                drain_dropped_boundary(items);
                return Some(Absorbed::CommaWithoutItem(comma));
            }
            Some(SeparatorToken::LineBreak) => {
                peek.commit();
            }
            None => break peek,
        }
    };
    let first = match &peek.view().item {
        BracketItem::Raw(token) => ChunkContentItem::NonBracket(*token),
        BracketItem::Bracketed(group) => ChunkContentItem::Group(chunk_group(group, errors)),
    };
    let first_location = peek.commit().location;
    let mut span = first_location;
    let mut contents = NonEmptyVec::of(WithSpan::new(first, first_location));
    while let Some(peek) = items.peek() {
        let Some(content_item) = as_content(peek.view(), errors) else {
            break;
        };
        let item = peek.commit();
        span = Span::join(span, item.location);
        contents.push(WithSpan::new(content_item, item.location));
    }

    let mut separators: Option<NonEmptyVec<WithSpan<SeparatorToken>>> = None;
    while let Some(peek) = items.peek() {
        let Some(separator) = separator_of(peek.view()) else {
            break;
        };
        if separator == SeparatorToken::Comma
            && separators.as_ref().is_some_and(|absorbed| {
                absorbed.iter().any(|token| token.item == SeparatorToken::Comma)
            })
        {
            // The second comma opens the next absorption.
            break;
        }
        let item = peek.commit();
        let token = WithSpan::new(separator, item.location);
        match &mut separators {
            None => separators = Some(NonEmptyVec::of(token)),
            Some(absorbed) => absorbed.push(token),
        }
    }

    let trailing_separator = separators.map(|separators| {
        let location = Span::join(separators.first().location, separators.last().location);
        WithSpan::new(ChunkSeparator(separators), location)
    });
    let span = match &trailing_separator {
        Some(separator) => Span::join(span, separator.location),
        None => span,
    };
    Some(Absorbed::Chunk(WithSpan::new(
        Chunk {
            contents,
            trailing_separator,
        },
        span,
    )))
}

/// The content this item contributes, `None` when it is a separator. Group interiors
/// chunk here.
fn as_content(
    item: &WithSpan<BracketItem>,
    errors: &mut Vec<CommaWithoutItem>,
) -> Option<ChunkContentItem> {
    match &item.item {
        BracketItem::Raw(token) => match separator_token(token.0) {
            Some(_) => None,
            None => Some(ChunkContentItem::NonBracket(*token)),
        },
        BracketItem::Bracketed(group) => Some(ChunkContentItem::Group(chunk_group(group, errors))),
    }
}

/// The rest of a dropped boundary: the line breaks after its comma, dropped with it. A
/// further comma is not absorbed; it opens the next absorption and its own error.
fn drain_dropped_boundary(items: &mut LevelItems<'_>) {
    while let Some(peek) = items.peek() {
        if separator_of(peek.view()) != Some(SeparatorToken::LineBreak) {
            break;
        }
        peek.commit();
    }
}
```

`separator_of` is unchanged. `chunk_group` threads the vec; before:

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
/// order, and nothing else. Line breaks at a level's start were captured by the opening
/// bracket (or the literal's start) and never arrive here, so a chunk with no contents
/// is always a comma no item precedes, holding that comma as its boundary's first
/// token.
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
/// One level of the chunk tree: the whole literal at the root, a group's interior
/// below — the same role `MatchedBrackets` has on the bracket tree. Its chunks, in
/// order, and nothing else: line breaks at a level's start were captured by the opening
/// bracket (or the literal's start), and a comma no item precedes is a
/// `CommaWithoutItem` error beside the tree, its boundary dropped.
```

The `absorb_chunk` and `chunk` comment changes are in their listings above; `Chunk::contents`' nonemptiness is the field's type.

## Downstream

Nothing downstream exists to update: the grammar stage is unimplemented, and parsing-standards.md already assumes this world (the `chunks()` nonempty guarantee, the total `contents_span` that ships with parse-fields.md, the three-list final sweep).

## Tests

Chunking joins the passes whose errors are output; no fixture leaves the comma vec unasserted. The helpers in chunk.rs's test module (its `tree` already destructures the matcher's pair); indexing and `len` on `NonEmptyVec` keep the existing content assertions compiling as written:

```rust
// from crates/isograph_parser/src/chunk.rs (test module)
    fn chunked(literal: &str) -> WithSpan<ChunkedLevel> {
        let (brackets, bracket_errors) = tree(literal);
        assert_eq!(bracket_errors, vec![]);
        let (chunked, comma_errors) = chunk(&brackets);
        assert_eq!(comma_errors, vec![]);
        chunked
    }

    /// For fixtures whose bracket tree is clean but whose chunking errs.
    fn chunked_with_commas(literal: &str) -> (WithSpan<ChunkedLevel>, Vec<CommaWithoutItem>) {
        let (brackets, bracket_errors) = tree(literal);
        assert_eq!(bracket_errors, vec![]);
        chunk(&brackets)
    }
```

The four tests that consume matcher-error fixtures (`a ) b`, `foo { bar`, and the two dropped-region resolution tests) currently bind `let tree = chunk(&brackets)`. Replace that call (do not leave it and add a second) with:

```rust
// from crates/isograph_parser/src/chunk.rs (test module)
        let (tree, comma_errors) = chunk(&brackets);
        assert_eq!(comma_errors, vec![]);
```

The rest of each test keeps using `tree` for the chunked result.

The empty-chunk tests are replaced:

```rust
// from crates/isograph_parser/src/chunk.rs (test module)
    #[test]
    fn a_comma_before_the_first_item_is_an_error() {
        let text = "\n, a";
        let (chunked, errors) = chunked_with_commas(text);
        assert_eq!(errors, vec![CommaWithoutItem(span_of(text, ","))]);
        assert_eq!(chunked.item.0.len(), 1);
        assert_eq!(render_chunk(text, &chunked.item.0[0].item), "a");
    }

    #[test]
    fn a_second_comma_after_line_breaks_is_an_error() {
        let text = "a,\n\n,b";
        let (chunked, errors) = chunked_with_commas(text);
        let second_comma = span_of(text, ",b");
        assert_eq!(
            errors,
            vec![CommaWithoutItem(Span::new(second_comma.start, second_comma.start + 1))]
        );
        assert_eq!(chunked.item.0.len(), 2);
        assert_eq!(render_chunk(text, &chunked.item.0[0].item), "a,\n\n");
        assert_eq!(render_chunk(text, &chunked.item.0[1].item), "b");
    }

    #[test]
    fn a_dropped_boundarys_line_break_goes_with_its_comma() {
        let text = "a,,\nb";
        let (chunked, errors) = chunked_with_commas(text);
        let commas = span_of(text, ",,");
        assert_eq!(
            errors,
            vec![CommaWithoutItem(Span::new(commas.start + 1, commas.end))]
        );
        assert_eq!(chunked.item.0.len(), 2);
        assert_eq!(render_chunk(text, &chunked.item.0[0].item), "a,");
        assert_eq!(render_chunk(text, &chunked.item.0[1].item), "b");
        match chunked.resolve(ChunkedLevelParent::Root, span_of(text, "\n")) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Root));
            }
            node => panic!("expected the root level, got {node:?}"),
        }
    }

    #[test]
    fn doubled_leading_commas_are_two_errors_in_order() {
        let text = ",,a";
        let (chunked, errors) = chunked_with_commas(text);
        let commas = span_of(text, ",,");
        assert_eq!(
            errors,
            vec![
                CommaWithoutItem(Span::new(commas.start, commas.start + 1)),
                CommaWithoutItem(Span::new(commas.start + 1, commas.end)),
            ]
        );
        assert_eq!(chunked.item.0.len(), 1);
        assert_eq!(render_chunk(text, &chunked.item.0[0].item), "a");
    }

    #[test]
    fn a_trailing_doubled_comma_keeps_the_item() {
        let text = "a, ,";
        let (chunked, errors) = chunked_with_commas(text);
        let anchor = span_of(text, ", ,");
        assert_eq!(
            errors,
            vec![CommaWithoutItem(Span::new(anchor.end - 1, anchor.end))]
        );
        assert_eq!(chunked.item.0.len(), 1);
        assert_eq!(render_chunk(text, &chunked.item.0[0].item), "a,");
    }

    #[test]
    fn an_interior_comma_without_item_resolves_to_its_level() {
        let text = "foo {,}";
        let (chunked, errors) = chunked_with_commas(text);
        assert_eq!(errors, vec![CommaWithoutItem(span_of(text, ","))]);
        let top = &chunked.item.0[0].item;
        assert_eq!(top.contents.len(), 2);
        let brace = as_group(content_item(top, 1));
        assert_eq!(brace.children.item.0.len(), 0);
        match chunked.resolve(ChunkedLevelParent::Root, span_of(text, ",")) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Interior(_)));
            }
            node => panic!("expected the interior level, got {node:?}"),
        }
    }

    #[test]
    fn an_interior_comma_then_an_item_keeps_the_item() {
        let text = "{, a }";
        let (chunked, errors) = chunked_with_commas(text);
        assert_eq!(errors, vec![CommaWithoutItem(span_of(text, ","))]);
        assert_eq!(chunked.item.0.len(), 1);
        let brace = as_group(content_item(&chunked.item.0[0].item, 0));
        assert_eq!(brace.children.item.0.len(), 1);
        assert_eq!(render_chunk(text, &brace.children.item.0[0].item), "a");
    }

    #[test]
    fn a_list_trailing_comma_is_not_a_chunking_error() {
        let text = "foo { a, }";
        let chunked = chunked(text);
        let brace = as_group(content_item(&chunked.item.0[0].item, 1));
        assert_eq!(brace.children.item.0.len(), 1);
    }
```

Every other chunk test asserts through `chunked` and so pins both upstream vecs empty. `non_empty_vec` gets its own unit tests for `first`, `last`, `len`, `get`, `iter`, and `Index`, including the single-element and pushed cases.

## Landing checklist

1. The `non_empty_vec` crate and its tests; `cargo test -p non_empty_vec` passes.
2. The derive's `NonEmptyVec` support; `cargo test -p resolve_position_macros` (and the parser suite) pass untouched.
3. The signature and threading changes, the `Absorbed` restructure, the type and comment updates, and the test changes; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
4. Move this doc to refactors/past. Downstream of chunking, no empty chunk exists, and none can be built.
