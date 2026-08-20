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
    VariableDeclarationOrUsage,
    #[error("a type, like 'String', 'String!', or '[String]'")]
    TypeAnnotation,
    #[error("a constant value; variables are not allowed here")]
    ConstantValue,
    #[error("the end of the type")]
    EndOfType,
```

## Change 3: `ItemCursor::parse_nested_singleton`

`parse_singleton` takes `text`, `tokens`, and `errors`. A nested `[...]` is parsed from a cursor that already holds those. This method forwards.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
impl<'a> ItemCursor<'a> {
    pub(crate) fn parse_nested_singleton<T>(
        &mut self,
        level: &'a WithSpan<ChunkedLevel>,
        end: Expectation,
        extra_chunks: impl FnOnce(&'a WithSpan<Chunk>) -> WithSpan<ParseError>,
        parse: impl FnOnce(&mut ItemCursor<'_>) -> Result<T, WithSpan<ParseError>>,
    ) -> Singleton<Slot<T, UnparsedChunkItems>, ExtraChunks> {
        parse_singleton(
            level,
            self.text,
            self.tokens,
            self.errors,
            end,
            extra_chunks,
            parse,
        )
    }
}
```

`parse_singleton` is `pub(crate)` so this body can call it. Empty is still the caller: `parse_singleton` indexes chunk 0.

## New module: variables.rs

Origin for the declaration: `crates/isograph_lang_types/src/declarations/variable_declaration.rs` (`VariableDeclaration`, field `type_`, `default_value`). Origin for the annotation shape written here: GraphQL named / list / `!`, not isograph's post-conversion `TypeAnnotationDeclaration` (`Scalar` / `Union` / `Plural`). Delta: i2 names the type `VariableDeclarationOrUsage`; each declaration sits in a `Slot`; `VariableDeclarationOrUsageList` wraps the vec (isograph stores `Vec<VariableDeclaration>` on the field); `!` is not a `NonNull` variant; leftover inside `[...]` is stored on `ListTypeAnnotation` (see below).

```rust
// from crates/isograph_parser/src/variables.rs
use intern::string_key::Intern;
use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    BracketKind, ChunkedLevel, ClientFieldDeclarationPath, ConstantValue, EntityNameWrapper,
    Expectation, Found, IsographResolutionNode, NonBracketTokenKind, ParseError, SemanticToken,
    Slot, UnparsedChunkItems, VariableNameWrapper, parse_constant_value,
};

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ClientFieldDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclarationOrUsageList(
    #[resolve_field] pub Vec<WithSpan<Slot<VariableDeclarationOrUsage, UnparsedChunkItems>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableDeclarationOrUsageSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclarationOrUsage {
    #[resolve_field]
    #[parent_variant(Declaration)]
    pub name: WithSpan<VariableNameWrapper>,
    #[resolve_field]
    #[parent_variant(Variable)]
    pub type_: WithSpan<TypeAnnotation>,
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
    #[parent_variant(NamedTypeAnnotation)]
    pub name: WithSpan<EntityNameWrapper>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ListTypeAnnotation {
    #[resolve_field]
    #[parent_variant(List)]
    pub inner: Option<WithSpan<TypeAnnotation>>,
    #[resolve_field]
    #[parent_from]
    pub extra_tokens: Option<WithSpan<UnparsedChunkItems>>,
}

#[derive(Debug)]
pub enum TypeAnnotationParent<'a> {
    Variable(VariableDeclarationOrUsagePath<'a>),
    List(Box<ListTypeAnnotationPath<'a>>),
}

pub type VariableDeclarationOrUsageListPath<'a> =
    PositionResolutionPath<&'a VariableDeclarationOrUsageList, ClientFieldDeclarationPath<'a>>;

pub type VariableDeclarationOrUsageSlotPath<'a> = PositionResolutionPath<
    &'a Slot<VariableDeclarationOrUsage, UnparsedChunkItems>,
    VariableDeclarationOrUsageListPath<'a>,
>;

pub type VariableDeclarationOrUsagePath<'a> =
    PositionResolutionPath<&'a VariableDeclarationOrUsage, VariableDeclarationOrUsageSlotPath<'a>>;

pub type NamedTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a NamedTypeAnnotation, TypeAnnotationParent<'a>>;

pub type ListTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a ListTypeAnnotation, TypeAnnotationParent<'a>>;
```

