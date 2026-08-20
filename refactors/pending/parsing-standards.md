# Parsing standards

Rules for grammar-stage code. Feature docs define each form. This doc lists the functions those forms call. If an implementation disagrees with this doc, the same review amends the doc or changes the code.

Every call a parse function makes on a chunk is a method or free function listed here. A new call is a new listing here.

## The unit of work

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    errors: &mut Vec<WithSpan<ParseError>>,
    tokens: &mut Vec<WithSpan<SemanticToken>>,
) -> Option<WithSpan<IsoLiteralParse>>
```

`parse_iso_literal` takes `text: &str`, the chunked literal, `errors`, and `tokens`. Each chunk is passed to `Chunk::stream(text, tokens, errors)`, which returns one `ChunkStream`. A group's interior is the `ChunkedLevel` in `group.children`.

A group is one item. `require_group` and `consume_group_if` take a function that parses the inside. They commit the open, run that function against the group's children, wrap the result with the group's span, and record the close when the function returns.

Each token and group has a span. A parse function assigns a span to a value made of more than one item by calling `spanning`. `expected` on an exhausted cursor is `Expected(_, EndOfChunk)` at `end_span`.

## `ItemCursor` and `ChunkStream`

`parse_one_chunk` takes a `ChunkStream` (from `Chunk::stream` at the root, or `parent.stream_chunk` for a nested list) and passes `stream.cursor()` (`&mut ItemCursor`) into the parse function. It then calls `stream.remaining_contents` and builds a `Slot<P, UnparsedChunkItems>`. Diagnostics are not leftover items. Leftover items sit on `Slot.extra_tokens`. Extra chunks sit on `Singleton.extra_chunks`. `Slot.item` is `Some` when the form parsed. `Slot.extra_tokens` is `Some` when extra items are present. Diagnostics go through `report_error` on the child cursor in `parse_one_chunk`, and `errors.push` in `parse_singleton` and `parse_iso_literal`. Inner `parse_*` stays `Result`. Artifact generation requires `errors.is_empty()` and that the earlier-stage lists are empty. `require_end` is a method on `ChunkStream`.

This pass is `IsoLiteralParse`. Resolve walks that tree only. Artifact generation does not resolve.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct TokenText<'a> {
    pub location: Span,
    // Whole literal. A Span plus `cursor.text()` at the call site copies 16 fewer bytes per consume.
    text: &'a str,
}

impl<'a> TokenText<'a> {
    pub(crate) fn text(self) -> &'a str;
    pub(crate) fn interned<T: From<intern::string_key::StringKey>>(self) -> WithSpan<T>;
}

pub(crate) struct ItemCursor<'a> { /* ... */ }

pub(crate) struct ChunkStream<'a>(ItemCursor<'a>);

impl<'a> ChunkStream<'a> {
    pub(crate) fn new(
        contents: &'a NonEmpty<WithSpan<ChunkContentItem>>,
        text: &'a str,
        tokens: &'a mut Vec<WithSpan<SemanticToken>>,
        errors: &'a mut Vec<WithSpan<ParseError>>,
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
    ) -> Option<TokenText<'a>>;
    pub(crate) fn consume_group_if<R>(
        &mut self,
        kind: BracketKind,
        token: SemanticToken,
        parse_inside: impl FnOnce(&mut Self, &'a WithSpan<ChunkedLevel>) -> R,
    ) -> Option<WithSpan<R>>;
    pub(crate) fn expected(&mut self, expected: Expectation) -> WithSpan<ParseError>;
    pub(crate) fn require_token(
        &mut self,
        kind: NonBracketTokenKind,
        token: SemanticToken,
    ) -> Result<TokenText<'a>, ()>;
    pub(crate) fn require_group<R>(
        &mut self,
        kind: BracketKind,
        token: SemanticToken,
        parse_inside: impl FnOnce(&mut Self, &'a WithSpan<ChunkedLevel>) -> R,
    ) -> Result<WithSpan<R>, ()>;
    pub(crate) fn report_error(&mut self, error: WithSpan<ParseError>);
    pub(crate) fn stream_chunk<'c>(&'c mut self, chunk: &'c Chunk) -> ChunkStream<'c>;
    pub(crate) fn text(&self) -> &'a str;
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

- Leaf: the `location` of the `TokenText` from `require_token` or `consume_token_if`, or the `WithSpan` from `require_group` or `consume_group_if`.
- Parsed list item: the `WithSpan` `spanning` returned, stored on `Slot.item` when the form parsed.
- Slot: form `Ok` and end is the item span. Form `Ok` and leftover is the join of the item span and the leftover items' span. Form `Err` is `contents_span`.
- Value made of several items: one `spanning` call. The closure's first advance is a `consume_*` or `require_*`. Remaining items of that value are read inside the same `spanning`.

`TokenText`'s `text` field is the whole literal. `text()` indexes it at `location`. A name in the tree is `token.interned()`. The converted scalar is the `i64`. The wrapper span is location only.

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
    resolved_node = IsographResolutionNode<'a>,
    on_unmatched_span = from_path,
    pins = [
        (<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>),
        (<SelectionFieldArgument, UnparsedChunkItems>, ArgumentListPath<'a>),
        (<ObjectEntry, UnparsedChunkItems>, ObjectLiteralPath<'a>),
    ]
)]
pub struct Slot<T, E> {
    #[resolve_field]
    pub item: Option<WithSpan<T>>,
    #[resolve_field]
    #[parent_from]
    pub extra_tokens: Option<WithSpan<E>>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    resolved_node = IsographResolutionNode<'a>,
    pins = [
        (<Slot<IsoLiteralItem, UnparsedChunkItems>, ExtraChunks>, ()),
    ]
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

Bare `#[resolve_field]` on `item` passes `self.path(parent)`, a path to this `Slot`. `T::Parent` is that path. `on_unmatched_span = from_path` makes the unmatched-span arm `self.path(parent).to()`. Each pin's `From` builds that pin's `ResolvedNode` variant (`IsoLiteralSlot`, `SelectionFieldArgumentSlot`, `ObjectEntrySlot`). `#[parent_from]` on `extra_tokens` passes `From::from(self.path(parent))`. Leftover's parent is an enum of those slot paths. A position in leftover walks `extra_tokens`. A position in the slot span but in neither field answers that `Slot<T, E>`'s `ResolvedNode` variant. `{ item: None, extra_tokens: None }` is the same arm.

