# Parsing standards

Rules for grammar-stage code. Feature docs define each form. This doc lists the functions those forms call. If an implementation disagrees with this doc, the same review amends the doc or changes the code.

Every call a parse function makes on a chunk is a method or free function listed here. A new call is a new listing here.

## The unit of work

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    push_error: impl FnMut(WithSpan<ParseError>),
    tokens: &mut Vec<WithSpan<SemanticToken>>,
) -> Option<WithSpan<IsoLiteralParse>>
```

`parse_iso_literal` takes `text: &str`, the chunked literal, `push_error`, and `tokens`. Each chunk is passed to `Chunk::stream(text, tokens)`, which returns one `ChunkStream`. A group's interior is the `ChunkedLevel` in `group.children`.

A group is one item. `require_group` and `consume_group_if` return it in one call. The interior is parsed by calling `parse_items` or `parse_singleton` on `group.children`.

Each token and group has a span. A parse function assigns a span to a value made of more than one item by calling `spanning`. `expected` on an exhausted cursor is `Expected(_, EndOfChunk)` at `end_span`.

## `ItemCursor` and `ChunkStream`

`parse_chunk` calls `Chunk::stream`, then passes `stream.cursor()` (`&mut ItemCursor`) into the parse function. `parse_one_item` then calls `stream.remaining_contents` and builds a `Slot<P, UnparsedChunkItems>`. Diagnostics are not leftover items. Leftover items sit on `Slot.extra_tokens`. Extra chunks sit on `Singleton.extra_chunks`. `Slot.item` is `Some` when the form parsed. `Slot.extra_tokens` is `Some` when extra items are present. Diagnostics go through `push_error: impl FnMut(WithSpan<ParseError>)` on `parse_one_item`, `parse_items`, `parse_singleton`, and `parse_iso_literal`. Inner `parse_*` stays `Result`. Artifact generation requires that no one called `push_error` and that the earlier-stage lists are empty. `require_end` is a method on `ChunkStream`.

This pass is `IsoLiteralParse`. Resolve walks that tree only. Artifact generation does not resolve.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
pub(crate) struct ItemCursor<'a> { /* ... */ }

pub(crate) struct ChunkStream<'a>(ItemCursor<'a>);

impl<'a> ChunkStream<'a> {
    pub(crate) fn new(
        contents: &'a NonEmpty<WithSpan<ChunkContentItem>>,
        text: &'a str,
        tokens: &'a mut Vec<WithSpan<SemanticToken>>,
    ) -> Self;
    pub(crate) fn cursor(&mut self) -> &mut ItemCursor<'a>;
    pub(crate) fn require_end(&mut self) -> Result<(), ()>;
    pub(crate) fn remaining_contents(&mut self) -> Option<NonEmpty<WithSpan<ChunkContentItem>>>;
}

impl<'a> ItemCursor<'a> {
    pub(crate) fn peek(&mut self) -> Option<CursorPeek<'_, 'a>>;
    pub(crate) fn consume_token_if(
        &mut self,
        kind: NonBracketTokenKind,
        token: SemanticToken,
    ) -> Option<Span>;
    pub(crate) fn consume_group_if(
        &mut self,
        kind: BracketKind,
        token: SemanticToken,
    ) -> Option<WithSpan<&'a ChunkedGroup>>;
    pub(crate) fn expected(&mut self, expected: Expectation) -> WithSpan<ParseError>;
    pub(crate) fn require_token(
        &mut self,
        kind: NonBracketTokenKind,
        token: SemanticToken,
    ) -> Result<Span, ()>;
    pub(crate) fn require_group(
        &mut self,
        kind: BracketKind,
        token: SemanticToken,
    ) -> Result<WithSpan<&'a ChunkedGroup>, ()>;
    pub(crate) fn record_group_close(&mut self, group: &ChunkedGroup, token: SemanticToken);
    pub(crate) fn text(&self) -> &'a str;
    pub(crate) fn token_text(&self, span: Span) -> &'a str;
    pub(crate) fn spanning<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, WithSpan<ParseError>>,
    ) -> Result<WithSpan<T>, WithSpan<ParseError>>;
}

impl<'c, 'a> CursorPeek<'c, 'a> {
    pub(crate) fn view(&self) -> &'a WithSpan<ChunkContentItem>;
    pub(crate) fn commit(self, token: SemanticToken) -> &'a WithSpan<ChunkContentItem>;
}
```