A position on `$` answers `VariableDeclarationOrUsage`. `parse_type_annotation`'s `spanning` covers a trailing `!`. A position on `!` answers `NamedTypeAnnotation` or `ListTypeAnnotation`. There is no `Exclamation` field and no `NonNull` variant.

`TypeAnnotation` is a field of `VariableDeclarationOrUsage` and of `ListTypeAnnotation`. A vanilla `Slot<T, E>` pin's `T::Parent` is the slot path, so `TypeAnnotation` cannot be that `T` while also having `TypeAnnotationParent`. `ListTypeAnnotation` stores the singleton's `item` and `extra_tokens` as its own fields. `inner: None` is an empty or failed `[...]`. Extra chunks after the first are `EndOfType` diagnostics from `parse_nested_singleton`; they are not stored.

`TypeAnnotationParent::List` is boxed to break the cycle.

`EntityNameWrapper` gains `NamedTypeAnnotation`. Before, `EntityNameWrapperParent` is `EntrypointDeclaration | ClientFieldDeclaration`. After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub enum EntityNameWrapperParent<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
    NamedTypeAnnotation(NamedTypeAnnotationPath<'a>),
}
```

`VariableNameWrapper` gains a declaration parent. Before:

```rust
// from crates/isograph_parser/src/arguments.rs
pub type VariableNameWrapperPath<'a> =
    PositionResolutionPath<&'a VariableNameWrapper, VariableUsePath<'a>>;
