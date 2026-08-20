# parse-variables: variable declarations and type annotations

Field declarations gain variable-declaration lists. Type annotations land here; parse-pointers.md reuses them for `to` targets. Defaults reuse parse-arguments.md's value grammar with variables rejected.

## The grammar this doc accepts

```
field <Identifier> . <Identifier> [<paren group>] <brace group>
```

Each contentful chunk of the paren group's interior is one variable declaration:

```
$ <Identifier> : <type> [= <value>]
```

A type is a name with an optional `!`, or a bracket group holding exactly one type, with an optional `!` after the group:

```
Pet    Pet!    [Pet]    [Pet!]!    [[Pet]]
```

A default value is a `ConstantValue`: the same scalar and object forms as a value, with `$` rejected at the `$`.

## Change 1: `Box` delegation in resolve_position

```rust
// from crates/resolve_position/src/lib.rs
impl<T: ResolvePosition> ResolvePosition for Box<T> {
    type Parent<'a>
        = T::Parent<'a>
    where
        Self: 'a;
    type ResolvedNode<'a>
        = T::ResolvedNode<'a>
    where
        Self: 'a;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: Span) -> Self::ResolvedNode<'a> {
        (**self).resolve(parent, position)
    }
}
```

## Change 2: `Expectation`

```rust
// from crates/isograph_parser/src/parse_error.rs
    #[error("a variable declaration, like '$id: ID!'")]
    VariableDeclaration,
    #[error("a type, like 'String', 'String!', or '[String]'")]
    TypeAnnotation,
    #[error("a constant value; variables are not allowed here")]
    ConstantValue,
    #[error("the end of the type")]
    EndOfType,
```

## Change 3: `ItemCursor::buffers`

