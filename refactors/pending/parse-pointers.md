# parse-pointers: pointer declarations

`pointer Type.name to Type { ... }`. Removes `UnsupportedDeclarationType`.

## The grammar this doc accepts

```
pointer <Identifier> . <Identifier> [<paren group>] to <type> [<description>] <brace group>
```

The `to` keyword is an identifier whose text is `to`. The target type is a type annotation. Variable definitions, the description, and the selection set behave as on field declarations. Directives land in parse-directives.md.

Origin: `ClientPointerDeclaration` in `crates/isograph_lang_types/src/declarations/client_selectable_declaration.rs` and `parse_iso_client_pointer_declaration` in `crates/isograph_lang_parser/src/parse_iso_literal.rs`. Delta: no `const_export_name`, `definition_path`, `directives`, or `semantic_tokens`. The name wrapper is `ClientObjectSelectableNameWrapper`.

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
        "field" => IsoLiteralItem::Field(parse_field(cursor)?).wrap_ok(),
        "pointer" => ParseError::UnsupportedDeclarationType
            .with_span(keyword.location)
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
        "field" => IsoLiteralItem::Field(parse_field(cursor)?).wrap_ok(),
        "pointer" => IsoLiteralItem::Pointer(parse_pointer(cursor)?).wrap_ok(),
```

The test `field_and_pointer_declarations_do_not_parse_yet` is deleted.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ClientPointerDeclaration {
    #[resolve_field]
    #[parent_variant(ClientPointerDeclaration)]
    pub parent_type: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    pub client_pointer_name: WithSpan<ClientObjectSelectableNameWrapper>,
    #[resolve_field]
    #[parent_variant(ClientPointerDeclaration)]
    pub variable_definitions: Option<WithSpan<VariableDeclarationOrUsageList>>,
    #[resolve_field]
    #[parent_variant(PointerTarget)]
    pub target_type: WithSpan<TypeAnnotation>,
    #[resolve_field]
    #[parent_variant(ClientPointerDeclaration)]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(ClientPointerDeclaration)]
    pub selection_set: WithSpan<SelectionSet>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ClientPointerDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ClientObjectSelectableNameWrapper(common_lang_types::SelectableName);

pub type ClientPointerDeclarationPath<'a> =
    PositionResolutionPath<&'a ClientPointerDeclaration, IsoLiteralSlotPath<'a>>;

pub type ClientObjectSelectableNameWrapperPath<'a> = PositionResolutionPath<
    &'a ClientObjectSelectableNameWrapper,
    ClientPointerDeclarationPath<'a>,
>;
```

There is no `PointerKeyword` and no `ToKeyword` node. A position on `pointer` or `to` answers `ClientPointerDeclaration`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_pointer(
    cursor: &mut ItemCursor<'_>,
) -> Result<ClientPointerDeclaration, WithSpan<ParseError>> {
    let (parent_type, client_pointer_name) = parse_type_dot_name(cursor)?;
    let variable_definitions = consume_variable_declaration_list(cursor);
    let to_keyword = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Keyword)
        .map_err(|()| cursor.expected(Expectation::ToKeyword))?;
    if to_keyword.text() != "to" {
        return ParseError::expected(
            Expectation::ToKeyword,
            Found::Token(NonBracketTokenKind::Identifier),
        )
        .with_span(to_keyword.location)
        .wrap_err();
    }
    let target_type = parse_type_annotation(cursor)?;
    let description = consume_description(cursor);
    let selection_set = require_selection_set(cursor)?;
    ClientPointerDeclaration {
        parent_type,
        client_pointer_name: client_pointer_name.map(ClientObjectSelectableNameWrapper),
        variable_definitions,
        target_type,
        description,
        selection_set,
    }
    .wrap_ok()
}
```

## The parent-enum conversions

1. `EntityNameWrapperParent` gains `ClientPointerDeclaration(ClientPointerDeclarationPath<'a>)`.
2. `SelectionSetParent` gains `ClientPointerDeclaration(ClientPointerDeclarationPath<'a>)`.
3. `TypeAnnotationParent` gains `PointerTarget(ClientPointerDeclarationPath<'a>)`.
4. `VariableDeclarationOrUsageList` and `Description` parents become enums:

```rust
// from crates/isograph_parser/src/variables.rs
#[derive(Debug)]
pub enum VariableDeclarationOrUsageListParent<'a> {
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
}

pub type VariableDeclarationOrUsageListPath<'a> =
    PositionResolutionPath<&'a VariableDeclarationOrUsageList, VariableDeclarationOrUsageListParent<'a>>;
```

Origin: `VariableDeclarationParentType` in isograph. The i2 enum is on the list wrapper, not on each `VariableDeclarationOrUsage`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[derive(Debug)]
pub enum DescriptionParent<'a> {
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
}

pub type DescriptionPath<'a> = PositionResolutionPath<&'a Description, DescriptionParent<'a>>;
```

Origin: `DescriptionParent` in `string_key_wrappers.rs`. Variant names match.

`ClientFieldDeclaration`'s `variable_definitions` and `description` fields respell to `#[resolve_field]` + `#[parent_variant(ClientFieldDeclaration)]`. `VariableDeclarationOrUsageList`'s `parent_type` becomes `VariableDeclarationOrUsageListParent`. `Description`'s `parent_type` becomes `DescriptionParent`.

## The resolution surface

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
    ClientObjectSelectableNameWrapper(ClientObjectSelectableNameWrapperPath<'a>),
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
            ClientObjectSelectableNameWrapper("BestFriend".intern().to())
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
            IsographResolutionNode::ClientObjectSelectableNameWrapper(name) => {
                assert_eq!(
                    name.parent.inner.target_type.location,
                    span_of(text, "Owner")
                );
            }
            node => panic!("expected the pointer name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "Owner")) {
            IsographResolutionNode::EntityNameWrapper(name) => match name.parent {
                EntityNameWrapperParent::NamedTypeAnnotation(named) => {
                    match named.parent.reference() {
                        TypeAnnotationParent::PointerTarget(_) => {}
                        parent => panic!("expected the pointer-target parent, got {parent:?}"),
                    }
                }
                parent => panic!("expected a named type annotation, got {parent:?}"),
            },
            node => panic!("expected the type name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "id")) {
            IsographResolutionNode::SelectionNameWrapper(name) => {
                match name.parent.parent.parent.parent {
                    SelectionSetParent::ClientPointerDeclaration(_) => {}
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
