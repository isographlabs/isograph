# parse-fields: field declarations

`field Type.name { ... }`. Lands after parse-selection-sets.md. Wires `require_selection_set` onto `IsoLiteralItem::Field`.

## The grammar this doc accepts

```
field <Identifier> . <Identifier> <brace group>
```

The brace group is required and is the last item of the chunk. Its interior is a selection set.

## Change 1: `IsoLiteralItem::Field`

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        "entrypoint" => IsoLiteralItem::Entrypoint(parse_entrypoint(cursor)?).wrap_ok(),
        "field" | "pointer" => ParseError::UnsupportedDeclarationType
            .with_span(keyword.location)
            .wrap_err(),
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
    Field(ClientFieldDeclaration),
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        "entrypoint" => IsoLiteralItem::Entrypoint(parse_entrypoint(cursor)?).wrap_ok(),
        "field" => IsoLiteralItem::Field(parse_field(cursor)?).wrap_ok(),
        "pointer" => ParseError::UnsupportedDeclarationType
            .with_span(keyword.location)
            .wrap_err(),
```

The test `field_and_pointer_declarations_do_not_parse_yet` narrows to its pointer case.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
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

pub type ClientFieldDeclarationPath<'a> =
    PositionResolutionPath<&'a ClientFieldDeclaration, IsoLiteralSlotPath<'a>>;
```

There is no `FieldKeyword`. A position on `field` answers `ClientFieldDeclaration`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_field(
    cursor: &mut ItemCursor<'_>,
) -> Result<ClientFieldDeclaration, WithSpan<ParseError>> {
    let parent_type = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Type)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Period, SemanticToken::Period)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Period)))?;
    let client_field_name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    let selection_set = require_selection_set(cursor)?;
    ClientFieldDeclaration {
        parent_type: parent_type.interned().map(EntityNameWrapper),
        client_field_name: client_field_name.interned().map(ClientFieldNameWrapper),
        selection_set,
    }
    .wrap_ok()
}
```

## Change 2: `EntityNameWrapper` / `ClientFieldNameWrapper`

`EntityName` and `ClientFieldName` gain a second parent and become wrappers without `From<StringKey>`.

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EntrypointDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntityName(common_lang_types::EntityName);

impl From<intern::string_key::StringKey> for EntityName {
    fn from(key: intern::string_key::StringKey) -> Self {
        EntityName(key.to())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EntrypointDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ClientFieldName(common_lang_types::SelectableName);

impl From<intern::string_key::StringKey> for ClientFieldName {
    fn from(key: intern::string_key::StringKey) -> Self {
        ClientFieldName(key.to())
    }
}

pub type EntityNamePath<'a> = PositionResolutionPath<&'a EntityName, EntrypointDeclarationPath<'a>>;

pub type ClientFieldNamePath<'a> =
    PositionResolutionPath<&'a ClientFieldName, EntrypointDeclarationPath<'a>>;
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct EntrypointDeclaration {
    #[resolve_field]
    pub parent_type: WithSpan<EntityName>,
    #[resolve_field]
    pub client_field_name: WithSpan<ClientFieldName>,
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    EntrypointDeclaration {
        parent_type: parent_type.interned(),
        client_field_name: client_field_name.interned(),
    }
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EntityNameWrapperParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntityNameWrapper(common_lang_types::EntityName);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ClientFieldNameWrapperParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ClientFieldNameWrapper(common_lang_types::SelectableName);

#[derive(Debug)]
pub enum EntityNameWrapperParent<'a> {
    Entrypoint(EntrypointDeclarationPath<'a>),
    Field(ClientFieldDeclarationPath<'a>),
}

#[derive(Debug)]
pub enum ClientFieldNameWrapperParent<'a> {
    Entrypoint(EntrypointDeclarationPath<'a>),
    Field(ClientFieldDeclarationPath<'a>),
}

pub type EntityNameWrapperPath<'a> =
    PositionResolutionPath<&'a EntityNameWrapper, EntityNameWrapperParent<'a>>;

pub type ClientFieldNameWrapperPath<'a> =
    PositionResolutionPath<&'a ClientFieldNameWrapper, ClientFieldNameWrapperParent<'a>>;
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct EntrypointDeclaration {
    #[resolve_field]
    #[parent_variant(Entrypoint)]
    pub parent_type: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(Entrypoint)]
    pub client_field_name: WithSpan<ClientFieldNameWrapper>,
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    EntrypointDeclaration {
        parent_type: parent_type.interned().map(EntityNameWrapper),
        client_field_name: client_field_name.interned().map(ClientFieldNameWrapper),
    }
```

