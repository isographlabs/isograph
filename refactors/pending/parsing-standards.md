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

Each token and group has a span. A parse function assigns a span to a value made of more than one item by calling `spanning`. `take_next` returning `None` means the chunk has no remaining item.

## `ItemCursor` and `ChunkStream`

`parse_items` and `parse_singleton` call `Chunk::stream`, then pass `stream.cursor()` (`&mut ItemCursor`) into the parse function, then call `stream.require_end`. `require_end` is a method on `ChunkStream`.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
use nonempty::NonEmpty;
use safe_peekable::{IntoSafePeekable, SafePeekable};
use span::{Span, WithSpan};

use crate::{
    BracketKind, ChunkContentItem, ChunkedGroup, Expectation, Found, NonBracketTokenKind, ParseError,
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
                    && self.token_text(item.location) == keyword =>
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

    pub(crate) fn text(&self) -> &'a str {
        self.text
    }

    pub(crate) fn take_next(&mut self) -> Option<&'a WithSpan<ChunkContentItem>> {
        let item = self.items.next()?;
        self.previous_end = item.location.end;
        Some(item)
    }

    pub(crate) fn end_span(&self) -> Span {
        Span::new(self.previous_end, self.previous_end)
    }

    pub(crate) fn token_text(&self, span: Span) -> &'a str {
        &self.text[span.as_usize_range()]
    }

    pub(crate) fn integer(&self, span: Span) -> Result<i64, WithSpan<ParseError>> {
        match self.token_text(span).parse() {
            Ok(value) => Ok(value),
            Err(_) => Err(WithSpan::new(ParseError::IntegerOutOfRange, span)),
        }
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

`nonempty::Iter` yields `&'a WithSpan<ChunkContentItem>`. `require_token` and `consume_token_if` copy that reference out of `view` and then `commit`. `consume_group_if` and `require_group` rematch after `commit` so the `&'a ChunkedGroup` is borrowed from the committed reference.

### Span sources

- Leaf: the `Span` from `require_token`, `consume_token_if`, `consume_token_if_any`, `require_keyword`, or the `WithSpan` from `consume_group_if`, `require_group`, `take_next`.
- Parsed list item: the span `spanning` returned.
- `LevelSlot::Unparsed`: `contents_span`.
- Value made of several items: one `spanning` call. The closure calls `take_next` or the first `require_*`. If the caller already advanced past the first item, those remaining items are read with `require_*` / `consume_*`; if they must share one span with the first item, they are all read inside that first `spanning`.

`token_text` is `&self.text[span.as_usize_range()]`. A span that is not a range of that string panics, the same as any `&str` index. Names in the tree are spans. The converted scalar is the `i64`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(text: &str, root: WithSpan<ChunkedLevel>) -> WithSpan<IsoLiteralParse> {
    let location = root.location;
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
    pub(crate) fn stream<'a>(&'a self, text: &'a str) -> ChunkStream<'a> {
        ChunkStream::new(&self.contents, text)
    }

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

`Chunk`'s fields are private to the `chunk` module. `contents` is a `NonEmpty<WithSpan<ChunkContentItem>>` (chunk-contents-nonempty.md). `Chunk` is `pub`; `stream` is `pub(crate)`.

## Lists and one-item levels

`ChunkedLevel`'s vec is private to the `chunk` module. `parse_items` iterates a list level (a selection set, an argument list, an object literal, a variable-declaration list). `parse_singleton` iterates a one-item level (the root, a `[...]` interior). Tests call `#[cfg(test)] ChunkedLevel::chunks`.

```rust
// from crates/isograph_parser/src/chunk.rs
use resolve_position::ResolvePosition;
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan};

use crate::{
    Chunk, ChunkStream, Expectation, Found, ItemCursor, NonBracketTokenKind, ParseError,
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
    pub(crate) fn parse_items<'a, P>(
        &'a self,
        text: &'a str,
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

pub(crate) fn parse_singleton<'a, T>(
    level: &'a WithSpan<ChunkedLevel>,
    text: &'a str,
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

`parse_items` returns `Vec<WithSpan<LevelSlot<P>>>`. Length equals chunk count. `Err` from the parse function is `LevelSlot::Unparsed`. `Ok` plus `require_end` `Err` is `ParsedSlot::trailing`. `foo bar` is the selection `foo` (span on `foo`) and a trailing error at `bar`. A position on `bar` resolves to the selection set. `foo bar { baz }` is the scalar `foo` and leftover from `bar` on.

`parse_singleton` returns the `require_end` `Err` (the declaration or `[...]` type is dropped). The comma uses the same `end_expectation`.

### `LevelSlot` and `ResolvePosition`

A position in `Parsed` is resolved by `T::resolve`. A position in `Unparsed` is resolved by `UnparsedItem::resolve`. `trailing` has no `#[resolve_field]`.

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

```rust
// from crates/isograph_parser/src/selections.rs
pub struct SelectionSet(#[resolve_field] pub Vec<WithSpan<LevelSlot<Selection>>>);

pub enum Selection {
    Scalar(ScalarSelection),
    Object(ObjectSelection),
}
```

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

- `require_*`: `ItemCursor` method. Match: `commit` and return the item. Mismatch: `Err` on the next item, no `commit`. Empty: `Err` at `end_span`.
- `consume_*`: `ItemCursor` method. Match: `commit` and `Some`. Else: `None`.
- `parse_*`: implements a form made of several items. Parameter is `&mut ItemCursor`. First `Err` is returned. Shared iteration is `parse_items`, `parse_singleton`, or `spanning`.

A group plus its interior:

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

## Dispatch

When the next item may start several forms, the parse function calls `take_next()` and `match`es. If those arms are one value, the `match` is inside `spanning`. Arms that return `Ok` continue with `require_*` / `consume_*`. The `_` arm returns `Err` with that item as `found`.

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

Keyword text after `require_token(Identifier, ...)`: `match` on `token_text` (`"entrypoint"` / `"field"` / `"pointer"`; `"true"` / `"false"` / `"null"`). One required keyword (`to`) is `require_keyword`.

One optional item is `consume_*`. Several kinds for one optional item is `consume_token_if_any`. The optional `!` after a type name is `consume_token_if(Exclamation)`: the next item may be the caller's `=`. A form that starts on its first item and is then required (`$name`) is a `take_next` arm; the rest is `require_*`. After `require_token` on an identifier, `consume_token_if(Colon)` is the alias; both arms use the identifier.

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

`parse_items` wraps `parse_selection` in `spanning` and then calls `require_end`.

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

One global `Expectation`. An error is `WithSpan<ParseError>`. The span is the offending item, or empty at `end_span` where the missing item would go.

`UnsupportedDeclarationType` is in parse-entrypoint.md and is removed by parse-pointers.md.

Errors are stored on the tree (`UnparsedLiteral`, `UnparsedItem`, `ParsedSlot::trailing`). `errors()` collects them in source order. Bracket errors are the matcher's vec. Comma-without-item errors are chunking's vec. An error-free literal has three empty lists.

A failed list chunk is `LevelSlot::Unparsed`; sibling chunks are parsed. A failed declaration is `UnparsedLiteral`. One reason per those regions. `Display` formats `ParseError`. Suggestions are produced later from `(expected, found)`.

## Totality

`parse_iso_literal` returns a tree for every `&str`. A position in a parsed region resolves to a grammar leaf. A position in `UnparsedLiteral` or `UnparsedItem` resolves through the retained chunk. A position on whitespace, leftover, or a dropped comma or unmatched-bracket region resolves to the nearest containing node.

Find-references, rename, and go-to-definition run when the resolved leaf is a name leaf. In `foo { bar baz }`, a position on `baz` resolves to the selection set; find-references returns no references. Hover and completion read the resolution path.

## Trees and spans

A tree enum is wrapped in `WithSpan` at its slot. Variant payloads are bare. Each struct field that is a node is `WithSpan`. A name is a fieldless marker struct in a `WithSpan`; each role is its own type. The name's text is the wrapper's span. The converted scalar is the `i64`. A position on `.`, `$`, `!`, or `to` resolves to the containing node.

`ResolvePosition` is derived. The two blanket delegations are `Box<T>` (parse-variables.md) and `LevelSlot<T>` above. A parent is a path alias at one parent, an enum at the second. Chunk-stage `IsographResolutionNode` variants resolve inside `UnparsedLiteral` and `UnparsedItem`.

## Performance

One pass by reference. The output copies spans and `Copy` tokens. Cloning happens when a region becomes `UnparsedItem`: allocation beyond the output vecs is proportional to the error count. Each item is advanced past at most once. The functions take `&str` and the chunk tree.

## Catalog of parsing tasks

- Required token: `ItemCursor::require_token`
- Optional token: `ItemCursor::consume_token_if`
- Optional token, several kinds: `ItemCursor::consume_token_if_any`
- Required keyword: `ItemCursor::require_keyword`
- Required group: `ItemCursor::require_group`
- Optional group: `ItemCursor::consume_group_if`
- Multi-form position: `take_next` inside `spanning`
- Keyword / boolean / null text: `token_text` after an identifier
- Integer conversion: `ItemCursor::integer`
- Composite span: `ItemCursor::spanning`
- Missing-item error span: `ItemCursor::end_span`
- List of items: `ChunkedLevel::parse_items` → `Vec<WithSpan<LevelSlot<P>>>`
- One-item context: `parse_singleton`
- First item of an extra chunk: `Chunk::first_item`
- Trailing comma in a one-item context: `parse_singleton` via `boundary_comma`
- Leftover after a list item: `ParsedSlot::trailing`
- Leftover after a singleton: `parse_singleton`'s `require_end`
- Group interior: `require_group` / `consume_group_if`, then `parse_items` or `parse_singleton` on `group.children`
- Constant-only value: `parse_constant_value` → `ConstantValue`

## Shipping and amending

Each method lands with the feature doc of its first caller.

- parse-entrypoint.md: `ItemCursor`, `ChunkStream`, `Chunk::stream`, `require_token`, `require_end`, `token_text`, `end_span`, `parse_singleton`, `boundary_comma`
- parse-fields.md: `consume_token_if`, `consume_group_if`, `require_group`, `spanning` (via `parse_items`), `contents_span`, `LevelSlot`, `ParsedSlot`, `UnparsedItem`, `parse_items`, `collect_slot_errors`, `Clone` on the chunk tree, `ChunkParent::UnparsedItem`
- parse-arguments.md: `take_next`, `spanning` around `parse_value`, `integer`, `BooleanValue(Boolean::{True, False})`
- parse-variables.md: `parse_singleton` on `[...]`, `Chunk::first_item`, `ConstantValue`, `parse_constant_value`, `Box<T>` delegation in `resolve_position`
- parse-descriptions.md: `consume_token_if_any`
- parse-pointers.md: `require_keyword`

A feature is reviewed against this doc when it lands. Amendment sites: the `ItemCursor` and `ChunkStream` impls, `parse_items`, and `parse_singleton`. This doc stays in `refactors/pending`.
