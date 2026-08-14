# Parsing standards

Rules for all grammar-stage code. The feature docs define the grammar: each form (a selection, a value, a type) and the items that make it up. This doc lists the functions that implement those forms. If an implementation disagrees with this doc, the same review either amends the doc or changes the code.

Every call a parse function makes on a chunk is a method or free function listed here. A new call is a new listing here, not a local helper. Feature docs call this surface; they do not add parallel helpers.

## The unit of work

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(text: &str, root: WithSpan<ChunkedLevel>) -> WithSpan<IsoLiteralParse>
```

`parse_iso_literal` takes one chunked literal. It wraps `text` as `LiteralText` once and passes that value down. Each chunk is passed to `Chunk::stream`, which returns one `ChunkStream`. A group's interior is the `ChunkedLevel` in `group.children`. A `ChunkStream` or `ItemCursor` is built from one chunk.

A group is one item. `require_group` and `consume_group_if` return it in one call. The group is closed. The interior is parsed by calling `parse_items` or `parse_singleton` on `group.children`.

Each token and group already has a span. A parse function assigns a span to a value made of more than one item by calling `spanning`.

Chunking stored separators in boundaries. `take_next` returning `None` means the chunk has no remaining item. The matcher removed unmatched brackets and the tail after a cut. Chunking recorded a comma with no item as `CommaWithoutItem` and did not emit a chunk for it. A parse function's input does not include unmatched-bracket state or an empty chunk.

## `ItemCursor` and `ChunkStream`

`parse_items` and `parse_singleton` call `Chunk::stream` and receive a `ChunkStream`. They pass `&mut ItemCursor` into the parse function by calling `stream.cursor()`. `require_end` is a method on `ChunkStream`, not on `ItemCursor`. A parse function's parameter is `&mut ItemCursor`, so that function cannot call `require_end`. The methods on the value a function receives are the operations that function can call. This is the same split as `SafePeekable`.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
use nonempty::NonEmpty;
use safe_peekable::{IntoSafePeekable, SafePeekable};
use span::{Span, WithSpan};

use crate::{
    BracketKind, ChunkContentItem, ChunkedGroup, Expectation, Found, LiteralText, NonBracketTokenKind,
    ParseError, TokenText,
};

/// Parameter of a parse function. No rewind. No `peek` method. No `require_end`.
/// `commit` advances past the current item. The next call reads at most the
/// following item. `parse_items` and `parse_singleton` call `require_end` after
/// the parse function returns.
pub(crate) struct ItemCursor<'a> {
    items: SafePeekable<nonempty::Iter<'a, WithSpan<ChunkContentItem>>>,
    /// The end of the last item this cursor advanced past (the chunk's start before
    /// any). An `Expected(_, EndOfChunk)` error uses this offset.
    previous_end: u32,
    text: LiteralText<'a>,
}

/// Returned by `Chunk::stream`. `parse_items` and `parse_singleton` call its methods.
pub(crate) struct ChunkStream<'a> {
    cursor: ItemCursor<'a>,
}

impl<'a> ChunkStream<'a> {
    pub(crate) fn new(contents: &'a NonEmpty<WithSpan<ChunkContentItem>>, text: LiteralText<'a>) -> Self {
        ChunkStream {
            cursor: ItemCursor {
                previous_end: contents.first().location.start,
                items: contents.iter().safe_peekable(),
                text,
            },
        }
    }

    pub(crate) fn cursor(&mut self) -> &mut ItemCursor<'a> {
        &mut self.cursor
    }

    /// `Ok(())` when no item remains. If an item remains, `Err` on that item without `commit`.
    pub(crate) fn require_end(&mut self, expected: Expectation) -> Result<(), WithSpan<ParseError>> {
        match self.cursor.items.peek() {
            None => Ok(()),
            Some(peek) => {
                let item = *peek.view();
                Err(WithSpan::new(
                    ParseError::expected(expected, Found::from(&item.item)),
                    item.location,
                ))
            }
        }
    }
}

impl<'a> ItemCursor<'a> {
    /// The next item's span when it is a non-bracket token of `kind`; the error
    /// otherwise, on the found item, unconsumed, or empty at `previous_end` when the
    /// chunk ran out.
    pub(crate) fn require_token(
        &mut self,
        kind: NonBracketTokenKind,
        expected: Expectation,
    ) -> Result<Span, WithSpan<ParseError>> {
        let Some(peek) = self.items.peek() else {
            return Err(self.missing(expected));
        };
        let item = *peek.view();
        match &item.item {
            ChunkContentItem::NonBracket(token) if token.0 == kind => {
                peek.commit();
                self.previous_end = item.location.end;
                Ok(item.location)
            }
            other => Err(WithSpan::new(
                ParseError::expected(expected, Found::from(other)),
                item.location,
            )),
        }
    }

    /// The next item's span, consumed, when it is a non-bracket token of `kind`;
    /// `None`, nothing consumed, otherwise.
    pub(crate) fn consume_token_if(&mut self, kind: NonBracketTokenKind) -> Option<Span> {
        let peek = self.items.peek()?;
        let item = *peek.view();
        match &item.item {
            ChunkContentItem::NonBracket(token) if token.0 == kind => {
                peek.commit();
                self.previous_end = item.location.end;
                Some(item.location)
            }
            _ => None,
        }
    }

    /// The next item's span, and `commit`, when it is a non-bracket token whose kind
    /// is in `kinds`; `None` and no `commit` otherwise. One optional item, several
    /// kinds (a description: string or block string).
    pub(crate) fn consume_token_if_any(&mut self, kinds: &[NonBracketTokenKind]) -> Option<Span> {
        let peek = self.items.peek()?;
        let item = *peek.view();
        match &item.item {
            ChunkContentItem::NonBracket(token) if kinds.contains(&token.0) => {
                peek.commit();
                self.previous_end = item.location.end;
                Some(item.location)
            }
            _ => None,
        }
    }

    /// The next item's span when it is the identifier whose text is `keyword`; the
    /// error otherwise, on the found item, unconsumed, or empty at `previous_end`.
    pub(crate) fn require_keyword(
        &mut self,
        keyword: &'static str,
        expected: Expectation,
    ) -> Result<Span, WithSpan<ParseError>> {
        let Some(peek) = self.items.peek() else {
            return Err(self.missing(expected));
        };
        let item = *peek.view();
        match &item.item {
            ChunkContentItem::NonBracket(token)
                if token.0 == NonBracketTokenKind::Identifier
                    && self.text.at(item.location) == keyword =>
            {
                peek.commit();
                self.previous_end = item.location.end;
                Ok(item.location)
            }
            other => Err(WithSpan::new(
                ParseError::expected(expected, Found::from(other)),
                item.location,
            )),
        }
    }

    /// The next item, consumed, when it is a group opened by `kind`; `None`, nothing
    /// consumed, otherwise.
    pub(crate) fn consume_group_if(
        &mut self,
        kind: BracketKind,
    ) -> Option<WithSpan<&'a ChunkedGroup>> {
        let peek = self.items.peek()?;
        let matches = matches!(
            &peek.view().item,
            ChunkContentItem::Group(group) if group.opening.item.0 == kind
        );
        if !matches {
            return None;
        }
        let item = peek.commit();
        match &item.item {
            ChunkContentItem::Group(group) => {
                self.previous_end = item.location.end;
                Some(WithSpan::new(group, item.location))
            }
            ChunkContentItem::NonBracket(_) => None,
        }
    }

    /// The next item when it is a group opened by `kind`; the error otherwise, on the
    /// found item, unconsumed, or empty at `previous_end`.
    pub(crate) fn require_group(
        &mut self,
        kind: BracketKind,
        expected: Expectation,
    ) -> Result<WithSpan<&'a ChunkedGroup>, WithSpan<ParseError>> {
        let Some(peek) = self.items.peek() else {
            return Err(self.missing(expected));
        };
        let matches = matches!(
            &peek.view().item,
            ChunkContentItem::Group(group) if group.opening.item.0 == kind
        );
        if !matches {
            let item = *peek.view();
            return Err(WithSpan::new(
                ParseError::expected(expected, Found::from(&item.item)),
                item.location,
            ));
        }
        let item = peek.commit();
        match &item.item {
            ChunkContentItem::Group(group) => {
                self.previous_end = item.location.end;
                Ok(WithSpan::new(group, item.location))
            }
            ChunkContentItem::NonBracket(token) => Err(WithSpan::new(
                ParseError::expected(expected, Found::Token(token.0)),
                item.location,
            )),
        }
    }

    pub(crate) fn text(&self) -> LiteralText<'a> {
        self.text
    }

    /// The next item after `next`, or `None` at the chunk's end. Total; the caller
    /// builds any error.
    pub(crate) fn take_next(&mut self) -> Option<&'a WithSpan<ChunkContentItem>> {
        let item = self.items.next()?;
        self.previous_end = item.location.end;
        Some(item)
    }

    /// The empty span at `previous_end`, for error arms that found `EndOfChunk`.
    pub(crate) fn end_span(&self) -> Span {
        Span::new(self.previous_end, self.previous_end)
    }

    pub(crate) fn token_text(&self, span: Span) -> TokenText<'a> {
        self.text.at(span)
    }

    pub(crate) fn integer(&self, span: Span) -> Result<i64, WithSpan<ParseError>> {
        self.text.at(span).integer(span)
    }

    /// Calls `parse` and wraps `Ok` in a `WithSpan`. The span starts at the first
    /// item `parse` advanced past and ends at `previous_end`. If `parse` does not
    /// advance, the span is empty at `previous_end`. On `Err`, returns that error.
    pub(crate) fn spanning<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, WithSpan<ParseError>>,
    ) -> Result<WithSpan<T>, WithSpan<ParseError>> {
        let start = match self.items.peek() {
            Some(peek) => peek.view().location.start,
            None => self.previous_end,
        };
        let before = self.previous_end;
        let value = parse(self)?;
        let span = if self.previous_end == before {
            Span::new(before, before)
        } else {
            Span::new(start, self.previous_end)
        };
        Ok(WithSpan::new(value, span))
    }

    fn missing(&self, expected: Expectation) -> WithSpan<ParseError> {
        WithSpan::new(
            ParseError::expected(expected, Found::EndOfChunk),
            self.end_span(),
        )
    }
}
```