Before:

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    EntityName(EntityNamePath<'a>),
    ClientFieldName(ClientFieldNamePath<'a>),
```

After:

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    EntityNameWrapper(EntityNameWrapperPath<'a>),
    ClientFieldNameWrapper(ClientFieldNameWrapperPath<'a>),
```

`as_entrypoint` gains an exhaustive arm:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn as_entrypoint(parse: &WithSpan<IsoLiteralParse>) -> &EntrypointDeclaration {
        let item = parsed_item(parse).expect("the fixture's literal parsed an item");
        match item {
            IsoLiteralItem::Entrypoint(declaration) => declaration,
            item => panic!("expected an entrypoint declaration, got {item:?}"),
        }
    }
```

The resolve test `names_resolve_to_their_leaves_and_the_rest_to_the_declaration` matches through `EntityNameWrapperParent::Entrypoint`. The item-equality assertions in `an_entrypoint_declaration_parses_with_tight_spans` become `EntityNameWrapper("Query".intern().to())` and `ClientFieldNameWrapper("foo".intern().to())`.

## Change 3: `SelectionSetParent::Field`

Before:

```rust
// from crates/isograph_parser/src/selections.rs
pub enum SelectionSetParent<'a> {
    Object(Box<ObjectSelectionPath<'a>>),
}
```

After:

```rust
// from crates/isograph_parser/src/selections.rs
pub enum SelectionSetParent<'a> {
    Field(ClientFieldDeclarationPath<'a>),
    Object(Box<ObjectSelectionPath<'a>>),
}
```

`ClientFieldDeclaration.selection_set` is `#[resolve_field]` + `#[parent_variant(Field)]`.

## The resolution surface

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
```

A position on a list comma or on whitespace inside a selection set answers `SelectionSet`. A leftover token answers `NonBracketToken` through `UnparsedChunkItemsParent::SelectionSlot`. A failed selection chunk is the same walk.

## Tests

Extending the `parse_iso_literal.rs` test module. Helpers `parsed`, `parsed_with_errors`, `span_of`, `expected`, `token`, `first_slot`, `parsed_item` stay.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn as_field(parse: &WithSpan<IsoLiteralParse>) -> &ClientFieldDeclaration {
        match parsed_item(parse).expect("the fixture's literal parsed an item") {
            IsoLiteralItem::Field(declaration) => declaration,
            item => panic!("expected a field declaration, got {item:?}"),
        }
    }

    fn selections(
        selection_set: &WithSpan<SelectionSet>,
    ) -> &[WithSpan<Slot<Selection, UnparsedChunkItems>>] {
        selection_set.item.0.reference()
    }

    fn as_scalar(slot: &Slot<Selection, UnparsedChunkItems>) -> &ScalarSelection {
        match slot.item.as_ref().map(|wrapped| wrapped.item.reference()) {
            Some(Selection::Scalar(scalar)) => scalar,
            other => panic!("expected a scalar selection, got {other:?}"),
        }
    }

    fn as_object(slot: &Slot<Selection, UnparsedChunkItems>) -> &ObjectSelection {
        match slot.item.as_ref().map(|wrapped| wrapped.item.reference()) {
            Some(Selection::Object(object)) => object,
            other => panic!("expected an object selection, got {other:?}"),
        }
    }

    #[test]
    fn a_field_declaration_parses_with_scalar_selections() {
        let text = "field Query.Foo {\n  bar,\n  baz\n}";
        let (parse, errors) = parsed(text);
        let declaration = as_field(parse.reference());
        assert_eq!(
            declaration.parent_type.item,
            EntityNameWrapper("Query".intern().to())
        );
        assert_eq!(
            declaration.client_field_name.item,
            ClientFieldNameWrapper("Foo".intern().to())
        );
        assert_eq!(declaration.parent_type.location, span_of(text, "Query"));
        assert_eq!(declaration.client_field_name.location, span_of(text, "Foo"));
        assert_eq!(
            declaration.selection_set.location,
            Span::new(span_of(text, "{").start, span_of(text, "}").end)
        );
        let items = selections(declaration.selection_set.reference());
        assert_eq!(items.len(), 2);
        assert_eq!(
            as_scalar(items[0].item.reference()).name.item,
            SelectionNameWrapper("bar".intern().to())
        );
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "bar"));
        assert_eq!(as_scalar(items[1].item.reference()).name.location, span_of(text, "baz"));
        assert_eq!(items[0].location, span_of(text, "bar"));
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn empty_selection_sets_hold_zero_selections() {
        for text in [
            "field Query.Foo {}",
            "field Query.Foo { }",
            "field Query.Foo {\n}",
        ] {
            let (parse, errors) = parsed(text);
            assert_eq!(
                selections(as_field(parse.reference()).selection_set.reference()).len(),
                0,
                "for literal {text:?}"
            );
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_field_declaration_without_a_selection_set_is_a_failed_item() {
        let text = "field Query.Foo";
        let end = span_of(text, "Foo").end;
        assert_no_declaration(
            text,
            expected(Expectation::SelectionSet, Found::EndOfChunk),
            Span::new(end, end),
        );
    }

    #[test]
    fn a_selection_set_split_onto_its_own_line_is_a_failed_item() {
        let text = "field Query.Foo\n{ bar }";
        let end = span_of(text, "Foo").end;
        assert_no_declaration(
            text,
            expected(Expectation::SelectionSet, Found::EndOfChunk),
            Span::new(end, end),
        );
    }

    #[test]
    fn a_final_comma_after_the_field_declaration_is_an_error() {
        let text = "field Query.Foo { bar },";
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
    fn tokens_after_the_selection_set_are_leftover() {
        let text = "field Query.Foo { bar } junk";
        let (parse, errors) = parsed(text);
        as_field(parse.reference());
        assert!(first_slot(parse.reference()).extra_tokens.is_some());
        assert_eq!(
            errors,
            expected(Expectation::EndOfDeclaration, Found::Token(Identifier))
                .with_span(span_of(text, "junk"))
                .wrap_vec(),
        );
    }

    #[test]
    fn selection_names_resolve_with_their_ancestry() {
        let text = "field Query.Foo { pet { name } }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "name")) {
            IsographResolutionNode::SelectionNameWrapper(name) => {
                let scalar = match name.parent {
                    SelectionNameWrapperParent::Scalar(scalar) => scalar,
                    parent => panic!("expected a scalar parent, got {parent:?}"),
                };
                let object = match scalar.parent.parent.parent {
                    SelectionSetParent::Object(object) => object,
                    parent => panic!("expected an object-selection parent, got {parent:?}"),
                };
                assert_eq!(object.inner.name.location, span_of(text, "pet"));
                match object.parent.parent.parent {
                    SelectionSetParent::Field(declaration) => {
                        assert_eq!(
                            declaration.inner.client_field_name.location,
                            span_of(text, "Foo")
                        );
                    }
                    parent => panic!("expected the declaration at the top, got {parent:?}"),
                }
            }
            node => panic!("expected the selection name leaf, got {node:?}"),
        }
    }

    #[test]
    fn leftover_positions_resolve_to_the_leftover_token() {
        let text = "field Query.Foo { bar baz }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "baz")) {
            IsographResolutionNode::NonBracketToken(_) => {}
            node => panic!("expected the leftover token, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "bar")) {
            IsographResolutionNode::SelectionNameWrapper(_) => {}
            node => panic!("expected the selection name, got {node:?}"),
        }
    }

    #[test]
    fn positions_inside_a_failed_selection_resolve_through_unparsed_items() {
        let text = "field Query.Foo { 42 }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "42")) {
            IsographResolutionNode::NonBracketToken(token) => {
                match token.parent {
                    ChunkContentItemParent::Unparsed(unparsed) => match unparsed.parent {
                        UnparsedChunkItemsParent::SelectionSlot(_) => {}
                        parent => panic!("expected a selection-slot parent, got {parent:?}"),
                    },
                    parent => panic!("expected an unparsed parent, got {parent:?}"),
                }
            }
            node => panic!("expected the token leaf, got {node:?}"),
        }
    }

    #[test]
    fn whitespace_and_separators_inside_a_selection_set_resolve_to_the_set() {
        let text = "field Query.Foo { bar, baz }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, ",")) {
            IsographResolutionNode::SelectionSet(_) => {}
            node => panic!("expected the selection set, got {node:?}"),
        }
    }

    #[test]
    fn argument_names_resolve_through_the_selection() {
        let text = "field Query.Foo { bar(id: $x) }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "id")) {
            IsographResolutionNode::FieldArgumentNameWrapper(name) => match name.parent {
                FieldArgumentNameWrapperParent::NamedArgument(argument) => {
                    match argument.parent.parent.parent {
                        ArgumentListParent::Scalar(_) => {}
                        parent => panic!("expected a scalar argument list, got {parent:?}"),
                    }
                }
                parent => panic!("expected a named-argument parent, got {parent:?}"),
            },
            node => panic!("expected the argument name, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "$")) {
            IsographResolutionNode::VariableUse(_) => {}
            node => panic!("expected the variable use, got {node:?}"),
        }
    }

    #[test]
    fn a_field_records_keyword_type_period_name_and_braces() {
        let text = "field Query.Foo { bar }";
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
                SemanticToken::Brace.with_span(span_of(text, "{")),
                SemanticToken::FieldName.with_span(span_of(text, "bar")),
                SemanticToken::Brace.with_span(span_of(text, "}")),
            ],
        );
    }

    #[test]
    fn names_resolve_to_their_leaves_and_the_rest_to_the_declaration() {
        let text = "entrypoint Query.foo";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "Query")) {
            IsographResolutionNode::EntityNameWrapper(name) => {
                match name.parent {
                    EntityNameWrapperParent::Entrypoint(declaration) => {
                        assert_eq!(
                            declaration.inner.client_field_name.location,
                            span_of(text, "foo")
                        );
                    }
                    parent => panic!("expected an entrypoint parent, got {parent:?}"),
                }
            }
            node => panic!("expected the entity name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "foo")) {
            IsographResolutionNode::ClientFieldNameWrapper(_) => {}
            node => panic!("expected the client field name leaf, got {node:?}"),
        }
        for span in [
            span_of(text, "entrypoint"),
            span_of(text, "."),
            Span::new(
                span_of(text, "entrypoint").end,
                span_of(text, "Query").start,
            ),
        ] {
            match parse.resolve((), span) {
                IsographResolutionNode::EntrypointDeclaration(_) => {}
                node => panic!("expected the declaration leaf at {span}, got {node:?}"),
            }
        }
    }
```

`field_and_pointer_declarations_do_not_parse_yet` keeps only the pointer fixture.

## Landing checklist

1. `IsoLiteralItem::Field`, `ClientFieldDeclaration`, `SelectionSetParent::Field`, `EntityNameWrapper` / `ClientFieldNameWrapper`, the tests. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