```

After. Origin: `VariableNameWrapperParentType` in isograph has only the declaration. Delta: i2 also resolves `$foo` uses, so the enum has both.

```rust
// from crates/isograph_parser/src/arguments.rs
#[derive(Debug)]
pub enum VariableNameWrapperParent<'a> {
    Use(VariableUsePath<'a>),
    Declaration(VariableDeclarationOrUsagePath<'a>),
}

pub type VariableNameWrapperPath<'a> =
    PositionResolutionPath<&'a VariableNameWrapper, VariableNameWrapperParent<'a>>;
```

`VariableUse`'s `name` field respells to `#[resolve_field]` + `#[parent_variant(Use)]`.

```rust
// from crates/isograph_parser/src/chunk.rs
    pins = [
        (<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>),
        (<SelectionFieldArgument, UnparsedChunkItems>, ArgumentListPath<'a>),
        (<ObjectEntry, UnparsedChunkItems>, ObjectLiteralPath<'a>),
        (<Selection, UnparsedChunkItems>, SelectionSetPath<'a>),
        (<VariableDeclarationOrUsage, UnparsedChunkItems>, VariableDeclarationOrUsageListPath<'a>),
    ]
```

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum UnparsedChunkItemsParent<'a> {
    IsoLiteralSlot(IsoLiteralSlotPath<'a>),
    SelectionFieldArgumentSlot(SelectionFieldArgumentSlotPath<'a>),
    ObjectEntrySlot(ObjectEntrySlotPath<'a>),
    SelectionSlot(SelectionSlotPath<'a>),
    VariableDeclarationOrUsageSlot(VariableDeclarationOrUsageSlotPath<'a>),
    ListTypeAnnotation(ListTypeAnnotationPath<'a>),
}
```

`From<VariableDeclarationOrUsageSlotPath>` and `From<ListTypeAnnotationPath>` into `UnparsedChunkItemsParent`.

```rust
// from crates/isograph_parser/src/variables.rs
impl<'a> From<VariableDeclarationOrUsageSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: VariableDeclarationOrUsageSlotPath<'a>) -> Self {
        IsographResolutionNode::VariableDeclarationOrUsageSlot(path)
    }
}
```

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    VariableDeclarationOrUsageSlot(VariableDeclarationOrUsageSlotPath<'a>),
```

`ConstantValue` and `parse_constant_value` land in arguments.rs. The constant-value ladder is the value ladder without the `$` arm; `$` is `expected(Expectation::ConstantValue)`.

```rust
// from crates/isograph_parser/src/arguments.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum ConstantValue {
    String(StringLiteralValueWrapper),
    Integer(IntegerValue),
    Boolean(BooleanValue),
    Null(NullValue),
    Object(ConstantObjectLiteral),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ConstantObjectLiteral(
    #[resolve_field] pub Vec<WithSpan<Slot<ConstantObjectEntry, UnparsedChunkItems>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ConstantObjectEntrySlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ConstantObjectEntry {
    #[resolve_field]
    pub name: WithSpan<ValueKeyNameWrapper>,
    #[resolve_field]
    #[parent_variant(ConstantObjectEntry)]
    pub value: WithSpan<ConstantValue>,
}

#[derive(Debug)]
pub enum ConstantValueParent<'a> {
    VariableDefault(VariableDeclarationOrUsagePath<'a>),
    ConstantObjectEntry(Box<ConstantObjectEntryPath<'a>>),
}
```

`ValueKeyNameWrapper` gains a second parent:

```rust
// from crates/isograph_parser/src/arguments.rs
#[derive(Debug)]
pub enum ValueKeyNameWrapperParent<'a> {
    ObjectEntry(ObjectEntryPath<'a>),
    ConstantObjectEntry(ConstantObjectEntryPath<'a>),
}
```

`ObjectEntry.name` respells to `#[parent_variant(ObjectEntry)]`. The constant-object pin:

```
(<ConstantObjectEntry, UnparsedChunkItems>, ConstantObjectLiteralPath<'a>)
```

`UnparsedChunkItemsParent::ConstantObjectEntrySlot`. `From` into `IsographResolutionNode::ConstantObjectEntrySlot`. `IntegerValue` / `BooleanValue` / `NullValue` / `StringLiteralValueWrapper` gain `ConstantValueParent` as a second parent via a shared enum, or they stay on `NonConstantValueParent` and `ConstantValue` uses the same payload types with a new parent enum that includes both trees.

Payload types used by both `NonConstantValue` and `ConstantValue` need a parent enum that names both:

```rust
// from crates/isograph_parser/src/arguments.rs
#[derive(Debug)]
pub enum IntegerValueParent<'a> {
    NonConstant(NonConstantValueParent<'a>),
    Constant(ConstantValueParent<'a>),
}
```

The same for `BooleanValue`, `NullValue`, `StringLiteralValueWrapper`. Each payload's `parent_type` becomes that enum. `NonConstantValue::Integer` and `ConstantValue::Integer` mark `#[parent_variant(NonConstant)]` / `#[parent_variant(Constant)]`.

constant-value.md deletes this second tree.

```rust
// from crates/isograph_parser/src/variables.rs
pub(crate) fn consume_variable_declaration_list(
    cursor: &mut ItemCursor<'_>,
) -> Option<WithSpan<VariableDeclarationOrUsageList>> {
    cursor.consume_group_if(
        BracketKind::Parenthesis,
        SemanticToken::Parenthesis,
        |cursor, children| {
            VariableDeclarationOrUsageList(children.item.parse_each_chunk(
                cursor,
                Expectation::Separator(BracketKind::Parenthesis),
                parse_variable_declaration,
            ))
        },
    )
}

fn parse_variable_declaration(
    cursor: &mut ItemCursor<'_>,
) -> Result<VariableDeclarationOrUsage, WithSpan<ParseError>> {
    cursor
        .require_token(NonBracketTokenKind::Dollar, SemanticToken::Variable)
        .map_err(|()| cursor.expected(Expectation::VariableDeclarationOrUsage))?;
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Variable)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Colon, SemanticToken::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Colon)))?;
    let type_ = parse_type_annotation(cursor)?;
    let default_value = match cursor.consume_token_if(NonBracketTokenKind::Equals, SemanticToken::Equals)
    {
        Some(_) => parse_constant_value(cursor)?.wrap_some(),
        None => None,
    };
    VariableDeclarationOrUsage {
        name: name.interned().map(VariableNameWrapper),
        type_,
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
            cursor.consume_token_if(
                NonBracketTokenKind::Exclamation,
                SemanticToken::GraphQLTypeName,
            );
            return TypeAnnotation::Named(NamedTypeAnnotation {
                name: name.interned().map(EntityNameWrapper),
            })
            .wrap_ok();
        }
        if let Some(parsed) = cursor.consume_group_if(
            BracketKind::Bracket,
            SemanticToken::GraphQLTypeName,
            |cursor, children| parse_bracket_interior_type(cursor, children),
        ) {
            let parsed = parsed.item?;
            cursor.consume_token_if(
                NonBracketTokenKind::Exclamation,
                SemanticToken::GraphQLTypeName,
            );
            return TypeAnnotation::List(
                ListTypeAnnotation {
                    inner: parsed.item,
                    extra_tokens: parsed.extra_tokens,
                }
                .boxed(),
            )
            .wrap_ok();
        }
        cursor.expected(Expectation::TypeAnnotation).wrap_err()
    })
}

struct BracketInteriorType {
    item: Option<WithSpan<TypeAnnotation>>,
    extra_tokens: Option<WithSpan<UnparsedChunkItems>>,
}

fn parse_bracket_interior_type(
    cursor: &mut ItemCursor<'_>,
    level: &WithSpan<ChunkedLevel>,
) -> Result<BracketInteriorType, WithSpan<ParseError>> {
    if level.item.len() == 0 {
        return ParseError::expected(Expectation::TypeAnnotation, Found::EndOfChunk)
            .with_span(Span::new(level.location.end, level.location.end))
            .wrap_err();
    }
    let singleton = cursor.parse_nested_singleton(
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
    BracketInteriorType {
        item: singleton.item.item,
        extra_tokens: singleton.item.extra_tokens,
    }
    .wrap_ok()
}
```

`parse_type_annotation` on empty `[]` returns `Err`, so the enclosing `VariableDeclarationOrUsage` is `item: None`.

`[Pet,]` is one chunk plus a boundary comma: `inner: Some(Pet)` plus `Expected(EndOfType, Token(Comma))` at the comma, from `parse_singleton`. The variable declaration parses.

`[Pet\n!]` is two chunks. Chunk 0 parses as `Named(Pet)`. Chunk 1 is `EndOfType` at `!`. `ListTypeAnnotation.inner` is `Some(Named(Pet))`. `ListTypeAnnotation` does not store `Singleton.extra_chunks`. The diagnostic is on the `!` span.

`[Pet junk]` is leftover items in chunk 0: `inner: Some(Pet)`, `extra_tokens: Some(junk)`.

`BracketInteriorType` is a parse-only struct, not in the tree.

## Changes to parse_iso_literal.rs

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct ClientFieldDeclaration {
    #[resolve_field]
    #[parent_variant(ClientFieldDeclaration)]
    pub parent_type: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(ClientFieldDeclaration)]
    pub client_field_name: WithSpan<ClientScalarSelectableNameWrapper>,
    #[resolve_field]
    #[parent_variant(ClientFieldDeclaration)]
    pub selection_set: WithSpan<SelectionSet>,
}
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct ClientFieldDeclaration {
    #[resolve_field]
    #[parent_variant(ClientFieldDeclaration)]
    pub parent_type: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(ClientFieldDeclaration)]
    pub client_field_name: WithSpan<ClientScalarSelectableNameWrapper>,
    #[resolve_field]
    pub variable_definitions: Option<WithSpan<VariableDeclarationOrUsageList>>,
    #[resolve_field]
    #[parent_variant(ClientFieldDeclaration)]
    pub selection_set: WithSpan<SelectionSet>,
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let variable_definitions = consume_variable_declaration_list(cursor);
    let selection_set = require_selection_set(cursor)?;
```

`lib.rs` adds `mod variables;` and `pub use variables::*;`.

`first_item` on `Chunk` is already `pub(crate)` for tests; `parse_nested_singleton`'s extra-chunks closure uses it.

## The resolution surface

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    VariableDeclarationOrUsageList(VariableDeclarationOrUsageListPath<'a>),
    VariableDeclarationOrUsage(VariableDeclarationOrUsagePath<'a>),
    NamedTypeAnnotation(NamedTypeAnnotationPath<'a>),
    ListTypeAnnotation(ListTypeAnnotationPath<'a>),
```

`EntityNameWrapper` already has a resolution-node variant. A type name answers that variant with `EntityNameWrapperParent::NamedTypeAnnotation`.

The boxed recursive field uses the `Box<T>` blanket. `VariableDeclarationOrUsageList` expands like `SelectionSet`. `VariableDeclarationOrUsage` like `SelectionFieldArgument`. `TypeAnnotation` like `NonConstantValue`.

## Tests

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn variables_of(parse: &WithSpan<IsoLiteralParse>) -> &WithSpan<VariableDeclarationOrUsageList> {
        as_field(parse)
            .variable_definitions
            .as_ref()
            .expect("the fixture's declaration carries variable definitions")
    }

    fn as_declared(slot: &Slot<VariableDeclarationOrUsage, UnparsedChunkItems>) -> &VariableDeclarationOrUsage {
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
        match declared.type_.item.reference() {
            TypeAnnotation::Named(named) => {
                assert_eq!(named.name.location, span_of(text, "ID"));
                assert_eq!(declared.type_.location, span_of(text, "ID !"));
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
        assert_eq!(declared.type_.location, span_of(text, "[Pet!]!"));
        let list = match declared.type_.item.reference() {
            TypeAnnotation::List(list) => list.as_ref(),
            annotation => panic!("expected a list type, got {annotation:?}"),
        };
        let inner = list.inner.as_ref().expect("the list holds an element type");
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
    fn a_line_break_inside_a_list_type_does_not_attach_bang() {
        let text = "field Query.Foo($pets: [Pet\n!]) { bar }";
        let (parse, errors) = parsed(text);
        let declared = as_declared(variables_of(parse.reference()).item.0[0].item.reference());
        match declared.type_.item.reference() {
            TypeAnnotation::List(list) => {
                let inner = list.inner.as_ref().expect("chunk 0 parsed Pet");
                assert!(matches!(inner.item, TypeAnnotation::Named(_)));
                assert_eq!(inner.location, span_of(text, "Pet"));
            }
            annotation => panic!("expected a list type, got {annotation:?}"),
        }
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
            IsographResolutionNode::EntityNameWrapper(name) => {
                let list = match name.parent {
                    EntityNameWrapperParent::NamedTypeAnnotation(named) => {
                        match named.parent.reference() {
                            TypeAnnotationParent::List(list) => list.as_ref(),
                            parent => panic!("expected a list parent, got {parent:?}"),
                        }
                    }
                    parent => panic!("expected a named type annotation, got {parent:?}"),
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
```

## Landing checklist

1. The `Box<T>` blanket; `cargo test -p resolve_position` passes.
2. `parse_nested_singleton`, variables.rs, `ConstantValue`, the `ClientFieldDeclaration` slot, the resolution-node variants, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
3. Move this doc to refactors/past.
