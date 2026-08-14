# parse-pointers: pointer declarations

Sixth and last doc of the series parsing-plan.md orders, after parse-descriptions.md. It lands `pointer Type.name to Type { ... }` and removes `UnsupportedDeclarationType`: every declaration keyword now parses.

## The grammar this doc accepts

```
pointer <Identifier> . <Identifier> [<paren group>] to <type> [<description>] <brace group>
```

The `to` keyword is an identifier whose text is `to`. The target type is a type annotation (parse-variables.md), read from the declaration chunk's items. Variable definitions, the description, and the selection set behave exactly as on field declarations; the field order above matches upstream's parse order.

## Changes to parse_error.rs

`UnsupportedDeclarationType` and its `Display` arm are deleted; the enum's remaining structural variants are `Expected`, `EmptyLiteral`, `MultipleDeclarations`, and `IntegerOutOfRange`. `Expectation` gains one variant:

```rust
// from crates/isograph_parser/src/parse_error.rs
    /// The keyword `to`, between a pointer's name and its target type.
    ToKeyword,
```

```rust
// from crates/isograph_parser/src/parse_error.rs
            Expectation::ToKeyword => write!(f, "the keyword `to`"),
```

## Changes to parse_iso_literal.rs

The enum and dispatch complete. Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub enum IsoLiteralParse {
    Entrypoint(EntrypointDeclaration),
    Field(ClientFieldDeclaration),
    Unparsed(UnparsedLiteral),
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        text if text == "entrypoint" => {
            Ok(IsoLiteralParse::Entrypoint(parse_entrypoint(keyword, cursor)?))
        }
        text if text == "field" => Ok(IsoLiteralParse::Field(parse_field(keyword, cursor)?)),
        text if text == "pointer" => {
            Err(WithSpan::new(ParseError::UnsupportedDeclarationType, keyword))
        }
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub enum IsoLiteralParse {
    Entrypoint(EntrypointDeclaration),
    Field(ClientFieldDeclaration),
    Pointer(ClientPointerDeclaration),
    Unparsed(UnparsedLiteral),
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        text if text == "entrypoint" => {
            Ok(IsoLiteralParse::Entrypoint(parse_entrypoint(keyword, cursor)?))
        }
        text if text == "field" => Ok(IsoLiteralParse::Field(parse_field(keyword, cursor)?)),
        text if text == "pointer" => {
            Ok(IsoLiteralParse::Pointer(parse_pointer(keyword, cursor)?))
        }
```

The declaration type, its markers, and its parse function:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = IsographResolutionNode<'a>)]
pub struct ClientPointerDeclaration {
    pub pointer_keyword: WithSpan<PointerKeyword>,
    #[resolve_field(parent_variant = Pointer)]
    pub parent_type: WithSpan<EntityName>,
    pub dot: WithSpan<Dot>,
    #[resolve_field]
    pub client_pointer_name: WithSpan<ClientPointerName>,
    #[resolve_field(parent_variant = Pointer)]
    pub variable_definitions: Option<WithSpan<VariableDeclarationList>>,
    pub to_keyword: WithSpan<ToKeyword>,
    #[resolve_field(parent_variant = PointerTarget)]
    pub target_type: WithSpan<TypeAnnotation>,
    #[resolve_field(parent_variant = Pointer)]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field(parent_variant = Pointer)]
    pub selection_set: WithSpan<SelectionSet>,
}

/// The `pointer` keyword. Positions on it answer the declaration.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PointerKeyword;

/// The `to` keyword. Positions on it answer the declaration.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ToKeyword;

/// The name of the client pointer being declared. Its text is its span.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ClientPointerDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ClientPointerName;

pub type ClientPointerDeclarationPath<'a> =
    PositionResolutionPath<&'a ClientPointerDeclaration, ()>;

pub type ClientPointerNamePath<'a> =
    PositionResolutionPath<&'a ClientPointerName, ClientPointerDeclarationPath<'a>>;
```

```rust
// from crates/isograph_parser/src/chunk_stream.rs
impl<'a> ItemCursor<'a> {
    pub(crate) fn require_keyword(
        &mut self,
        keyword: &'static str,
        expected: Expectation,
    ) -> Result<Span, WithSpan<ParseError>> { /* parsing-standards.md */ }
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_pointer(
    keyword: Span,
    cursor: &mut ItemCursor<'_>,
) -> Result<ClientPointerDeclaration, WithSpan<ParseError>> {
    let parent_type = cursor.require_token(
        NonBracketTokenKind::Identifier,
        Expectation::Token(NonBracketTokenKind::Identifier),
    )?;
    let dot = cursor.require_token(
        NonBracketTokenKind::Period,
        Expectation::Token(NonBracketTokenKind::Period),
    )?;
    let client_pointer_name = cursor.require_token(
        NonBracketTokenKind::Identifier,
        Expectation::Token(NonBracketTokenKind::Identifier),
    )?;
    let variable_definitions = consume_variable_declaration_list(cursor);
    let to_keyword = cursor.require_keyword("to", Expectation::ToKeyword)?;
    let target_type = parse_type_annotation(cursor)?;
    let description = consume_description(cursor);
    let selection_set = require_selection_set(cursor)?;
    Ok(ClientPointerDeclaration {
        pointer_keyword: WithSpan::new(PointerKeyword, keyword),
        parent_type: WithSpan::new(EntityName, parent_type),
        dot: WithSpan::new(Dot, dot),
        client_pointer_name: WithSpan::new(ClientPointerName, client_pointer_name),
        variable_definitions,
        to_keyword: WithSpan::new(ToKeyword, to_keyword),
        target_type,
        description,
        selection_set,
    })
}
```

`parse_pointer` does not call `require_end`. `parse_singleton` calls `require_end`, checks `boundary_comma`, and returns `Err` on a second chunk.
`errors()` gains the pointer arm, mirroring the field arm (the target type contributes nothing: a bad target fails the header, degrading the whole literal):

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
            IsoLiteralParse::Pointer(declaration) => {
                let mut errors = Vec::new();
                collect_variable_errors(&declaration.variable_definitions, &mut errors);
                collect_selection_set_errors(&declaration.selection_set.item, &mut errors);
                errors
            }
```

## The parent-enum conversions

The pointer is a second or third parent for four existing types; each converts per the established pattern, and the fields marked above name the new variants.

1. `EntityNameParent` gains `Pointer(ClientPointerDeclarationPath<'a>)`. (`ClientFieldNameParent` is untouched: a pointer's name is a `ClientPointerName`.)
2. `SelectionSetParent` in selections.rs gains `Pointer(ClientPointerDeclarationPath<'a>)`.
3. `TypeAnnotationParent` in variables.rs gains `PointerTarget(ClientPointerDeclarationPath<'a>)`.
4. `VariableDeclarationList`'s and `Description`'s direct parent aliases become enums:

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

`ClientFieldDeclaration`'s `variable_definitions` and `description` fields respell from bare `#[resolve_field]` to `#[resolve_field(parent_variant = Field)]`, and the `#[resolve_position(parent_type = ...)]` attributes on `VariableDeclarationList` and `Description` repoint to the new enums.

## The resolution surface

`IsographResolutionNode` gains:

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
    ClientPointerName(ClientPointerNamePath<'a>),
```

`ClientPointerDeclaration` expands like `ClientFieldDeclaration`; `ClientPointerName` like `EntityName`.

## Tests

Extending the parse_iso_literal.rs test module. The parse-entrypoint.md test `field_and_pointer_declarations_do_not_parse_yet` (already narrowed to pointers by parse-fields.md) is deleted.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs (test module)
    fn as_pointer(parse: &WithSpan<IsoLiteralParse>) -> &ClientPointerDeclaration {
        match &parse.item {
            IsoLiteralParse::Pointer(declaration) => declaration,
            parse => panic!("expected a pointer declaration, got {parse:?}"),
        }
    }

    #[test]
    fn a_final_comma_after_the_pointer_declaration_is_an_error() {
        let text = "pointer Pet.BestFriend to Pet { id },";
        assert_unparsed(
            text,
            expected(Expectation::EndOfDeclaration, Found::Token(Comma)),
            span_of(text, ","),
        );
    }

    #[test]
    fn a_minimal_pointer_declaration_parses() {
        let text = "pointer Pet.BestFriend to Pet { id }";
        let parse = parsed(text);
        assert_eq!(parse.item.errors(), vec![]);
        let declaration = as_pointer(&parse);
        assert_eq!(declaration.pointer_keyword.location, span_of(text, "pointer"));
        assert_eq!(declaration.client_pointer_name.location, span_of(text, "BestFriend"));
        assert_eq!(declaration.to_keyword.location, span_of(text, "to"));
        let target_anchor = span_of(text, "Pet {");
        match &declaration.target_type.item {
            TypeAnnotation::Named(named) => {
                assert_eq!(named.name.location, Span::new(target_anchor.start, target_anchor.start + 3));
            }
            annotation => panic!("expected a named target, got {annotation:?}"),
        }
        assert_eq!(selections(&declaration.selection_set).len(), 1);
    }

    #[test]
    fn a_full_pointer_declaration_parses_in_order() {
        let text = "pointer Pet.Owner($limit: Int) to Person! \"the owner\" { name }";
        let parse = parsed(text);
        assert_eq!(parse.item.errors(), vec![]);
        let declaration = as_pointer(&parse);
        assert!(declaration.variable_definitions.is_some());
        assert_eq!(declaration.target_type.location, span_of(text, "Person!"));
        assert!(declaration.description.is_some());
    }

    #[test]
    fn a_bracketed_pointer_target_parses() {
        let text = "pointer Pet.Friends to [Pet!]! { id }";
        let parse = parsed(text);
        assert_eq!(parse.item.errors(), vec![]);
        assert_eq!(as_pointer(&parse).target_type.location, span_of(text, "[Pet!]!"));
    }

    #[test]
    fn a_missing_to_keyword_reports_at_the_found_item() {
        let text = "pointer Pet.BestFriend Owner { id }";
        assert_unparsed(
            text,
            expected(Expectation::ToKeyword, Found::Token(Identifier)),
            span_of(text, "Owner"),
        );
    }

    #[test]
    fn a_missing_target_type_reports_after_to() {
        let text = "pointer Pet.BestFriend to { id }";
        assert_unparsed(
            text,
            expected(Expectation::TypeAnnotation, Found::Group(BracketKind::Brace)),
            span_of(text, "{ id }"),
        );
    }

    #[test]
    fn pointer_names_and_targets_resolve_with_their_ancestry() {
        let text = "pointer Pet.BestFriend to Owner { id }";
        let parse = parsed(text);
        match parse.resolve((), span_of(text, "BestFriend")) {
            IsographResolutionNode::ClientPointerName(name) => {
                assert_eq!(name.parent.inner.to_keyword.location, span_of(text, "to"));
            }
            node => panic!("expected the pointer name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "Owner")) {
            IsographResolutionNode::TypeName(name) => {
                match &name.parent.parent {
                    TypeAnnotationParent::PointerTarget(_) => {}
                    parent => panic!("expected the pointer-target parent, got {parent:?}"),
                }
            }
            node => panic!("expected the type name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "id")) {
            IsographResolutionNode::SelectionName(name) => {
                let scalar = match name.parent {
                    SelectionNameParent::Scalar(scalar) => scalar,
                    parent => panic!("expected a scalar parent, got {parent:?}"),
                };
                match scalar.parent.parent {
                    SelectionSetParent::Pointer(_) => {}
                    parent => panic!("expected the pointer at the top, got {parent:?}"),
                }
            }
            node => panic!("expected the selection name leaf, got {node:?}"),
        }
    }
```

## Landing checklist

1. The parse_iso_literal.rs, parse_error.rs, selections.rs, and variables.rs changes, the resolution-node variants, the test deletions, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past. The series is complete: every isograph literal form parses, and `UnsupportedDeclarationType` no longer exists.
