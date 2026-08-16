# Parsing standards

Rules for grammar-stage code. Feature docs define each form (a selection, a value, a type) and the items that make it up. This doc lists the functions that implement those forms. If an implementation disagrees with this doc, the same review amends the doc or changes the code.

Every call a parse function makes on a chunk is a method or free function listed here. A new call is a new listing here.

## The unit of work

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(text: &str, root: WithSpan<ChunkedLevel>) -> WithSpan<IsoLiteralParse>
```

`parse_iso_literal` takes `text: &str` and the chunked literal. Each chunk is passed to `Chunk::stream(text)`, which returns one `ChunkStream`. A group's interior is the `ChunkedLevel` in `group.children`. A `ChunkStream` is built from one chunk.

A group is one item. `require_group` and `consume_group_if` return it in one call. The interior is parsed by calling `parse_items` or `parse_singleton` on `group.children`.

Each token and group has a span. A parse function assigns a span to a value made of more than one item by calling `spanning`. `expected` on an exhausted cursor is `Expected(_, EndOfChunk)` at `end_span`.

## `ItemCursor` and `ChunkStream`

`parse_chunk` calls `Chunk::stream`, then passes `stream.cursor()` (`&mut ItemCursor`) into the parse function. `parse_one_item` then calls `stream.require_end` and builds a `LevelSlot`. Diagnostics are not leftover items. Leftover and failed items are `UnparsedChunkItems`. `item` on a slot is `Some` for `Complete` and `Both`. Artifact generation requires the tree's `errors()` and the earlier-stage error lists to be empty. `require_end` is a method on `ChunkStream`.

`Tok` (unparsed chunk items) and `E` (diagnostics) may later become type parameters on the slot. This pass hardcodes `UnparsedChunkItems` and `Vec<WithSpan<ParseError>>`.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
use nonempty::NonEmpty;
use safe_peekable::{IntoSafePeekable, SafePeekable};
use span::{Span, WithSpan};

use crate::{
    BracketKind, ChunkContentItem, ChunkedGroup, Expectation, Found, NonBracketTokenKind,
    ParseError,
};

/// Sequential reader of one chunk. Parameter of a parse function.
pub(crate) struct ItemCursor<'a> {
    items: SafePeekable<nonempty::Iter<'a, WithSpan<ChunkContentItem>>>,
    /// The end of the last item this cursor advanced past (the chunk's start before
    /// any). An `Expected(_, EndOfChunk)` error uses this offset.
    previous_end: u32,
    text: &'a str,
}

/// Sequential reader of one chunk, plus `require_end`.
pub(crate) struct ChunkStream<'a> {
    cursor: ItemCursor<'a>,
}

impl<'a> ChunkStream<'a> {
    pub(crate) fn new(contents: &'a NonEmpty<WithSpan<ChunkContentItem>>, text: &'a str) -> Self {
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

    pub(crate) fn require_end(&mut self) -> Result<(), ()> {
        self.cursor.items.peek().map_or(().wrap_ok(), |_| ().wrap_err())
    }

    /// Unread content items. `None` when the cursor is at end.
    pub(crate) fn remaining_contents(&mut self) -> Option<NonEmpty<WithSpan<ChunkContentItem>>> {
        let first = self.cursor.items.next()?;
        let mut tail = Vec::new();
        while let Some(item) = self.cursor.items.next() {
            tail.push(*item);
        }
        NonEmpty {
            head: *first,
            tail,
        }
        .wrap_some()
    }
}

impl<'a> ItemCursor<'a> {
    pub(crate) fn consume_token_if(&mut self, kind: NonBracketTokenKind) -> Option<Span> {
        let peek = self.items.peek()?;
        let item = *peek.view();
        match item.item.reference() {
            ChunkContentItem::NonBracket(token) if token.0 == kind => {
                peek.commit();
                self.previous_end = item.location.end;
                item.location.wrap_some()
            }
            _ => None,
        }
    }

    pub(crate) fn consume_group_if(
        &mut self,
        kind: BracketKind,
    ) -> Option<WithSpan<&'a ChunkedGroup>> {
        let peek = self.items.peek()?;
        let item = *peek.view();
        match item.item.reference() {
            ChunkContentItem::Group(group) if group.opening.item.0 == kind => {
                let location = item.location;
                peek.commit();
                self.previous_end = location.end;
                WithSpan::new(group, location).wrap_some()
            }
            _ => None,
        }
    }

    pub(crate) fn expected(&mut self, expected: Expectation) -> WithSpan<ParseError> {
        match self.items.peek() {
            None => WithSpan::new(
                ParseError::expected(expected, Found::EndOfChunk),
                self.end_span(),
            ),
            Some(peek) => {
                let item = *peek.view();
                WithSpan::new(
                    ParseError::expected(expected, Found::from(item.item.reference())),
                    item.location,
                )
            }
        }
    }

    pub(crate) fn require_token(&mut self, kind: NonBracketTokenKind) -> Result<Span, ()> {
        self.consume_token_if(kind).ok_or(())
    }

    pub(crate) fn require_group(
        &mut self,
        kind: BracketKind,
    ) -> Result<WithSpan<&'a ChunkedGroup>, ()> {
        self.consume_group_if(kind).ok_or(())
    }

    pub(crate) fn text(&self) -> &'a str {
        self.text
    }

    pub(crate) fn token_text(&self, span: Span) -> &'a str {
        &self.text[span.as_usize_range()]
    }

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
        WithSpan::new(value, span).wrap_ok()
    }

    fn end_span(&self) -> Span {
        Span::new(self.previous_end, self.previous_end)
    }
}
```