`take_next`'s iterator item is `&'a WithSpan<ChunkContentItem>` (`nonempty::Iter` yields references). `require_token` / `consume_token_if` copy that reference out of `view` (`I::Item` is `Copy`) and then `commit`. `consume_group_if` / `require_group` rematch after `commit` so the `&'a ChunkedGroup` is borrowed from the committed reference, not from the peek guard. The `NonBracket` arm after a successful `Group` view is the item-cannot-change-between-view-and-commit invariant; it is a typed error or `None`, not a panic.

`ChunkStream::require_end` calls `peek` on the cursor's iterator. `require_end` is not a method on `ItemCursor`. A parse function's parameter is `&mut ItemCursor`, so it has no `require_end` to call.

There is no `consume_keyword_if` and no `spanning` sibling that returns `T` or `Option<T>`. Neither has a caller. A method with no caller exists only in this doc, and these do not.

### Span sources

A leaf's span is the `Span` returned by `require_token`, `consume_token_if`, `consume_token_if_any`, or `require_keyword`, or the `WithSpan` on the value returned by `consume_group_if`, `require_group`, or `take_next`. The span of a parsed list item is the span `spanning` returned. The span of `LevelSlot::Unparsed` is `contents_span`. The span of a value made of several items is the span `spanning` returned. A parse function does not call `Span::join`. If no listed method returns the needed span, add a method here.