`parse_singleton` at the root takes `text`, `tokens`, and `errors`. A nested `[...]` type has those on the parent cursor.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
impl<'a> ItemCursor<'a> {
    pub(crate) fn buffers(
        &mut self,
    ) -> (
        &'a str,
        &mut Vec<WithSpan<SemanticToken>>,
        &mut Vec<WithSpan<ParseError>>,
    ) {
        (self.text, self.tokens, self.errors)
    }
}
```

```rust
// from crates/isograph_parser/src/chunk.rs
impl ItemCursor<'_> {
    pub(crate) fn parse_singleton<'c, T>(
        &'c mut self,
        level: &'c WithSpan<ChunkedLevel>,
        end: Expectation,
        extra_chunks: impl FnOnce(&'c WithSpan<Chunk>) -> WithSpan<ParseError>,
        parse: impl FnOnce(&mut ItemCursor<'_>) -> Result<T, WithSpan<ParseError>>,
    ) -> Singleton<Slot<T, UnparsedChunkItems>, ExtraChunks> {
        let (text, tokens, errors) = self.buffers();
        parse_singleton(level, text, tokens, errors, end, extra_chunks, parse)
    }
}
```

## New module: variables.rs

```rust
// from crates/isograph_parser/src/variables.rs
use intern::string_key::Intern;
use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    parse_constant_value, BracketKind, ChunkedLevel, ClientFieldDeclarationPath, Expectation,
    Found, IsographResolutionNode, NonBracketTokenKind, ParseError, SemanticToken, Slot,
    UnparsedChunkItems, VariableNameWrapper,
};

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ClientFieldDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclarationList(
    #[resolve_field] pub Vec<WithSpan<Slot<DeclaredVariable, UnparsedChunkItems>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableDeclarationSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct DeclaredVariable {
    #[resolve_field]
    #[parent_variant(Declaration)]
    pub name: WithSpan<VariableNameWrapper>,
    #[resolve_field]
    #[parent_variant(Variable)]
    pub type_annotation: WithSpan<TypeAnnotation>,
    #[resolve_field]
    #[parent_variant(VariableDefault)]
    pub default_value: Option<WithSpan<ConstantValue>>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum TypeAnnotation {
    Named(NamedTypeAnnotation),
    List(Box<ListTypeAnnotation>),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NamedTypeAnnotation {
    #[resolve_field]
    pub name: WithSpan<TypeNameWrapper>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ListTypeAnnotation {
    #[resolve_field]
    #[parent_variant(List)]
    pub inner: WithSpan<Slot<TypeAnnotation, UnparsedChunkItems>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NamedTypeAnnotationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct TypeNameWrapper(common_lang_types::EntityName);

#[derive(Debug)]
pub enum TypeAnnotationParent<'a> {
    Variable(DeclaredVariablePath<'a>),
    List(Box<ListTypeAnnotationPath<'a>>),
}

pub type VariableDeclarationListPath<'a> =
    PositionResolutionPath<&'a VariableDeclarationList, ClientFieldDeclarationPath<'a>>;

pub type VariableDeclarationSlotPath<'a> = PositionResolutionPath<
    &'a Slot<DeclaredVariable, UnparsedChunkItems>,
    VariableDeclarationListPath<'a>,
>;

pub type DeclaredVariablePath<'a> =
    PositionResolutionPath<&'a DeclaredVariable, VariableDeclarationSlotPath<'a>>;

pub type NamedTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a NamedTypeAnnotation, TypeAnnotationParent<'a>>;

pub type ListTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a ListTypeAnnotation, TypeAnnotationParent<'a>>;

pub type TypeNameWrapperPath<'a> =
    PositionResolutionPath<&'a TypeNameWrapper, NamedTypeAnnotationPath<'a>>;

pub type TypeAnnotationSlotPath<'a> =
    PositionResolutionPath<&'a Slot<TypeAnnotation, UnparsedChunkItems>, TypeAnnotationParent<'a>>;
```

A position on `$` answers `DeclaredVariable`. `parse_type_annotation`'s `spanning` covers a trailing `!`. A position on `!` answers `NamedTypeAnnotation` or `ListTypeAnnotation` (the `Foo!` / `[Foo]!` node). Hover uses that node. There is no `Exclamation` field and no `NonNull` variant.

`TypeAnnotation` is the slot item inside `[...]`. The pin parent is `TypeAnnotationParent`. `#[resolve_field]` + `#[parent_variant(List)]` on `inner` wraps the `ListTypeAnnotation` path in `TypeAnnotationParent::List`. Leftover converts the slot path:

```rust
// from crates/isograph_parser/src/chunk.rs
impl<'a> From<TypeAnnotationSlotPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: TypeAnnotationSlotPath<'a>) -> Self {
        UnparsedChunkItemsParent::TypeAnnotationSlot(path)
    }
}
```

`TypeAnnotationParent::List` is boxed to break the cycle.

`VariableNameWrapper` gains a second parent. Before:

```rust
// from crates/isograph_parser/src/arguments.rs
#[resolve_position(parent_type = VariableUsePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableNameWrapper(common_lang_types::VariableName);

pub type VariableNameWrapperPath<'a> =
    PositionResolutionPath<&'a VariableNameWrapper, VariableUsePath<'a>>;
```

```rust
// from crates/isograph_parser/src/arguments.rs
pub struct VariableUse(#[resolve_field] pub WithSpan<VariableNameWrapper>);
```

After:

```rust
// from crates/isograph_parser/src/arguments.rs
#[resolve_position(parent_type = VariableNameWrapperParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableNameWrapper(common_lang_types::VariableName);

#[derive(Debug)]
pub enum VariableNameWrapperParent<'a> {
    Use(VariableUsePath<'a>),
    Declaration(DeclaredVariablePath<'a>),
}

pub type VariableNameWrapperPath<'a> =
    PositionResolutionPath<&'a VariableNameWrapper, VariableNameWrapperParent<'a>>;
```

```rust
// from crates/isograph_parser/src/arguments.rs
pub struct VariableUse(
    #[resolve_field]
    #[parent_variant(Use)]
    pub WithSpan<VariableNameWrapper>,
);
```

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
    self_type_generics = [
        (<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>),
        (<NamedArgument, UnparsedChunkItems>, ArgumentListPath<'a>),
        (<ObjectEntry, UnparsedChunkItems>, ObjectLiteralPath<'a>),
        (<Selection, UnparsedChunkItems>, SelectionSetPath<'a>),
    ]
```

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum UnparsedChunkItemsParent<'a> {
    IsoLiteralSlot(IsoLiteralSlotPath<'a>),
    NamedArgumentSlot(NamedArgumentSlotPath<'a>),
    ObjectEntrySlot(ObjectEntrySlotPath<'a>),
    SelectionSlot(SelectionSlotPath<'a>),
}
```

After. Origin: those two listings. Delta: the `DeclaredVariable`, `TypeAnnotation`, and `NamedConstantObjectEntry` pins and leftover variants.

```rust
// from crates/isograph_parser/src/chunk.rs
    self_type_generics = [
        (<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>),
        (<NamedArgument, UnparsedChunkItems>, ArgumentListPath<'a>),
        (<ObjectEntry, UnparsedChunkItems>, ObjectLiteralPath<'a>),
        (<Selection, UnparsedChunkItems>, SelectionSetPath<'a>),
        (<DeclaredVariable, UnparsedChunkItems>, VariableDeclarationListPath<'a>),
        (<TypeAnnotation, UnparsedChunkItems>, TypeAnnotationParent<'a>),
        (<NamedConstantObjectEntry, UnparsedChunkItems>, ConstantObjectLiteralPath<'a>),
    ]
```

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum UnparsedChunkItemsParent<'a> {
    IsoLiteralSlot(IsoLiteralSlotPath<'a>),
    NamedArgumentSlot(NamedArgumentSlotPath<'a>),
    ObjectEntrySlot(ObjectEntrySlotPath<'a>),
    SelectionSlot(SelectionSlotPath<'a>),
    VariableDeclarationSlot(VariableDeclarationSlotPath<'a>),
    TypeAnnotationSlot(TypeAnnotationSlotPath<'a>),
    NamedConstantObjectEntrySlot(NamedConstantObjectEntrySlotPath<'a>),
}
```

`From` impls for the three new slot paths, matching the existing slot-path `From`s.

```rust
// from crates/isograph_parser/src/variables.rs
impl<'a> From<VariableDeclarationSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: VariableDeclarationSlotPath<'a>) -> Self {
        IsographResolutionNode::VariableDeclarationSlot(path)
    }
}

impl<'a> From<TypeAnnotationSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: TypeAnnotationSlotPath<'a>) -> Self {
        IsographResolutionNode::TypeAnnotationSlot(path)
    }
}
```

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    VariableDeclarationSlot(VariableDeclarationSlotPath<'a>),
    TypeAnnotationSlot(TypeAnnotationSlotPath<'a>),
```

## Change 4: `ConstantValue`

`ConstantValue` and `parse_constant_value` land in arguments.rs. The constant-value ladder is the value ladder without the `$` arm; `$` is `expected(Expectation::ConstantValue)`.

Integer, boolean, null, and string leaves appear under both value enums. Each leaf's parent becomes an enum of those two.

Before:

```rust
// from crates/isograph_parser/src/arguments.rs
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct StringLiteralValueWrapper(common_lang_types::StringLiteralValue);

#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct IntegerValue(pub i64);

#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct BooleanValue(pub Boolean);

#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NullValue;
```

```rust
// from crates/isograph_parser/src/arguments.rs
pub enum FieldArgumentNameWrapperParent<'a> {
    NamedArgument(NamedArgumentPath<'a>),
    ObjectEntry(ObjectEntryPath<'a>),
}

pub enum NonConstantValue {
    Variable(VariableUse),
    String(StringLiteralValueWrapper),
    Integer(IntegerValue),
    Boolean(BooleanValue),
    Null(NullValue),
    Object(ObjectLiteral),
}
```

After. Origin: those listings. Delta: leaf parents become two-variant enums; `NonConstantValue` variants take `#[parent_variant(NonConstant)]`; `FieldArgumentNameWrapperParent` gains `ConstantObjectEntry`.

```rust
// from crates/isograph_parser/src/arguments.rs
#[resolve_position(parent_type = StringLiteralValueWrapperParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct StringLiteralValueWrapper(common_lang_types::StringLiteralValue);

#[resolve_position(parent_type = IntegerValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct IntegerValue(pub i64);

#[resolve_position(parent_type = BooleanValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct BooleanValue(pub Boolean);

#[resolve_position(parent_type = NullValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NullValue;

#[derive(Debug)]
pub enum StringLiteralValueWrapperParent<'a> {
    NonConstant(NonConstantValueParent<'a>),
    Constant(ConstantValueParent<'a>),
}

#[derive(Debug)]
pub enum IntegerValueParent<'a> {
    NonConstant(NonConstantValueParent<'a>),
    Constant(ConstantValueParent<'a>),
}

#[derive(Debug)]
pub enum BooleanValueParent<'a> {
    NonConstant(NonConstantValueParent<'a>),
    Constant(ConstantValueParent<'a>),
}

#[derive(Debug)]
pub enum NullValueParent<'a> {
    NonConstant(NonConstantValueParent<'a>),
    Constant(ConstantValueParent<'a>),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum NonConstantValue {
    Variable(VariableUse),
    #[parent_variant(NonConstant)]
    String(StringLiteralValueWrapper),
    #[parent_variant(NonConstant)]
    Integer(IntegerValue),
    #[parent_variant(NonConstant)]
    Boolean(BooleanValue),
    #[parent_variant(NonConstant)]
    Null(NullValue),
    Object(ObjectLiteral),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum ConstantValue {
    #[parent_variant(Constant)]
    String(StringLiteralValueWrapper),
    #[parent_variant(Constant)]
    Integer(IntegerValue),
    #[parent_variant(Constant)]
    Boolean(BooleanValue),
    #[parent_variant(Constant)]
    Null(NullValue),
    Object(ConstantObjectLiteral),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ConstantObjectLiteral(
    #[resolve_field] pub Vec<WithSpan<Slot<NamedConstantObjectEntry, UnparsedChunkItems>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NamedConstantObjectEntrySlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NamedConstantObjectEntry {
    #[resolve_field]
    #[parent_variant(ConstantObjectEntry)]
    pub name: WithSpan<FieldArgumentNameWrapper>,
    #[resolve_field]
    #[parent_variant(ConstantObjectEntry)]
    pub value: WithSpan<ConstantValue>,
}

#[derive(Debug)]
pub enum ConstantValueParent<'a> {
    VariableDefault(DeclaredVariablePath<'a>),
    ConstantObjectEntry(Box<NamedConstantObjectEntryPath<'a>>),
}

#[derive(Debug)]
pub enum FieldArgumentNameWrapperParent<'a> {
    NamedArgument(NamedArgumentPath<'a>),
    ObjectEntry(ObjectEntryPath<'a>),
    ConstantObjectEntry(NamedConstantObjectEntryPath<'a>),
}

pub type ConstantObjectLiteralPath<'a> =
    PositionResolutionPath<&'a ConstantObjectLiteral, ConstantValueParent<'a>>;

pub type NamedConstantObjectEntrySlotPath<'a> = PositionResolutionPath<
    &'a Slot<NamedConstantObjectEntry, UnparsedChunkItems>,
    ConstantObjectLiteralPath<'a>,
>;

pub type NamedConstantObjectEntryPath<'a> = PositionResolutionPath<
    &'a NamedConstantObjectEntry,
    NamedConstantObjectEntrySlotPath<'a>,
>;
```

```rust
// from crates/isograph_parser/src/arguments.rs
impl<'a> From<NamedConstantObjectEntrySlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: NamedConstantObjectEntrySlotPath<'a>) -> Self {
        IsographResolutionNode::NamedConstantObjectEntrySlot(path)
    }
}
```

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    NamedConstantObjectEntrySlot(NamedConstantObjectEntrySlotPath<'a>),
    ConstantObjectLiteral(ConstantObjectLiteralPath<'a>),
    NamedConstantObjectEntry(NamedConstantObjectEntryPath<'a>),
```

Path aliases for the leaf-parent enums replace `NonConstantValueParent` on `StringLiteralValueWrapperPath`, `IntegerValuePath`, `BooleanValuePath`, and `NullValuePath`.

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn parse_constant_value(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<ConstantValue>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if cursor
            .consume_token_if(NonBracketTokenKind::Dollar, SemanticToken::Variable)
            .is_some()
        {
            return cursor.expected(Expectation::ConstantValue).wrap_err();
        }
        if let Some(span) =
            cursor.consume_token_if(NonBracketTokenKind::StringLiteral, SemanticToken::String)
        {
            return ConstantValue::String(span.interned().map(StringLiteralValueWrapper).item)
                .wrap_ok();
        }
        if let Some(span) =
            cursor.consume_token_if(NonBracketTokenKind::IntegerLiteral, SemanticToken::Integer)
        {
            let value = match span.token_text().parse() {
                Ok(value) => value,
                Err(_) => {
                    return ParseError::IntegerDoesNotFitI64
                        .with_span(span.location)
                        .wrap_err();
                }
            };
            return ConstantValue::Integer(IntegerValue(value)).wrap_ok();
        }
        if let Some(span) = cursor.consume_token_if(
            NonBracketTokenKind::Identifier,
            SemanticToken::BooleanOrNull,
        ) {
            return match span.token_text() {
                "true" => ConstantValue::Boolean(BooleanValue(Boolean::True)).wrap_ok(),
                "false" => ConstantValue::Boolean(BooleanValue(Boolean::False)).wrap_ok(),
                "null" => ConstantValue::Null(NullValue).wrap_ok(),
                _ => ParseError::expected(
                    Expectation::ConstantValue,
                    Found::Token(NonBracketTokenKind::Identifier),
                )
                .with_span(span.location)
                .wrap_err(),
            };
        }
        if let Some(group) = cursor.consume_group_if(BracketKind::Brace, SemanticToken::Brace) {
            let object = ConstantObjectLiteral(group.item.children.item.parse_each_chunk(
                cursor,
                Expectation::Separator(BracketKind::Brace),
                parse_constant_object_entry,
            ));
            cursor.record_group_close(group.item, SemanticToken::Brace);
            return ConstantValue::Object(object).wrap_ok();
        }
        cursor.expected(Expectation::ConstantValue).wrap_err()
    })
}

fn parse_constant_object_entry(
    cursor: &mut ItemCursor<'_>,
) -> Result<NamedConstantObjectEntry, WithSpan<ParseError>> {
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::ObjectKey)
        .map_err(|()| cursor.expected(Expectation::ObjectEntry))?;
    cursor
        .require_token(NonBracketTokenKind::Colon, SemanticToken::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Colon)))?;
    let value = parse_constant_value(cursor)?;
    NamedConstantObjectEntry {
        name: name.interned().map(FieldArgumentNameWrapper),
        value,
    }
    .wrap_ok()
}
```

The `$` arm consumes the dollar (so the span is the `$`) and then `expected(ConstantValue)`.

## The parsers

```rust
// from crates/isograph_parser/src/variables.rs
pub(crate) fn consume_variable_declaration_list(
    cursor: &mut ItemCursor<'_>,
) -> Option<WithSpan<VariableDeclarationList>> {
    let group = cursor.consume_group_if(BracketKind::Parenthesis, SemanticToken::Parenthesis)?;
    let list = VariableDeclarationList(group.item.children.item.parse_each_chunk(
        cursor,
        Expectation::Separator(BracketKind::Parenthesis),
        parse_variable_declaration,
    ));
    cursor.record_group_close(group.item, SemanticToken::Parenthesis);
    list.with_span(group.location).wrap_some()
}

fn parse_variable_declaration(
    cursor: &mut ItemCursor<'_>,
) -> Result<DeclaredVariable, WithSpan<ParseError>> {
    cursor
        .require_token(NonBracketTokenKind::Dollar, SemanticToken::Variable)
        .map_err(|()| cursor.expected(Expectation::VariableDeclaration))?;
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Variable)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Colon, SemanticToken::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Colon)))?;
    let type_annotation = parse_type_annotation(cursor)?;
    let default_value = match cursor.consume_token_if(NonBracketTokenKind::Equals, SemanticToken::Equals)
    {
        Some(_) => parse_constant_value(cursor)?.wrap_some(),
        None => None,
    };
    DeclaredVariable {
        name: name.interned().map(VariableNameWrapper),
        type_annotation,
        default_value,
    }
    .wrap_ok()
}

pub(crate) fn parse_type_annotation(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<TypeAnnotation>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if let Some(name) =
            cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::GraphQLTypeName)
        {
            cursor.consume_token_if(NonBracketTokenKind::Exclamation, SemanticToken::GraphQLTypeName);
            return TypeAnnotation::Named(NamedTypeAnnotation {
                name: name.interned().map(TypeNameWrapper),
            })
            .wrap_ok();
        }
        if let Some(group) =
            cursor.consume_group_if(BracketKind::Bracket, SemanticToken::GraphQLTypeName)
        {
            let inner = parse_bracket_interior_type(cursor, group.item.children.reference())?;
            cursor.record_group_close(group.item, SemanticToken::GraphQLTypeName);
            cursor.consume_token_if(NonBracketTokenKind::Exclamation, SemanticToken::GraphQLTypeName);
            return TypeAnnotation::List(ListTypeAnnotation { inner }.boxed()).wrap_ok();
        }
        cursor.expected(Expectation::TypeAnnotation).wrap_err()
    })
}

fn parse_bracket_interior_type(
    cursor: &mut ItemCursor<'_>,
    level: &WithSpan<ChunkedLevel>,
) -> Result<WithSpan<Slot<TypeAnnotation, UnparsedChunkItems>>, WithSpan<ParseError>> {
    if level.item.len() == 0 {
        return ParseError::expected(Expectation::TypeAnnotation, Found::EndOfChunk)
            .with_span(Span::new(level.location.end, level.location.end))
            .wrap_err();
    }
    if level.item.len() > 1 {
        let extra = level.item.0[1].reference();
        return ParseError::expected(
            Expectation::EndOfType,
            Found::from(extra.item.first_item().item.reference()),
        )
        .with_span(extra.location)
        .wrap_err();
    }
    let singleton = cursor.parse_singleton(
        level,
        Expectation::EndOfType,
        |extra| {
            ParseError::expected(
                Expectation::EndOfType,
                Found::from(extra.item.first_item().item.reference()),
            )
            .with_span(extra.location)
        },
        |cursor| parse_type_annotation(cursor).map(|wrapped| wrapped.item),
    );
    singleton.item.wrap_ok()
}
```

`parse_type_annotation` returns `WithSpan<TypeAnnotation>` via `spanning`. The singleton interior maps that to `TypeAnnotation`; `parse_one_chunk` spans the first-chunk attempt again. `ListTypeAnnotation.inner` is that attempt.

`[Pet,]` is one chunk plus a boundary comma: `inner.item: Some(Pet)` plus `errors.push(Expected(EndOfType, Token(Comma)))` at the comma. The variable declaration parses.

`[Pet\n!]` is two chunks. The type is `Err` at `!`. The enclosing variable declaration is `item: None`. The `!` does not attach to `Pet`.

Empty `[]` is `Expected(TypeAnnotation, EndOfChunk)` and fails `parse_type_annotation`, so the enclosing variable declaration is `item: None`.

## Changes to parse_iso_literal.rs

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct ClientFieldDeclaration {
    #[resolve_field]
    #[parent_variant(Field)]
    pub parent_type: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(Field)]
    pub client_field_name: WithSpan<ClientFieldNameWrapper>,
    #[resolve_field]
    #[parent_variant(Field)]
    pub selection_set: WithSpan<SelectionSet>,
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let selection_set = require_selection_set(cursor)?;
    ClientFieldDeclaration {
        parent_type: parent_type.interned().map(EntityNameWrapper),
        client_field_name: client_field_name.interned().map(ClientFieldNameWrapper),
        selection_set,
    }
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct ClientFieldDeclaration {
    #[resolve_field]
    #[parent_variant(Field)]
    pub parent_type: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(Field)]
    pub client_field_name: WithSpan<ClientFieldNameWrapper>,
    #[resolve_field]
    pub variable_definitions: Option<WithSpan<VariableDeclarationList>>,
    #[resolve_field]
    #[parent_variant(Field)]
    pub selection_set: WithSpan<SelectionSet>,
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let variable_definitions = consume_variable_declaration_list(cursor);
    let selection_set = require_selection_set(cursor)?;
    ClientFieldDeclaration {
        parent_type: parent_type.interned().map(EntityNameWrapper),
        client_field_name: client_field_name.interned().map(ClientFieldNameWrapper),
        variable_definitions,
        selection_set,
    }
```

`lib.rs` adds `mod variables;` and `pub use variables::*;`.

## The resolution surface

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    VariableDeclarationList(VariableDeclarationListPath<'a>),
    DeclaredVariable(DeclaredVariablePath<'a>),
    NamedTypeAnnotation(NamedTypeAnnotationPath<'a>),
    ListTypeAnnotation(ListTypeAnnotationPath<'a>),
    TypeNameWrapper(TypeNameWrapperPath<'a>),
```

The boxed recursive field uses the `Box<T>` blanket. `VariableDeclarationList` expands like `SelectionSet`. `DeclaredVariable` like `NamedArgument`. `TypeAnnotation` like `NonConstantValue`. `TypeNameWrapper` like `EntityNameWrapper`.

## Tests

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn variables_of(parse: &WithSpan<IsoLiteralParse>) -> &WithSpan<VariableDeclarationList> {
        as_field(parse)
            .variable_definitions
            .as_ref()
            .expect("the fixture's declaration carries variable definitions")
    }

    fn as_declared(slot: &Slot<DeclaredVariable, UnparsedChunkItems>) -> &DeclaredVariable {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected a declared variable")
    }

    #[test]
    fn a_multi_line_variable_list_parses_in_the_demo_style() {
        let text = "field Query.PetCheckinListRoute(\n  $id: ID !\n) {\n  pets\n}";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let variables = variables_of(parse.reference());
        assert_eq!(variables.item.0.len(), 1);
        let declared = as_declared(variables.item.0[0].item.reference());
        assert_eq!(
            declared.name.item,
            VariableNameWrapper("id".intern().to())
        );
        assert_eq!(declared.name.location, span_of(text, "id"));
        match declared.type_annotation.item.reference() {
            TypeAnnotation::Named(named) => {
                assert_eq!(named.name.location, span_of(text, "ID"));
                assert_eq!(declared.type_annotation.location, span_of(text, "ID !"));
            }
            annotation => panic!("expected a named type, got {annotation:?}"),
        }
    }

    #[test]
    fn list_types_nest_with_non_null_markers() {
        let text = "field Query.Foo($pets: [Pet!]!) { bar }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let declared = as_declared(variables_of(parse.reference()).item.0[0].item.reference());
        assert_eq!(declared.type_annotation.location, span_of(text, "[Pet!]!"));
        let list = match declared.type_annotation.item.reference() {
            TypeAnnotation::List(list) => list.as_ref(),
            annotation => panic!("expected a list type, got {annotation:?}"),
        };
        let inner = list
            .inner
            .item
            .item
            .as_ref()
            .expect("the list holds an element type");
        match inner.item.reference() {
            TypeAnnotation::Named(named) => {
                assert_eq!(named.name.location, span_of(text, "Pet"));
                assert_eq!(inner.location, span_of(text, "Pet!"));
            }
            annotation => panic!("expected the named element type, got {annotation:?}"),
        }
    }

    #[test]
    fn defaults_parse_and_reject_variables_at_any_depth() {
        let text = "field Query.Foo($limit: Int = 10) { bar }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let declared = as_declared(variables_of(parse.reference()).item.0[0].item.reference());
        let default = declared
            .default_value
            .as_ref()
            .expect("the fixture declares a default");
        assert!(matches!(
            default.item,
            ConstantValue::Integer(IntegerValue(10))
        ));

        let shallow = "field Query.Foo($limit: Int = $other) { bar }";
        let (parse, errors) = parsed(shallow);
        assert!(
            variables_of(parse.reference()).item.0[0]
                .item
                .item
                .is_none()
        );
        assert!(errors.iter().any(|error| {
            error.item
                == expected(
                    Expectation::ConstantValue,
                    Found::Token(NonBracketTokenKind::Dollar),
                )
                && error.location == span_of(shallow, "$")
        }));

        let deep = "field Query.Foo($input: Input = { pet: $pet }) { bar }";
        let (parse, errors) = parsed(deep);
        assert!(errors.iter().any(|error| error.location == span_of(deep, "$")));
    }

    #[test]
    fn each_malformed_variable_declaration_degrades_alone() {
        let text = "field Query.Foo($a Int, $b: , id: ID, $c: Float) { bar }";
        let (parse, errors) = parsed(text);
        let variables = variables_of(parse.reference());
        assert_eq!(variables.item.0.len(), 4);
        assert!(variables.item.0[0].item.item.is_none());
        assert!(variables.item.0[1].item.item.is_none());
        assert!(variables.item.0[2].item.item.is_none());
        as_declared(variables.item.0[3].item.reference());
        assert_eq!(errors.len(), 3);
        assert_eq!(errors[0].location, span_of(text, "Int"));
        assert!(errors[1].item == expected(Expectation::TypeAnnotation, Found::EndOfChunk));
        assert_eq!(errors[2].location, span_of(text, "id"));
    }

    #[test]
    fn a_final_comma_inside_a_list_type_is_end_of_type() {
        let text = "field Query.Foo($pets: [Pet,]) { bar }";
        let (parse, errors) = parsed(text);
        as_declared(variables_of(parse.reference()).item.0[0].item.reference());
        assert_eq!(
            errors,
            expected(Expectation::EndOfType, Found::Token(Comma))
                .with_span(span_of(text, ","))
                .wrap_vec(),
        );
    }

    #[test]
    fn a_line_break_inside_a_list_type_does_not_parse() {
        let text = "field Query.Foo($pets: [Pet\n!]) { bar }";
        let (parse, errors) = parsed(text);
        assert!(
            variables_of(parse.reference()).item.0[0]
                .item
                .item
                .is_none()
        );
        assert!(errors.iter().any(|error| {
            error.item
                == expected(
                    Expectation::EndOfType,
                    Found::Token(NonBracketTokenKind::Exclamation),
                )
                && error.location == span_of(text, "!")
        }));
    }

    #[test]
    fn type_names_resolve_through_their_annotation_ancestry() {
        let text = "field Query.Foo($pets: [Pet]) { bar }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "Pet")) {
            IsographResolutionNode::TypeNameWrapper(name) => {
                let list = match name.parent.parent.reference() {
                    TypeAnnotationParent::List(list) => list.as_ref(),
                    parent => panic!("expected a list parent, got {parent:?}"),
                };
                match list.parent.reference() {
                    TypeAnnotationParent::Variable(variable) => {
                        assert_eq!(variable.inner.name.location, span_of(text, "pets"));
                    }
                    parent => panic!("expected the declared variable, got {parent:?}"),
                }
            }
            node => panic!("expected the type name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "pets")) {
            IsographResolutionNode::VariableNameWrapper(name) => {
                assert!(matches!(name.parent, VariableNameWrapperParent::Declaration(_)));
            }
            node => panic!("expected the variable name leaf, got {node:?}"),
        }
    }

    #[test]
    fn a_bang_resolves_to_the_annotation() {
        let text = "field Query.Foo($id: ID!) { bar }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "!")) {
            IsographResolutionNode::NamedTypeAnnotation(annotation) => {
                assert_eq!(annotation.inner.name.location, span_of(text, "ID"));
            }
            node => panic!("expected the named type, got {node:?}"),
        }
    }

    #[test]
    fn a_variable_list_records_parens_dollar_name_colon_and_type() {
        let text = "field Query.Foo($id: ID!) { bar }";
        let (parse, errors, bracket_errors, comma_errors, tokens) = parsed_with_tokens(text);
        assert!(bracket_errors.is_empty());
        assert_eq!(comma_errors, vec![]);
        let parse = parse.expect("the fixture is not an empty literal");
        as_field(parse.reference());
        assert_eq!(errors, vec![]);
        assert_eq!(
            tokens,
            vec![
                SemanticToken::Keyword.with_span(span_of(text, "field")),
                SemanticToken::Type.with_span(span_of(text, "Query")),
                SemanticToken::Period.with_span(span_of(text, ".")),
                SemanticToken::FieldName.with_span(span_of(text, "Foo")),
                SemanticToken::Parenthesis.with_span(span_of(text, "(")),
                SemanticToken::Variable.with_span(span_of(text, "$")),
                SemanticToken::Variable.with_span(span_of(text, "id")),
                SemanticToken::Colon.with_span(span_of(text, ":")),
                SemanticToken::GraphQLTypeName.with_span(span_of(text, "ID")),
                SemanticToken::GraphQLTypeName.with_span(span_of(text, "!")),
                SemanticToken::Parenthesis.with_span(span_of(text, ")")),
                SemanticToken::Brace.with_span(span_of(text, "{")),
                SemanticToken::FieldName.with_span(span_of(text, "bar")),
                SemanticToken::Brace.with_span(span_of(text, "}")),
            ],
        );
    }
```

The resolve walk for `Pet` in `[Pet]`: `TypeNameWrapper` parent is `NamedTypeAnnotationPath`, whose parent is `TypeAnnotationParent::List`. `variable.inner` is `DeclaredVariable`; `DeclaredVariablePath` parent is the slot. `variable.inner.name` is still the name field.

## Landing checklist

1. The `Box<T>` blanket; `cargo test -p resolve_position` passes.
2. `ItemCursor::buffers` / `parse_singleton`, variables.rs, `ConstantValue`, the ClientFieldDeclaration slot, the resolution-node variants, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
3. Move this doc to refactors/past.