Leftover span is tight to the leftover tokens. The gap after the item is a third region: the slot leaf.

Each list that stores a `Slot` appends a pin, a `ResolvedNode` variant whose payload is that `Slot<T, E>`'s path, a `From` into `IsographResolutionNode`, and a `From` into `UnparsedChunkItemsParent`.

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug)]
pub enum UnparsedChunkItemsParent<'a> {
    IsoLiteralSlot(IsoLiteralSlotPath<'a>),
    SelectionFieldArgumentSlot(SelectionFieldArgumentSlotPath<'a>),
    ObjectEntrySlot(ObjectEntrySlotPath<'a>),
}

pub type UnparsedChunkItemsPath<'a> =
    PositionResolutionPath<&'a UnparsedChunkItems, UnparsedChunkItemsParent<'a>>;

impl<'a> From<IsoLiteralSlotPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: IsoLiteralSlotPath<'a>) -> Self {
        UnparsedChunkItemsParent::IsoLiteralSlot(path)
    }
}

impl<'a> From<SelectionFieldArgumentSlotPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: SelectionFieldArgumentSlotPath<'a>) -> Self {
        UnparsedChunkItemsParent::SelectionFieldArgumentSlot(path)
    }
}

impl<'a> From<ObjectEntrySlotPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: ObjectEntrySlotPath<'a>) -> Self {
        UnparsedChunkItemsParent::ObjectEntrySlot(path)
    }
}
```

`IsoLiteralItem`'s `parent_type` is `IsoLiteralSlotPath<'a>`. `EntrypointDeclaration`'s `parent_type` is `IsoLiteralSlotPath<'a>`. `SelectionFieldArgument`'s `parent_type` is `SelectionFieldArgumentSlotPath<'a>`. `ObjectEntry`'s `parent_type` is `ObjectEntrySlotPath<'a>`.