Construction is `WithSpan::new` and plain `Ok` / `Some`.

A value made of several items is parsed by one `spanning` call. The closure passed to `spanning` calls `take_next` or the first `require_*`. If the caller already advanced past the first item, a later `spanning` does not include that item: the caller already has that item's span, and the remaining items are read with `require_*` / `consume_*`. If those items and the first item must share one span, they are all read inside the `spanning` that advanced past the first item. There is no `spanning_from` method.

## `LiteralText`

`parse_iso_literal` takes `text: &str` and passes `LiteralText::new(text)` into the rest of the stage. Later functions take `LiteralText` or call `ItemCursor::text`. They do not take `&str`. The text is compared to a keyword via `PartialEq` or converted with `TokenText::integer`.

```rust
// from crates/isograph_parser/src/literal_text.rs
use span::{Span, WithSpan};

use crate::ParseError;

#[derive(Copy, Clone)]
pub(crate) struct LiteralText<'a>(&'a str);

#[derive(Copy, Clone)]
pub(crate) struct TokenText<'a>(&'a str);

impl<'a> LiteralText<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        LiteralText(text)
    }

    pub(crate) fn at(self, span: Span) -> TokenText<'a> {
        match self.0.get(span.as_usize_range()) {
            Some(text) => TokenText(text),
            None => TokenText(""),
        }
    }
}

impl TokenText<'_> {
    pub(crate) fn integer(self, span: Span) -> Result<i64, WithSpan<ParseError>> {
        match self.0.parse() {
            Ok(value) => Ok(value),
            Err(_) => Err(WithSpan::new(ParseError::IntegerOutOfRange, span)),
        }
    }
}

impl PartialEq<str> for TokenText<'_> {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for TokenText<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}
```

`at` on a span that is not a slice of this literal yields the empty `TokenText`. The invariant (every span passed here came from a token of this literal) is compiler-invisible; the fallback is the empty read, not a panic.

`TokenText` has no method that yields `&str` or `String`. A name in the output is a span. The one parsed scalar is the `i64`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(text: &str, root: WithSpan<ChunkedLevel>) -> WithSpan<IsoLiteralParse> {
    let location = root.location;
    let text = LiteralText::new(text);
    match try_parse(text, &root) {
        Ok(parse) => WithSpan::new(parse, location),
        Err(reason) => WithSpan::new(
            IsoLiteralParse::Unparsed(UnparsedLiteral { reason, level: root }),
            location,
        ),
    }
}
```

## `Chunk`

```rust
// from crates/isograph_parser/src/chunk.rs
impl Chunk {
    /// A `ChunkStream` over this chunk.
    pub(crate) fn stream<'a>(&'a self, text: LiteralText<'a>) -> ChunkStream<'a> {
        ChunkStream::new(&self.contents, text)
    }

    /// The span of the contents, without the boundary: a degraded slot's span. Total,
    /// because every chunk has contents.
    pub fn contents_span(&self) -> Span {
        Span::join(
            self.contents.first().location,
            self.contents.last().location,
        )
    }

    /// The first content item. Total: every chunk has contents. `parse_singleton`'s
    /// `extra` callback passes this to `Found::from`.
    pub(crate) fn first_item(&self) -> &WithSpan<ChunkContentItem> {
        self.contents.first()
    }

