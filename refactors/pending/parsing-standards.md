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

`parse_chunk` calls `Chunk::stream`, then passes `stream.cursor()` (`&mut ItemCursor`) into the parse function. `parse_items` and `parse_singleton` then call `stream.require_end`. Their result type is `ParseResult<T>`: `Ok` is a complete item, `Err((Some(item), e))` is that item plus leftover (or, for `parse_singleton`, a boundary comma or a second chunk), `Err((None, e))` is a failed parse. Every parse is this recovered tree: keep the item when one exists, store the error on the tree. `item` reads that tree: `Ok(t)` and `Err((Some(t), _))` are `Some(t)`, `Err((None, _))` is `None`. Artifact generation requires the tree's `errors()` and the earlier-stage error lists to be empty. `require_end` is a method on `ChunkStream`.

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
- Parsed list item: the span `spanning` returned.
- `LevelSlot::Unparsed`: `contents_span`.
- Value made of several items: one `spanning` call. The closure's first advance is a `consume_*` or `require_*`. Remaining items of that value are read inside the same `spanning`.

`token_text` is `&self.text[span.as_usize_range()]`. A span that is not a range of that string panics, the same as any `&str` index. Names in the tree are spans. The converted scalar is the `i64`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(text: &str, root: WithSpan<ChunkedLevel>) -> WithSpan<IsoLiteralParse> {
    let location = root.location;
    let parse = match parse_singleton(
        root.reference(),
        text,
        || WithSpan::new(ParseError::EmptyLiteral, location),
        |extra| WithSpan::new(ParseError::MultipleDeclarations, extra.location),
        parse_iso_literal_item,
    ) {
        Ok(item) => IsoLiteralParse {
            item,
            trailing: None,
        },
        Err((Some(item), error)) => IsoLiteralParse {
            item,
            trailing: error.wrap_some(),
        },
        Err((None, reason)) => IsoLiteralParse {
            item: WithSpan::new(
                IsoLiteralItem::Unparsed(UnparsedLiteral { reason, level: root }),
                location,
            ),
            trailing: None,
        },
    };
    WithSpan::new(parse, location)
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

`Chunk`'s fields are private to the `chunk` module. `contents` is a `NonEmpty<WithSpan<ChunkContentItem>>` (chunk-contents-nonempty.md). `Chunk` is `pub`; `stream` is `pub(crate)`. `WithSpan<Chunk>` runs from the first content item through the trailing separator. `contents_span` stops at the last content item. An `Unparsed` slot uses `contents_span`, so a position on that chunk's comma resolves to the list, matching a comma after a `Parsed` slot.

## Lists and one-item levels

`ChunkedLevel`'s vec is private to the `chunk` module. `len` is the chunk count. `parse_items` is the list combinator (a selection set, an argument list, an object literal, a variable-declaration list): one chunk, one slot, leftover is `ParsedSlot::trailing`. `parse_singleton` is a one-item level (the root, a `[...]` interior): it parses the first chunk whenever one exists. Tests call `#[cfg(test)] ChunkedLevel::chunks`.

```rust
// from crates/isograph_parser/src/chunk.rs
use resolve_position::ResolvePosition;
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan};

use crate::{
    Chunk, ChunkStream, Expectation, Found, ItemCursor, NonBracketTokenKind, ParseError,
};

pub type ParseResult<T, E = WithSpan<ParseError>> = Result<T, (Option<T>, E)>;

/// One chunk's outcome in a list. The wrapping `WithSpan`'s span is the parsed
/// item's span, or the chunk's `contents_span` when unparsed.
/// Combinator result only; resolve-position-generic-slot.md derives this on the tree.
#[derive(Debug, PartialEq, Eq)]
pub enum LevelSlot<T> {
    Parsed(ParsedSlot<T>),
    Unparsed(UnparsedItem),
}

/// Combinator Ok payload, or `Err((Some(item), e))`: the item and leftover.
/// resolve-position-generic-slot.md.
#[derive(Debug, PartialEq, Eq)]
pub struct ParsedSlot<T> {
    pub item: T,
    pub trailing: Option<WithSpan<ParseError>>,
}

impl<T> LevelSlot<T> {
    pub fn item(&self) -> Option<&T> {
        match self {
            LevelSlot::Parsed(parsed) => (&parsed.item).wrap_some(),
            LevelSlot::Unparsed(_) => None,
        }
    }
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

fn unparsed_slot<P>(chunk: &WithSpan<Chunk>, reason: WithSpan<ParseError>) -> WithSpan<LevelSlot<P>> {
    WithSpan::new(
        LevelSlot::Unparsed(UnparsedItem {
            reason,
            chunk: chunk.clone(),
        }),
        chunk.item.contents_span(),
    )
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
                let (mut stream, result) = parse_chunk(chunk, text, parse_item.reference());
                match result {
                    Ok(item) => {
                        let trailing = stream
                            .require_end()
                            .err()
                            .map(|()| stream.cursor().expected(Expectation::Separator));
                        WithSpan::new(
                            LevelSlot::Parsed(ParsedSlot {
                                item: item.item,
                                trailing,
                            }),
                            item.location,
                        )
                    }
                    Err(reason) => unparsed_slot(chunk, reason),
                }
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
) -> ParseResult<WithSpan<T>> {
    if level.item.len() == 0 {
        return (None, empty()).wrap_err();
    }
    let chunk = &level.item.0[0];
    let (mut stream, result) = parse_chunk(chunk, text, parse);
    let item = match result {
        Ok(item) => item,
        Err(reason) => return (None, reason).wrap_err(),
    };
    if stream.require_end().is_err() {
        return (
            item.wrap_some(),
            stream.cursor().expected(Expectation::EndOfDeclaration),
        )
            .wrap_err();
    }
    if let Some(comma) = chunk.item.boundary_comma() {
        return (
            item.wrap_some(),
            WithSpan::new(
                ParseError::expected(
                    Expectation::EndOfDeclaration,
                    Found::Token(NonBracketTokenKind::Comma),
                ),
                comma,
            ),
        )
            .wrap_err();
    }
    if level.item.len() > 1 {
        return (item.wrap_some(), extra(&level.item.0[1])).wrap_err();
    }
    item.wrap_ok()
}

fn item<T, E>(result: ParseResult<T, E>) -> Option<T> {
    match result {
        Ok(item) => item.wrap_some(),
        Err((item, _)) => item,
    }
}
```

`parse_items` is one chunk, one slot: `parse_chunk`, then `require_end`. Length equals chunk count. `Err` from the parse function is `LevelSlot::Unparsed`. Leftover is `Ok` plus `require_end` `Err`: `ParsedSlot::trailing` via `expected(Separator)`, item kept. `foo { bar } asdf` is the object selection `foo { bar }` (span on that) and a trailing error at `asdf`. A position on `asdf` resolves to the selection set.

`parse_singleton` parses the first chunk whenever `len() >= 1` and returns `ParseResult<WithSpan<T>>`. Empty is `Err((None, empty()))`. A failed first item is `Err((None, reason))`; extra is not reported. A successful first item plus leftover, a boundary comma, or a second chunk is `Err((Some(item), e))`. Leftover and the comma use `Expectation::EndOfDeclaration`. Extra is the `extra` callback. There is no mode that skips the first chunk when extra exists. The index into `.0` is in this module. `item(result)` is `Some` whenever a first item parsed.

`LevelSlot` is the combinator's result. It does not implement `ResolvePosition`. Each list stores a concrete slot enum that derives. resolve-position-generic-slot.md puts `LevelSlot<T>` on the tree instead.

### Concrete slots

```rust
// from crates/isograph_parser/src/selections.rs
/// Derived stand-in for `LevelSlot<Selection>`. resolve-position-generic-slot.md.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSetPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum SelectionSlot {
    Parsed(ParsedSelection),
    Unparsed(#[resolve_field(parent_variant = SelectionSet)] UnparsedItem),
}

// resolve-position-generic-slot.md: this is ParsedSlot<Selection>; not a path segment or ResolvedNode variant.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSetPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ParsedSelection {
    #[resolve_field]
    pub item: WithSpan<Selection>,
    pub trailing: Option<WithSpan<ParseError>>,
}

// resolve-position-generic-slot.md: the field is Vec<WithSpan<LevelSlot<Selection>>>.
pub struct SelectionSet(#[resolve_field] pub Vec<WithSpan<SelectionSlot>>);

// resolve-position-generic-slot.md: parent is SelectionSetPath.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ParsedSelectionPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum Selection {
    Scalar(ScalarSelection),
    Object(ObjectSelection),
}
```

`ArgumentSlot` / `ParsedArgument`, `ObjectEntrySlot` / `ParsedObjectEntry`, `ConstantObjectEntrySlot` / `ParsedConstantObjectEntry`, and `VariableDeclarationSlot` / `ParsedVariableDeclaration` are the same shape, each with that list's path as `parent_type` and `parent_variant` on `Unparsed`. resolve-position-generic-slot.md.

```rust
// from crates/isograph_parser/src/selections.rs
// resolve-position-generic-slot.md: the list field is LevelSlot<Selection>.
impl From<WithSpan<LevelSlot<Selection>>> for WithSpan<SelectionSlot> {
    fn from(slot: WithSpan<LevelSlot<Selection>>) -> Self {
        let location = slot.location;
        let item = match slot.item {
            LevelSlot::Parsed(ParsedSlot { item, trailing }) => {
                SelectionSlot::Parsed(ParsedSelection {
                    item: WithSpan::new(item, location),
                    trailing,
                })
            }
            LevelSlot::Unparsed(unparsed) => SelectionSlot::Unparsed(unparsed),
        };
        WithSpan::new(item, location)
    }
}
```

The same `From` exists per list. A list site maps the combinator output:

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

`UnparsedItemParent` is the parent of `UnparsedItem`. The derive's `parent_variant` wraps the list path. There is no `From` into `UnparsedItemParent`.

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
            SelectionSlot::Parsed(parsed) => {
                nested(parsed.item.item.reference(), errors);
                if let Some(trailing) = parsed.trailing {
                    errors.push(trailing);
                }
            }
            SelectionSlot::Unparsed(unparsed) => errors.push(unparsed.reason),
        }
    }
}
```

Nested errors (arguments, nested selections) precede that slot's trailing error, matching source order.

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

`parse_items` wraps `parse_selection` in `spanning` and then calls `require_end`. Leftover is `ParsedSlot::trailing`.

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

Errors are stored on the tree (`UnparsedLiteral`, `UnparsedItem`, `IsoLiteralParse::trailing`, `ParsedSelection::trailing` and the other `Parsed*` trailing fields). resolve-position-generic-slot.md: those list trailing fields are `ParsedSlot::trailing`. `errors()` collects them in source order. Bracket errors are the matcher's vec. Comma-without-item errors are chunking's vec. An error-free literal has three empty lists. Artifact generation runs only then.

A failed list chunk is the concrete slot's `Unparsed` variant; sibling chunks are parsed. Leftover after a successful list item is `ParsedSlot::trailing`. A failed first declaration is `UnparsedLiteral`. Leftover, a boundary comma, and a second chunk after a successful first item are `IsoLiteralParse::trailing`. `Display` formats `ParseError`. Suggestions are produced later from `(expected, found)`.

## Totality

`parse_iso_literal` returns a tree for every `&str`. A position in a parsed region resolves to a grammar leaf. A position in `UnparsedLiteral` or `UnparsedItem` resolves through the retained chunk. A position on whitespace or a dropped comma or unmatched-bracket region resolves to the nearest containing node.

Find-references, rename, and go-to-definition run when the resolved leaf is a name leaf. In `foo { bar } asdf`, a position on `asdf` resolves to the selection set; find-references returns no references. Hover and completion read the resolution path.

## Trees and spans

A tree enum is wrapped in `WithSpan` at its slot. Variant payloads are bare. Each struct field that is a node is `WithSpan`. A name is a fieldless marker struct in a `WithSpan`; each role is its own type. The name's text is the wrapper's span. The converted scalar is the `i64`. A position on `.`, `$`, `!`, or `to` resolves to the containing node.

`ResolvePosition` is derived. The one blanket delegation is `Box<T>` (parse-variables.md). A parent is a path alias at one parent, an enum at the second. Chunk-stage `IsographResolutionNode` variants resolve inside `UnparsedLiteral` and `UnparsedItem`.

## Performance

One pass by reference. The output copies spans and `Copy` tokens. Cloning happens when a region becomes `UnparsedItem`: allocation beyond the output vecs is proportional to the error count. Each item is advanced past at most once. The functions take `&str` and the chunk tree.

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
- One-item context: `parse_singleton` → `ParseResult<WithSpan<T>>`
- Recovered item: `item(result)` → `Option<T>`; `LevelSlot::item` / `IsoLiteralParse::item` are the same read on the tree
- Chunk count: `ChunkedLevel::len`
- First item of an extra chunk: `Chunk::first_item`
- Trailing comma in a one-item context: `parse_singleton` via `boundary_comma` on `Err((Some(item), e))`
- Leftover after a list item: `ParsedSlot::trailing`
- Leftover after a singleton: `Err((Some(item), e))` from `parse_singleton`
- Group interior: `require_group` / `consume_group_if`, then `parse_items` or `parse_singleton` on `group.children`
- Constant-only value: `parse_constant_value` → `ConstantValue`

## Shipping and amending

The first implementation step is the shared surface, with tests, before any grammar feature. That step lands:

- `ItemCursor` / `ChunkStream`: `new`, `cursor`, `require_end`, `consume_token_if`, `require_token`, `consume_group_if`, `require_group`, `expected`, `text`, `token_text`, `end_span`, `spanning`
- `Chunk::stream`, `Chunk::contents_span`, `Chunk::first_item`, `Chunk::boundary_comma`, `ChunkedLevel::len`
- `LevelSlot`, `ParsedSlot`, `ParseResult`, `parse_chunk`, `parse_items`, `parse_singleton`, `item`
- `ParseError` / `Expectation` / `Found` as the error types those methods return

Tests assert facts about that surface: `require_*` / `consume_*` match and mismatch, `expected` names the next item or `EndOfChunk`, `require_end` is `Ok` only on an empty remainder, `spanning` covers what the closure advanced past, `parse_items` leftover is `ParsedSlot::trailing`, `parse_singleton` `Err((Some, e))` keeps the item. No grammar tree, no `parse_iso_literal`.

Each grammar feature then lands on that surface.

- parse-entrypoint.md: `parse_iso_literal`, `parse_singleton` at the root, `UnparsedLiteral`, `entrypoint Type.field`
- parse-fields.md: `SelectionSlot`, `ParsedSelection`, `UnparsedItem`, `collect_selection_slot_errors`, `Clone` on the chunk tree, `ChunkParent::UnparsedItem`. resolve-position-generic-slot.md: the slot, wrapper, and per-list collect become `LevelSlot<Selection>` / `ParsedSlot<Selection>` / one walk
- parse-arguments.md: `parse_value`, `IntegerDoesNotFitI64`, `BooleanValue(Boolean::{True, False})`
- parse-variables.md: `parse_type_annotation`, `parse_singleton` on `[...]`, `ConstantValue`, `parse_constant_value`, `Box<T>` delegation in `resolve_position`
- parse-descriptions.md: description via two `consume_token_if`
- parse-pointers.md: `to` via `require_token(Identifier)` and `token_text`

A feature is reviewed against this doc when it lands. Amendment sites: the `ItemCursor` and `ChunkStream` impls, `parse_chunk`, `parse_items`, and `parse_singleton`. This doc stays in `refactors/pending`.
