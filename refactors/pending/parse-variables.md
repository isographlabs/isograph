# parse-variables: variable declarations and type annotations

Fourth doc of the series parsing-plan.md orders, after parse-arguments.md. Field declarations gain variable-declaration lists, and the type-annotation grammar arrives; parse-pointers.md reuses it for `to` targets. Defaults reuse parse-arguments.md's value grammar with variables rejected.

## The grammar this doc accepts

The declaration header may carry a paren group between the field name and the selection set:

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

A default value is a `ConstantValue` (parsing-standards.md): the same scalar and object forms as a value, with `$` rejected at the `$`. `DeclaredVariable::default_value` cannot hold a variable.

## Change 1: `Box` delegation in resolve_position

`ListTypeAnnotation` stores its element type boxed, so the blanket the located wrapper already has extends to boxes. This is a crate feature, per the no-manual-impls invariant:

```rust
// from crates/resolve_position/src/lib.rs
/// A boxed node resolves as the node: recursion in a tree boxes storage, never meaning.
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

## Changes to parse_error.rs

`Expectation` gains four variants and their `Display` arms:

```rust
// from crates/isograph_parser/src/parse_error.rs
pub enum Expectation {
    // ... the earlier docs' variants ...
    /// One variable declaration: `$name: Type`, with an optional default.
    VariableDeclaration,
    /// One type: a name, a name with `!`, or a bracketed list type.
    TypeAnnotation,
    /// A value containing no variable, at any depth.
    ConstantValue,
    /// The type is complete; nothing further belongs to it.
    EndOfType,
}
```

```rust
// from crates/isograph_parser/src/parse_error.rs
            Expectation::VariableDeclaration => {
                write!(f, "a variable declaration, like '$id: ID!'")
            }
            Expectation::TypeAnnotation => {
                write!(f, "a type, like 'String', 'String!', or '[String]'")
            }
            Expectation::ConstantValue => {
                write!(f, "a constant value; variables are not allowed here")
            }
            Expectation::EndOfType => write!(f, "the end of the type"),
```

## New module: variables.rs

```rust
// from crates/isograph_parser/src/variables.rs
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use safe_peekable::IntoSafePeekable;
use span::{Span, WithSpan};

use crate::{
    parse_constant_value, parse_singleton, BracketKind, ChunkContentItem, ChunkedLevel,
    ClientFieldDeclarationPath, Dollar, Expectation, Found, IsographResolutionNode, ItemCursor,
    LevelSlot, NonBracketTokenKind, ParseError, VariableName,
};

/// The variable declarations a header's `( ... )` group holds, one per contentful chunk
/// of its interior. The wrapping `WithSpan`'s span covers the parens.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ClientFieldDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclarationList(#[resolve_field] pub Vec<WithSpan<LevelSlot<VariableDeclaration>>>);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableDeclarationListPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum VariableDeclaration {
    Declaration(DeclaredVariable),
}

/// `$name: Type = default`. The dollar's position answers this node.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableDeclarationListPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct DeclaredVariable {
    pub dollar: WithSpan<Dollar>,
    #[resolve_field(parent_variant = Declaration)]
    pub name: WithSpan<VariableName>,
    #[resolve_field(parent_variant = Variable)]
    pub type_annotation: WithSpan<TypeAnnotation>,
    #[resolve_field(parent_variant = VariableDefault)]
    pub default_value: Option<WithSpan<ConstantValue>>,
}