    /// The comma in the trailing boundary, when one exists. Only `parse_singleton`
    /// calls this.
    pub fn boundary_comma(&self) -> Option<Span> {
        let separator = self.trailing_separator.as_ref()?;
        separator
            .0
            .iter()
            .find(|token| token.item == SeparatorToken::Comma)
            .map(|token| token.location)
    }
}
```

`Chunk`'s fields are private to the `chunk` module. `contents` is a `NonEmpty<WithSpan<ChunkContentItem>>` (the `nonempty` crate; chunk-contents-nonempty.md). These methods are the only item access and the only boundary read. `Chunk` is `pub` and re-exported at the crate root, so `stream` is `pub(crate)`: `ChunkStream` never crosses the crate boundary.

`contents_span` is allowed `Span::join`: it is a `Chunk` method, not a parser.

## Lists and one-item levels

`ChunkedLevel`'s vec is private to the `chunk` module. The only functions that iterate it are `parse_items` and `parse_singleton`. A parse function does not iterate a `ChunkedLevel`. Tests that need the slice call `#[cfg(test)] ChunkedLevel::chunks`.

`parse_items` is called on a list level (a selection set, an argument list, an object literal, a variable-declaration list). `parse_singleton` is called on a one-item level (the root, a `[...]` interior).

```rust
// from crates/isograph_parser/src/chunk.rs
use resolve_position::ResolvePosition;
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan};

use crate::{
    Chunk, ChunkStream, Expectation, Found, ItemCursor, LiteralText, NonBracketTokenKind, ParseError,
};

/// One chunk's outcome in a list. The wrapping `WithSpan`'s span is the parsed
/// item's span, or the chunk's `contents_span` when unparsed.
#[derive(Debug, PartialEq, Eq)]
pub enum LevelSlot<T> {
    Parsed(ParsedSlot<T>),
    Unparsed(UnparsedItem),
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParsedSlot<T> {
    pub item: T,
    /// Present when the parse function returned `Ok` and leftover items followed it. Unmarked
    /// for resolution: the slot span excludes the leftover, so those positions answer
    /// the containing level.
    pub trailing: Option<WithSpan<ParseError>>,
}

/// A chunk that failed to parse as its level's item: the reason, and the chunk itself
/// for positions to resolve against.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = UnparsedItemParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct UnparsedItem {
    pub reason: WithSpan<ParseError>,
    #[resolve_field(parent_variant = UnparsedItem)]
    pub chunk: WithSpan<Chunk>,
}

impl ChunkedLevel {
    /// One `LevelSlot` per chunk, same length as the level. Each chunk is a new
    /// `ChunkStream`. The parse function receives `&mut ItemCursor`. This function
    /// calls `require_end`.
    pub(crate) fn parse_items<'a, P>(
        &'a self,
        text: LiteralText<'a>,
        parse_item: impl Fn(&mut ItemCursor<'a>) -> Result<P, WithSpan<ParseError>>,
    ) -> Vec<WithSpan<LevelSlot<P>>> {
        self.0
            .iter()
            .map(|chunk| {
                let mut stream = chunk.item.stream(text);
                match stream.cursor().spanning(&parse_item) {
                    Ok(item) => {
                        let trailing = match stream.require_end(Expectation::Separator) {
                            Ok(()) => None,
                            Err(error) => Some(error),
                        };
                        WithSpan::new(
                            LevelSlot::Parsed(ParsedSlot {
                                item: item.item,
                                trailing,
                            }),
                            item.location,
                        )
                    }
                    Err(reason) => WithSpan::new(
                        LevelSlot::Unparsed(UnparsedItem {
                            reason,
                            chunk: chunk.clone(),
                        }),
                        chunk.item.contents_span(),
                    ),
                }
            })
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn chunks(&self) -> &[WithSpan<Chunk>] {
        &self.0
    }
}

/// Calls `parse` on the first chunk with `&mut ItemCursor`, then `require_end`,
/// then `Err` if `boundary_comma` is `Some`, then `Err` if a second chunk exists.
pub(crate) fn parse_singleton<'a, T>(
    level: &'a WithSpan<ChunkedLevel>,
    text: LiteralText<'a>,
    empty: impl FnOnce() -> WithSpan<ParseError>,
    extra: impl FnOnce(&'a WithSpan<Chunk>) -> WithSpan<ParseError>,
    parse: impl FnOnce(&mut ItemCursor<'a>) -> Result<T, WithSpan<ParseError>>,
    end_expectation: Expectation,
) -> Result<T, WithSpan<ParseError>> {
    let mut chunks = level.item.0.iter();
    let Some(chunk) = chunks.next() else {
        return Err(empty());
    };
    let mut stream = chunk.item.stream(text);
    let item = parse(stream.cursor())?;
    stream.require_end(end_expectation)?;
    if let Some(comma) = chunk.item.boundary_comma() {
        return Err(WithSpan::new(
            ParseError::expected(end_expectation, Found::Token(NonBracketTokenKind::Comma)),
            comma,
        ));
    }
    if let Some(more) = chunks.next() {
        return Err(extra(more));
    }
    Ok(item)
}
```

`parse_items` returns `Vec<WithSpan<LevelSlot<P>>>`, not `Result`. If the parse function returns `Err`, the slot is `LevelSlot::Unparsed`. That `Err` is not returned from `parse_items`, so `?` in the parse function does not skip later chunks. The vec length equals the chunk count.

