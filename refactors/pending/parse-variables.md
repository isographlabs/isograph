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
    parse_constant_value, parse_singleton, BracketKind, ChunkedLevel, ClientFieldDeclarationPath,
    Expectation, Found, IsographResolutionNode, NonBracketTokenKind, ParseError, Slot,
    UnparsedChunkItems, UnparsedChunkItemsParent, VariableName,
};

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ClientFieldDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclarationList(
    #[resolve_field] pub Vec<WithSpan<Slot<DeclaredVariable, UnparsedChunkItems>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableDeclarationListPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct DeclaredVariable {
    #[resolve_field]
    #[parent_variant(Declaration)]
    pub name: WithSpan<VariableName>,
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
    pub name: WithSpan<TypeName>,
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
pub struct TypeName(common_lang_types::EntityName);

impl From<intern::string_key::StringKey> for TypeName {
    fn from(key: intern::string_key::StringKey) -> Self {
        TypeName(key.to())
    }
}

#[derive(Debug)]
pub enum TypeAnnotationParent<'a> {
    Variable(DeclaredVariablePath<'a>),
    List(Box<ListTypeAnnotationPath<'a>>),
}

pub type VariableDeclarationListPath<'a> =
    PositionResolutionPath<&'a VariableDeclarationList, ClientFieldDeclarationPath<'a>>;

pub type DeclaredVariablePath<'a> =
    PositionResolutionPath<&'a DeclaredVariable, VariableDeclarationListPath<'a>>;

pub type NamedTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a NamedTypeAnnotation, TypeAnnotationParent<'a>>;

pub type ListTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a ListTypeAnnotation, TypeAnnotationParent<'a>>;

pub type TypeNamePath<'a> = PositionResolutionPath<&'a TypeName, NamedTypeAnnotationPath<'a>>;
```

A position on `$` answers `DeclaredVariable`. `parse_type_annotation`'s `spanning` covers a trailing `!`. A position on `!` answers `NamedTypeAnnotation` or `ListTypeAnnotation` (the `Foo!` / `[Foo]!` node). Hover uses that node. There is no `Exclamation` field and no `NonNull` variant.

`TypeAnnotation` is the slot item inside `[...]`. `Slot<TypeAnnotation, UnparsedChunkItems>::Parent` is `TypeAnnotation::Parent`, which is `TypeAnnotationParent`. `#[resolve_field]` + `#[parent_variant(List)]` on `inner` wraps the `ListTypeAnnotation` path in `TypeAnnotationParent::List`. Leftover `parent_from` wraps that same parent:

```rust
// from crates/isograph_parser/src/chunk.rs
impl<'a> From<TypeAnnotationParent<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(parent: TypeAnnotationParent<'a>) -> Self {
        UnparsedChunkItemsParent::TypeAnnotation(parent)
    }
}
```

`TypeAnnotationParent::List` is boxed to break the cycle.

`VariableName` gains a second parent. Before:

```rust
// from crates/isograph_parser/src/arguments.rs
pub type VariableNamePath<'a> = PositionResolutionPath<&'a VariableName, VariableUsePath<'a>>;
```

After:

```rust
// from crates/isograph_parser/src/arguments.rs
#[derive(Debug)]
pub enum VariableNameParent<'a> {
    Use(VariableUsePath<'a>),
    Declaration(DeclaredVariablePath<'a>),
}

pub type VariableNamePath<'a> = PositionResolutionPath<&'a VariableName, VariableNameParent<'a>>;
```

`VariableUse`'s `name` field respells to `#[resolve_field]` + `#[parent_variant(Use)]`.

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum UnparsedChunkItemsParent<'a> {
    Literal(IsoLiteralParsePath<'a>),
    SelectionSet(SelectionSetPath<'a>),
    ArgumentList(ArgumentListPath<'a>),
    ObjectLiteral(ObjectLiteralPath<'a>),
    VariableDeclarationList(VariableDeclarationListPath<'a>),
    TypeAnnotation(TypeAnnotationParent<'a>),
}
```

`From` impls for the two new variants.

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
impl<'a> From<VariableDeclarationListPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: VariableDeclarationListPath<'a>) -> Self {
        IsographResolutionNode::VariableDeclarationList(path)
    }
}
```

A gap in a variable-declaration slot answers `VariableDeclarationList`. A `[...]` slot's parent is `TypeAnnotationParent`; this doc writes `From<TypeAnnotationParent> for IsographResolutionNode` so a gap there answers the type-annotation node. The constant-object list adds `From<that list path> for IsographResolutionNode` when that type is named.

`ConstantValue` and `parse_constant_value` land in arguments.rs. The constant-value ladder is the value ladder without the `$` arm; `$` is `expected(Expectation::ConstantValue)`.

```rust
// from crates/isograph_parser/src/arguments.rs
#[derive(Debug)]
pub enum ConstantValueParent<'a> {
    VariableDefault(DeclaredVariablePath<'a>),
    ConstantObjectEntry(Box<NamedConstantObjectEntryPath<'a>>),
}
```

`ConstantValue` mirrors `NonConstantValue` without `Variable`. An object default uses `Slot<NamedConstantObjectEntry, UnparsedChunkItems>`.

```rust
// from crates/isograph_parser/src/variables.rs
pub(crate) fn consume_variable_declaration_list<F>(
    cursor: &mut ItemCursor<'_>,
    push_error: &mut F,
) -> Option<WithSpan<VariableDeclarationList>>
where
    F: FnMut(WithSpan<ParseError>),
{
    let group = cursor.consume_group_if(BracketKind::Parenthesis)?;
    VariableDeclarationList(group.item.children.item.parse_items(
        cursor.text(),
        Expectation::Separator(ClosingDelimiter::Parenthesis),
        parse_variable_declaration,
        push_error,
    ))
    .with_span(group.location)
    .wrap_some()
}

fn parse_variable_declaration<F>(
    cursor: &mut ItemCursor<'_>,
    push_error: &mut F,
) -> Result<DeclaredVariable, WithSpan<ParseError>>
where
    F: FnMut(WithSpan<ParseError>),
{
    cursor
        .require_token(NonBracketTokenKind::Dollar)
        .map_err(|()| cursor.expected(Expectation::VariableDeclaration))?;
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Colon)))?;
    let type_annotation = parse_type_annotation(cursor, push_error)?;
    let default_value = match cursor.consume_token_if(NonBracketTokenKind::Equals) {
        Some(_) => parse_constant_value(cursor, push_error)?.wrap_some(),
        None => None,
    };
    DeclaredVariable {
        name: cursor
            .token_text(name)
            .intern()
            .to::<VariableName>()
            .with_span(name),
        type_annotation,
        default_value,
    }
    .wrap_ok()
}

pub(crate) fn parse_type_annotation<F>(
    cursor: &mut ItemCursor<'_>,
    push_error: &mut F,
) -> Result<WithSpan<TypeAnnotation>, WithSpan<ParseError>>
where
    F: FnMut(WithSpan<ParseError>),
{
    cursor.spanning(|cursor| {
        if let Some(name) = cursor.consume_token_if(NonBracketTokenKind::Identifier) {
            cursor.consume_token_if(NonBracketTokenKind::Exclamation);
            return TypeAnnotation::Named(NamedTypeAnnotation {
                name: cursor
                    .token_text(name)
                    .intern()
                    .to::<TypeName>()
                    .with_span(name),
            })
            .wrap_ok();
        }
        if let Some(group) = cursor.consume_group_if(BracketKind::Bracket) {
            let inner = parse_bracket_interior_type(
                cursor.text(),
                group.item.children.reference(),
                push_error,
            )?;
            cursor.consume_token_if(NonBracketTokenKind::Exclamation);
            return TypeAnnotation::List(ListTypeAnnotation { inner }.boxed()).wrap_ok();
        }
        cursor.expected(Expectation::TypeAnnotation).wrap_err()
    })
}

fn parse_bracket_interior_type<F>(
    text: &str,
    level: &WithSpan<ChunkedLevel>,
    push_error: &mut F,
) -> Result<WithSpan<Slot<TypeAnnotation, UnparsedChunkItems>>, WithSpan<ParseError>>
where
    F: FnMut(WithSpan<ParseError>),
{
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
    let singleton = parse_singleton(
        level,
        text,
        Expectation::EndOfType,
        |extra| {
            ParseError::expected(
                Expectation::EndOfType,
                Found::from(extra.item.first_item().item.reference()),
            )
            .with_span(extra.location)
        },
        |cursor, push_error| parse_type_annotation(cursor, push_error).map(|wrapped| wrapped.item),
        push_error,
    );
    singleton.item.wrap_ok()
}
```

`parse_type_annotation` returns `WithSpan<TypeAnnotation>` via `spanning`. The singleton interior maps that to `TypeAnnotation`; `parse_one_item` spans the first-chunk attempt again. `ListTypeAnnotation.inner` is that attempt.

`[Pet,]` is one chunk plus a boundary comma: `inner.item: Some(Pet)` plus `push_error(Expected(EndOfType, Token(Comma)))` at the comma. The variable declaration parses.

`[Pet\n!]` is two chunks. The type is `Err` at `!`. The enclosing variable declaration is `item: None`. The `!` does not attach to `Pet`.

Empty `[]` is `Expected(TypeAnnotation, EndOfChunk)` and fails `parse_type_annotation`, so the enclosing variable declaration is `item: None`.

## Changes to parse_iso_literal.rs

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct ClientFieldDeclaration {
    #[resolve_field]
    #[parent_variant(Field)]
    pub parent_type: WithSpan<EntityName>,
    #[resolve_field]
    #[parent_variant(Field)]
    pub client_field_name: WithSpan<ClientFieldName>,
    #[resolve_field]
    #[parent_variant(Field)]
    pub selection_set: WithSpan<SelectionSet>,
}
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct ClientFieldDeclaration {
    #[resolve_field]
    #[parent_variant(Field)]
    pub parent_type: WithSpan<EntityName>,
    #[resolve_field]
    #[parent_variant(Field)]
    pub client_field_name: WithSpan<ClientFieldName>,
    #[resolve_field]
    pub variable_definitions: Option<WithSpan<VariableDeclarationList>>,
    #[resolve_field]
    #[parent_variant(Field)]
    pub selection_set: WithSpan<SelectionSet>,
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let variable_definitions = consume_variable_declaration_list(cursor, push_error);
    let selection_set = require_selection_set(cursor, push_error)?;
```

`lib.rs` adds `mod variables;` and `pub use variables::*;`.

## The resolution surface

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    VariableDeclarationList(VariableDeclarationListPath<'a>),
    DeclaredVariable(DeclaredVariablePath<'a>),
    NamedTypeAnnotation(NamedTypeAnnotationPath<'a>),
    ListTypeAnnotation(ListTypeAnnotationPath<'a>),
    TypeName(TypeNamePath<'a>),
```

The boxed recursive field uses the `Box<T>` blanket. `VariableDeclarationList` expands like `SelectionSet`. `DeclaredVariable` like `NamedArgument`. `TypeAnnotation` like `NonConstantValue`. `TypeName` like `EntityName`.

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
        assert_eq!(declared.name.item, "id".intern().to());
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
            IsographResolutionNode::TypeName(name) => {
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
            IsographResolutionNode::VariableName(name) => {
                assert!(matches!(name.parent, VariableNameParent::Declaration(_)));
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
2. variables.rs, `ConstantValue`, the ClientFieldDeclaration slot, the resolution-node variants, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
3. Move this doc to refactors/past.
