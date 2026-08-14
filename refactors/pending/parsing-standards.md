# Parsing standards

Rules for grammar-stage code. Feature docs define each form (a selection, a value, a type) and the items that make it up. This doc lists the functions that implement those forms. If an implementation disagrees with this doc, the same review amends the doc or changes the code.

Every call a parse function makes on a chunk is a method or free function listed here. A new call is a new listing here.

## The unit of work

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(text: &str, root: WithSpan<ChunkedLevel>) -> WithSpan<IsoLiteralParse>
```

`parse_iso_literal` takes `text: &str` and the chunked literal. Each chunk is passed to `Chunk::stream(text)`, which returns one `ChunkStream`. A group's interior is the `ChunkedLevel` in `group.children`. A `ChunkStream` is built from one chunk.

A group is one item. `require_group` and `consume_group_if` return it in one call. The interior is parsed by calling `parse_items_with_trailing` or `parse_singleton` on `group.children`.

Each token and group has a span. A parse function assigns a span to a value made of more than one item by calling `spanning`. `expected` on an exhausted cursor is `Expected(_, EndOfChunk)` at `end_span`.

## `ItemCursor` and `ChunkStream`

`parse_chunk` calls `Chunk::stream`, then passes `stream.cursor()` (`&mut ItemCursor`) into the parse function. `parse_items_with_trailing` and `parse_singleton` then call `stream.require_end`. `require_end` is a method on `ChunkStream`.

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
        match self.cursor.items.peek() {
            None => Ok(()),
            Some(_) => Err(()),
        }
    }
}

impl<'a> ItemCursor<'a> {
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

    pub(crate) fn consume_group_if(
        &mut self,
        kind: BracketKind,
    ) -> Option<WithSpan<&'a ChunkedGroup>> {
        let peek = self.items.peek()?;
        let item = *peek.view();
        match &item.item {
            ChunkContentItem::Group(group) if group.opening.item.0 == kind => {
                let location = item.location;
                peek.commit();
                self.previous_end = location.end;
                Some(WithSpan::new(group, location))
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
                    ParseError::expected(expected, Found::from(&item.item)),
                    item.location,
                )
            }
        }
    }

    pub(crate) fn require_token(
        &mut self,
        kind: NonBracketTokenKind,
        expected: Expectation,
    ) -> Result<Span, WithSpan<ParseError>> {
        match self.consume_token_if(kind) {
            Some(span) => Ok(span),
            None => Err(self.expected(expected)),
        }
    }

    pub(crate) fn require_group(
        &mut self,
        kind: BracketKind,
        expected: Expectation,
    ) -> Result<WithSpan<&'a ChunkedGroup>, WithSpan<ParseError>> {
        match self.consume_group_if(kind) {
            Some(group) => Ok(group),
            None => Err(self.expected(expected)),
        }
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
        Ok(WithSpan::new(value, span))
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
    match parse_singleton(
        &root,
        text,
        || WithSpan::new(ParseError::EmptyLiteral, location),
        |extra| WithSpan::new(ParseError::MultipleDeclarations, extra.location),
        parse_declaration,
        Expectation::EndOfDeclaration,
    ) {
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

`ChunkedLevel`'s vec is private to the `chunk` module. `len` is the chunk count. `parse_items` maps each chunk to a slot. `parse_items_with_trailing` is the list combinator (a selection set, an argument list, an object literal, a variable-declaration list). `parse_singleton` is a one-item level (the root, a `[...]` interior): it matches `len` before it parses. Tests call `#[cfg(test)] ChunkedLevel::chunks`.

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
            .map(|chunk| match parse_chunk(chunk, text, &parse_item) {
                (_, Ok(item)) => WithSpan::new(
                    LevelSlot::Parsed(ParsedSlot {
                        item: item.item,
                        trailing: None,
                    }),
                    item.location,
                ),
                (_, Err(reason)) => unparsed_slot(chunk, reason),
            })
            .collect()
    }

    pub(crate) fn parse_items_with_trailing<'a, P>(
        &'a self,
        text: &'a str,
        parse_item: impl Fn(&mut ItemCursor<'a>) -> Result<P, WithSpan<ParseError>>,
    ) -> Vec<WithSpan<LevelSlot<P>>> {
        self.0
            .iter()
            .map(|chunk| {
                let (mut stream, result) = parse_chunk(chunk, text, &parse_item);
                match result {
                    Ok(item) => {
                        let trailing = match stream.require_end() {
                            Ok(()) => None,
                            Err(()) => Some(stream.cursor().expected(Expectation::Separator)),
                        };
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
    match level.item.len() {
        0 => Err(empty()),
        1 => {
            let chunk = &level.item.0[0];
            let mut stream = chunk.item.stream(text);
            let item = parse(stream.cursor())?;
            if stream.require_end().is_err() {
                return Err(stream.cursor().expected(end_expectation));
            }
            if let Some(comma) = chunk.item.boundary_comma() {
                return Err(WithSpan::new(
                    ParseError::expected(
                        end_expectation,
                        Found::Token(NonBracketTokenKind::Comma),
                    ),
                    comma,
                ));
            }
            Ok(item)
        }
        _ => Err(extra(&level.item.0[1])),
    }
}
```

`parse_items` is one chunk, one slot: `spanning` around the parse function, leftover ignored, `trailing` is `None`. Length equals chunk count. `Err` from the parse function is `LevelSlot::Unparsed`.

`parse_items_with_trailing` is `parse_items` plus `require_end` on each chunk. `Ok` plus `require_end` `Err` is `ParsedSlot::trailing`. List sites call this one. `foo bar` is the selection `foo` (span on `foo`) and a trailing error at `bar`. A position on `bar` resolves to the selection set. `foo bar { baz }` is the scalar `foo` and leftover from `bar` on.

`parse_singleton` matches `len()` first. Empty is `empty()`. Two or more is `extra` on the second chunk; the first is not parsed. One chunk is `parse`, then `require_end`, then `boundary_comma`. The comma uses the same `end_expectation`. The index into `.0` is in this module.

`LevelSlot` is the combinator's result. It does not implement `ResolvePosition`. Each list stores a concrete slot enum that derives.

### Concrete slots

```rust
// from crates/isograph_parser/src/selections.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSetPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum SelectionSlot {
    Parsed(ParsedSelection),
    Unparsed(#[resolve_field(parent_variant = SelectionSet)] UnparsedItem),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSetPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ParsedSelection {
    #[resolve_field]
    pub item: WithSpan<Selection>,
    pub trailing: Option<WithSpan<ParseError>>,
}

pub struct SelectionSet(#[resolve_field] pub Vec<WithSpan<SelectionSlot>>);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ParsedSelectionPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum Selection {
    Scalar(ScalarSelection),
    Object(ObjectSelection),
}
```

`ArgumentSlot` / `ParsedArgument`, `ObjectEntrySlot` / `ParsedObjectEntry`, `ConstantObjectEntrySlot` / `ParsedConstantObjectEntry`, and `VariableDeclarationSlot` / `ParsedVariableDeclaration` are the same shape, each with that list's path as `parent_type` and `parent_variant` on `Unparsed`.

```rust
// from crates/isograph_parser/src/selections.rs
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
                .parse_items_with_trailing(cursor.text(), parse_selection)
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
pub(crate) fn collect_selection_slot_errors(
    slots: &[WithSpan<SelectionSlot>],
    nested: impl Fn(&Selection, &mut Vec<WithSpan<ParseError>>),
    errors: &mut Vec<WithSpan<ParseError>>,
) {
    for slot in slots {
        match &slot.item {
            SelectionSlot::Parsed(parsed) => {
                nested(&parsed.item.item, errors);
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
- `require_*`: `consume_*` or `expected()`.
- `parse_*`: implements a form made of several items. Parameter is `&mut ItemCursor`. First `Err` is returned. Shared iteration is `parse_items`, `parse_items_with_trailing`, `parse_singleton`, or `spanning`.

A group plus its interior:

```rust
// from crates/isograph_parser/src/selections.rs
pub(crate) fn consume_selection_set(
    cursor: &mut ItemCursor<'_>,
) -> Option<WithSpan<SelectionSet>> {
    let group = cursor.consume_group_if(BracketKind::Brace)?;
    Some(WithSpan::new(
        SelectionSet(
            group
                .item
                .children
                .item
                .parse_items_with_trailing(cursor.text(), parse_selection)
                .into_iter()
                .map(WithSpan::<SelectionSlot>::from)
                .collect(),
        ),
        group.location,
    ))
}

pub(crate) fn require_selection_set(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<SelectionSet>, WithSpan<ParseError>> {
    let group = cursor.require_group(BracketKind::Brace, Expectation::SelectionSet)?;
    Ok(WithSpan::new(
        SelectionSet(
            group
                .item
                .children
                .item
                .parse_items_with_trailing(cursor.text(), parse_selection)
                .into_iter()
                .map(WithSpan::<SelectionSlot>::from)
                .collect(),
        ),
        group.location,
    ))
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
            let name = cursor.require_token(
                NonBracketTokenKind::Identifier,
                Expectation::Token(NonBracketTokenKind::Identifier),
            )?;
            return Ok(NonConstantValue::Variable(VariableUse {
                dollar: WithSpan::new(Dollar, dollar),
                name: WithSpan::new(VariableName, name),
            }));
        }
        if cursor
            .consume_token_if(NonBracketTokenKind::StringLiteral)
            .is_some()
        {
            return Ok(NonConstantValue::String(StringValue));
        }
        if let Some(span) = cursor.consume_token_if(NonBracketTokenKind::IntegerLiteral) {
            let value = match cursor.token_text(span).parse() {
                Ok(value) => value,
                Err(_) => {
                    return Err(WithSpan::new(ParseError::IntegerDoesNotFitI64, span));
                }
            };
            return Ok(NonConstantValue::Integer(IntegerValue(value)));
        }
        if let Some(span) = cursor.consume_token_if(NonBracketTokenKind::Identifier) {
            return match cursor.token_text(span) {
                "true" => Ok(NonConstantValue::Boolean(BooleanValue(Boolean::True))),
                "false" => Ok(NonConstantValue::Boolean(BooleanValue(Boolean::False))),
                "null" => Ok(NonConstantValue::Null(NullValue)),
                _ => Err(WithSpan::new(
                    ParseError::expected(
                        Expectation::Value,
                        Found::Token(NonBracketTokenKind::Identifier),
                    ),
                    span,
                )),
            };
        }
        if let Some(group) = cursor.consume_group_if(BracketKind::Brace) {
            return Ok(NonConstantValue::Object(ObjectLiteral(
                group
                    .item
                    .children
                    .item
                    .parse_items_with_trailing(cursor.text(), parse_object_entry)
                    .into_iter()
                    .map(WithSpan::<ObjectEntrySlot>::from)
                    .collect(),
            )));
        }
        Err(cursor.expected(Expectation::Value))
    })
}
```

Keyword text after `require_token(Identifier, ...)` or `consume_token_if(Identifier)`: `match` on `token_text` (`"entrypoint"` / `"field"` / `"pointer"`; `"true"` / `"false"` / `"null"`; `"to"`).

One optional item is `consume_*`. Two optional kinds in one position is two `consume_token_if` calls. The optional `!` after a type name is `consume_token_if(Exclamation)`: the next item may be the caller's `=`. `$name` is `consume_token_if(Dollar)` then `require_token(Identifier)`. After `require_token` on an identifier, `consume_token_if(Colon)` is the alias; both arms use the identifier.

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

`parse_items_with_trailing` wraps `parse_selection` in `spanning` and then calls `require_end`.

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
            return Ok(ConstantValue::String(StringValue));
        }
        if let Some(span) = cursor.consume_token_if(NonBracketTokenKind::IntegerLiteral) {
            let value = match cursor.token_text(span).parse() {
                Ok(value) => value,
                Err(_) => {
                    return Err(WithSpan::new(ParseError::IntegerDoesNotFitI64, span));
                }
            };
            return Ok(ConstantValue::Integer(IntegerValue(value)));
        }
        if let Some(span) = cursor.consume_token_if(NonBracketTokenKind::Identifier) {
            return match cursor.token_text(span) {
                "true" => Ok(ConstantValue::Boolean(BooleanValue(Boolean::True))),
                "false" => Ok(ConstantValue::Boolean(BooleanValue(Boolean::False))),
                "null" => Ok(ConstantValue::Null(NullValue)),
                _ => Err(WithSpan::new(
                    ParseError::expected(
                        Expectation::ConstantValue,
                        Found::Token(NonBracketTokenKind::Identifier),
                    ),
                    span,
                )),
            };
        }
        if let Some(group) = cursor.consume_group_if(BracketKind::Brace) {
            return Ok(ConstantValue::Object(ConstantObjectLiteral(
                group
                    .item
                    .children
                    .item
                    .parse_items_with_trailing(cursor.text(), parse_constant_object_entry)
                    .into_iter()
                    .map(WithSpan::<ConstantObjectEntrySlot>::from)
                    .collect(),
            )));
        }
        Err(cursor.expected(Expectation::ConstantValue))
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

Errors are stored on the tree (`UnparsedLiteral`, `UnparsedItem`, `ParsedSelection::trailing` and the other `Parsed*` trailing fields). `errors()` collects them in source order. Bracket errors are the matcher's vec. Comma-without-item errors are chunking's vec. An error-free literal has three empty lists.

A failed list chunk is the concrete slot's `Unparsed` variant; sibling chunks are parsed. A failed declaration is `UnparsedLiteral`. One reason per those regions. `Display` formats `ParseError`. Suggestions are produced later from `(expected, found)`.

## Totality

`parse_iso_literal` returns a tree for every `&str`. A position in a parsed region resolves to a grammar leaf. A position in `UnparsedLiteral` or `UnparsedItem` resolves through the retained chunk. A position on whitespace, leftover, or a dropped comma or unmatched-bracket region resolves to the nearest containing node.

Find-references, rename, and go-to-definition run when the resolved leaf is a name leaf. In `foo { bar baz }`, a position on `baz` resolves to the selection set; find-references returns no references. Hover and completion read the resolution path.

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
- List of items, leftover as trailing: `ChunkedLevel::parse_items_with_trailing`
- One-item context: `parse_singleton`
- Chunk count: `ChunkedLevel::len`
- First item of an extra chunk: `Chunk::first_item`
- Trailing comma in a one-item context: `parse_singleton` via `boundary_comma`
- Leftover after a list item: `ParsedSelection::trailing` (and the other `Parsed*` trailing fields)
- Leftover after a singleton: `parse_singleton`'s `require_end`
- Group interior: `require_group` / `consume_group_if`, then `parse_items_with_trailing` or `parse_singleton` on `group.children`
- Constant-only value: `parse_constant_value` → `ConstantValue`

## Shipping and amending

Each method lands with the feature doc of its first caller.

- parse-entrypoint.md: `ItemCursor`, `ChunkStream`, `Chunk::stream`, `consume_token_if`, `require_token`, `expected`, `require_end`, `text`, `token_text`, `parse_singleton`, `boundary_comma`, `ChunkedLevel::len`
- parse-fields.md: `consume_group_if`, `require_group`, `spanning` (via `parse_chunk`), `contents_span`, `LevelSlot`, `ParsedSlot`, `SelectionSlot`, `ParsedSelection`, `UnparsedItem`, `parse_chunk`, `parse_items`, `parse_items_with_trailing`, `collect_selection_slot_errors`, `Clone` on the chunk tree, `ChunkParent::UnparsedItem`
- parse-arguments.md: `parse_value`, `spanning` around `parse_value`, `IntegerDoesNotFitI64`, `BooleanValue(Boolean::{True, False})`
- parse-variables.md: `parse_type_annotation`, `parse_singleton` on `[...]`, `Chunk::first_item`, `ConstantValue`, `parse_constant_value`, `Box<T>` delegation in `resolve_position`
- parse-descriptions.md: description via two `consume_token_if`
- parse-pointers.md: `to` via `require_token(Identifier)` and `token_text`

A feature is reviewed against this doc when it lands. Amendment sites: the `ItemCursor` and `ChunkStream` impls, `parse_chunk`, `parse_items`, `parse_items_with_trailing`, and `parse_singleton`. This doc stays in `refactors/pending`.