`require_*` is `consume_*` or `Err(())`. The caller maps `Err` with `expected`.

### Span sources

- Leaf: the `Span` from `require_token` or `consume_token_if`, or the `WithSpan` from `require_group` or `consume_group_if`.
- Parsed list item: the `WithSpan` `spanning` returned, stored on `Slot.item` when the form parsed.
- Slot: form `Ok` and end is the item span. Form `Ok` and leftover is the join of the item span and the leftover items' span. Form `Err` is `contents_span`.
- Value made of several items: one `spanning` call. The closure's first advance is a `consume_*` or `require_*`. Remaining items of that value are read inside the same `spanning`.

`token_text` is `&self.text[span.as_usize_range()]`. A name in the tree is an interned string key (`token_text(span).intern().to::<EntityName>()`). The converted scalar is the `i64`. The wrapper span is location only.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub type IsoLiteralParse = Singleton<Slot<IsoLiteralItem, UnparsedChunkItems>, ExtraChunks>;

pub type IsoLiteralParsePath<'a> = PositionResolutionPath<&'a IsoLiteralParse, ()>;
```

`IsoLiteralItem` is the declaration enum. `parse_singleton` returns `IsoLiteralParse`. That value is the tree. Empty is `None` from `parse_iso_literal`.

## The tree

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = <T as ResolvePosition>::Parent<'a>,
    resolved_node = IsographResolutionNode<'a>,
    fallback = from_path
)]
pub struct Slot<T: ResolvePosition, E: ResolvePosition>
where
    for<'a> T: ResolvePosition<ResolvedNode<'a> = IsographResolutionNode<'a>>,
    for<'a> E: ResolvePosition<ResolvedNode<'a> = IsographResolutionNode<'a>>,
    for<'a> <E as ResolvePosition>::Parent<'a>: From<<T as ResolvePosition>::Parent<'a>>,
    for<'a> IsographResolutionNode<'a>: From<
        PositionResolutionPath<&'a Slot<T, E>, <T as ResolvePosition>::Parent<'a>>,
    >,
{
    #[resolve_field]
    #[parent_from]
    pub item: Option<WithSpan<T>>,
    #[resolve_field]
    #[parent_from]
    pub extra_tokens: Option<WithSpan<E>>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = (),
    resolved_node = IsographResolutionNode<'a>,
    self_type_generics = <Slot<IsoLiteralItem, UnparsedChunkItems>, ExtraChunks>
)]
pub struct Singleton<T, E> {
    #[resolve_field]
    pub item: WithSpan<T>,
    #[resolve_field]
    pub extra_chunks: Option<WithSpan<E>>,
}

pub struct UnparsedChunkItems(
    #[resolve_field]
    #[parent_variant(Unparsed)]
    pub NonEmpty<WithSpan<ChunkContentItem>>,
);

pub struct ExtraChunks(
    #[resolve_field]
    #[parent_variant(Extra)]
    pub NonEmpty<WithSpan<Chunk>>,
);
```

`#[resolve_field]` + `#[parent_from]` on a struct field passes `From::from(parent)` as the child's parent. Fallback is not suppressed. `fallback = from_path` makes the no-hit arm `self.path(parent).to()`. `T::Parent` equals `Slot<T, E>::Parent`. The item conversion is the blanket `From<P> for P`. Leftover is `From<T::Parent> for E::Parent`. A position in leftover walks `extra_tokens`. A position in the slot span but in neither field answers that monomorph's `ResolvedNode` variant. `{ item: None, extra_tokens: None }` is the same fallback.

Leftover span is tight to the leftover tokens. The gap after the item is a third region: the slot leaf.

