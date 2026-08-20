# optional-to: `to Type` on field declarations

`field Type.name [vars] [to type] [description] { set }`. Fields and pointers are one struct. `to` is optional syntax. The target is parse-variables.md's type annotation (`Pet`, `Pet!`, `[Pet]`, `[Pet!]!`, `[[Pet]]`). The keyword is `field`. There is no `pointer` keyword and no `ClientPointerDeclaration`. Removes `UnsupportedDeclarationType`.

Lands after parse-type-dot-name.md. Type annotations are parse-variables.md's. Directives land in parse-directives.md.

Origin: `ClientPointerDeclaration` in `crates/isograph_lang_types/src/declarations/client_selectable_declaration.rs` and `parse_iso_client_pointer_declaration` in `crates/isograph_lang_parser/src/parse_iso_literal.rs`. Delta: the target sits on `ClientFieldDeclaration` as `target_type: Option<WithSpan<TypeAnnotation>>`; the name wrapper stays `ClientScalarSelectableNameWrapper`; no `const_export_name`, `definition_path`, `directives`, or `semantic_tokens`.

## The grammar this doc accepts

```
field <Identifier> . <Identifier> [<paren group>] [to <type>] [<description>] <brace group>
```

`<type>` is parse-variables.md's type annotation. Origin: the type forms in parse-variables.md. Delta: none.

```
Pet    Pet!    [Pet]    [Pet!]!    [[Pet]]
```

`consume_to_target` calls `parse_type_annotation`. A bracket group after `to` is a list type, not leftover.

The `to` keyword is an identifier whose text is `to`. A position on `to` answers `ClientFieldDeclaration`. There is no `ToKeyword` node.

`pointer` is not a declaration keyword. `parse_iso_literal_item`'s `_` arm reports `DeclarationKeyword` at that identifier, including `pointer`.

## Changes to parse_error.rs

`UnsupportedDeclarationType` is deleted. Remaining structural variants: `Expected`, `EmptyLiteral`, `MultipleDeclarations`, `IntegerDoesNotFitI64`.

```rust
// from crates/isograph_parser/src/parse_error.rs
    #[error("one of `entrypoint` or `field`")]
    DeclarationKeyword,
```

```rust
// from crates/isograph_parser/src/parse_error.rs
    #[error("the keyword `to`")]
    ToKeyword,
```

`expectation_unit_variants_use_their_messages` asserts the new `DeclarationKeyword` string and `ToKeyword`:

```rust
// from crates/isograph_parser/src/parse_error.rs
        assert_eq!(
            Expectation::DeclarationKeyword.to_string(),
            "one of `entrypoint` or `field`",
        );
        assert_eq!(
            Expectation::ToKeyword.to_string(),
            "the keyword `to`",
        );
```

## Changes to ClientFieldDeclaration

