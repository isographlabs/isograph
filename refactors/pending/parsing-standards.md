# Parsing standards

Rules for grammar-stage code. Feature docs define each form (a selection, a value, a type) and the items that make it up. This doc lists the functions that implement those forms. If an implementation disagrees with this doc, the same review amends the doc or changes the code.

Every call a parse function makes on a chunk is a method or free function listed here. A new call is a new listing here.

## The unit of work

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    push_error: impl FnMut(WithSpan<ParseError>),
) -> WithSpan<IsoLiteralParse<OptimisticStage>>
```

`parse_iso_literal` takes `text: &str`, the chunked literal, and `push_error`. Each chunk is passed to `Chunk::stream(text)`, which returns one `ChunkStream`. A group's interior is the `ChunkedLevel` in `group.children`. A `ChunkStream` is built from one chunk.

A group is one item. `require_group` and `consume_group_if` return it in one call. The interior is parsed by calling `parse_items` or `parse_singleton` on `group.children`.

Each token and group has a span. A parse function assigns a span to a value made of more than one item by calling `spanning`. `expected` on an exhausted cursor is `Expected(_, EndOfChunk)` at `end_span`.

## `ItemCursor` and `ChunkStream`

`parse_chunk` calls `Chunk::stream`, then passes `stream.cursor()` (`&mut ItemCursor`) into the parse function. `parse_one_item` then calls `stream.require_end` and builds a `Slot<Option<WithSpan<P>>, Option<WithSpan<UnparsedChunkItems>>>`. Diagnostics are not leftover items. Leftover items sit on `Slot.extra`. Extra root chunks sit on `IsoLiteralParse.extra`. `IsoLiteralSlot.item` is `Some` when the form parsed. `IsoLiteralSlot.extra` is `Some` when extra items are present. Diagnostics go through `push_error: impl FnMut(WithSpan<ParseError>)` on `parse_one_item`, `parse_items`, `parse_singleton`, and `parse_iso_literal`. Inner `parse_*` stays `Result`. Artifact generation requires that no one called `push_error` and that the earlier-stage lists are empty. `require_end` is a method on `ChunkStream`.

This pass is `IsoLiteralParse<OptimisticStage>`. Resolve walks that tree only. Artifact generation does not resolve.

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
pub(crate) struct ChunkStream<'a>(ItemCursor<'a>);

impl<'a> ChunkStream<'a> {
    pub(crate) fn new(contents: &'a NonEmpty<WithSpan<ChunkContentItem>>, text: &'a str) -> Self {
        ChunkStream(ItemCursor {
            previous_end: contents.first().location.start,
            items: contents.iter().safe_peekable(),
            text,
        })
    }

    pub(crate) fn cursor(&mut self) -> &mut ItemCursor<'a> {
        &mut self.0
    }

    pub(crate) fn require_end(&mut self) -> Result<(), ()> {
        self.0.items.peek().map_or(().wrap_ok(), |_| ().wrap_err())
    }

    /// Unread content items. `None` when the cursor is at end.
    pub(crate) fn remaining_contents(&mut self) -> Option<NonEmpty<WithSpan<ChunkContentItem>>> {
        let first = self.0.items.next()?;
        let mut tail = Vec::new();
        while let Some(item) = self.0.items.next() {
            tail.push(item.clone());
        }
        NonEmpty {
            head: first.clone(),
            tail,
        }
        .wrap_some()
    }
}

impl<'a> ItemCursor<'a> {
    pub(crate) fn consume_token_if(&mut self, kind: NonBracketTokenKind) -> Option<Span> {
        let peek = self.items.peek()?;
        match peek.view().item.reference() {
            ChunkContentItem::NonBracket(token) if token.0 == kind => {}
            _ => return None,
        }
        let item = peek.commit();
        self.previous_end = item.location.end;
        item.location.wrap_some()
    }

    pub(crate) fn consume_group_if(
        &mut self,
        kind: BracketKind,
    ) -> Option<WithSpan<&'a ChunkedGroup>> {
        let peek = self.items.peek()?;
        match peek.view().item.reference() {
            ChunkContentItem::Group(group) if group.opening.item.0 == kind => {}
            _ => return None,
        }
        let item = peek.commit();
        self.previous_end = item.location.end;
        match item.item.reference() {
            ChunkContentItem::Group(group) => WithSpan::new(group, item.location).wrap_some(),
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
                let item = peek.view();
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

`nonempty::Iter` yields `&'a WithSpan<ChunkContentItem>`. `view` returns that reference. `consume_*` views to decide, then `commit`s and reads the `'a` item. `expected` only views.

### Span sources

- Leaf: the `Span` from `require_token` or `consume_token_if`, or the `WithSpan` from `require_group` or `consume_group_if`.
- Parsed list item: the `WithSpan` `spanning` returned, stored on `Slot.item` when the form parsed.
- Slot: form `Ok` and end is the item span. Form `Ok` and leftover is the join of the item span and the leftover items' span. Form `Err` is `contents_span`.
- Value made of several items: one `spanning` call. The closure's first advance is a `consume_*` or `require_*`. Remaining items of that value are read inside the same `spanning`.

`token_text` is `&self.text[span.as_usize_range()]`. A span that is not a range of that string panics, the same as any `&str` index. Names in the tree are spans. The converted scalar is the `i64`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    mut push_error: impl FnMut(WithSpan<ParseError>),
) -> WithSpan<IsoLiteralParse<OptimisticStage>> {
    let location = root.location;
    if root.item.len() == 0 {
        push_error(WithSpan::new(ParseError::EmptyLiteral, location));
        return WithSpan::new(
            IsoLiteralParse {
                item: WithSpan::new(
                    IsoLiteralSlot {
                        item: None,
                        extra: None,
                    },
                    location,
                ),
                extra: None,
            },
            location,
        );
    }
    let singleton = parse_singleton(
        root.reference(),
        text,
        Expectation::EndOfDeclaration,
        |extra| WithSpan::new(ParseError::MultipleDeclarations, extra.location),
        |cursor, _| parse_iso_literal_item(cursor),
        &mut push_error,
    );
    WithSpan::new(IsoLiteralParse::from(singleton), location)
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

`ChunkedLevel`'s vec is private to the `chunk` module. `len` is the chunk count. `parse_items` maps each chunk through `parse_one_item` (a selection set, an argument list, an object literal, a variable-declaration list). `parse_singleton` is a one-item level (the root, a `[...]` interior) with at least one chunk: chunk 0 through `parse_one_item`, then extra chunks and, at the root, a boundary comma. Empty is the caller's. Tests call `#[cfg(test)] ChunkedLevel::chunks`.

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