Each list that stores a `Slot` adds a `ResolvedNode` variant whose payload is that monomorph's path, and a `From` into `IsographResolutionNode`. `SlotPath` stays the root alias.

`UnparsedChunkItems`'s parent is an enum. Each list that stores a `Slot` adds a variant and a `From`:

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug)]
pub enum UnparsedChunkItemsParent<'a> {
    Literal(IsoLiteralParsePath<'a>),
}

pub type UnparsedChunkItemsPath<'a> =
    PositionResolutionPath<&'a UnparsedChunkItems, UnparsedChunkItemsParent<'a>>;

impl<'a> From<IsoLiteralParsePath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(parent: IsoLiteralParsePath<'a>) -> Self {
        UnparsedChunkItemsParent::Literal(parent)
    }
}
```

`IsoLiteralItem`'s `parent_type` is `IsoLiteralParsePath<'a>`. `EntrypointDeclaration`'s `parent_type` is `IsoLiteralParsePath<'a>`.

`Singleton` at the root stays pinned (`parent_type = ()`, `self_type_generics` as above). parse-variables.md adds a generic `Singleton` impl when `[...]` stores one.

`item: None` and `extra_tokens: None` together is representable and never constructed.

## Lists and one-item levels

`ChunkedLevel`'s vec is private to the `chunk` module. `len` is the chunk count. `parse_items` maps each chunk through `parse_one_item`. `parse_singleton` is a one-item level with at least one chunk: chunk 0 through `parse_one_item`, then extra chunks and a boundary comma. Empty is the caller's.

```rust
// from crates/isograph_parser/src/chunk.rs
fn parse_one_item<'a, P, F>(
    chunk: &'a WithSpan<Chunk>,
    text: &'a str,
    leftover: Expectation,
    parse: impl FnOnce(&mut ItemCursor<'a>, &mut F) -> Result<P, WithSpan<ParseError>>,
    push_error: &mut F,
) -> WithSpan<Slot<P, UnparsedChunkItems>>
where
    F: FnMut(WithSpan<ParseError>),
{
    let (mut stream, result) = parse_chunk(chunk, text, |cursor| parse(cursor, push_error));
    match result {
        Ok(item) => match stream.remaining_contents() {
            None => {
                let location = item.location;
                Slot {
                    item: item.wrap_some(),
                    extra_tokens: None,
                }
                .with_span(location)
            }
            Some(remaining) => {
                push_error(
                    ParseError::expected(leftover, Found::from(remaining.first().item.reference()))
                        .with_span(remaining.first().location),
                );
                let leftover_span =
                    Span::new(item.location.end, remaining.last().location.end);
                let location = Span::join(item.location, leftover_span);
                Slot {
                    item: item.wrap_some(),
                    extra_tokens: UnparsedChunkItems(remaining)
                        .with_span(leftover_span)
                        .wrap_some(),
                }
                .with_span(location)
            }
        },
        Err(reason) => {
            push_error(reason);
            let location = chunk.item.contents_span();
            Slot {
                item: None,
                extra_tokens: UnparsedChunkItems(chunk.item.contents.clone())
                    .with_span(location)
                    .wrap_some(),
            }
            .with_span(location)
        }
    }
}

impl ChunkedLevel {
    pub(crate) fn parse_items<'a, P, F>(
        &'a self,
        text: &'a str,
        leftover: Expectation,
        parse_item: impl Fn(&mut ItemCursor<'a>, &mut F) -> Result<P, WithSpan<ParseError>>,
        push_error: &mut F,
    ) -> Vec<WithSpan<Slot<P, UnparsedChunkItems>>>
    where
        F: FnMut(WithSpan<ParseError>),
    {
        self.0
            .iter()
            .map(|chunk| {
                parse_one_item(
                    chunk,
                    text,
                    leftover,
                    |cursor, push_error| parse_item(cursor, push_error),
                    push_error,
                )
            })
            .collect()
    }
}

pub(crate) fn parse_singleton<'a, T, F>(
    level: &'a WithSpan<ChunkedLevel>,
    text: &'a str,
    end: Expectation,
    extra_chunks: impl FnOnce(&'a WithSpan<Chunk>) -> WithSpan<ParseError>,
    parse: impl FnOnce(&mut ItemCursor<'a>, &mut F) -> Result<T, WithSpan<ParseError>>,
    push_error: &mut F,
) -> Singleton<Slot<T, UnparsedChunkItems>, ExtraChunks>
where
    F: FnMut(WithSpan<ParseError>),
{
    let item = parse_one_item(&level.item.0[0], text, end, parse, push_error);
    if let Some(comma) = level.item.0[0].item.boundary_comma() {
        push_error(
            ParseError::expected(end, Found::Token(NonBracketTokenKind::Comma)).with_span(comma),
        );
    }
    let extra_chunks = (level.item.len() > 1).then(|| {
        push_error(extra_chunks(&level.item.0[1]));
        let rest = NonEmpty {
            head: level.item.0[1].clone(),
            tail: level.item.0[2..].to_vec(),
        };
        let location = Span::join(rest.head.location, rest.last().location);
        ExtraChunks(rest).with_span(location)
    });
    Singleton { item, extra_chunks }
}
```