If the parse function returns `Ok` and `require_end` returns `Err`, the item is kept and the error is stored in `ParsedSlot::trailing`. `foo bar` is the selection `foo` and a trailing error at `bar`. Find-references, rename, and go-to-definition use the `foo` name leaf. The item's span does not cover `bar`, so a position on `bar` resolves to the selection set.

The leftover is the suffix of the chunk after the last item the parse function advanced past. `foo bar { baz }` is the scalar `foo` and leftover starting at `bar`. `baz` is not a node in the tree.

In `parse_singleton`, if `require_end` returns `Err`, that error is returned and the declaration or `[...]` type is not produced. The comma check uses the same `end_expectation` (`EndOfDeclaration`, `EndOfType`).

### `LevelSlot` and `ResolvePosition`

`LevelSlot<T>` is a `ResolvePosition` wrapper, not a leaf. A position in a `Parsed` slot is resolved by `T::resolve`. A position in an `Unparsed` slot is resolved by `UnparsedItem::resolve`. `trailing` is not a `#[resolve_field]`, so resolution does not descend into it.

This is a blanket `ResolvePosition` impl, like the `Box<T>` impl in parse-variables.md. It is in `chunk.rs` next to `LevelSlot` because it names `UnparsedItem` and `UnparsedItemParent`.

```rust
// from crates/isograph_parser/src/chunk.rs
impl<T> ResolvePosition for LevelSlot<T>
where
    T: ResolvePosition,
    for<'a> UnparsedItemParent<'a>: From<T::Parent<'a>>,
{
    type Parent<'a>
        = T::Parent<'a>
    where
        Self: 'a;
    type ResolvedNode<'a>
        = T::ResolvedNode<'a>
    where
        Self: 'a;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: Span) -> Self::ResolvedNode<'a> {
        match self {
            LevelSlot::Parsed(parsed) => parsed.item.resolve(parent, position),
            LevelSlot::Unparsed(unparsed) => unparsed.resolve(UnparsedItemParent::from(parent), position),
        }
    }
}
```

Each list path converts into `UnparsedItemParent`:

```rust
// from crates/isograph_parser/src/selections.rs
impl<'a> From<SelectionSetPath<'a>> for UnparsedItemParent<'a> {
    fn from(path: SelectionSetPath<'a>) -> Self {
        UnparsedItemParent::SelectionSet(path)
    }
}
```

The same `From` exists for `ArgumentListPath`, `ObjectLiteralPath`, and `VariableDeclarationListPath`, each landing with that list.

A list in the tree is `Vec<WithSpan<LevelSlot<P>>>`. `P` has no `Unparsed` variant.

```rust
// from crates/isograph_parser/src/selections.rs
pub struct SelectionSet(#[resolve_field] pub Vec<WithSpan<LevelSlot<Selection>>>);

pub enum Selection {
    Scalar(ScalarSelection),
    Object(ObjectSelection),
}
```

The derive on `SelectionSet` generates a loop over `Vec<WithSpan<LevelSlot<Selection>>>` and calls `LevelSlot::resolve` on each element. That is the same generated shape as `Vec<WithSpan<Selection>>`, with `LevelSlot<Selection>` as the inner type.

### Errors from slots

```rust
// from crates/isograph_parser/src/chunk.rs
pub(crate) fn collect_slot_errors<T>(
    slots: &[WithSpan<LevelSlot<T>>],
    nested: impl Fn(&T, &mut Vec<WithSpan<ParseError>>),
    errors: &mut Vec<WithSpan<ParseError>>,
) {
    for slot in slots {
        match &slot.item {
            LevelSlot::Parsed(parsed) => {
                nested(&parsed.item, errors);
                if let Some(trailing) = parsed.trailing {
                    errors.push(trailing);
                }
            }
            LevelSlot::Unparsed(unparsed) => errors.push(unparsed.reason),
        }
    }
}
```

Nested errors (arguments, nested selections) precede that slot's trailing error, matching source order.

## Function shapes

- `require_*`: methods on `ItemCursor` for a required item. On match they call `commit` and return that item. On mismatch they return `Err` on the next item without `commit`, or `Err` at `end_span` if there is no next item.
- `consume_*`: methods on `ItemCursor` for one optional item. On match they call `commit` and return `Some`. Otherwise they return `None` and do not call `commit`. They do not return `Err`.
- `parse_*`: a function that implements a grammar form made of several items. The parameter is `&mut ItemCursor`. The first `Err` is returned. Shared iteration is `parse_items`, `parse_singleton`, or `spanning`. A second loop over a level is not written.

A parse function that reads a group and then calls `parse_items` or `parse_singleton` on the interior is named `parse_*` or `consume_*` and is built from `require_group` / `consume_group_if` plus that call:

```rust
// from crates/isograph_parser/src/selections.rs
pub(crate) fn consume_selection_set(
    cursor: &mut ItemCursor<'_>,
) -> Option<WithSpan<SelectionSet>> {
    let group = cursor.consume_group_if(BracketKind::Brace)?;
    Some(WithSpan::new(
        SelectionSet(group.item.children.item.parse_items(cursor.text(), parse_selection)),
        group.location,
    ))
}

pub(crate) fn require_selection_set(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<SelectionSet>, WithSpan<ParseError>> {
    let group = cursor.require_group(BracketKind::Brace, Expectation::SelectionSet)?;
    Ok(WithSpan::new(
        SelectionSet(group.item.children.item.parse_items(cursor.text(), parse_selection)),
        group.location,
    ))
}
```