`nonempty::Iter` yields `&'a WithSpan<ChunkContentItem>`. `view` returns that reference. `consume_group_if` copies it, matches out the `&'a ChunkedGroup` from the chunk item, then `commit`s.

### Span sources

- Leaf: the `Span` from `require_token` or `consume_token_if`, or the `WithSpan` from `require_group` or `consume_group_if`.
- Parsed list item: the `WithSpan` `spanning` returned, stored on `Complete` and on `Both.item`.
- Slot: `Complete` is that same item span. `Both` is the join of the item span and the leftover items' span. `Failed` / an `UnparsedChunkItems` is `contents_span`.
- Value made of several items: one `spanning` call. The closure's first advance is a `consume_*` or `require_*`. Remaining items of that value are read inside the same `spanning`.

`token_text` is `&self.text[span.as_usize_range()]`. A span that is not a range of that string panics, the same as any `&str` index. Names in the tree are spans. The converted scalar is the `i64`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(text: &str, root: WithSpan<ChunkedLevel>) -> WithSpan<IsoLiteralParse> {
    let location = root.location;
    let singleton = parse_singleton(
        root.reference(),
        text,
        || WithSpan::new(ParseError::EmptyLiteral, location),
        |extra| WithSpan::new(ParseError::MultipleDeclarations, extra.location),
        parse_iso_literal_item,
    );
    WithSpan::new(
        IsoLiteralParse {
            first: singleton.first.map(|slot| {
                // resolve-position-generic-slot.md: this map is gone.
                WithSpan::new(RootSlot::from(slot.item), slot.location)
            }),
            extra: singleton.extra,
            errors: singleton.errors,
        },
        location,
    )
}

fn parse_iso_literal_item(
    cursor: &mut ItemCursor<'_>,
) -> Result<IsoLiteralItem, WithSpan<ParseError>> {
    let keyword = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::DeclarationKeyword))?;
    match cursor.token_text(keyword) {
        text if text == "entrypoint" => {
            IsoLiteralItem::Entrypoint(parse_entrypoint(keyword, cursor)?).wrap_ok()
        }
        text if text == "field" || text == "pointer" => {
            WithSpan::new(ParseError::UnsupportedDeclarationType, keyword).wrap_err()
        }
        _ => WithSpan::new(
            ParseError::expected(
                Expectation::DeclarationKeyword,
                Found::Token(NonBracketTokenKind::Identifier),
            ),
            keyword,
        )
        .wrap_err(),
    }
}
```

## `Chunk`

```rust
// from crates/isograph_parser/src/chunk.rs
impl Chunk {
    pub(crate) fn stream<'a>(&'a self, text: &'a str) -> ChunkStream<'a> {
        ChunkStream::new(self.contents.reference(), text)
    }

    /// First content item through last content item. `WithSpan<Chunk>` also covers
    /// the trailing separator.
    pub fn contents_span(&self) -> Span {
        Span::join(
            self.contents.first().location,
            self.contents.last().location,
        )
    }