`Singleton` at the root stays pinned (`pins` as above). parse-variables.md calls `parse_nested_singleton` for `[...]` and stores the item and leftover on `ListTypeAnnotation`; it does not pin `Singleton` a second time.

`item: None` and `extra_tokens: None` together is representable and never constructed.

## Lists and one-item levels

`ChunkedLevel`'s vec is private to the `chunk` module. `len` is the chunk count. `parse_each_chunk` maps each chunk through `parse_one_chunk`. `parse_singleton` is a one-item level with at least one chunk: chunk 0 through `parse_one_chunk`, then extra chunks and a boundary comma. Empty is the caller's.

```rust
// from crates/isograph_parser/src/chunk.rs
fn parse_one_chunk<'a, P>(
    chunk: &'a WithSpan<Chunk>,
    mut stream: ChunkStream<'a>,
    leftover: Expectation,
    parse: impl FnOnce(&mut ItemCursor<'_>) -> Result<P, WithSpan<ParseError>>,
) -> WithSpan<Slot<P, UnparsedChunkItems>>
```

```rust
// from crates/isograph_parser/src/chunk.rs
impl ChunkedLevel {
    pub(crate) fn parse_each_chunk<'a, P>(
        &'a self,
        parent: &mut ItemCursor<'_>,
        leftover: Expectation,
        parse_item: impl Fn(&mut ItemCursor<'_>) -> Result<P, WithSpan<ParseError>>,
    ) -> Vec<WithSpan<Slot<P, UnparsedChunkItems>>>
```

```rust
// from crates/isograph_parser/src/chunk.rs
pub(crate) fn parse_singleton<'a, T>(
    level: &'a WithSpan<ChunkedLevel>,
    text: &'a str,
    tokens: &'a mut Vec<WithSpan<SemanticToken>>,
    errors: &'a mut Vec<WithSpan<ParseError>>,
    end: Expectation,
    extra_chunks: impl FnOnce(&'a WithSpan<Chunk>) -> WithSpan<ParseError>,
    parse: impl FnOnce(&mut ItemCursor<'_>) -> Result<T, WithSpan<ParseError>>,
) -> Singleton<Slot<T, UnparsedChunkItems>, ExtraChunks>
```

`parse_one_chunk` leftover and failed-form diagnostics go through `stream.cursor().report_error`. `parse_each_chunk` builds each child stream with `parent.stream_chunk`. `parse_singleton` at the root builds the first stream with `Chunk::stream` and `errors.push` for the boundary comma and extra chunks.

`parse_each_chunk` leftover is `Expectation::Separator(BracketKind::...)` at a selection set (`}`), an argument list (`)`), an object literal (`}`), and a variable-declaration list (`)`). A list trailing comma is legal and is not a diagnostic.

`parse_singleton` leftover and boundary comma use `end` (`EndOfDeclaration` at the root, `EndOfType` inside `[...]`). A boundary comma is a tokenless diagnostic via `errors.push`. Extra chunks clone. Empty is the caller (`parse_iso_literal` pushes `EmptyLiteral` and returns `None`; a `[...]` with zero chunks is `Expected(TypeAnnotation, EndOfChunk)`).

`parse_*` is all-or-nothing. There is no recovered prefix of a selection. `entrypoint Foo.$ asdf` fails at `$`. `item` is `None`. `extra_tokens` is the whole chunk.

A type that contains a group stores `Vec<WithSpan<Slot<P, UnparsedChunkItems>>>` or a `Singleton<Slot<T, UnparsedChunkItems>, ExtraChunks>`. Feature docs write those types.

## Function shapes