`parse_items` leftover is `Expectation::Separator(ClosingDelimiter::...)` at a selection set (`}`), an argument list (`)`), an object literal (`}`), and a variable-declaration list (`)`). A list trailing comma is legal and is not a diagnostic.

`parse_singleton` leftover and boundary comma use `end` (`EndOfDeclaration` at the root, `EndOfType` inside `[...]`). A boundary comma is a tokenless diagnostic via `push_error`. Extra chunks clone. Empty is the caller (`parse_iso_literal` pushes `EmptyLiteral` and returns `None`; a `[...]` with zero chunks is `Expected(TypeAnnotation, EndOfChunk)`).

`parse_*` is all-or-nothing. There is no recovered prefix of a selection. `entrypoint Foo.$ asdf` fails at `$`. `item` is `None`. `extra_tokens` is the whole chunk.

A type that contains a group stores `Vec<WithSpan<Slot<P, UnparsedChunkItems>>>` or a `Singleton<Slot<T, UnparsedChunkItems>, ExtraChunks>`. Feature docs write those types.

## Function shapes

- `consume_*`: `ItemCursor` method. Match: `commit` and `Some`. Else: `None`.
- `expected`: `ItemCursor` method. Peek, no `commit`. Next item or `EndOfChunk` becomes `Expected(expected, found)`.
- `require_*`: `consume_*` or `Err(())`. The caller maps `Err` with `expected`.
- `parse_*`: implements a form made of several items. Parameter is `&mut ItemCursor`. First `Err` is returned. Shared iteration is `parse_items`, `parse_singleton`, or `spanning`. A nested list also takes `push_error`.
- Diagnostic: `push_error` on `parse_one_item`, `parse_items`, `parse_singleton`, `parse_iso_literal`. Not stored on the tree.

A group plus its interior is `consume_group_if` or `require_group`, then `parse_items` or `parse_singleton` on `group.children`:

```rust
    let group = cursor.consume_group_if(BracketKind::Brace)?;
    group.item.children.item.parse_items(
        cursor.text(),
        Expectation::Separator(ClosingDelimiter::Brace),
        parse_item,
        push_error,
    )
```