    pub(crate) fn first_item(&self) -> &WithSpan<ChunkContentItem> {
        self.contents.first()
    }

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

A `Chunk` is the whole unit: its chunk items plus the optional trailing separator. A chunk item is a `ChunkContentItem` (a token or a group). `UnparsedChunkItems` is unread or failed items from the chunk under parse, not a chunk. `ExtraChunks` is whole extra chunks after chunk 0.

`Chunk`'s fields are private to the `chunk` module. `contents` is a `NonEmpty<WithSpan<ChunkContentItem>>` (chunk-contents-nonempty.md). `Chunk` is `pub`; `stream` is `pub(crate)`. `WithSpan<Chunk>` runs from the first content item through the trailing separator. `contents_span` stops at the last content item. Leftover and failed `UnparsedChunkItems` are those items only; a list chunk's comma is not among them. A position on that comma resolves to the list.

## Lists and one-item levels

`ChunkedLevel`'s vec is private to the `chunk` module. `len` is the chunk count. `parse_items` maps each chunk through `parse_one_item` (a selection set, an argument list, an object literal, a variable-declaration list). `parse_singleton` is a one-item level (the root, a `[...]` interior): chunk 0 through `parse_one_item`, then extra chunks and, at the root, a boundary comma. Tests call `#[cfg(test)] ChunkedLevel::chunks`.

```rust
// from crates/isograph_parser/src/chunk.rs
use nonempty::NonEmpty;
use resolve_position::ResolvePosition;
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan};

use crate::{
    Chunk, ChunkContentItem, ChunkStream, Expectation, Found, ItemCursor, NonBracketTokenKind,
    ParseError,
};

/// Unread or failed items from the chunk under parse. No diagnostic field.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = UnparsedChunkItemsParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct UnparsedChunkItems {
    #[resolve_field(parent_variant = Unparsed)]
    pub items: NonEmpty<WithSpan<ChunkContentItem>>,
}

#[derive(Debug)]
pub enum UnparsedChunkItemsParent<'a> {
    Both(BothSelectionPath<'a>),
    Failed(FailedPath<'a>),
}

/// Extra root chunks after the first. Resolve walks each chunk.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralParsePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ExtraChunks {
    #[resolve_field(parent_variant = Extra)]
    pub chunks: NonEmpty<WithSpan<Chunk>>,
}

/// One chunk. Does not implement `ResolvePosition`.
/// resolve-position-generic-slot.md puts this on the tree instead of a concrete copy.
#[derive(Debug, PartialEq, Eq)]
pub enum LevelSlot<T> {
    Complete(WithSpan<T>),
    Both(Both<T>),
    Failed(Failed),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Both<T> {
    pub item: WithSpan<T>,
    pub leftover: UnparsedChunkItems,
    pub errors: Vec<WithSpan<ParseError>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Failed {
    pub items: UnparsedChunkItems,
    pub errors: Vec<WithSpan<ParseError>>,
}

impl<T> LevelSlot<T> {
    pub fn item(&self) -> Option<&T> {
        match self {
            LevelSlot::Complete(item) => item.item.reference().wrap_some(),
            LevelSlot::Both(both) => both.item.item.reference().wrap_some(),
            LevelSlot::Failed(_) => None,
        }
    }

    pub fn errors(&self) -> Vec<&WithSpan<ParseError>> {
        match self {
            LevelSlot::Complete(_) => Vec::new(),
            LevelSlot::Both(both) => both.errors.iter().collect(),
            LevelSlot::Failed(failed) => failed.errors.iter().collect(),
        }
    }
}

fn parse_chunk<'a, P>(
    chunk: &'a WithSpan<Chunk>,
    text: &'a str,
    parse_item: impl FnOnce(&mut ItemCursor<'a>) -> Result<P, WithSpan<ParseError>>,
) -> (
    ChunkStream<'a>,
    Result<WithSpan<P>, WithSpan<ParseError>>,
) {
    let mut stream = chunk.item.stream(text);
    let result = stream.cursor().spanning(parse_item);
    (stream, result)
}

fn parse_one_item<'a, P>(
    chunk: &'a WithSpan<Chunk>,
    text: &'a str,
    leftover_error: impl FnOnce(&mut ItemCursor<'a>) -> WithSpan<ParseError>,
    parse: impl FnOnce(&mut ItemCursor<'a>) -> Result<P, WithSpan<ParseError>>,
) -> WithSpan<LevelSlot<P>> {
    let (mut stream, result) = parse_chunk(chunk, text, parse);
    match result {
        Ok(item) => {
            if stream.require_end().is_ok() {
                return WithSpan::new(LevelSlot::Complete(item), item.location);
            }
            let error = leftover_error(stream.cursor());
            match stream.remaining_contents() {
                Some(remaining) => {
                    let leftover_span =
                        Span::join(remaining.first().location, remaining.last().location);
                    let location = Span::join(item.location, leftover_span);
                    WithSpan::new(
                        LevelSlot::Both(Both {
                            item,
                            leftover: UnparsedChunkItems { items: remaining },
                            errors: error.wrap_vec(),
                        }),
                        location,
                    )
                }
                None => WithSpan::new(LevelSlot::Complete(item), item.location),
            }
        }
        Err(reason) => WithSpan::new(
            LevelSlot::Failed(Failed {
                items: UnparsedChunkItems {
                    items: chunk.item.contents.clone(),
                },
                errors: reason.wrap_vec(),
            }),
            chunk.item.contents_span(),
        ),
    }
}

pub struct Singleton<T> {
    pub first: Option<WithSpan<LevelSlot<T>>>,
    pub extra: Option<ExtraChunks>,
    pub errors: Vec<WithSpan<ParseError>>,
}

impl ChunkedLevel {
    pub(crate) fn parse_items<'a, P>(
        &'a self,
        text: &'a str,
        parse_item: impl Fn(&mut ItemCursor<'a>) -> Result<P, WithSpan<ParseError>>,
    ) -> Vec<WithSpan<LevelSlot<P>>> {
        self.0
            .iter()
            .map(|chunk| {
                parse_one_item(
                    chunk,
                    text,
                    |cursor| cursor.expected(Expectation::Separator),
                    parse_item.reference(),
                )
            })
            .collect()
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    #[cfg(test)]
    pub(crate) fn chunks(&self) -> &[WithSpan<Chunk>] {
        self.0.reference()
    }
}

pub(crate) fn parse_singleton<'a, T>(
    level: &'a WithSpan<ChunkedLevel>,
    text: &'a str,
    empty: impl FnOnce() -> WithSpan<ParseError>,
    extra: impl FnOnce(&'a WithSpan<Chunk>) -> WithSpan<ParseError>,
    parse: impl FnOnce(&mut ItemCursor<'a>) -> Result<T, WithSpan<ParseError>>,
) -> Singleton<T> {
    if level.item.len() == 0 {
        return Singleton {
            first: None,
            extra: None,
            errors: empty().wrap_vec(),
        };
    }
    let first = parse_one_item(
        &level.item.0[0],
        text,
        |cursor| cursor.expected(Expectation::EndOfDeclaration),
        parse,
    );
    let mut errors = Vec::new();
    if let Some(comma) = level.item.0[0].item.boundary_comma() {
        errors.push(WithSpan::new(
            ParseError::expected(
                Expectation::EndOfDeclaration,
                Found::Token(NonBracketTokenKind::Comma),
            ),
            comma,
        ));
    }
    let extra_chunks = (level.item.len() > 1).then(|| {
        let rest = NonEmpty {
            head: level.item.0[1].clone(),
            tail: level.item.0[2..].to_vec(),
        };
        errors.push(extra(&rest.head));
        ExtraChunks { chunks: rest }
    });
    Singleton {
        first: first.wrap_some(),
        extra: extra_chunks,
        errors,
    }
}
```

`ChunkStream::remaining_contents` returns the unread chunk items after `require_end` `Err`. That list is nonempty. Leftover `UnparsedChunkItems` is those items.

`parse_one_item` is one chunk. `Complete` is parse `Ok` and `require_end` `Ok`. `Both` is parse `Ok` and leftover items plus `expected(Separator)` (lists) or `expected(EndOfDeclaration)` (singleton). `Failed` is parse `Err` and a clone of the source chunk's items. `parse_*` is all-or-nothing. There is no recovered prefix of a selection.

`parse_items` is `parse_one_item` per chunk. Length equals chunk count. A list trailing comma is legal and is not a diagnostic. `foo { bar } asdf` is `Both`: the object selection `foo { bar }` and leftover items `asdf`. A position on `asdf` resolves through `UnparsedChunkItems`, not the selection set.

`parse_singleton` is not `Vec<LevelSlot>`. Chunk 0 is `parse_one_item`. Remaining chunks are `ExtraChunks` (every chunk after the first) plus `extra` (at the root, `MultipleDeclarations` on the first extra chunk). A boundary comma is a tokenless diagnostic in `Singleton::errors`. Empty is `first: None` and `empty()`. `item` on the first slot is `Some` when that slot is `Complete` or `Both`.

`LevelSlot` does not implement `ResolvePosition`. Each list stores a concrete slot enum that derives. The root stores `RootSlot` / `BothRoot`. resolve-position-generic-slot.md puts `LevelSlot<T>` on the tree instead.

### Concrete slots

```rust
// from crates/isograph_parser/src/selections.rs
/// Derived stand-in for `LevelSlot<Selection>`. resolve-position-generic-slot.md.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSetPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum SelectionSlot {
    Complete(#[resolve_field(parent_variant = Complete)] WithSpan<Selection>),
    Both(BothSelection),
    Failed(Failed),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSetPath<'a>, resolved_node = IsographResolutionNode<'a>)]
/// Derived stand-in for `Both<Selection>`. resolve-position-generic-slot.md.
pub struct BothSelection {
    #[resolve_field(parent_variant = Both)]
    pub item: WithSpan<Selection>,
    #[resolve_field]
    pub leftover: UnparsedChunkItems,
    pub errors: Vec<WithSpan<ParseError>>,
}

pub type BothSelectionPath<'a> =
    PositionResolutionPath<&'a BothSelection, SelectionSetPath<'a>>;

// resolve-position-generic-slot.md: the field is Vec<WithSpan<LevelSlot<Selection>>>.
pub struct SelectionSet(#[resolve_field] pub Vec<WithSpan<SelectionSlot>>);

#[derive(Debug)]
pub enum SelectionParent<'a> {
    Complete(SelectionSetPath<'a>),
    Both(BothSelectionPath<'a>),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum Selection {
    Scalar(ScalarSelection),
    Object(ObjectSelection),
}
```

`ArgumentSlot` / `BothArgument`, `ObjectEntrySlot` / `BothObjectEntry`, `ConstantObjectEntrySlot` / `BothConstantObjectEntry`, and `VariableDeclarationSlot` / `BothVariableDeclaration` are the same three arms, each with that list's path as `parent_type`. resolve-position-generic-slot.md.

```rust
// from crates/isograph_parser/src/selections.rs
// resolve-position-generic-slot.md: the list field is LevelSlot<Selection>.
impl From<WithSpan<LevelSlot<Selection>>> for WithSpan<SelectionSlot> {
    fn from(slot: WithSpan<LevelSlot<Selection>>) -> Self {
        let location = slot.location;
        let item = match slot.item {
            LevelSlot::Complete(item) => SelectionSlot::Complete(item),
            LevelSlot::Both(both) => SelectionSlot::Both(BothSelection {
                item: both.item,
                leftover: both.leftover,
                errors: both.errors,
            }),
            LevelSlot::Failed(failed) => SelectionSlot::Failed(failed),
        };
        WithSpan::new(item, location)
    }
}
```

The same `From` exists per list. A list site maps `parse_items`:

```rust
// from crates/isograph_parser/src/selections.rs
        SelectionSet(
            group
                .item
                .children
                .item
                .parse_items(cursor.text(), parse_selection)
                // resolve-position-generic-slot.md: this map is gone.
                .into_iter()
                .map(WithSpan::<SelectionSlot>::from)
                .collect(),
        )
```

`UnparsedChunkItemsParent` is the parent of leftover and failed `UnparsedChunkItems`. The derive's `parent_variant` wraps `Both` or `Failed`. Chunk items in `UnparsedChunkItems` use `parent_variant = Unparsed`. Extra chunks use `Chunk`'s parent variant `Extra`. There is no `From` into those parent enums.

### Errors from slots

`collect_slot_errors` is one function per list. The selection-set copy:

```rust
// from crates/isograph_parser/src/selections.rs
// resolve-position-generic-slot.md: one walk over LevelSlot; the per-list copies go away.
pub(crate) fn collect_selection_slot_errors(
    slots: &[WithSpan<SelectionSlot>],
    nested: impl Fn(&Selection, &mut Vec<WithSpan<ParseError>>),
    errors: &mut Vec<WithSpan<ParseError>>,
) {
    for slot in slots {
        match slot.item.reference() {
            SelectionSlot::Complete(selection) => nested(selection.item.reference(), errors),
            SelectionSlot::Both(both) => {
                nested(both.item.item.reference(), errors);
                errors.extend(both.errors.iter().copied());
            }
            SelectionSlot::Failed(failed) => errors.extend(failed.errors.iter().copied()),
        }
    }
}
```

Nested errors (arguments, nested selections) precede that slot's leftover or failed diagnostics, matching source order.

## Function shapes

- `consume_*`: `ItemCursor` method. Match: `commit` and `Some`. Else: `None`.
- `expected`: `ItemCursor` method. Peek, no `commit`. Next item or `EndOfChunk` becomes `Expected(expected, found)`.
- `require_*`: `consume_*` or `Err(())`. The caller maps `Err` with `expected`.
- `parse_*`: implements a form made of several items. Parameter is `&mut ItemCursor`. First `Err` is returned. Shared iteration is `parse_items`, `parse_singleton`, or `spanning`.

A group plus its interior:

```rust
// from crates/isograph_parser/src/selections.rs
pub(crate) fn consume_selection_set(
    cursor: &mut ItemCursor<'_>,
) -> Option<WithSpan<SelectionSet>> {
    let group = cursor.consume_group_if(BracketKind::Brace)?;
    WithSpan::new(
        SelectionSet(
            group
                .item
                .children
                .item
                .parse_items(cursor.text(), parse_selection)
                // resolve-position-generic-slot.md: this map is gone.
                .into_iter()
                .map(WithSpan::<SelectionSlot>::from)
                .collect(),
        ),
        group.location,
    ).wrap_some()
}

pub(crate) fn require_selection_set(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<SelectionSet>, WithSpan<ParseError>> {
    let group = cursor
        .require_group(BracketKind::Brace)
        .map_err(|()| cursor.expected(Expectation::SelectionSet))?;
    WithSpan::new(
        SelectionSet(
            group
                .item
                .children
                .item
                .parse_items(cursor.text(), parse_selection)
                // resolve-position-generic-slot.md: this map is gone.
                .into_iter()
                .map(WithSpan::<SelectionSlot>::from)
                .collect(),
        ),
        group.location,
    ).wrap_ok()
}
```

## Dispatch

When the next item may start several forms, the parse function is a `consume_*` ladder. The last arm is `expected`. If those arms are one value, the ladder is inside `spanning`. An arm that has taken its first item continues with `require_*` / `consume_*`.

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn parse_value(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<NonConstantValue>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if let Some(dollar) = cursor.consume_token_if(NonBracketTokenKind::Dollar) {
            let name = cursor
                .require_token(NonBracketTokenKind::Identifier)
                .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
            return NonConstantValue::Variable(VariableUse {
                dollar: WithSpan::new(Dollar, dollar),
                name: WithSpan::new(VariableName, name),
            }).wrap_ok();
        }
        if cursor
            .consume_token_if(NonBracketTokenKind::StringLiteral)
            .is_some()
        {
            return NonConstantValue::String(StringValue).wrap_ok();
        }
        if let Some(span) = cursor.consume_token_if(NonBracketTokenKind::IntegerLiteral) {
            let value = match cursor.token_text(span).parse() {
                Ok(value) => value,
                Err(_) => {
                    return WithSpan::new(ParseError::IntegerDoesNotFitI64, span).wrap_err();
                }
            };
            return NonConstantValue::Integer(IntegerValue(value)).wrap_ok();
        }
        if let Some(span) = cursor.consume_token_if(NonBracketTokenKind::Identifier) {
            return match cursor.token_text(span) {
                "true" => NonConstantValue::Boolean(BooleanValue(Boolean::True)).wrap_ok(),
                "false" => NonConstantValue::Boolean(BooleanValue(Boolean::False)).wrap_ok(),
                "null" => NonConstantValue::Null(NullValue).wrap_ok(),
                _ => WithSpan::new(
                    ParseError::expected(
                        Expectation::Value,
                        Found::Token(NonBracketTokenKind::Identifier),
                    ),
                    span,
                ).wrap_err(),
            };
        }
        if let Some(group) = cursor.consume_group_if(BracketKind::Brace) {
            return NonConstantValue::Object(ObjectLiteral(
                group
                    .item
                    .children
                    .item
                    .parse_items(cursor.text(), parse_object_entry)
                    // resolve-position-generic-slot.md: this map is gone.
                    .into_iter()
                    .map(WithSpan::<ObjectEntrySlot>::from)
                    .collect(),
            )).wrap_ok();
        }
        cursor.expected(Expectation::Value).wrap_err()
    })
}
```

Keyword text after `require_token(Identifier, ...)` or `consume_token_if(Identifier)`: `match` on `token_text` (`"entrypoint"` / `"field"` / `"pointer"`; `"true"` / `"false"` / `"null"`; `"to"`).

One optional item is `consume_*`. Two optional kinds in one position is two `consume_token_if` calls. The optional `!` after a type name is `consume_token_if(Exclamation)`: the next item may be the caller's `=`. `$name` is `consume_token_if(Dollar)` then `require_token(Identifier)`. After `require_token` on an identifier, `consume_token_if(Colon)` is the alias; both arms use the identifier.

```rust
// from crates/isograph_parser/src/selections.rs
fn parse_selection(cursor: &mut ItemCursor<'_>) -> Result<Selection, WithSpan<ParseError>> {
    let first = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Selection))?;
    let (reader_alias, name) = match cursor.consume_token_if(NonBracketTokenKind::Colon) {
        Some(_) => {
            let name = cursor
                .require_token(NonBracketTokenKind::Identifier)
                .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
            (
                WithSpan::new(SelectionAlias, first).wrap_some(),
                WithSpan::new(SelectionName, name),
            )
        }
        None => (None, WithSpan::new(SelectionName, first)),
    };
    let arguments = consume_argument_list(cursor);
    let selection_set = consume_selection_set(cursor);
    (match selection_set {
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
    }).wrap_ok()
}
```

`parse_one_item` wraps `parse_selection` in `spanning` and then calls `require_end`. Leftover is `Both`.

## Narrower types for narrower grammars

Variable defaults are `ConstantValue`. `parse_constant_value` returns `Err` at `$`. `DeclaredVariable::default_value` is `Option<WithSpan<ConstantValue>>`.

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

// resolve-position-generic-slot.md: the field is Vec<WithSpan<LevelSlot<ObjectEntry>>>.
pub struct ObjectLiteral(#[resolve_field] pub Vec<WithSpan<ObjectEntrySlot>>);

pub enum ObjectEntry {
    Named(NamedObjectEntry),
}

pub struct NamedObjectEntry {
    #[resolve_field]
    pub name: WithSpan<ObjectEntryName>,
    #[resolve_field(parent_variant = ObjectEntry)]
    pub value: WithSpan<NonConstantValue>,
}

// resolve-position-generic-slot.md: the field is Vec<WithSpan<LevelSlot<ConstantObjectEntry>>>.
pub struct ConstantObjectLiteral(#[resolve_field] pub Vec<WithSpan<ConstantObjectEntrySlot>>);

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

`parse_constant_value` is the same ladder without the `$` arm. `$` falls through to `expected(Expectation::ConstantValue)`. Object entries call `parse_constant_object_entry`. The integer arm is the same `token_text(span).parse()` match; that `span` is the one `consume_token_if(IntegerLiteral)` just returned. `parse::<i64>()` on an `IntegerLiteral` token (`-?(0|[1-9][0-9]*)`) fails only as overflow or underflow.

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn parse_constant_value(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<ConstantValue>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if cursor
            .consume_token_if(NonBracketTokenKind::StringLiteral)
            .is_some()
        {
            return ConstantValue::String(StringValue).wrap_ok();
        }
        if let Some(span) = cursor.consume_token_if(NonBracketTokenKind::IntegerLiteral) {
            let value = match cursor.token_text(span).parse() {
                Ok(value) => value,
                Err(_) => {
                    return WithSpan::new(ParseError::IntegerDoesNotFitI64, span).wrap_err();
                }
            };
            return ConstantValue::Integer(IntegerValue(value)).wrap_ok();
        }
        if let Some(span) = cursor.consume_token_if(NonBracketTokenKind::Identifier) {
            return match cursor.token_text(span) {
                "true" => ConstantValue::Boolean(BooleanValue(Boolean::True)).wrap_ok(),
                "false" => ConstantValue::Boolean(BooleanValue(Boolean::False)).wrap_ok(),
                "null" => ConstantValue::Null(NullValue).wrap_ok(),
                _ => WithSpan::new(
                    ParseError::expected(
                        Expectation::ConstantValue,
                        Found::Token(NonBracketTokenKind::Identifier),
                    ),
                    span,
                ).wrap_err(),
            };
        }
        if let Some(group) = cursor.consume_group_if(BracketKind::Brace) {
            return ConstantValue::Object(ConstantObjectLiteral(
                group
                    .item
                    .children
                    .item
                    .parse_items(cursor.text(), parse_constant_object_entry)
                    // resolve-position-generic-slot.md: this map is gone.
                    .into_iter()
                    .map(WithSpan::<ConstantObjectEntrySlot>::from)
                    .collect(),
            )).wrap_ok();
        }
        cursor.expected(Expectation::ConstantValue).wrap_err()
    })
}
```