- `consume_*`: `ItemCursor` method. Match: `commit` and `Some`. Else: `None`. A group is: Match: `commit`, run `parse_inside` on the children, record close, `Some` of that result with the group's span. Else: `None`.
- `expected`: `ItemCursor` method. Peek, no `commit`. Next item or `EndOfChunk` becomes `Expected(expected, found)`.
- `require_*`: `consume_*` or `Err(())`. The caller maps `Err` with `expected`.
- `parse_*`: implements a form made of several items. Parameter is `&mut ItemCursor`. First `Err` is returned. Shared iteration is `parse_each_chunk`, `parse_singleton`, or `spanning`. A nested list takes the cursor.
- Diagnostic: `report_error` on the child cursor in `parse_one_chunk`; `errors.push` in `parse_singleton` and `parse_iso_literal`. Not stored on the tree.

A group plus its interior is `consume_group_if` or `require_group` with a function that parses the inside:

```rust
    cursor.consume_group_if(BracketKind::Brace, SemanticToken::Brace, |cursor, children| {
        children.item.parse_each_chunk(
            cursor,
            Expectation::Separator(BracketKind::Brace),
            parse_item,
        )
    })
```

```rust
    cursor
        .require_group(BracketKind::Brace, SemanticToken::Brace, |cursor, children| {
            children.item.parse_each_chunk(
                cursor,
                Expectation::Separator(BracketKind::Brace),
                parse_item,
            )
        })
        .map_err(|()| cursor.expected(expectation))?
```

## Dispatch

When the next item may start several forms, peek without `commit`, `drop` the peek, then call a parse function that requires its first token. The last arm is `expected`. If those arms are one value, the match is inside `spanning`.

`parse_non_constant_value`'s object arm is `{ ... }`. The same `name : value` list in `( ... )` is `consume_argument_list`, not a value.

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn parse_non_constant_value(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<NonConstantValue>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if let Some(peek) = cursor.peek() {
            match peek.view().item.reference() {
                ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::Dollar)) => {
                    drop(peek);
                    return NonConstantValue::Variable(VariableUse(parse_variable_name(
                        cursor,
                        Expectation::Token(NonBracketTokenKind::Dollar),
                    )?))
                    .wrap_ok();
                }
                ChunkContentItem::NonBracket(NonBracketToken(
                    NonBracketTokenKind::StringLiteral,
                )) => {
                    drop(peek);
                    return NonConstantValue::String(parse_string_literal(cursor)?).wrap_ok();
                }
                ChunkContentItem::NonBracket(NonBracketToken(
                    NonBracketTokenKind::IntegerLiteral,
                )) => {
                    drop(peek);
                    return NonConstantValue::Integer(parse_integer_value(cursor)?).wrap_ok();
                }
                ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::Identifier)) => {
                    drop(peek);
                    return parse_boolean_or_null(cursor);
                }
                ChunkContentItem::Group(group)
                    if group.opening.item.0 == BracketKind::Brace =>
                {
                    drop(peek);
                    return NonConstantValue::Object(parse_object_literal(cursor)?).wrap_ok();
                }
                _ => {}
            }
        }
        cursor.expected(Expectation::Value).wrap_err()
    })
}
```

`VariableUse` stores the interned name. A position on `$` answers `VariableUse`. There is no `Dollar` field. `string_key_newtype!` implements `From<StringKey>` for the inner lang types. Parser wrappers do not add a second `From`. Construction is `name.interned().map(VariableNameWrapper)`. A selection's name and `reader_alias` are `SelectionNameWrapper` over `SelectableName`. A field declaration's name is `ClientScalarSelectableNameWrapper`. A pointer declaration's name is `ClientObjectSelectableNameWrapper`. The integer arm is `span.text().parse()` on the token `require_token(IntegerLiteral)` just returned. `parse::<i64>()` on an `IntegerLiteral` token (`-?(0|[1-9][0-9]*)`) fails only as overflow or underflow. Variable defaults call this same function.

Keyword text after `require_token(Identifier, token)` or `consume_token_if(Identifier, token)`: `match` on `text()` (`"entrypoint"` / `"field"` / `"pointer"`; `"true"` / `"false"` / `"null"`; `"to"`).

One optional item is `consume_*`. Two optional kinds in one position is two `consume_token_if` calls. The optional `!` after a type name is `consume_token_if(Exclamation, SemanticToken::GraphQLTypeName)`: the next item may be the caller's `=`. `$name` is `parse_variable_name(cursor, missing_dollar)`. After `require_token` on an identifier, `consume_token_if(Colon, SemanticToken::Colon)` is the alias; both arms use the identifier.

```rust
    let first = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
        .map_err(|()| cursor.expected(expectation))?;
    let (alias, name) = match cursor.consume_token_if(NonBracketTokenKind::Colon, SemanticToken::Colon)
    {
        Some(_) => {
            let name = cursor
                .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
                .map_err(|()| {
                    cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier))
                })?;
            (
                first.interned().map(SelectionNameWrapper).wrap_some(),
                name.interned().map(SelectionNameWrapper),
            )
        }
        None => (None, first.interned().map(SelectionNameWrapper)),
    };
