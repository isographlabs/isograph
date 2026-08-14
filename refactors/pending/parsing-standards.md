# Parsing standards

Rules for all grammar-stage code. The feature docs define the grammar; this doc defines how parsers are written. An implementation this doc forbids is resolved by amending this doc or fixing the code, in the same review, never by shipping the deviation.

Every operation a parser performs is a method or free function listed here. A new operation is a new listing here, never a local helper. Feature docs write productions against this surface; they do not extend it in place.

## The unit of work

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(text: &str, root: WithSpan<ChunkedLevel>) -> WithSpan<IsoLiteralParse>
```

The stage consumes one chunked literal. `text` is wrapped as `LiteralText` at this entry and nowhere else. Each chunk is read by one `ChunkStream`. A group's interior is a fresh `ChunkedLevel`. No cursor spans two chunks.

A group is one item, consumed whole, always closed. Its interior re-enters parsing only as a fresh level walk.

Items arrive pre-spanned. A parser computes a span only for a multi-item composite, via `spanning`.

Separators were absorbed into boundaries by chunking: "a separator comes next" is the stream exhausted. An unmatched bracket and its level's tail never left the matcher. A comma no item precedes never left chunking. No bracket or empty-chunk state reaches a parser.

## `ItemCursor` and `ChunkStream`

A chunk has two readers. `ItemCursor` is what a production sees: it can accept items and span composites. `ChunkStream` is what a walker sees: it lends the cursor, then checks that the production stopped. A production cannot call `require_end`. That is the same move as `SafePeekable`: the type you hold is the set of operations you can perform.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
use nonempty::NonEmpty;
use safe_peekable::{IntoSafePeekable, SafePeekable};
use span::{Span, WithSpan};

use crate::{
    BracketKind, ChunkContentItem, ChunkedGroup, Expectation, Found, LiteralText, NonBracketTokenKind,
    ParseError, TokenText,
};

/// What a production reads. No rewind, no raw peek, no end check: a committed item is
/// committed, a decision is made on at most the next item, and leftover is the walker's.
pub(crate) struct ItemCursor<'a> {
    items: SafePeekable<nonempty::Iter<'a, WithSpan<ChunkContentItem>>>,
    /// The end of the last accepted item (the chunk's start before any): where an
    /// `Expected(_, EndOfChunk)` error points.
    previous_end: u32,
    text: LiteralText<'a>,
}

/// The walker-facing reader of one chunk. Only `Chunk::stream` constructs one.
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

    /// Nothing further may exist. The first leftover item is the error, unconsumed.
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

    /// The next item's span, consumed, when it is a non-bracket token whose kind is
    /// one of `kinds`; `None`, nothing consumed, otherwise. One optional item that
    /// admits several kinds (a description: string or block string).
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

    /// The next item, committed, or `None` at the chunk's end. Total; errors are the
    /// caller's to construct.
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

    /// Runs a sub-parse and wraps its result in the span it consumed: from the start
    /// of the first item the closure accepts to the end of its last, empty at
    /// `previous_end` when it accepts none. On `Err`, returns that error as-is.
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

`ChunkStream::require_end` peeks through the cursor's iterator. It does not go through `ItemCursor`, so a production holding `&mut ItemCursor` cannot write it.

There is no `consume_keyword_if` and no `spanning` sibling that returns `T` or `Option<T>`. Neither has a caller. A method with no caller exists only in this doc, and these do not.

### Span sources

A leaf's span is what a cursor method returned (`require_token`, `consume_token_if`, `consume_token_if_any`, `require_keyword`) or the wrapper a consumed item carried (`consume_group_if`, `require_group`, `take_next`). An item's span is what its parse consumed (a degraded slot's is its chunk's `contents_span`). A composite's span comes from `spanning`. `Span::join` in a parser is banned; a span no source provides is a missing method here.

Construction is `WithSpan::new` and plain `Ok` / `Some`.

Every composite production is one `spanning` with its discriminating `take_next` (or first `require_*`) inside the closure. A production whose first item a caller already committed is not a composite of that item plus more: the caller owns the first item's span, the continuation is `require_*` / `consume_*`, and if the two must become one span the continuation is inside the same `spanning` that accepted the first item. `spanning_from` is not a method.

## `LiteralText`

Parsers do not receive `&str`. The entry wraps the literal once. The only reads are keyword equality and the `i64` conversion.

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
    /// The stream a walker reads this chunk through.
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

    /// The first content item. Total: every chunk has contents. The one-item walker's
    /// extra-chunk error reads this as `Found`.
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

`Chunk`'s fields are private to the `chunk` module. `contents` is a `NonEmpty<WithSpan<ChunkContentItem>>` (the `nonempty` crate). These methods are the only item access and the only boundary read. `Chunk` is `pub` and re-exported at the crate root, so `stream` is `pub(crate)`: `ChunkStream` never crosses the crate boundary.

`contents_span` is allowed `Span::join`: it is a `Chunk` method, not a parser.

## Level walks

`ChunkedLevel`'s vec is private to the `chunk` module. The only walks are the two functions below. A production never iterates a level. Tests that need the vec use `#[cfg(test)] ChunkedLevel::chunks`.

