# parse-pointers: pointer declarations

`pointer Type.name to Type { ... }`. Removes `UnsupportedDeclarationType`.

## The grammar this doc accepts

```
pointer <Identifier> . <Identifier> [<paren group>] to <type> [<description>] <brace group>
```

The `to` keyword is an identifier whose text is `to`. The target type is a type annotation. Variable definitions, the description, and the selection set behave as on field declarations.

## Changes to parse_error.rs

`UnsupportedDeclarationType` is deleted. Remaining structural variants: `Expected`, `EmptyLiteral`, `MultipleDeclarations`, `IntegerDoesNotFitI64`.

```rust
// from crates/isograph_parser/src/parse_error.rs
    #[error("the keyword `to`")]
    ToKeyword,
```

## Changes to parse_iso_literal.rs

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
    Field(ClientFieldDeclaration),
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        "field" => IsoLiteralItem::Field(parse_field(cursor, push_error)?).wrap_ok(),
        "pointer" => ParseError::UnsupportedDeclarationType
            .with_span(keyword)
            .wrap_err(),
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
    Field(ClientFieldDeclaration),
    Pointer(ClientPointerDeclaration),
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        "field" => IsoLiteralItem::Field(parse_field(cursor, push_error)?).wrap_ok(),
        "pointer" => IsoLiteralItem::Pointer(parse_pointer(cursor, push_error)?).wrap_ok(),
```

The test `field_and_pointer_declarations_do_not_parse_yet` is deleted.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralParsePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ClientPointerDeclaration {
    #[resolve_field(parent_variant = Pointer)]
    pub parent_type: WithSpan<EntityName>,
    #[resolve_field]
    pub client_pointer_name: WithSpan<ClientPointerName>,
    #[resolve_field(parent_variant = Pointer)]
    pub variable_definitions: Option<WithSpan<VariableDeclarationList>>,
    #[resolve_field(parent_variant = PointerTarget)]
    pub target_type: WithSpan<TypeAnnotation>,
    #[resolve_field(parent_variant = Pointer)]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field(parent_variant = Pointer)]
    pub selection_set: WithSpan<SelectionSet>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ClientPointerDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ClientPointerName(common_lang_types::SelectableName);

impl From<intern::string_key::StringKey> for ClientPointerName {
    fn from(key: intern::string_key::StringKey) -> Self {
        ClientPointerName(key.to())
    }
}

pub type ClientPointerDeclarationPath<'a> =
    PositionResolutionPath<&'a ClientPointerDeclaration, IsoLiteralParsePath<'a>>;

pub type ClientPointerNamePath<'a> =
    PositionResolutionPath<&'a ClientPointerName, ClientPointerDeclarationPath<'a>>;
```