```

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
    #[error("a comma, a line break, or {}", .0.closing())]
    Separator(BracketKind),
    #[error("an argument, like 'id: $id'")]
    Argument,
    #[error("a value, like $foo, 42, \"bar\", true, false, null, or an object literal")]
    Value,
    #[error("an object entry, like 'id: 4'")]
    ObjectEntry,
    #[error("a variable declaration, like '$id: ID!'")]
    VariableDeclarationOrUsage,
    #[error("a type, like 'String', 'String!', or '[String]'")]
    TypeAnnotation,
    #[error("the end of the type")]
    EndOfType,
    #[error("the keyword `to`")]
    ToKeyword,
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

impl BracketKind {
    pub fn closing(self) -> &'static str {
        match self {
            BracketKind::Parenthesis => "')'",
            BracketKind::Brace => "'}'",
            BracketKind::Bracket => "']'",
        }
    }
}
```

One global `Expectation`. The listing above is the eventual enum. Variants land with the feature that first constructs them. parse-arguments.md adds `Argument`, `Value`, `ObjectEntry`, `IntegerDoesNotFitI64`, and `Separator(BracketKind)`. parse-selection-sets.md adds `SelectionSet` and `Selection`. parse-variables.md adds `VariableDeclarationOrUsage`, `TypeAnnotation`, and `EndOfType`. parse-pointers.md adds `ToKeyword` and removes `UnsupportedDeclarationType`.

An error is `WithSpan<ParseError>`. The span is the offending item, or empty at `end_span` where the missing item would go. `IntegerDoesNotFitI64` is the `parse::<i64>()` `Err` on an `IntegerLiteral` token.

Diagnostics are not on the tree. Nested lists report as they parse, inner first. Resolve walks leftover and failed items, not diagnostics. Bracket errors are the matcher's vec. Comma-without-item errors are chunking's vec. Grammar diagnostics go through `report_error` on a cursor, or `errors.push` at the root where there is no cursor. Artifact generation runs only when those three lists are empty.

A failed list chunk is `item: None` plus the chunk's items in `Slot.extra_tokens`. Leftover after a successful list item is `item: Some` plus leftover items and `report_error(Expected(Separator, ...))`. Tokenless diagnostics (empty literal, a root comma) go through `errors.push` with no `UnparsedChunkItems`. Extra root chunks are `IsoLiteralParse.extra_chunks` plus `errors.push(MultipleDeclarations)`. `thiserror` formats `ParseError`. Suggestions are produced later from `(expected, found)`.

There is no `errors()` walk on the tree. Tests read the vec `parse_iso_literal` was passed.

## Totality

`parse_iso_literal` returns `None` on an empty chunked literal and a tree otherwise. A position in a parsed region resolves to a grammar leaf. A position in leftover or failed items resolves through `UnparsedChunkItems`. Extra root chunks resolve through `ExtraChunks`. A position on whitespace or a dropped comma or unmatched-bracket region resolves to the nearest containing node. An empty literal has no grammar tree; the diagnostic is `EmptyLiteral`.

Find-references, rename, and go-to-definition run when the resolved leaf is a name leaf. In `foo { bar } asdf`, a position on `asdf` resolves through the leftover `UnparsedChunkItems`; find-references returns no references. Completion reads the resolution path. Diagnostics are a separate list.