## Errors

```rust
// from crates/isograph_parser/src/parse_error.rs
pub enum ParseError {
    Expected(ExpectedFound),
    EmptyLiteral,
    MultipleDeclarations,
    IntegerDoesNotFitI64,
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

One global `Expectation`. An error is `WithSpan<ParseError>`. The span is the offending item, or empty at `end_span` where the missing item would go. `IntegerDoesNotFitI64` is the `parse::<i64>()` `Err` on an `IntegerLiteral` token.

`UnsupportedDeclarationType` is in parse-entrypoint.md and is removed by parse-pointers.md.

Errors are stored on the tree (`Both::errors`, `Failed::errors`, `IsoLiteralParse::errors`). Resolve walks leftover and failed items, not those diagnostics. `errors()` collects diagnostics in source order. Bracket errors are the matcher's vec. Comma-without-item errors are chunking's vec. An error-free literal has three empty lists. Artifact generation runs only then.

A failed list chunk is `Failed`. Leftover after a successful list item is `Both` plus `Expected(Separator, ...)`. Tokenless diagnostics (empty literal, a root comma) are `Vec<WithSpan<ParseError>>` with no `UnparsedChunkItems`. Extra root chunks are `ExtraChunks` plus `MultipleDeclarations`. `Display` formats `ParseError`. Suggestions are produced later from `(expected, found)`.

## Totality

`parse_iso_literal` returns a tree for every `&str`. A position in a parsed region resolves to a grammar leaf. A position in leftover or failed items resolves through `UnparsedChunkItems`. Extra root chunks resolve through `ExtraChunks`. A position on whitespace or a dropped comma or unmatched-bracket region resolves to the nearest containing node.

Find-references, rename, and go-to-definition run when the resolved leaf is a name leaf. In `foo { bar } asdf`, a position on `asdf` resolves through the leftover `UnparsedChunkItems`; find-references returns no references. Hover reads the leftover diagnostic by span. Completion reads the resolution path.

## Trees and spans

A tree enum is wrapped in `WithSpan` at its slot. The parsed item is `WithSpan` on `Complete` and on `Both.item`. Other variant payloads are bare. Each other struct field that is a node is `WithSpan`. A name is a fieldless marker struct in a `WithSpan`; each role is its own type. The name's text is the wrapper's span. The converted scalar is the `i64`. A position on `.`, `$`, `!`, or `to` resolves to the containing node.

`ResolvePosition` is derived. The one blanket delegation is `Box<T>` (parse-variables.md). A parent is a path alias at one parent, an enum at the second. Chunk-stage `IsographResolutionNode` variants resolve inside `UnparsedChunkItems` and `ExtraChunks`.

## Performance

One pass by reference. The output copies spans and `Copy` tokens. Cloning happens when leftover or failed items are stored. A later change can store a range into the original chunk instead. Each item is advanced past at most once. The functions take `&str` and the chunk tree.

## Catalog of parsing tasks

- Required token: `ItemCursor::require_token`
- Optional token: `ItemCursor::consume_token_if`
- Required group: `ItemCursor::require_group`
- Optional group: `ItemCursor::consume_group_if`
- Wrong or missing item: `ItemCursor::expected`
- Multi-form position: `consume_*` ladder, last arm `expected`
- Keyword / boolean / null text: `token_text` after an identifier
- Integer conversion: `token_text(span).parse()` on an `IntegerLiteral` span
- Composite span: `ItemCursor::spanning`
- List of items: `ChunkedLevel::parse_items` → `Vec<WithSpan<LevelSlot<P>>>`
- One-item context: `parse_singleton` → `Singleton<T>`
- Recovered item: `LevelSlot::item` / `IsoLiteralParse::item` → `Option<&T>`
- Chunk count: `ChunkedLevel::len`
- First item of an extra chunk: `Chunk::first_item`
- Trailing comma in a one-item context: `Singleton::errors` (tokenless)
- Leftover after a list item: `LevelSlot::Both` plus `Expected(Separator, ...)`
- Leftover after a singleton first chunk: `LevelSlot::Both` plus `Expected(EndOfDeclaration, ...)`
- Extra root chunks: `ExtraChunks` plus `MultipleDeclarations`
- Group interior: `require_group` / `consume_group_if`, then `parse_items` or `parse_singleton` on `group.children`
- Constant-only value: `parse_constant_value` → `ConstantValue`

## Shipping and amending

The first implementation step is the shared surface, with tests, before any grammar feature. That step lands:

- `ItemCursor` / `ChunkStream`: `new`, `cursor`, `require_end`, `consume_token_if`, `require_token`, `consume_group_if`, `require_group`, `expected`, `text`, `token_text`, `end_span`, `spanning`
- `Chunk::stream`, `Chunk::contents_span`, `Chunk::first_item`, `Chunk::boundary_comma`, `ChunkedLevel::len`
- `LevelSlot`, `Both`, `Failed`, `UnparsedChunkItems`, `ExtraChunks`, `Singleton`, `parse_chunk`, `parse_one_item`, `parse_items`, `parse_singleton`, `ChunkContentItemParent`
- `ParseError` / `Expectation` / `Found` as the error types those methods return

Tests assert facts about that surface: `require_*` / `consume_*` match and mismatch, `expected` names the next item or `EndOfChunk`, `require_end` is `Ok` only on an empty remainder, `spanning` covers what the closure advanced past, `parse_one_item` leftover is `Both` with leftover items, `parse_singleton` extra is `ExtraChunks`. No grammar tree, no `parse_iso_literal`.

Each grammar feature then lands on that surface.

- parse-entrypoint.md: `parse_iso_literal`, `parse_singleton` at the root, `entrypoint Type.field`
- parse-fields.md: `SelectionSlot`, `BothSelection`, `Failed`, `collect_selection_slot_errors`, `Clone` on leftover and failed items, `ChunkContentItemParent` variant `Unparsed`, `ChunkParent` variant `Extra`. resolve-position-generic-slot.md: the slot becomes `LevelSlot<Selection>`
- parse-arguments.md: `parse_value`, `IntegerDoesNotFitI64`, `BooleanValue(Boolean::{True, False})`
- parse-variables.md: `parse_type_annotation`, `parse_singleton` on `[...]`, `ConstantValue`, `parse_constant_value`, `Box<T>` delegation in `resolve_position`
- parse-descriptions.md: description via two `consume_token_if`
- parse-pointers.md: `to` via `require_token(Identifier)` and `token_text`

A feature is reviewed against this doc when it lands. Amendment sites: the `ItemCursor` and `ChunkStream` impls, `parse_chunk`, `parse_items`, and `parse_singleton`. This doc stays in `refactors/pending`.