/// A type. The wrapping `WithSpan`'s span covers the name or brackets plus any `!`.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum TypeAnnotation {
    Named(NamedTypeAnnotation),
    List(ListTypeAnnotation),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NamedTypeAnnotation {
    #[resolve_field]
    pub name: WithSpan<TypeName>,
    pub exclamation: Option<WithSpan<Exclamation>>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ListTypeAnnotation {
    /// The wrapped span covers the bracket group.
    #[resolve_field(parent_variant = List)]
    pub inner: WithSpan<Box<TypeAnnotation>>,
    pub exclamation: Option<WithSpan<Exclamation>>,
}

/// A type's name. Its text is its span.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NamedTypeAnnotationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct TypeName;

/// A `!` on a type. Positions on it answer the annotation.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Exclamation;

#[derive(Debug)]
pub enum TypeAnnotationParent<'a> {
    Variable(DeclaredVariablePath<'a>),
    List(Box<ListTypeAnnotationPath<'a>>),
    // parse-pointers.md adds PointerTarget
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

`TypeAnnotationParent::List` is boxed to break the cycle `TypeAnnotationParent -> ListTypeAnnotationPath -> TypeAnnotationParent`. `VariableDeclarationList`'s parent stays a direct alias until parse-pointers.md adds the second parent.

`VariableName` moves from a direct parent to an enum, since a variable name is now a use or a declaration; in arguments.rs, `VariableUse`'s `name` field respells to `#[resolve_field(parent_variant = Use)]`. Before:

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

`UnparsedItemParent` in chunk.rs gains `VariableDeclarationList(VariableDeclarationListPath<'a>)`. `ConstantValue` and `parse_constant_value` land in arguments.rs (parsing-standards.md). `ConstantValueParent` is:

```rust
// from crates/isograph_parser/src/arguments.rs
pub enum ConstantValueParent<'a> {
    VariableDefault(DeclaredVariablePath<'a>),
    ConstantObjectEntry(Box<NamedConstantObjectEntryPath<'a>>),
}
```

```rust
// from crates/isograph_parser/src/variables.rs
impl<'a> From<VariableDeclarationListPath<'a>> for UnparsedItemParent<'a> {
    fn from(path: VariableDeclarationListPath<'a>) -> Self {
        UnparsedItemParent::VariableDeclarationList(path)
    }
}
```

The parse functions:

```rust
// from crates/isograph_parser/src/variables.rs
pub(crate) fn consume_variable_declaration_list(
    cursor: &mut ItemCursor<'_>,
) -> Option<WithSpan<VariableDeclarationList>> {
    let group = cursor.consume_group_if(BracketKind::Parenthesis)?;
    Some(WithSpan::new(
        VariableDeclarationList(
            group.item.children.item.parse_items(cursor.text(), parse_variable_declaration),
        ),
        group.location,
    ))
}

fn parse_variable_declaration(
    cursor: &mut ItemCursor<'_>,
) -> Result<VariableDeclaration, WithSpan<ParseError>> {
    let dollar = cursor.require_token(
        NonBracketTokenKind::Dollar,
        Expectation::VariableDeclaration,
    )?;
    let name = cursor.require_token(
        NonBracketTokenKind::Identifier,
        Expectation::Token(NonBracketTokenKind::Identifier),
    )?;
    cursor.require_token(
        NonBracketTokenKind::Colon,
        Expectation::Token(NonBracketTokenKind::Colon),
    )?;
    let type_annotation = parse_type_annotation(cursor)?;
    let default_value = match cursor.consume_token_if(NonBracketTokenKind::Equals) {
        Some(_) => Some(parse_constant_value(cursor)?),
        None => None,
    };
    Ok(VariableDeclaration::Declaration(DeclaredVariable {
        dollar: WithSpan::new(Dollar, dollar),
        name: WithSpan::new(VariableName, name),
        type_annotation,
        default_value,
    }))
}

pub(crate) fn parse_type_annotation(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<TypeAnnotation>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if let Some(name) = cursor.consume_token_if(NonBracketTokenKind::Identifier) {
            let exclamation = cursor
                .consume_token_if(NonBracketTokenKind::Exclamation)
                .map(|span| WithSpan::new(Exclamation, span));
            return Ok(TypeAnnotation::Named(NamedTypeAnnotation {
                name: WithSpan::new(TypeName, name),
                exclamation,
            }));
        }
        if let Some(group) = cursor.consume_group_if(BracketKind::Bracket) {
            let inner = parse_bracket_interior_type(cursor.text(), &group.item.children)?;
            let exclamation = cursor
                .consume_token_if(NonBracketTokenKind::Exclamation)
                .map(|span| WithSpan::new(Exclamation, span));
            return Ok(TypeAnnotation::List(ListTypeAnnotation {
                inner: WithSpan::new(Box::new(inner.item), group.location),
                exclamation,
            }));
        }
        Err(cursor.expected(Expectation::TypeAnnotation))
    })
}

fn parse_bracket_interior_type(
    text: &str,
    level: &WithSpan<ChunkedLevel>,
) -> Result<WithSpan<TypeAnnotation>, WithSpan<ParseError>> {
    parse_singleton(
        level,
        text,
        || {
            WithSpan::new(
                ParseError::expected(Expectation::TypeAnnotation, Found::EndOfChunk),
                Span::new(level.location.end, level.location.end),
            )
        },
        |extra| {
            WithSpan::new(
                ParseError::expected(
                    Expectation::EndOfType,
                    Found::from(&extra.item.first_item().item),
                ),
                extra.location,
            )
        },
        parse_type_annotation,
        Expectation::EndOfType,
    )
}
```

`Chunk::first_item` lands here, the listing in parsing-standards.md.

## Changes to parse_iso_literal.rs

`ClientFieldDeclaration` gains the slot between the name and the selection set, bare-marked because the list's parent is the declaration's own path:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct ClientFieldDeclaration {
    pub field_keyword: WithSpan<FieldKeyword>,
    #[resolve_field(parent_variant = Field)]
    pub parent_type: WithSpan<EntityName>,
    pub dot: WithSpan<Dot>,
    #[resolve_field(parent_variant = Field)]
    pub client_field_name: WithSpan<ClientFieldName>,
    #[resolve_field]
    pub variable_definitions: Option<WithSpan<VariableDeclarationList>>,
    #[resolve_field(parent_variant = Field)]
    pub selection_set: WithSpan<SelectionSet>,
}
```

`parse_field`, between the name and the selection set:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let variable_definitions = consume_variable_declaration_list(cursor);
    let selection_set = require_selection_set(cursor)?;
```

`errors()`'s field arm walks the variables before the selection set, matching source order:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
            IsoLiteralParse::Field(declaration) => {
                let mut errors = Vec::new();
                collect_variable_errors(&declaration.variable_definitions, &mut errors);
                collect_selection_set_errors(&declaration.selection_set.item, &mut errors);
                errors
            }
```

```rust
// from crates/isograph_parser/src/variables.rs
pub(crate) fn collect_variable_errors(
    declarations: &Option<WithSpan<VariableDeclarationList>>,
    errors: &mut Vec<WithSpan<ParseError>>,
) {
    let Some(declarations) = declarations else {
        return;
    };
    for declaration in &declarations.item.0 {
        collect_slot_errors(&declarations.item.0, |declaration, errors| match declaration {
            VariableDeclaration::Declaration(declared) => {
                if let Some(default) = &declared.default_value {
                    crate::collect_constant_value_errors(&default.item, errors);
                }
            }
        }, errors);
    }
}
```

## The resolution surface

`IsographResolutionNode` gains:

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    VariableDeclarationList(VariableDeclarationListPath<'a>),
    DeclaredVariable(DeclaredVariablePath<'a>),
    NamedTypeAnnotation(NamedTypeAnnotationPath<'a>),
    ListTypeAnnotation(ListTypeAnnotationPath<'a>),
    TypeName(TypeNamePath<'a>),
```

## Generated code

The novel expansion is the boxed recursive field; the blanket impl from Change 1 carries the delegation:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for ListTypeAnnotation {
    type Parent<'a> = TypeAnnotationParent<'a>;
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        if self.inner.location.contains(position) {
            let new_parent = <Box<TypeAnnotation> as ::resolve_position::ResolvePosition>::Parent::List(self.path(parent).into());
            return self.inner.item.resolve(new_parent, position);
        }
        return Self::ResolvedNode::ListTypeAnnotation(self.path(parent).into());
    }
}
```

`<Box<TypeAnnotation>>::Parent` is `TypeAnnotation`'s parent through the blanket, and `self.path(parent).into()` boxes through `From<T> for Box<T>`. Every other new type expands per the earlier docs' patterns: `VariableDeclarationList` like `SelectionSet`, `VariableDeclaration` like `Selection`, `DeclaredVariable` and `NamedTypeAnnotation` like `NamedArgument` (bare and wrapped descents; the unmarked `dollar` and `exclamation` fields answer their containers), `TypeAnnotation` like `NonConstantValue`, `TypeName` like `EntityName`.

## Tests

Extending the parse_iso_literal.rs test module.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs (test module)
    fn variables_of(parse: &WithSpan<IsoLiteralParse>) -> &WithSpan<VariableDeclarationList> {
        as_field(parse)
            .variable_definitions
            .as_ref()
            .expect("the fixture's declaration carries variable definitions")
    }

    fn as_declared(slot: &LevelSlot<VariableDeclaration>) -> &DeclaredVariable {
        match slot {
            LevelSlot::Parsed(parsed) => match &parsed.item {
                VariableDeclaration::Declaration(declared) => declared,
            },
            slot => panic!("expected a declared variable, got {slot:?}"),
        }
    }

    #[test]
    fn a_multi_line_variable_list_parses_in_the_demo_style() {
        let text = "field Query.PetCheckinListRoute(\n  $id: ID !\n) {\n  pets\n}";
        let parse = parsed(text);
        assert_eq!(parse.item.errors(), vec![]);
        let variables = variables_of(&parse);
        assert_eq!(variables.item.0.len(), 1);
        let declared = as_declared(&variables.item.0[0].item);
        assert_eq!(declared.name.location, span_of(text, "id"));
        match &declared.type_annotation.item {
            TypeAnnotation::Named(named) => {
                assert_eq!(named.name.location, span_of(text, "ID"));
                assert!(named.exclamation.is_some());
            }
            annotation => panic!("expected a named type, got {annotation:?}"),
        }
    }

    #[test]
    fn list_types_nest_with_non_null_markers() {
        let text = "field Query.Foo($pets: [Pet!]!) { bar }";
        let parse = parsed(text);
        assert_eq!(parse.item.errors(), vec![]);
        let declared = as_declared(&variables_of(&parse).item.0[0].item);
        assert_eq!(declared.type_annotation.location, span_of(text, "[Pet!]!"));
        let list = match &declared.type_annotation.item {
            TypeAnnotation::List(list) => list,
            annotation => panic!("expected a list type, got {annotation:?}"),
        };
        assert!(list.exclamation.is_some());
        assert_eq!(list.inner.location, span_of(text, "[Pet!]"));
        match list.inner.item.as_ref() {
            TypeAnnotation::Named(named) => {
                assert_eq!(named.name.location, span_of(text, "Pet"));
                assert!(named.exclamation.is_some());
            }
            annotation => panic!("expected the named element type, got {annotation:?}"),
        }
    }

    #[test]
    fn defaults_parse_and_reject_variables_at_any_depth() {
        let text = "field Query.Foo($limit: Int = 10) { bar }";
        let parse = parsed(text);
        assert_eq!(parse.item.errors(), vec![]);
        let declared = as_declared(&variables_of(&parse).item.0[0].item);
        let default = declared.default_value.as_ref().expect("the fixture declares a default");
        assert!(matches!(default.item, ConstantValue::Integer(IntegerValue(10))));

        let shallow = "field Query.Foo($limit: Int = $other) { bar }";
        let parse = parsed(shallow);
        let unparsed = match &variables_of(&parse).item.0[0].item {
            LevelSlot::Unparsed(unparsed) => unparsed.reason,
            declaration => panic!("expected an unparsed declaration, got {declaration:?}"),
        };
        assert_eq!(
            unparsed.item,
            expected(Expectation::ConstantValue, Found::Token(NonBracketTokenKind::Dollar))
        );
        assert_eq!(unparsed.location, span_of(shallow, "$other"));

        let deep = "field Query.Foo($input: Input = { pet: $pet }) { bar }";
        let parse = parsed(deep);
        let unparsed = match &variables_of(&parse).item.0[0].item {
            LevelSlot::Unparsed(unparsed) => unparsed.reason,
            declaration => panic!("expected an unparsed declaration, got {declaration:?}"),
        };
        assert_eq!(unparsed.location, span_of(deep, "$pet"));
    }

    #[test]
    fn each_malformed_variable_declaration_degrades_alone() {
        let text = "field Query.Foo($a Int, $b: , id: ID, $c: Float) { bar }";
        let parse = parsed(text);
        let variables = variables_of(&parse);
        assert_eq!(variables.item.0.len(), 4);
        let missing_colon = match &variables.item.0[0].item {
            LevelSlot::Unparsed(unparsed) => unparsed.reason,
            declaration => panic!("expected an unparsed declaration, got {declaration:?}"),
        };
        assert_eq!(
            missing_colon.item,
            expected(token(NonBracketTokenKind::Colon), Found::Token(Identifier))
        );
        assert_eq!(missing_colon.location, span_of(text, "Int"));
        let missing_type = match &variables.item.0[1].item {
            LevelSlot::Unparsed(unparsed) => unparsed.reason,
            declaration => panic!("expected an unparsed declaration, got {declaration:?}"),
        };
        assert_eq!(
            missing_type.item,
            expected(Expectation::TypeAnnotation, Found::EndOfChunk)
        );
        let dollarless = match &variables.item.0[2].item {
            LevelSlot::Unparsed(unparsed) => unparsed.reason,
            declaration => panic!("expected an unparsed declaration, got {declaration:?}"),
        };
        assert_eq!(
            dollarless.item,
            expected(Expectation::VariableDeclaration, Found::Token(Identifier))
        );
        as_declared(&variables.item.0[3].item);
        assert_eq!(parse.item.errors().len(), 3);
    }

    #[test]
    fn a_final_comma_inside_a_list_type_degrades_that_declaration() {
        let text = "field Query.Foo($pets: [Pet,]) { bar }";
        let parse = parsed(text);
        let unparsed = match &variables_of(&parse).item.0[0].item {
            LevelSlot::Unparsed(unparsed) => unparsed.reason,
            declaration => panic!("expected an unparsed declaration, got {declaration:?}"),
        };
        assert_eq!(
            unparsed.item,
            expected(Expectation::EndOfType, Found::Token(Comma))
        );
        assert_eq!(unparsed.location, span_of(text, ","));
    }

    #[test]
    fn a_line_break_inside_a_list_type_degrades_that_declaration() {
        let text = "field Query.Foo($pets: [Pet\n!]) { bar }";
        let parse = parsed(text);
        let unparsed = match &variables_of(&parse).item.0[0].item {
            LevelSlot::Unparsed(unparsed) => unparsed.reason,
            declaration => panic!("expected an unparsed declaration, got {declaration:?}"),
        };
        assert_eq!(
            unparsed.item,
            expected(Expectation::EndOfType, Found::Token(NonBracketTokenKind::Exclamation))
        );
    }

    #[test]
    fn type_names_resolve_through_their_annotation_ancestry() {
        let text = "field Query.Foo($pets: [Pet]) { bar }";
        let parse = parsed(text);
        match parse.resolve((), span_of(text, "Pet")) {
            IsographResolutionNode::TypeName(name) => {
                let list = match &name.parent.parent {
                    TypeAnnotationParent::List(list) => list,
                    parent => panic!("expected a list parent, got {parent:?}"),
                };
                match &list.parent {
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
```

## Landing checklist

1. The resolve_position blanket impl; `cargo test -p resolve_position` passes.
2. variables.rs, the arguments.rs, selections.rs, parse_iso_literal.rs, and parse_error.rs changes, the resolution-node variants, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
3. Move this doc to refactors/past.