```rust
    let group = cursor
        .require_group(BracketKind::Brace)
        .map_err(|()| cursor.expected(expectation))?;
    group.item.children.item.parse_items(
        cursor.text(),
        Expectation::Separator(ClosingDelimiter::Brace),
        parse_item,
        push_error,
    )
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
                name: cursor
                    .token_text(name)
                    .intern()
                    .to::<VariableName>()
                    .with_span(name),
            })
            .wrap_ok();
        }
        if let Some(span) = cursor.consume_token_if(NonBracketTokenKind::StringLiteral) {
            // Quotes included. Unquoting is later.
            return NonConstantValue::String(
                cursor
                    .token_text(span)
                    .intern()
                    .to::<StringValue>(),
            )
            .wrap_ok();
        }
        if let Some(span) = cursor.consume_token_if(NonBracketTokenKind::IntegerLiteral) {
            let value = match cursor.token_text(span).parse() {
                Ok(value) => value,
                Err(_) => {
                    return ParseError::IntegerDoesNotFitI64.with_span(span).wrap_err();
                }
            };
            return NonConstantValue::Integer(IntegerValue(value)).wrap_ok();
        }
        if let Some(span) = cursor.consume_token_if(NonBracketTokenKind::Identifier) {
            return match cursor.token_text(span) {
                "true" => NonConstantValue::Boolean(BooleanValue(Boolean::True)).wrap_ok(),
                "false" => NonConstantValue::Boolean(BooleanValue(Boolean::False)).wrap_ok(),
                "null" => NonConstantValue::Null(NullValue).wrap_ok(),
                _ => ParseError::expected(
                    Expectation::Value,
                    Found::Token(NonBracketTokenKind::Identifier),
                )
                .with_span(span)
                .wrap_err(),
            };
        }
        if let Some(group) = cursor.consume_group_if(BracketKind::Brace) {
            return NonConstantValue::Object(ObjectLiteral(
                group.item.children.item.parse_items(
                    cursor.text(),
                    Expectation::Separator(ClosingDelimiter::Brace),
                    parse_object_entry,
                    push_error,
                ),
            ))
            .wrap_ok();
        }
        cursor.expected(Expectation::Value).wrap_err()
    })
}
```

`VariableUse` stores the interned name. A position on `$` answers `VariableUse`. There is no `Dollar` field.

Keyword text after `require_token(Identifier)` or `consume_token_if(Identifier)`: `match` on `token_text` (`"entrypoint"` / `"field"` / `"pointer"`; `"true"` / `"false"` / `"null"`; `"to"`).

One optional item is `consume_*`. Two optional kinds in one position is two `consume_token_if` calls. The optional `!` after a type name is `consume_token_if(Exclamation)`: the next item may be the caller's `=`. `$name` is `consume_token_if(Dollar)` then `require_token(Identifier)`. After `require_token` on an identifier, `consume_token_if(Colon)` is the alias; both arms use the identifier.

```rust
    let first = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(expectation))?;
    let (alias, name) = match cursor.consume_token_if(NonBracketTokenKind::Colon) {
        Some(_) => {
            let name = cursor
                .require_token(NonBracketTokenKind::Identifier)
                .map_err(|()| {
                    cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier))
                })?;
            (first.wrap_some(), name)
        }
        None => (None, first),
    };
```

## Narrower types for narrower grammars

Variable defaults are constant values. The constant-value ladder is the value ladder without the `$` arm. `$` falls through to `expected(Expectation::ConstantValue)`. The integer arm is the same `token_text(span).parse()` match; that `span` is the one `consume_token_if(IntegerLiteral)` just returned. `parse::<i64>()` on an `IntegerLiteral` token (`-?(0|[1-9][0-9]*)`) fails only as overflow or underflow.

## Errors