`ItemCursor::text` returns the `LiteralText` passed to `Chunk::stream`. The caller of `parse_items` on a group's interior passes `cursor.text()`.

## Dispatch

When the next item may start several forms, the parse function calls `take_next()` and `match`es on the result. If those arms together are one value, the `match` is inside `spanning`. `take_next` advances past the item. Arms that return `Ok` continue with `require_*` / `consume_*` on the same cursor. The `_` arm returns `Err` with that item as `found`.

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn parse_value(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<NonConstantValue>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| match cursor.take_next() {
        Some(item) => match &item.item {
            ChunkContentItem::NonBracket(token) => match token.0 {
                NonBracketTokenKind::Dollar => {
                    let name = cursor.require_token(
                        NonBracketTokenKind::Identifier,
                        Expectation::Token(NonBracketTokenKind::Identifier),
                    )?;
                    Ok(NonConstantValue::Variable(VariableUse {
                        dollar: WithSpan::new(Dollar, item.location),
                        name: WithSpan::new(VariableName, name),
                    }))
                }
                NonBracketTokenKind::StringLiteral => Ok(NonConstantValue::String(StringValue)),
                NonBracketTokenKind::IntegerLiteral => {
                    let value = cursor.integer(item.location)?;
                    Ok(NonConstantValue::Integer(IntegerValue(value)))
                }
                NonBracketTokenKind::Identifier => match cursor.token_text(item.location) {
                    text if text == "true" => {
                        Ok(NonConstantValue::Boolean(BooleanValue(Boolean::True)))
                    }
                    text if text == "false" => {
                        Ok(NonConstantValue::Boolean(BooleanValue(Boolean::False)))
                    }
                    text if text == "null" => Ok(NonConstantValue::Null(NullValue)),
                    _ => Err(WithSpan::new(
                        ParseError::expected(
                            Expectation::Value,
                            Found::Token(NonBracketTokenKind::Identifier),
                        ),
                        item.location,
                    )),
                },
                kind => Err(WithSpan::new(
                    ParseError::expected(Expectation::Value, Found::Token(kind)),
                    item.location,
                )),
            },
            ChunkContentItem::Group(group) if group.opening.item.0 == BracketKind::Brace => {
                Ok(NonConstantValue::Object(ObjectLiteral(
                    group.children.item.parse_items(cursor.text(), parse_object_entry),
                )))
            }
            other => Err(WithSpan::new(
                ParseError::expected(Expectation::Value, Found::from(other)),
                item.location,
            )),
        },
        None => Err(WithSpan::new(
            ParseError::expected(Expectation::Value, Found::EndOfChunk),
            cursor.end_span(),
        )),
    })
}
```

Content dispatch is the same shape one level down: `require_token(Identifier, ...)`, then a `match` on `token_text` (`"entrypoint"` / `"field"` / `"pointer"`; `"true"` / `"false"` / `"null"`). The `_` arm is the one reject. A single required keyword (`to`) is `require_keyword`, not this match.

`consume_*` is not used to choose among alternatives of one form. It is used for one optional item. That includes the case where the next item may start the caller's next form: the optional `!` after a type name, after which the next item may be the caller's `=`. The type parse function does not `match` on `take_next` there, because `=` is not part of the type. Two `consume_*` calls in sequence that together implement the alternatives of one form are not written. Several token kinds for one optional item is `consume_token_if_any`.

A form that is absent until its first item appears, then required (`$name`, later `@ name (args)`), is a `take_next` arm. After that item, the rest is `require_*`.

Whether a form is present is determined by the next item. One optional item is `consume_*` and does not return `Err`. A form of several items that starts optionally advances past the first item in the `take_next` match and may return `Err` on the second (`@@` returns `Err` at the second `@`). After `require_token` on an identifier, `consume_token_if(Colon)` tells whether that identifier is an alias; both arms of that `match` use the identifier. There is no method that advances past an item and then restores it. A grammar form that requires reading two items before the function can return `Some` or `None` cannot be implemented with this surface.

```rust
// from crates/isograph_parser/src/selections.rs
fn parse_selection(cursor: &mut ItemCursor<'_>) -> Result<Selection, WithSpan<ParseError>> {
    let first = cursor.require_token(NonBracketTokenKind::Identifier, Expectation::Selection)?;
    let (reader_alias, name) = match cursor.consume_token_if(NonBracketTokenKind::Colon) {
        Some(_) => {
            let name = cursor.require_token(
                NonBracketTokenKind::Identifier,
                Expectation::Token(NonBracketTokenKind::Identifier),
            )?;
            (
                Some(WithSpan::new(SelectionAlias, first)),
                WithSpan::new(SelectionName, name),
            )
        }
        None => (None, WithSpan::new(SelectionName, first)),
    };
    let arguments = consume_argument_list(cursor);
    let selection_set = consume_selection_set(cursor);
    Ok(match selection_set {
        Some(selection_set) => Selection::Object(ObjectSelection {
            reader_alias,
            name,
            arguments,
            selection_set,
        }),
        None => Selection::Scalar(ScalarSelection {
            reader_alias,
            name,
            arguments,
        }),
    })
}
```

`parse_selection` does not call `require_end`. `parse_level_items` wraps it in `spanning` and then checks leftover.

## Narrower types for narrower grammars

A grammar form that does not include a sub-form uses a type that does not have that variant. Variable defaults are `ConstantValue`. `parse_constant_value` returns `Err` at `$`. `DeclaredVariable::default_value` has type `Option<WithSpan<ConstantValue>>`.

```rust
// from crates/isograph_parser/src/arguments.rs
pub enum NonConstantValue {
    Variable(VariableUse),
    String(StringValue),
    Integer(IntegerValue),
    Boolean(BooleanValue),
    Null(NullValue),
    Object(ObjectLiteral),
}