Origin: landed `ClientFieldDeclaration` in `crates/isograph_parser/src/parse_iso_literal.rs`, plus `variable_definitions` from parse-variables.md. Delta: `target_type`.

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
    pub variable_definitions: Option<WithSpan<VariableDeclarationOrUsageList>>,
    #[resolve_field]
    pub description: Option<WithSpan<Description>>,
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
    pub target_type: Option<WithSpan<TypeAnnotation>>,
    #[resolve_field]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(ClientFieldDeclaration)]
    pub selection_set: WithSpan<SelectionSet>,
}
```

`IsoLiteralItem` is unchanged: `Entrypoint` and `Field`. No `Pointer` variant. `VariableDeclarationOrUsageList` and `Description` stay parented by `ClientFieldDeclarationPath`. `SelectionSetParent` is unchanged. `EntityNameWrapperParent` is unchanged: a target type name is `NamedTypeAnnotation`, not a new parent-type variant.

## Changes to TypeAnnotationParent

Origin: parse-variables.md. Delta: `ClientFieldDeclaration`.

Before:

```rust
// from crates/isograph_parser/src/variables.rs
pub enum TypeAnnotationParent<'a> {
    Variable(VariableDeclarationOrUsagePath<'a>),
    List(Box<ListTypeAnnotationPath<'a>>),
}
```

After:

```rust
// from crates/isograph_parser/src/variables.rs
pub enum TypeAnnotationParent<'a> {
    Variable(VariableDeclarationOrUsagePath<'a>),
    List(Box<ListTypeAnnotationPath<'a>>),
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
}
```

## Changes to parse_iso_literal_item

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        "field" => IsoLiteralItem::Field(parse_field(cursor)?).wrap_ok(),
        "pointer" => ParseError::UnsupportedDeclarationType
            .with_span(keyword.location)
            .wrap_err(),
        _ => ParseError::expected(
            Expectation::DeclarationKeyword,
            Found::Token(NonBracketTokenKind::Identifier),
        )
        .with_span(keyword.location)
        .wrap_err(),
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        "field" => IsoLiteralItem::Field(parse_field(cursor)?).wrap_ok(),
        _ => ParseError::expected(
            Expectation::DeclarationKeyword,
            Found::Token(NonBracketTokenKind::Identifier),
        )
        .with_span(keyword.location)
        .wrap_err(),
```

The test `field_and_pointer_declarations_do_not_parse_yet` is deleted.

## Changes to parse_field

Origin: parse-type-dot-name.md after. Delta: `consume_to_target` between the variable list and the description.

`use crate` in parse_iso_literal.rs gains `ChunkContentItem`, `NonBracketToken`, `parse_type_annotation`.

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_field(
    cursor: &mut ItemCursor<'_>,
) -> Result<ClientFieldDeclaration, WithSpan<ParseError>> {
    let (parent_type, client_field_name) = parse_type_dot_name(cursor)?;
    let variable_definitions = consume_variable_declaration_list(cursor);
    let description = consume_description(cursor);
    let selection_set = require_selection_set(cursor)?;
    ClientFieldDeclaration {
        parent_type,
        client_field_name: client_field_name.map(ClientScalarSelectableNameWrapper),
        variable_definitions,
        description,
        selection_set,
    }
    .wrap_ok()
}
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_field(
    cursor: &mut ItemCursor<'_>,
) -> Result<ClientFieldDeclaration, WithSpan<ParseError>> {
    let (parent_type, client_field_name) = parse_type_dot_name(cursor)?;
    let variable_definitions = consume_variable_declaration_list(cursor);
    let target_type = consume_to_target(cursor)?;
    let description = consume_description(cursor);
    let selection_set = require_selection_set(cursor)?;
    ClientFieldDeclaration {
        parent_type,
        client_field_name: client_field_name.map(ClientScalarSelectableNameWrapper),
        variable_definitions,
        target_type,
        description,
        selection_set,
    }
    .wrap_ok()
}

fn consume_to_target(
    cursor: &mut ItemCursor<'_>,
) -> Result<Option<WithSpan<TypeAnnotation>>, WithSpan<ParseError>> {
    let peek = cursor.peek();
    let Some(peek) = peek else {
        return None.wrap_ok();
    };
    let item = peek.view();
    let is_identifier = matches!(
        item.item.reference(),
        ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::Identifier))
    );
    let location = item.location;
    drop(peek);
    if !is_identifier || &cursor.text()[location.as_usize_range()] != "to" {
        return None.wrap_ok();
    }
    cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Keyword)
        .map_err(|()| cursor.expected(Expectation::ToKeyword))?;
    let target_type = parse_type_annotation(cursor)?;
    target_type.wrap_some().wrap_ok()
}
```

A non-`to` identifier is not consumed. `field Query.Foo Owner { id }` fails at `Owner` as `Expected(SelectionSet, Token(Identifier))`, not as `ToKeyword`. Peeking `to` commits to a type: `field Query.Foo to { id }` is `Expected(TypeAnnotation, Group(Brace))`.

`ItemCursor::text` loses `#[cfg_attr(not(test), expect(dead_code))]`. `consume_to_target` is the production caller.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    pub(crate) fn text(&self) -> &'a str {
        self.text
    }