```rust
// from crates/isograph_parser/src/parse_error.rs
use thiserror::Error;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum ParseError {
    #[error("{0}")]
    Expected(ExpectedFound),
    #[error("Expected a declaration. An isograph literal cannot be empty.")]
    EmptyLiteral,
    #[error("Expected nothing after the declaration. Each literal holds exactly one declaration.")]
    MultipleDeclarations,
    #[error("This declaration type is not supported yet.")]
    UnsupportedDeclarationType,
    #[error("This integer does not fit in a 64-bit signed integer.")]
    IntegerDoesNotFitI64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
#[error("Expected {expected}, found {found}.")]
pub struct ExpectedFound {
    pub expected: Expectation,
    pub found: Found,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum Expectation {
    #[error("{0}")]
    Token(NonBracketTokenKind),
    #[error("one of `entrypoint`, `field`, or `pointer`")]
    DeclarationKeyword,
    #[error("the end of the declaration")]
    EndOfDeclaration,
    #[error("a selection set, like '{{ id, name }}'")]
    SelectionSet,
    #[error("a field selection")]
    Selection,
    #[error("a comma, a line break, or {0}")]
    Separator(ClosingDelimiter),
    #[error("an argument, like 'id: $id'")]
    Argument,
    #[error("a value, like $foo, 42, \"bar\", true, false, null, or an object literal")]
    Value,
    #[error("an object entry, like 'id: 4'")]
    ObjectEntry,
    #[error("a variable declaration, like '$id: ID!'")]
    VariableDeclaration,
    #[error("a type, like 'String', 'String!', or '[String]'")]
    TypeAnnotation,
    #[error("a constant value; variables are not allowed here")]
    ConstantValue,
    #[error("the end of the type")]
    EndOfType,
    #[error("the keyword `to`")]
    ToKeyword,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum ClosingDelimiter {
    #[error("'}}'")]
    Brace,
    #[error("')'")]
    Parenthesis,
    #[error("']'")]
    Bracket,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum Found {
    #[error("{0}")]
    Token(NonBracketTokenKind),
    #[error("a group opened by {0}")]
    Group(BracketKind),
    #[error("nothing more")]
    EndOfChunk,
}

// from crates/isograph_parser/src/non_bracket_token.rs
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, strum::Display)]
pub enum BracketKind {
    #[strum(to_string = "'('")]
    Parenthesis,
    #[strum(to_string = "'{'")]
    Brace,
    #[strum(to_string = "'['")]
    Bracket,
}
```

One global `Expectation`. The listing above is the eventual enum. Variants land with the feature that first constructs them. parse-arguments.md adds `Argument`, `Value`, `ObjectEntry`, `IntegerDoesNotFitI64`, and `Separator(ClosingDelimiter)`. parse-selection-sets.md adds `SelectionSet` and `Selection`. parse-variables.md adds `VariableDeclaration`, `TypeAnnotation`, `ConstantValue`, and `EndOfType`. parse-pointers.md adds `ToKeyword` and removes `UnsupportedDeclarationType`.

An error is `WithSpan<ParseError>`. The span is the offending item, or empty at `end_span` where the missing item would go. `IntegerDoesNotFitI64` is the `parse::<i64>()` `Err` on an `IntegerLiteral` token.

Diagnostics are not on the tree. Nested lists push as they parse, inner first. Resolve walks leftover and failed items, not diagnostics. Bracket errors are the matcher's vec. Comma-without-item errors are chunking's vec. Grammar diagnostics go through `push_error`. Artifact generation runs only when those three lists are empty.

A failed list chunk is `item: None` plus the chunk's items in `Slot.extra_tokens`. Leftover after a successful list item is `item: Some` plus leftover items and `push_error(Expected(Separator, ...))`. Tokenless diagnostics (empty literal, a root comma) go through `push_error` with no `UnparsedChunkItems`. Extra root chunks are `IsoLiteralParse.extra_chunks` plus `push_error(MultipleDeclarations)`. `thiserror` formats `ParseError`. Suggestions are produced later from `(expected, found)`.

There is no `errors()` walk on the tree. Tests read the `push_error` vec.

## Totality

`parse_iso_literal` returns `None` on an empty chunked literal and a tree otherwise. A position in a parsed region resolves to a grammar leaf. A position in leftover or failed items resolves through `UnparsedChunkItems`. Extra root chunks resolve through `ExtraChunks`. A position on whitespace or a dropped comma or unmatched-bracket region resolves to the nearest containing node. An empty literal has no grammar tree; the diagnostic is `EmptyLiteral`.

Find-references, rename, and go-to-definition run when the resolved leaf is a name leaf. In `foo { bar } asdf`, a position on `asdf` resolves through the leftover `UnparsedChunkItems`; find-references returns no references. Completion reads the resolution path. Diagnostics are a separate list.