pub enum ConstantValue {
    String(StringValue),
    Integer(IntegerValue),
    Boolean(BooleanValue),
    Null(NullValue),
    Object(ConstantObjectLiteral),
}

pub struct BooleanValue(pub Boolean);

pub enum Boolean {
    True,
    False,
}

pub struct ObjectLiteral(#[resolve_field] pub Vec<WithSpan<LevelSlot<ObjectEntry>>>);

pub enum ObjectEntry {
    Named(NamedObjectEntry),
}

pub struct NamedObjectEntry {
    #[resolve_field]
    pub name: WithSpan<ObjectEntryName>,
    #[resolve_field(parent_variant = ObjectEntry)]
    pub value: WithSpan<NonConstantValue>,
}

pub struct ConstantObjectLiteral(#[resolve_field] pub Vec<WithSpan<LevelSlot<ConstantObjectEntry>>>);

pub enum ConstantObjectEntry {
    Named(NamedConstantObjectEntry),
}

pub struct NamedConstantObjectEntry {
    #[resolve_field]
    pub name: WithSpan<ObjectEntryName>,
    #[resolve_field(parent_variant = ConstantObjectEntry)]
    pub value: WithSpan<ConstantValue>,
}
```

`parse_value` and `parse_constant_value` share the scalar arms (string, integer, boolean, null) through a function that returns `ConstantValue`. `parse_value` wraps those as `NonConstantValue` and adds the `$` and non-constant object arms. `parse_constant_value` matches `$` and returns `Err` with `Expectation::ConstantValue`. It parses object entries by calling `parse_constant_object_entry`.

```rust
// from crates/isograph_parser/src/arguments.rs
fn parse_constant_scalar(
    cursor: &mut ItemCursor<'_>,
    item: &WithSpan<ChunkContentItem>,
) -> Option<Result<ConstantValue, WithSpan<ParseError>>> {
    match &item.item {
        ChunkContentItem::NonBracket(token) => match token.0 {
            NonBracketTokenKind::StringLiteral => Some(Ok(ConstantValue::String(StringValue))),
            NonBracketTokenKind::IntegerLiteral => {
                Some(cursor.integer(item.location).map(IntegerValue).map(ConstantValue::Integer))
            }
            NonBracketTokenKind::Identifier => match cursor.token_text(item.location) {
                text if text == "true" => Some(Ok(ConstantValue::Boolean(BooleanValue(Boolean::True)))),
                text if text == "false" => Some(Ok(ConstantValue::Boolean(BooleanValue(Boolean::False)))),
                text if text == "null" => Some(Ok(ConstantValue::Null(NullValue))),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}
```

## Errors

```rust
// from crates/isograph_parser/src/parse_error.rs
pub enum ParseError {
    Expected(ExpectedFound),
    EmptyLiteral,
    MultipleDeclarations,
    IntegerOutOfRange,
}

pub struct ExpectedFound {
    pub expected: Expectation,
    pub found: Found,
}

pub enum Expectation {
    Token(NonBracketTokenKind),
    DeclarationKeyword,
    EndOfDeclaration,
    SelectionSet,
    Selection,
    Separator,
    Argument,
    Value,
    ObjectEntry,
    VariableDeclaration,
    TypeAnnotation,
    ConstantValue,
    EndOfType,
    ToKeyword,
}

pub enum Found {
    Token(NonBracketTokenKind),
    Group(BracketKind),
    EndOfChunk,
}
```

One global `Expectation`. An error is `WithSpan<ParseError>`; the span covers the offending item or is empty where the missing item belonged.

`UnsupportedDeclarationType` exists only between parse-entrypoint.md and parse-pointers.md.

`ParseError` values are stored on the tree (`UnparsedLiteral`, `UnparsedItem`, `ParsedSlot::trailing`). `errors()` collects them in source order. There is no separate error vec next to the tree.

Bracket errors are the matcher's return value. Comma-without-item errors are chunking's return value. `ParseError` has no variant for either. An error-free literal has an empty matcher vec, an empty chunking vec, and an empty `errors()`.

A failed list chunk is `LevelSlot::Unparsed`; the other chunks of that level are parsed. A failed declaration is `UnparsedLiteral`. Each of those regions has one reason. `ParsedSlot::trailing` is an error on a parsed item, not one of those regions.

Parse functions do not construct message strings. `Display` impls format `ParseError`. Suggestions are produced by the rendering stage from the `(expected, found)` pair.

## Totality

No panics on any input. Parse functions do not call `unwrap`, `expect`, `unreachable!`, or use `Infallible`. If an invariant is not in the types, the function returns a defined value and a comment on that function states the invariant.

Every call to `parse_iso_literal` returns a tree. Every position resolves: a position in a parsed region resolves to a grammar leaf; a position in `UnparsedLiteral` or `UnparsedItem` resolves through the retained chunk; a position on whitespace, leftover, a region the matcher dropped, or a comma chunking dropped resolves to the nearest containing node.

`resolve` returns the leaf at the position. Find-references, rename, and go-to-definition run only when that leaf is a name leaf. They do not look at ancestors for a name. A position that resolves to a container, including leftover, does not use a parent's name: in `foo { bar baz }`, a position on `baz` resolves to the selection set, and find-references returns no references. Hover and completion read the resolution path.

## Trees and spans

Whether a type carries a span is fixed on the type: a tree enum is wrapped in `WithSpan` at its slot, variant payloads are bare, each struct field that is a node is `WithSpan`, and the comment on the type states what the wrapper covers.

A name is a fieldless marker struct in a `WithSpan`. Each role is its own type (an alias is not a name; an argument name is not an object key). The name's text is the wrapper's span. The tree stores no `String` and does not intern. The only converted scalar is the `i64`. The tree has no nodes for the dot, `$`, `!`, or `to`. A position on those tokens resolves to the containing node.

`ResolvePosition` is derive-only, except the two blanket delegations: `Box<T>` in `resolve_position` (parse-variables.md) and `LevelSlot<T>` next to `LevelSlot`. A third manual impl is a missing `resolve_position` feature and becomes a prefactor there. A parent is a direct path alias at one parent, an enum at the second. Chunk-stage `IsographResolutionNode` variants are reachable inside degraded regions only.

## Performance

One pass, by reference; the output copies spans and `Copy` tokens. Cloning happens only when a region degrades: allocation beyond the output vecs is proportional to the error count.

No backtracking exists (no rewind), so parse time is linear in the token count.

No pico, no interning: plain functions over `LiteralText` and the chunk tree.

## Catalog of parsing tasks

Every grammar-stage task is one row. A task that is not here is a missing method or a missing function.

- Required token: `ItemCursor::require_token`
- Optional token: `ItemCursor::consume_token_if`
- Optional token, several kinds: `ItemCursor::consume_token_if_any`
- Required keyword: `ItemCursor::require_keyword`
- Required group: `ItemCursor::require_group`
- Optional group: `ItemCursor::consume_group_if`
- Multi-form position: `take_next` inside `spanning`
- Keyword / boolean / null text: `token_text` after an identifier was accepted
- Integer conversion: `ItemCursor::integer`
- Composite span: `ItemCursor::spanning`
- Missing-item error span: `ItemCursor::end_span`
- List of items: `ChunkedLevel::parse_items` → `Vec<WithSpan<LevelSlot<P>>>`
- One-item context: `parse_singleton`
- First item of a rejected extra chunk: `Chunk::first_item`
- Trailing comma in a one-item context: `parse_singleton` (via `boundary_comma`)
- Leftover after a list item: `ParsedSlot::trailing`
- Leftover after a singleton: `parse_singleton`'s `require_end`
- Group interior: `require_group` / `consume_group_if`, then `parse_items` or `parse_singleton` on `group.children`
- Constant-only value: `parse_constant_value` → `ConstantValue`
- Leftover after a parse function returns: `parse_items` or `parse_singleton` call `require_end`. The parse function does not.

## Shipping and amending

Each structure and each method lands with the feature doc of its first caller. A method with no caller yet exists only in this doc.

- parse-entrypoint.md: `LiteralText`, `TokenText`, `ItemCursor`, `ChunkStream`, `Chunk::stream`, `require_token`, `require_end`, `token_text`, `end_span`, `parse_singleton`, `boundary_comma`
- parse-fields.md: `take_next` is not required yet; `consume_token_if`, `consume_group_if`, `require_group`, `spanning` (via `parse_items`), `contents_span`, `LevelSlot`, `ParsedSlot`, `UnparsedItem`, `parse_items`, `collect_slot_errors`, `Clone` on the chunk tree, `ChunkParent::UnparsedItem`
- parse-arguments.md: `take_next`, `spanning` around `parse_value`, `integer`, `BooleanValue(Boolean::{True, False})`
- parse-variables.md: `parse_singleton` on `[...]`, `Chunk::first_item`, `ConstantValue`, `parse_constant_value`, `Box<T>` delegation in `resolve_position`
- parse-descriptions.md: `consume_token_if_any`
- parse-pointers.md: `require_keyword`

A feature implementation is reviewed against this doc when it lands. The expected amendment sites are the two impl blocks (`ItemCursor`, `ChunkStream`), `parse_items`, `parse_singleton`, and `LiteralText`. If a feature adds a call that is not a method or function listed here, the review adds that method or function to this doc. This doc stays in `refactors/pending`.