There is no `PointerKeyword` and no `ToKeyword`. A position on `pointer` or `to` answers `ClientPointerDeclaration`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_pointer<F>(
    cursor: &mut ItemCursor<'_>,
    push_error: &mut F,
) -> Result<ClientPointerDeclaration, WithSpan<ParseError>>
where
    F: FnMut(WithSpan<ParseError>),
{
    let parent_type = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Period)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Period)))?;
    let client_pointer_name = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    let variable_definitions = consume_variable_declaration_list(cursor, push_error);
    let to_keyword = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::ToKeyword))?;
    if cursor.token_text(to_keyword) != "to" {
        return ParseError::expected(
            Expectation::ToKeyword,
            Found::Token(NonBracketTokenKind::Identifier),
        )
        .with_span(to_keyword)
        .wrap_err();
    }
    let target_type = parse_type_annotation(cursor, push_error)?;
    let description = consume_description(cursor);
    let selection_set = require_selection_set(cursor, push_error)?;
    ClientPointerDeclaration {
        parent_type: cursor
            .token_text(parent_type)
            .intern()
            .to::<EntityName>()
            .with_span(parent_type),
        client_pointer_name: cursor
            .token_text(client_pointer_name)
            .intern()
            .to::<ClientPointerName>()
            .with_span(client_pointer_name),
        variable_definitions,
        target_type,
        description,
        selection_set,
    }
    .wrap_ok()
}
```

## The parent-enum conversions

1. `EntityNameParent` gains `Pointer(ClientPointerDeclarationPath<'a>)`.
2. `SelectionSetParent` gains `Pointer(ClientPointerDeclarationPath<'a>)`.
3. `TypeAnnotationParent` gains `PointerTarget(ClientPointerDeclarationPath<'a>)`.
4. `VariableDeclarationList` and `Description` parents become enums:

```rust
// from crates/isograph_parser/src/variables.rs
#[derive(Debug)]
pub enum VariableDeclarationListParent<'a> {
    Field(ClientFieldDeclarationPath<'a>),
    Pointer(ClientPointerDeclarationPath<'a>),
}

pub type VariableDeclarationListPath<'a> =
    PositionResolutionPath<&'a VariableDeclarationList, VariableDeclarationListParent<'a>>;
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[derive(Debug)]
pub enum DescriptionParent<'a> {
    Field(ClientFieldDeclarationPath<'a>),
    Pointer(ClientPointerDeclarationPath<'a>),
}

pub type DescriptionPath<'a> = PositionResolutionPath<&'a Description, DescriptionParent<'a>>;
```

`ClientFieldDeclaration`'s `variable_definitions` and `description` fields respell to `#[resolve_field(parent_variant = Field)]`.

## The resolution surface

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
    ClientPointerName(ClientPointerNamePath<'a>),
```

## Tests

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn as_pointer(parse: &WithSpan<IsoLiteralParse>) -> &ClientPointerDeclaration {
        match parsed_item(parse).expect("the fixture's literal parsed an item") {
            IsoLiteralItem::Pointer(declaration) => declaration,
            item => panic!("expected a pointer declaration, got {item:?}"),
        }
    }

    #[test]
    fn a_final_comma_after_the_pointer_declaration_is_an_error() {
        let text = "pointer Pet.BestFriend to Pet { id },";
        let (parse, errors) = parsed(text);
        as_pointer(parse.reference());
        assert_eq!(
            errors,
            expected(Expectation::EndOfDeclaration, Found::Token(Comma))
                .with_span(span_of(text, ","))
                .wrap_vec(),
        );
    }

    #[test]
    fn a_minimal_pointer_declaration_parses() {
        let text = "pointer Pet.BestFriend to Pet { id }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let declaration = as_pointer(parse.reference());
        assert_eq!(
            declaration.client_pointer_name.item,
            "BestFriend".intern().to()
        );
        assert_eq!(
            declaration.client_pointer_name.location,
            span_of(text, "BestFriend")
        );
        let target_anchor = span_of(text, "Pet {");
        match declaration.target_type.item.reference() {
            TypeAnnotation::Named(named) => {
                assert_eq!(
                    named.name.location,
                    Span::new(target_anchor.start, target_anchor.start + 3)
                );
            }
            annotation => panic!("expected a named target, got {annotation:?}"),
        }
        assert_eq!(selections(declaration.selection_set.reference()).len(), 1);
    }

    #[test]
    fn a_full_pointer_declaration_parses_in_order() {
        let text = "pointer Pet.Owner($limit: Int) to Person! \"the owner\" { name }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let declaration = as_pointer(parse.reference());
        assert!(declaration.variable_definitions.is_some());
        assert_eq!(declaration.target_type.location, span_of(text, "Person!"));
        assert!(declaration.description.is_some());
    }

    #[test]
    fn a_bracketed_pointer_target_parses() {
        let text = "pointer Pet.Friends to [Pet!]! { id }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        assert_eq!(
            as_pointer(parse.reference()).target_type.location,
            span_of(text, "[Pet!]!")
        );
    }

    #[test]
    fn a_missing_to_keyword_reports_at_the_found_item() {
        let text = "pointer Pet.BestFriend Owner { id }";
        assert_no_declaration(
            text,
            expected(Expectation::ToKeyword, Found::Token(Identifier)),
            span_of(text, "Owner"),
        );
    }

    #[test]
    fn a_missing_target_type_reports_after_to() {
        let text = "pointer Pet.BestFriend to { id }";
        assert_no_declaration(
            text,
            expected(
                Expectation::TypeAnnotation,
                Found::Group(BracketKind::Brace),
            ),
            span_of(text, "{ id }"),
        );
    }

    #[test]
    fn pointer_names_and_targets_resolve_with_their_ancestry() {
        let text = "pointer Pet.BestFriend to Owner { id }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "BestFriend")) {
            IsographResolutionNode::ClientPointerName(name) => {
                assert_eq!(
                    name.parent.inner.target_type.location,
                    span_of(text, "Owner")
                );
            }
            node => panic!("expected the pointer name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "Owner")) {
            IsographResolutionNode::TypeName(name) => match name.parent.parent.reference() {
                TypeAnnotationParent::PointerTarget(_) => {}
                parent => panic!("expected the pointer-target parent, got {parent:?}"),
            },
            node => panic!("expected the type name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "id")) {
            IsographResolutionNode::SelectionName(name) => {
                let scalar = match name.parent {
                    SelectionNameParent::Scalar(scalar) => scalar,
                    parent => panic!("expected a scalar parent, got {parent:?}"),
                };
                match scalar.parent {
                    SelectionSetParent::Pointer(_) => {}
                    parent => panic!("expected the pointer at the top, got {parent:?}"),
                }
            }
            node => panic!("expected the selection name leaf, got {node:?}"),
        }
    }
```

## Landing checklist

1. The parse_iso_literal.rs, parse_error.rs, selections.rs, and variables.rs changes, the resolution-node variants, the test deletion, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