```

`CursorPeek` is unchanged. The peek copies the span, `drop`s, then slices `cursor.text()`.

## The resolution surface

No new `IsographResolutionNode` variants. A position on `to` answers `ClientFieldDeclaration`. A target type name answers `EntityNameWrapper` with `EntityNameWrapperParent::NamedTypeAnnotation` whose `TypeAnnotationParent` is `ClientFieldDeclaration`. A field without `to` has `target_type: None`.

## Tests

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn a_field_without_to_has_no_target_type() {
        let text = "field Query.Foo { bar }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        assert_eq!(as_field(parse.reference()).target_type, None);
    }

    #[test]
    fn a_field_with_to_parses_the_target_type() {
        let text = "field Pet.BestFriend to Owner { id }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let declaration = as_field(parse.reference());
        assert_eq!(
            declaration.client_field_name.item,
            ClientScalarSelectableNameWrapper("BestFriend".intern().to())
        );
        assert_eq!(
            declaration.client_field_name.location,
            span_of(text, "BestFriend")
        );
        let target = declaration
            .target_type
            .as_ref()
            .expect("the fixture writes to Owner");
        assert_eq!(target.location, span_of(text, "Owner"));
        match target.item.reference() {
            TypeAnnotation::Named(named) => {
                assert_eq!(named.name.location, span_of(text, "Owner"));
            }
            annotation => panic!("expected a named target, got {annotation:?}"),
        }
        assert_eq!(selections(declaration.selection_set.reference()).len(), 1);
    }

    #[test]
    fn a_full_field_parses_in_order() {
        let text = "field Pet.Owner($limit: Int) to Person! \"the owner\" { name }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let declaration = as_field(parse.reference());
        assert!(declaration.variable_definitions.is_some());
        assert_eq!(
            declaration
                .target_type
                .as_ref()
                .expect("the fixture writes to Person!")
                .location,
            span_of(text, "Person!")
        );
        assert!(declaration.description.is_some());
    }

    #[test]
    fn a_to_target_accepts_every_type_annotation_form() {
        for (text, target) in [
            ("field Query.Foo to Pet { id }", "Pet"),
            ("field Query.Foo to Pet! { id }", "Pet!"),
            ("field Query.Foo to [Pet] { id }", "[Pet]"),
            ("field Query.Foo to [Pet!]! { id }", "[Pet!]!"),
            ("field Query.Foo to [[Pet]] { id }", "[[Pet]]"),
        ] {
            let (parse, errors) = parsed(text);
            assert_eq!(errors, vec![], "for literal {text:?}");
            assert_eq!(
                as_field(parse.reference())
                    .target_type
                    .as_ref()
                    .expect("the fixture writes a target")
                    .location,
                span_of(text, target),
                "for literal {text:?}",
            );
        }
    }

    #[test]
    fn a_bracketed_target_is_a_list_annotation() {
        let text = "field Query.Friends to [Pet!]! { id }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let target = as_field(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes a list target");
        assert_eq!(target.location, span_of(text, "[Pet!]!"));
        match target.item.reference() {
            TypeAnnotation::List(list) => {
                let inner = list
                    .inner
                    .as_ref()
                    .expect("the list holds a type");
                match inner.item.reference() {
                    TypeAnnotation::Named(named) => {
                        assert_eq!(named.name.location, span_of(text, "Pet"));
                    }
                    annotation => panic!("expected a named inner type, got {annotation:?}"),
                }
            }
            annotation => panic!("expected a list target, got {annotation:?}"),
        }
    }

    #[test]
    fn an_empty_list_target_fails_as_a_type() {
        let text = "field Query.Foo to [] { id }";
        let interior = span_of(text, "[]").start + 1;
        assert_no_declaration(
            text,
            expected(Expectation::TypeAnnotation, Found::EndOfChunk),
            Span::new(interior, interior),
        );
    }

    #[test]
    fn a_non_to_identifier_is_not_consumed_as_to() {
        let text = "field Query.Foo Owner { id }";
        assert_no_declaration(
            text,
            expected(Expectation::SelectionSet, Found::Token(Identifier)),
            span_of(text, "Owner"),
        );
    }

    #[test]
    fn a_missing_target_type_reports_after_to() {
        let text = "field Pet.BestFriend to { id }";
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
    fn a_to_at_the_end_of_the_chunk_expects_a_type() {
        let text = "field Pet.BestFriend to";
        let end = span_of(text, "to").end;
        assert_no_declaration(
            text,
            expected(Expectation::TypeAnnotation, Found::EndOfChunk),
            Span::new(end, end),
        );
    }

    #[test]
    fn a_to_after_the_description_is_not_a_target() {
        let text = "field Query.Foo \"x\" to Owner { id }";
        assert_no_declaration(
            text,
            expected(Expectation::SelectionSet, Found::Token(Identifier)),
            span_of(text, "to"),
        );
    }

    #[test]
    fn a_pointer_keyword_is_not_a_declaration() {
        let text = "pointer Pet.BestFriend to Owner { id }";
        assert_no_declaration(
            text,
            expected(DeclarationKeyword, Found::Token(Identifier)),
            span_of(text, "pointer"),
        );
    }

    #[test]
    fn a_final_comma_after_a_field_with_to_is_an_error() {
        let text = "field Pet.BestFriend to Owner { id },";
        let (parse, errors) = parsed(text);
        as_field(parse.reference());
        assert_eq!(
            errors,
            expected(Expectation::EndOfDeclaration, Found::Token(Comma))
                .with_span(span_of(text, ","))
                .wrap_vec(),
        );
    }

    #[test]
    fn to_and_the_target_resolve_with_their_ancestry() {
        let text = "field Pet.BestFriend to Owner { id }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "to")) {
            IsographResolutionNode::ClientFieldDeclaration(_) => {}
            node => panic!("expected the declaration at `to`, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "BestFriend")) {
            IsographResolutionNode::ClientScalarSelectableNameWrapper(name) => {
                match name.parent {
                    ClientScalarSelectableNameWrapperParent::ClientFieldDeclaration(
                        declaration,
                    ) => {
                        assert_eq!(
                            declaration
                                .inner
                                .target_type
                                .as_ref()
                                .map(|target| target.location),
                            span_of(text, "Owner").wrap_some(),
                        );
                    }
                    parent => panic!("expected a field parent, got {parent:?}"),
                }
            }
            node => panic!("expected the field name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "Owner")) {
            IsographResolutionNode::EntityNameWrapper(name) => match name.parent {
                EntityNameWrapperParent::NamedTypeAnnotation(named) => match named.parent {
                    TypeAnnotationParent::ClientFieldDeclaration(_) => {}
                    parent => panic!("expected the field as type parent, got {parent:?}"),
                },
                parent => panic!("expected a named type annotation, got {parent:?}"),
            },
            node => panic!("expected the type name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "id")) {
            IsographResolutionNode::SelectionNameWrapper(name) => {
                match name.parent.parent.parent.parent {
                    SelectionSetParent::ClientFieldDeclaration(_) => {}
                    parent => panic!("expected the field at the top, got {parent:?}"),
                }
            }
            node => panic!("expected the selection name leaf, got {node:?}"),
        }
    }
```

## Landing checklist

1. The parse_iso_literal.rs, parse_error.rs, variables.rs, and chunk_stream.rs changes, the test deletion, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