/// Unread or failed items from the chunk under parse. Concrete leftover holder
/// parented at the root slot; goes away when resolve-position-generic-slot.md lands.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct UnparsedChunkItems(
    #[resolve_field(parent_variant = Unparsed)] pub NonEmpty<WithSpan<ChunkContentItem>>,
);

/// Concrete root singleton so resolve has a named type to parent at. Goes away
/// when resolve-position-generic-slot.md lands. Fields are the `OptimisticStage`
/// projection so the derive sees concrete `Option<WithSpan<_>>` types. Only that
/// monomorph impls `ResolvePosition`.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = (),
    resolved_node = IsographResolutionNode<'a>,
    self_type_generics = <OptimisticStage>
)]
pub struct IsoLiteralParse<S: Stage> {
    #[resolve_field]
    pub item: WithSpan<IsoLiteralSlot>,
    #[resolve_field]
    pub extra: Option<WithSpan<ExtraChunks>>,
}

/// Concrete first-chunk slot. Goes away when resolve-position-generic-slot.md lands.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralParsePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct IsoLiteralSlot {
    #[resolve_field]
    pub item: Option<WithSpan<IsoLiteralItem>>,
    #[resolve_field]
    pub extra: Option<WithSpan<UnparsedChunkItems>>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntrypointDeclaration {
    pub entrypoint_keyword: WithSpan<EntrypointKeyword>,
    #[resolve_field]
    pub parent_type: WithSpan<EntityName>,
    #[resolve_field]
    pub client_field_name: WithSpan<ClientFieldName>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EntrypointDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntityName;

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EntrypointDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ClientFieldName;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EntrypointKeyword;

// Concrete. Goes away when resolve-position-generic-slot.md lands.
pub type IsoLiteralParsePath<'a> =
    PositionResolutionPath<&'a IsoLiteralParse<OptimisticStage>, ()>;

pub type IsoLiteralSlotPath<'a> =
    PositionResolutionPath<&'a IsoLiteralSlot, IsoLiteralParsePath<'a>>;

pub type IsoLiteralItemPath<'a> =
    PositionResolutionPath<&'a IsoLiteralItem, IsoLiteralSlotPath<'a>>;

pub type EntrypointDeclarationPath<'a> =
    PositionResolutionPath<&'a EntrypointDeclaration, IsoLiteralSlotPath<'a>>;

pub type UnparsedChunkItemsPath<'a> =
    PositionResolutionPath<&'a UnparsedChunkItems, IsoLiteralSlotPath<'a>>;

/// Extra root chunks after the first. Resolve walks each chunk. Concrete holder
/// for the root singleton; goes away when resolve-position-generic-slot.md lands.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralParsePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ExtraChunks(#[resolve_field(parent_variant = Extra)] pub NonEmpty<WithSpan<Chunk>>);

// Concrete. Goes away when resolve-position-generic-slot.md lands.
pub type ExtraChunksPath<'a> = PositionResolutionPath<&'a ExtraChunks, IsoLiteralParsePath<'a>>;

/// How far along the tree is. Associated types are the entrypoint root's
/// fail-able pieces; feature docs add one when they introduce a new one.
/// Optimistic types carry the failure. Artifact types do not.
pub trait Stage {
    type IsoLiteral;
    type UnparsedTokens;
    type ExtraChunks;
}

pub struct OptimisticStage;

pub struct ArtifactGenerationStage;

impl Stage for OptimisticStage {
    type IsoLiteral = Option<IsoLiteralItem>;
    type UnparsedTokens = Option<WithSpan<UnparsedChunkItems>>;
    type ExtraChunks = Option<WithSpan<ExtraChunks>>;
}

impl Stage for ArtifactGenerationStage {
    type IsoLiteral = IsoLiteralItem;
    type UnparsedTokens = ();
    type ExtraChunks = ();
}

/// One chunk's parse result. Combinator only; does not impl ResolvePosition
/// until resolve-position-generic-slot.md lands.
pub struct Slot<T, E> {
    pub item: T,
    pub extra: E,
}

pub struct Singleton<T, E> {
    pub item: T,
    pub extra: E,
}

impl
    From<
        Singleton<
            WithSpan<Slot<Option<WithSpan<IsoLiteralItem>>, Option<WithSpan<UnparsedChunkItems>>>>,
            Option<WithSpan<ExtraChunks>>,
        >,
    > for IsoLiteralParse<OptimisticStage>
{
    fn from(
        singleton: Singleton<
            WithSpan<Slot<Option<WithSpan<IsoLiteralItem>>, Option<WithSpan<UnparsedChunkItems>>>>,
            Option<WithSpan<ExtraChunks>>,
        >,
    ) -> Self {
        IsoLiteralParse {
            item: WithSpan::new(
                IsoLiteralSlot {
                    item: singleton.item.item.item,
                    extra: singleton.item.item.extra,
                },
                singleton.item.location,
            ),
            extra: singleton.extra,
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

fn parse_one_item<'a, P, F>(
    chunk: &'a WithSpan<Chunk>,
    text: &'a str,
    leftover_error: impl FnOnce(&mut ItemCursor<'a>) -> WithSpan<ParseError>,
    parse: impl FnOnce(&mut ItemCursor<'a>, &mut F) -> Result<P, WithSpan<ParseError>>,
    push_error: &mut F,
) -> WithSpan<Slot<Option<WithSpan<P>>, Option<WithSpan<UnparsedChunkItems>>>>
where
    F: FnMut(WithSpan<ParseError>),
{
    let (mut stream, result) = parse_chunk(chunk, text, |cursor| parse(cursor, push_error));
    match result {
        Ok(item) => {
            if stream.require_end().is_ok() {
                return WithSpan::new(
                    Slot {
                        item: item.wrap_some(),
                        extra: None,
                    },
                    item.location,
                );
            }
            push_error(leftover_error(stream.cursor()));
            match stream.remaining_contents() {
                Some(remaining) => {
                    let leftover_span =
                        Span::join(remaining.first().location, remaining.last().location);
                    let location = Span::join(item.location, leftover_span);
                    WithSpan::new(
                        Slot {
                            item: item.wrap_some(),
                            extra: WithSpan::new(UnparsedChunkItems(remaining), leftover_span)
                                .wrap_some(),
                        },
                        location,
                    )
                }
                None => WithSpan::new(
                    Slot {
                        item: item.wrap_some(),
                        extra: None,
                    },
                    item.location,
                ),
            }
        }
        Err(reason) => {
            push_error(reason);
            let location = chunk.item.contents_span();
            WithSpan::new(
                Slot {
                    item: None,
                    extra: WithSpan::new(
                        UnparsedChunkItems(chunk.item.contents.clone()),
                        location,
                    )
                    .wrap_some(),
                },
                location,
            )
        }
    }
}



impl ChunkedLevel {
    pub(crate) fn parse_items<'a, P, F>(
        &'a self,
        text: &'a str,
        parse_item: impl Fn(&mut ItemCursor<'a>, &mut F) -> Result<P, WithSpan<ParseError>>,
        push_error: &mut F,
    ) -> Vec<WithSpan<Slot<Option<WithSpan<P>>, Option<WithSpan<UnparsedChunkItems>>>>>
    where
        F: FnMut(WithSpan<ParseError>),
    {
        self.0
            .iter()
            .map(|chunk| {
                parse_one_item(
                    chunk,
                    text,
                    |cursor| cursor.expected(Expectation::Separator),
                    |cursor, push_error| parse_item(cursor, push_error),
                    push_error,
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

pub(crate) fn parse_singleton<'a, T, F>(
    level: &'a WithSpan<ChunkedLevel>,
    text: &'a str,
    end: Expectation,
    extra: impl FnOnce(&'a WithSpan<Chunk>) -> WithSpan<ParseError>,
    parse: impl FnOnce(&mut ItemCursor<'a>, &mut F) -> Result<T, WithSpan<ParseError>>,
    push_error: &mut F,
) -> Singleton<
    WithSpan<Slot<Option<WithSpan<T>>, Option<WithSpan<UnparsedChunkItems>>>>,
    Option<WithSpan<ExtraChunks>>,
>
where
    F: FnMut(WithSpan<ParseError>),
{
    let item = parse_one_item(
        &level.item.0[0],
        text,
        |cursor| cursor.expected(end),
        parse,
        push_error,
    );
    if let Some(comma) = level.item.0[0].item.boundary_comma() {
        push_error(WithSpan::new(
            ParseError::expected(end, Found::Token(NonBracketTokenKind::Comma)),
            comma,
        ));
    }
    let extra_chunks = (level.item.len() > 1).then(|| {
        let rest = NonEmpty {
            head: level.item.0[1].clone(),
            tail: level.item.0[2..].to_vec(),
        };
        push_error(extra(&rest.head));
        let location = Span::join(rest.head.location, rest.last().location);
        WithSpan::new(ExtraChunks(rest), location)
    });
    Singleton {
        item,
        extra: extra_chunks,
    }
}
```

`ChunkStream::remaining_contents` returns the unread chunk items after `require_end` `Err`. That list is nonempty. Leftover `UnparsedChunkItems` is those items.

`parse_one_item` is one chunk. Form `Ok` and `require_end` `Ok` is `item: Some`, empty `extra`. Form `Ok` and leftover is `item: Some` plus leftover items in `extra`. Form `Err` is `item: None` and a clone of the source chunk's items. `parse_*` is all-or-nothing. There is no recovered prefix of a selection. `entrypoint Foo.$ asdf` fails at `$` (`Expected(Identifier, Dollar)`). `item` is `None`. `extra` is the whole chunk `entrypoint Foo.$ asdf`, not `$ asdf` and not `asdf`.

`parse_items` is `parse_one_item` per chunk. Length equals chunk count. A list trailing comma is legal and is not a diagnostic. `foo { bar } asdf` is `item: Some` (the object selection `foo { bar }`) and leftover items `asdf`. A position on `asdf` resolves through `UnparsedChunkItems`, not the selection set.

`parse_singleton` is not a vec of slots. The level has at least one chunk. Chunk 0 is `parse_one_item`. Remaining chunks are cloned into `ExtraChunks` (every chunk after the first) plus `S::ExtraChunks` (at the root, `MultipleDeclarations` on the first extra chunk). Extra chunks clone for now. Leftover in the first chunk and a boundary comma use `end` (`EndOfDeclaration` at the root, `EndOfType` inside `[...]`). A boundary comma is a tokenless diagnostic via `push_error`. Empty is handled by the caller (`parse_iso_literal` pushes `EmptyLiteral` and returns an empty `IsoLiteralSlot` plus `extra: None`). `item` on the first slot is `Some` when the form parsed.

`Slot<T, E>` is `item: T` and `extra: E`. Combinator only. `From` copies the first slot into `IsoLiteralSlot` under `IsoLiteralParse.item`. Extra chunks sit on `IsoLiteralParse.extra`. Resolve walks `IsoLiteralParse<OptimisticStage>` only. `Singleton<T, E>` is the combinator result.

A list is `parse_items`. A type that contains a group is generic over `Stage`. That includes a selection set, an argument list, an object literal, a `[...]` type, and a scalar selection (it may hold an argument list). Feature docs write those types.

`UnparsedChunkItems` sits on `Slot.extra`. Chunk items in `UnparsedChunkItems` use `parent_variant = Unparsed`. Extra chunks use `Chunk`'s parent variant `Extra`.

## Function shapes

- `consume_*`: `ItemCursor` method. Match: `commit` and `Some`. Else: `None`.
- `expected`: `ItemCursor` method. Peek, no `commit`. Next item or `EndOfChunk` becomes `Expected(expected, found)`.
- `require_*`: `consume_*` or `Err(())`. The caller maps `Err` with `expected`.
- `parse_*`: implements a form made of several items. Parameter is `&mut ItemCursor`. First `Err` is returned. Shared iteration is `parse_items`, `parse_singleton`, or `spanning`. A nested list also takes `push_error`.
- Diagnostic: `push_error` on `parse_one_item`, `parse_items`, `parse_singleton`, `parse_iso_literal`. Not stored on the tree.

A group plus its interior is `consume_group_if` or `require_group`, then `parse_items` or `parse_singleton` on `group.children`:

```rust
    let group = cursor.consume_group_if(BracketKind::Brace)?;
    group
        .item
        .children
        .item
        .parse_items(cursor.text(), parse_item, push_error)
```

```rust
    let group = cursor
        .require_group(BracketKind::Brace)
        .map_err(|()| cursor.expected(expectation))?;
    group
        .item
        .children
        .item
        .parse_items(cursor.text(), parse_item, push_error)
```

## Dispatch

When the next item may start several forms, the parse function is a `consume_*` ladder. The last arm is `expected`. If those arms are one value, the ladder is inside `spanning`. An arm that has taken its first item continues with `require_*` / `consume_*`.

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn parse_value<F>(
    cursor: &mut ItemCursor<'_>,
    push_error: &mut F,
) -> Result<WithSpan<NonConstantValue>, WithSpan<ParseError>>
where
    F: FnMut(WithSpan<ParseError>),
{
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
            return group
                .item
                .children
                .item
                .parse_items(cursor.text(), parse_object_entry, push_error)
                .wrap_ok();
        }
        cursor.expected(Expectation::Value).wrap_err()
    })
}
```

Keyword text after `require_token(Identifier, ...)` or `consume_token_if(Identifier)`: `match` on `token_text` (`"entrypoint"` / `"field"` / `"pointer"`; `"true"` / `"false"` / `"null"`; `"to"`).

One optional item is `consume_*`. Two optional kinds in one position is two `consume_token_if` calls. The optional `!` after a type name is `consume_token_if(Exclamation)`: the next item may be the caller's `=`. `$name` is `consume_token_if(Dollar)` then `require_token(Identifier)`. After `require_token` on an identifier, `consume_token_if(Colon)` is the alias; both arms use the identifier.

```rust
    let first = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(expectation))?;
    let (alias, name) = match cursor.consume_token_if(NonBracketTokenKind::Colon) {
        Some(_) => {
            let name = cursor
                .require_token(NonBracketTokenKind::Identifier)
                .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
            (first.wrap_some(), name)
        }
        None => (None, first),
    };
```

`parse_one_item` wraps the item parse in `spanning` and then calls `require_end`. Leftover is extra tokens.

## Narrower types for narrower grammars

Variable defaults are constant values. The constant-value ladder is the value ladder without the `$` arm. `$` falls through to `expected(Expectation::ConstantValue)`. The integer arm is the same `token_text(span).parse()` match; that `span` is the one `consume_token_if(IntegerLiteral)` just returned. `parse::<i64>()` on an `IntegerLiteral` token (`-?(0|[1-9][0-9]*)`) fails only as overflow or underflow.

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

Diagnostics are not on the tree. `parse_one_item` calls `push_error` for leftover and for a failed form. `parse_iso_literal` calls it for empty. `parse_singleton` calls it for a boundary comma and extra. Nested lists push as they parse, inner first. Resolve walks leftover and failed items, not diagnostics. Bracket errors are the matcher's vec. Comma-without-item errors are chunking's vec. Grammar diagnostics go through `push_error`. Artifact generation runs only when those three lists are empty.

A failed list chunk is `item: None` plus the chunk's items in `Slot.extra`. Leftover after a successful list item is `item: Some` plus leftover items and `push_error(Expected(Separator, ...))`. Tokenless diagnostics (empty literal, a root comma) go through `push_error` with no `UnparsedChunkItems`. Extra root chunks are `S::ExtraChunks` plus `push_error(MultipleDeclarations)`. `Display` formats `ParseError`. Suggestions are produced later from `(expected, found)`.

## Totality

`parse_iso_literal` returns a tree for every `&str`. A position in a parsed region resolves to a grammar leaf. A position in leftover or failed items resolves through `UnparsedChunkItems`. Extra root chunks resolve through `ExtraChunks`. A position on whitespace or a dropped comma or unmatched-bracket region resolves to the nearest containing node.

Find-references, rename, and go-to-definition run when the resolved leaf is a name leaf. In `foo { bar } asdf`, a position on `asdf` resolves through the leftover `UnparsedChunkItems`; find-references returns no references. Completion reads the resolution path. Diagnostics are a separate list.

## Trees and spans

A tree enum is wrapped in `WithSpan` at its slot. The parsed item on `IsoLiteralSlot` is `Option<WithSpan<IsoLiteralItem>>`. Each other struct field that is a node is `WithSpan`. A name is a fieldless marker struct in a `WithSpan`; each role is its own type. The name's text is the wrapper's span. The converted scalar is the `i64`. A position on `.`, `$`, `!`, or `to` resolves to the containing node. Resolve walks the optimistic tree only.

`ResolvePosition` is derived. The one blanket delegation is `Box<T>` (parse-variables.md). A parent is a path alias at one parent, an enum at the second. Chunk-stage `IsographResolutionNode` variants resolve inside `UnparsedChunkItems` and `ExtraChunks`.

## Performance

One pass by reference. The output copies spans and `Copy` tokens. Leftover and failed items clone. Extra chunks clone for now. A later change can store a range into the original chunk instead. Each item is advanced past at most once. The functions take `&str` and the chunk tree.

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
- List of items: `ChunkedLevel::parse_items` → `Vec<WithSpan<Slot<Option<P>, Option<WithSpan<UnparsedChunkItems>>>>>`
- One-item context: `parse_singleton` → `Singleton<WithSpan<Slot<Option<WithSpan<T>>, Option<WithSpan<UnparsedChunkItems>>>>, Option<WithSpan<ExtraChunks>>>`
- Recovered item: `Slot.item` → `Option<WithSpan<T>>`
- Extra items: `Slot.extra` → `Option<WithSpan<UnparsedChunkItems>>`
- Chunk count: `ChunkedLevel::len`
- First item of an extra chunk: `Chunk::first_item`
- Trailing comma in a one-item context: `push_error` (tokenless)
- Leftover after a list item: `item: Some` plus extra tokens and `push_error(Expected(Separator, ...))`
- Leftover after a singleton first chunk: `item: Some` plus extra tokens and `push_error(Expected(end, ...))`
- Extra root chunks: `ExtraChunks` plus `push_error(MultipleDeclarations)`
- Diagnostic: `push_error` on `parse_one_item` / `parse_singleton` / `parse_iso_literal`
- Group interior: `require_group` / `consume_group_if`, then `parse_items` or `parse_singleton` on `group.children`
- Constant-only value: `parse_constant_value` → `ConstantValue`

## Shipping and amending

The first implementation step is the shared surface, with tests, before any grammar feature. That step lands:

- `ItemCursor` / `ChunkStream`: `new`, `cursor`, `require_end`, `consume_token_if`, `require_token`, `consume_group_if`, `require_group`, `expected`, `text`, `token_text`, `end_span`, `spanning`
- `Chunk::stream`, `Chunk::contents_span`, `Chunk::first_item`, `Chunk::boundary_comma`, `ChunkedLevel::len`
- `Slot`, `Stage`, `OptimisticStage`, `ArtifactGenerationStage`, `UnparsedChunkItems`, `ExtraChunks`, `Singleton`, `parse_chunk`, `parse_one_item`, `parse_items`, `parse_singleton`, `push_error`, `ChunkContentItemParent`
- `Clone` on `ChunkedLevel`, `Chunk`, `ChunkContentItem`, `ChunkedGroup`, `ChunkSeparator`. Extra chunks clone for now.
- `ParseError` / `Expectation` / `Found` as the error types those methods return

Tests assert facts about that surface: `require_*` / `consume_*` match and mismatch, `expected` names the next item or `EndOfChunk`, `require_end` is `Ok` only on an empty remainder, `spanning` covers what the closure advanced past, `parse_one_item` leftover is `item: Some` plus extra tokens, `parse_singleton` extra is `ExtraChunks`. No grammar tree, no `parse_iso_literal`.

Each grammar feature then lands on that surface.

- parse-entrypoint.md: `parse_iso_literal`, `parse_singleton` at the root, `entrypoint Type.field`
- parse-fields.md: field declarations and selection sets via `parse_items`, `push_error` through `parse_items`, `ChunkContentItemParent` variant `Unparsed`, `ChunkParent` variant `Extra`
- parse-arguments.md: `parse_value`, `IntegerDoesNotFitI64`, `BooleanValue(Boolean::{True, False})`
- parse-variables.md: `parse_type_annotation`, `parse_singleton` on `[...]`, `ConstantValue`, `parse_constant_value`, `Box<T>` delegation in `resolve_position`
- parse-descriptions.md: description via two `consume_token_if`
- parse-pointers.md: `to` via `require_token(Identifier)` and `token_text`

A feature is reviewed against this doc when it lands. Amendment sites: the `ItemCursor` and `ChunkStream` impls, `parse_chunk`, `parse_items`, and `parse_singleton`. This doc stays in `refactors/pending`.