A list level (a selection set, an argument list, an object literal, a variable-declaration list) is `parse_level_items`. A one-item level (the root, a `[...]` interior) is `parse_singleton`.

```rust
// from crates/isograph_parser/src/chunk.rs
use resolve_position::ResolvePosition;
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan};

use crate::{
    Chunk, ChunkStream, Expectation, Found, ItemCursor, LiteralText, NonBracketTokenKind, ParseError,
};

/// One chunk's outcome in a list. The wrapping `WithSpan`'s span is the parsed
/// production's span, or the chunk's `contents_span` when unparsed.
#[derive(Debug, PartialEq, Eq)]
pub enum LevelSlot<T> {
    Parsed(ParsedSlot<T>),
    Unparsed(UnparsedItem),
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParsedSlot<T> {
    pub item: T,
    /// Present when the production succeeded and leftover items followed it. Unmarked
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
    /// One `LevelSlot` per chunk, same length as the level. Each chunk is a fresh
    /// stream. The production sees only the cursor; this walk owns `require_end`.
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

/// Exactly one chunk, parsed as `parse`, then exhausted under `end_expectation`, then
/// rejected if its boundary holds a comma, then rejected if a second chunk exists.
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

`parse_level_items` returns `Vec<WithSpan<LevelSlot<P>>>`, not `Result`. A production's `Err` becomes `LevelSlot::Unparsed`. `?` cannot leak a chunk's failure to its siblings. Output length equals chunk count.

Trailing leftover does not void a completed item: `foo bar` is the selection `foo` plus a trailing error at `bar`. Go-to-definition, find-references, and completion see `foo` as if the leftover were absent. The item's span excludes the leftover, so leftover positions answer the containing level.

Leftover is always the suffix; nothing after the first leftover is reattached. `foo bar { baz }` is the scalar `foo` with leftover from `bar` on; `baz` is not in the tree.

`parse_singleton` is the opposite on leftover: the failed `require_end` is the whole context's error (the declaration, the `[...]` type). The comma check uses the same `end_expectation` (`EndOfDeclaration`, `EndOfType`).

### `LevelSlot` and `ResolvePosition`

`LevelSlot<T>` is a resolve-position wrapper, not a leaf. Positions on a parsed slot resolve as `T`. Positions on an unparsed slot resolve as `UnparsedItem`. The trailing error is not a resolution target.

This is a blanket delegation, the same class as the `Box<T>` impl parse-variables.md lands in `resolve_position`. It lives next to `LevelSlot` because it names `UnparsedItem` / `UnparsedItemParent`. It is not a per-type resolve walk.

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

The derive on `SelectionSet` sees `Vec<WithSpan<LevelSlot<Selection>>>`, unwraps `Vec` and `WithSpan`, and calls `LevelSlot::resolve`. That is the same emission `Vec<WithSpan<Selection>>` already has, with `LevelSlot<Selection>` as the inner type.

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

- `require_*`: required grammar. Errors on the found item, unconsumed, or at `end_span` where the missing item belonged. Consumes exactly the accepted items. Methods on `ItemCursor`.
- `consume_*`: optional grammar, always a single item. Consumes and returns the item when the next item matches; consumes nothing and returns `None` otherwise. Never errors. Methods on `ItemCursor`.
- `parse_*`: a composite production. Takes `&mut ItemCursor`. Errors propagate from the first failing piece. Shared structure is a higher-order function (`parse_level_items`, `parse_singleton`, `spanning`); a hand-duplicated walk is banned.

A parse function that consumes a group and walks its interior is `parse_*` / `consume_*` at the grammar layer, built from `require_group` / `consume_group_if` plus a level walk:

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

`ItemCursor::text` is the `LiteralText` the stream was built with, so a group walk can reuse it.

## Dispatch

A position allowing several forms is one exhaustive `match` on `take_next()`, inside `spanning` when the arms together are one composite. The discriminating item commits up front; accepting arms continue from it; the rejecting arm reports it as the `found`.

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

`consume_*` is not a dispatch tool. It serves the single optional item, including at a composition boundary, where a sub-parser declines an item that belongs to its caller (the optional `!` after a type name, whose absence might be the caller's `=`) and so cannot own an exhaustive match there. A `consume_*` chain where one production owns all the alternatives is banned; several kinds of one optional item is `consume_token_if_any`.

A construct that is optional as a whole but required once its first item appears (`$name`, a future `@ name (args)`) is a dispatch arm: the first item commits in the match, the remainder is `require_*`.

Optionality is decided by the first item. A single optional item is a `consume_*`, infallible. A multi-item optional commits its first item and is fallible from its second on (`@@` errors at the second `@`). Committing before knowing the interpretation (the alias's colon deciding what the committed identifier was) is legal only when every continuation uses everything committed. Consuming and then declining does not exist, so a grammar addition needing more than one item of lookahead for optionality is unwritable.

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

A production that forbids a sub-form is a narrower type, not a post-pass over the wider type. Variable defaults are `ConstantValue`. `$` is an error at the `$`, constructed by `parse_constant_value`. `DeclaredVariable::default_value` cannot hold a variable.

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

`parse_value` and `parse_constant_value` share the scalar arms (string, integer, boolean, null) through a function that returns `ConstantValue`. `parse_value` wraps those as `NonConstantValue` and adds the `$` and non-constant object arms. `parse_constant_value` errors on `$` with `Expectation::ConstantValue` and walks object entries via `parse_constant_object_entry`.

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

Errors live in the tree (`UnparsedLiteral`, `UnparsedItem`, `ParsedSlot::trailing`). `errors()` derives the list in source order. No error list exists beside the grammar tree.

Bracket and comma errors are the earlier passes', returned beside their trees. No `ParseError` variant names either. The final sweep is all three lists: an error-free literal has empty vecs from the matcher and chunking and an empty `errors()` from the grammar.

Degradation is as local as the grammar allows: a failed list chunk degrades alone; a failed declaration header degrades the literal. One error per degraded region. A trailing leftover on a successful item is not a degraded region; the item stays parsed.

No prose in the parser. Messages are `Display` impls; contextual suggestions belong to rendering, keyed off the `(expected, found)` pair.

## Totality

No panics on any input: `unwrap`, `expect`, `unreachable!`, and type-level infallibility claims are banned. A real but compiler-invisible invariant gets a graceful fallback and a doc comment stating the invariant.

Every input yields a tree; every position resolves: parsed regions to grammar leaves, degraded regions through retained chunks, everything else (whitespace, leftover, the matcher's dropped regions, chunking's dropped commas) to the nearest container.

The resolution path is context, never a retargeting mechanism. An identity-bearing action (find-references, rename, go-to-definition) acts only when the resolved leaf is itself a name leaf; it never walks the path to a nearest actionable ancestor, so a container answer, and therefore leftover, can never borrow a parent's identity: in `foo { bar baz }`, the caret on `baz` finds nothing, not `foo`'s references. Context features (hover, completion) are the ones that read ancestry.

## Trees and spans

Span placement is decided at the type: a tree enum is `WithSpan`-wrapped once at its slot, variant payloads are bare, struct fields each carry their own `WithSpan`, and each wrapper's coverage is stated on the type.

A name is a `WithSpan`-wrapped fieldless marker struct, one type per role (an alias is not a name; an argument name is not an object key); its text is the wrapper's span. No strings, no interning; the one derived scalar is the converted `i64`. Punctuation and keywords (the dot, the `$`, the `!`, `to`) get no nodes of their own; their positions answer their container.

`ResolvePosition` is derive-only, except the two blanket delegations: `Box<T>` in `resolve_position` (parse-variables.md) and `LevelSlot<T>` next to `LevelSlot`. A third manual impl is a missing `resolve_position` feature and becomes a prefactor there. A parent is a direct path alias at one parent, an enum at the second. Chunk-stage `IsographResolutionNode` variants are reachable inside degraded regions only.

## Performance

One pass, by reference; the output copies spans and `Copy` tokens. Cloning happens only when a region degrades: allocation beyond the output vecs is proportional to the error count.

No backtracking exists (no rewind), so parse time is linear in the token count.

No pico, no interning: plain functions over `LiteralText` and the chunk tree.

## Catalog of parsing tasks

Every grammar-stage task is one row. A task that is not here is a missing method or a missing walk.

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
- End of a production: the walker, never the production

## Shipping and amending

Each structure and each method lands with the feature doc of its first production caller. A method with no caller yet exists only in this doc.

- parse-entrypoint.md: `LiteralText`, `TokenText`, `ItemCursor`, `ChunkStream`, `Chunk::stream`, `require_token`, `require_end`, `token_text`, `end_span`, `parse_singleton`, `boundary_comma`
- parse-fields.md: `take_next` is not required yet; `consume_token_if`, `consume_group_if`, `require_group`, `spanning` (via `parse_items`), `contents_span`, `LevelSlot`, `ParsedSlot`, `UnparsedItem`, `parse_items`, `collect_slot_errors`, `Clone` on the chunk tree, `ChunkParent::UnparsedItem`
- parse-arguments.md: `take_next`, `spanning` at a production (values), `integer`, `BooleanValue(Boolean::{True, False})`
- parse-variables.md: `parse_singleton` on `[...]`, `Chunk::first_item`, `ConstantValue`, `parse_constant_value`, `Box<T>` delegation in `resolve_position`
- parse-descriptions.md: `consume_token_if_any`
- parse-pointers.md: `require_keyword`

A feature implementation is reviewed against this doc when it lands. The expected amendment sites are the two impl blocks (`ItemCursor`, `ChunkStream`), the two walks, and `LiteralText`. A change that routes around a structure instead of extending it is what this doc exists to prevent. This doc itself never moves to refactors/past: it is normative and stays current.