## Trees and spans

A tree enum is wrapped in `WithSpan` at its slot. `Slot.item` and `Slot.extra_tokens` are `Option<WithSpan<_>>`. `Singleton.item` is `WithSpan<Slot<...>>`, the `parse_one_item` attempt. Each other struct field that is a node is `WithSpan`. A name is a newtype over an interned key in a `WithSpan`; each role is its own type. The converted scalar is the `i64`. A position on `.`, `$`, `!`, `:`, or `to` resolves to the containing node. There is no keyword-marker type. Resolve walks the optimistic tree only.

`ResolvePosition` is derived. The one blanket delegation is `Box<T>` (parse-variables.md). A parent is a path alias at one parent, an enum at the second. Chunk-stage `IsographResolutionNode` variants resolve inside `UnparsedChunkItems` and `ExtraChunks`.

## Performance

One pass by reference. The output copies spans and `Copy` tokens. Leftover and failed items clone. Extra chunks clone. A later change can store a range into the original chunk instead. Each item is advanced past at most once. The functions take `&str` and the chunk tree.

## Catalog of parsing tasks

- Required token: `ItemCursor::require_token`
- Optional token: `ItemCursor::consume_token_if`
- Required group: `ItemCursor::require_group`
- Optional group: `ItemCursor::consume_group_if`
- Wrong or missing item: `ItemCursor::expected`
- Multi-form position: `consume_*` ladder, last arm `expected`
- Keyword / boolean / null text: `token_text` after an identifier
- Integer conversion: `token_text(span).parse()` on an `IntegerLiteral` span
- Interned name: `token_text(span).intern().to::<EntityName>()`
- Composite span: `ItemCursor::spanning`
- List of items: `ChunkedLevel::parse_items` → `Vec<WithSpan<Slot<P, UnparsedChunkItems>>>`
- One-item context: `parse_singleton` → `Singleton<Slot<T, UnparsedChunkItems>, ExtraChunks>`
- Recovered item: `Slot.item` → `Option<WithSpan<T>>`
- Extra items: `Slot.extra_tokens` → `Option<WithSpan<UnparsedChunkItems>>`
- Chunk count: `ChunkedLevel::len`
- First item of an extra chunk: `Chunk::first_item`
- Trailing comma in a one-item context: `push_error` (tokenless)
- Leftover after a list item: `item: Some` plus extra tokens and `push_error(Expected(Separator, ...))`
- Leftover after a singleton first chunk: `item: Some` plus extra tokens and `push_error(Expected(end, ...))`
- Extra root chunks: `ExtraChunks` plus `push_error(MultipleDeclarations)`
- Diagnostic: `push_error` on `parse_one_item` / `parse_items` / `parse_singleton` / `parse_iso_literal`
- Group interior: `require_group` / `consume_group_if`, then `parse_items` or `parse_singleton` on `group.children`
- Constant-only value: `parse_constant_value` → `ConstantValue`

## Shipping and amending

Each grammar feature lands on this surface.

- generic-slot.md: generic `Slot` impl, `UnparsedChunkItemsParent`, `fallback = from_path`, one `ResolvedNode` variant per slot monomorph
- parse-arguments.md: `parse_items`, `ClosingDelimiter`, `parse_value`, `IntegerDoesNotFitI64`, `BooleanValue(Boolean::{True, False})`
- parse-selection-sets.md: selections, selection sets, arguments on selections
- parse-fields.md: `field Type.name { ... }` via `require_selection_set`
- parse-variables.md: `parse_type_annotation`, `parse_singleton` on `[...]`, `ConstantValue`, `parse_constant_value`, `Box<T>` delegation in `resolve_position`
- parse-descriptions.md: description via two `consume_token_if`
- parse-pointers.md: `to` via `require_token(Identifier)` and `token_text`

A feature is reviewed against this doc when it lands. Amendment sites: the `ItemCursor` and `ChunkStream` impls, `parse_chunk`, `parse_items`, and `parse_singleton`. This doc stays in `refactors/pending`.