## Trees and spans

A tree enum is wrapped in `WithSpan` at its slot. `Slot.item` and `Slot.extra_tokens` are `Option<WithSpan<_>>`. `Singleton.item` is `WithSpan<Slot<...>>`, the `parse_one_chunk` attempt. Each other struct field that is a node is `WithSpan`. A name is a newtype over an interned key in a `WithSpan`; each role is its own type. The converted scalar is the `i64`. A position on `.`, `$`, `!`, `:`, or `to` resolves to the containing node. There is no keyword-marker type. Resolve walks the optimistic tree only.

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
- Keyword / boolean / null text: `token.text()` after an identifier
- Integer conversion: `token.text().parse()` on an `IntegerLiteral` token
- Interned name: `token.interned().map(SelectionNameWrapper)` (the inner lang type implements `From<StringKey>`; the wrapper does not)
- Composite span: `ItemCursor::spanning`
- List of items: `ChunkedLevel::parse_each_chunk` → `Vec<WithSpan<Slot<P, UnparsedChunkItems>>>`
- One-item context: `parse_singleton` → `Singleton<Slot<T, UnparsedChunkItems>, ExtraChunks>`
- Recovered item: `Slot.item` → `Option<WithSpan<T>>`
- Extra items: `Slot.extra_tokens` → `Option<WithSpan<UnparsedChunkItems>>`
- Chunk count: `ChunkedLevel::len`
- First item of an extra chunk: `Chunk::first_item`
- Trailing comma in a one-item context: `errors.push` (tokenless)
- Leftover after a list item: `item: Some` plus extra tokens and `report_error(Expected(Separator, ...))`
- Leftover after a singleton first chunk: `item: Some` plus extra tokens and `report_error(Expected(end, ...))`
- Extra root chunks: `ExtraChunks` plus `errors.push(MultipleDeclarations)`
- Diagnostic: `report_error` on the child cursor in `parse_one_chunk`; `errors.push` in `parse_singleton` / `parse_iso_literal`
- Nested list stream: `ItemCursor::stream_chunk`
- Group interior: `require_group` / `consume_group_if` with a function that parses the inside; close is recorded when that function returns.
- lhs, colon, rhs: `parse_name_colon(cursor, parse_lhs, parse_rhs)` → `(L, R)`
- `$ ident`: `parse_variable_name(cursor, missing_dollar)`

## Shipping and amending

Each grammar feature lands on this surface.

- generic-slot.md: one `ResolvePosition` impl per `Slot<T, E>` pin, `on_unmatched_span = from_path`, one `ResolvedNode` variant per pin
- from-container-parent-field.md: `#[parent_from]` on a struct field
- parse-arguments.md: `Separator(BracketKind)`, the `SelectionFieldArgument` and `ObjectEntry` pins, `UnparsedChunkItemsParent`, `parse_non_constant_value`, `IntegerDoesNotFitI64`, `BooleanValue(Boolean::{True, False})`
- parse-selection-sets.md: selections, selection sets, arguments on selections
- parse-fields.md: `field Type.name { ... }` via `require_selection_set`
- parse-name-colon.md: `parse_name_colon(parse_lhs, parse_rhs)`
- peek-then-parse.md: peek without `commit`, `drop` the peek, parse function requires the first token; `parse_variable_name` requires `$` then the identifier
- token-kind-zst.md: `NonBracketTokenKind` ZST payloads as peek-match proof; consume/require use associated constants
- parse-variables.md: `parse_type_annotation`, `parse_singleton` on `[...]`, `NonConstantValueParent::VariableDefault`, `Box<T>` delegation in `resolve_position`
- parse-descriptions.md: description via two `consume_token_if`
- token-text.md: `TokenText` from `consume_token_if` / `require_token`; `text` and `interned` on that value
- parse-pointers.md: `to` via `require_token(Identifier)` and `text()`

A feature is reviewed against this doc when it lands. Amendment sites: the `ItemCursor` and `ChunkStream` impls, `parse_one_chunk`, `parse_each_chunk`, and `parse_singleton`. This doc stays in `refactors/pending`.
