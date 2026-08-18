# parse-descriptions: descriptions on declarations

A field declaration may carry a description between its variable definitions and its selection set. parse-pointers.md reuses the slot. Entrypoints carry none.

## The grammar this doc accepts

```
field <Identifier> . <Identifier> [<paren group>] [<description>] <brace group>
```

A description is one `StringLiteral` token (`"..."`) or one `BlockStringLiteral` token (`"""..."""`). A block string is a single token whatever it contains, line breaks included, so a multi-line description never splits the chunk. The tree stores the interned source slice, quotes included. Unquoting and block-string dedenting are derivations a consumer computes from that key.

## The types

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ClientFieldDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct Description(common_lang_types::DescriptionValue);

impl From<intern::string_key::StringKey> for Description {
    fn from(key: intern::string_key::StringKey) -> Self {
        Description(key.to())
    }
}

pub type DescriptionPath<'a> =
    PositionResolutionPath<&'a Description, ClientFieldDeclarationPath<'a>>;
```

parse-pointers.md converts the parent to an enum when the second parent arrives.

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct ClientFieldDeclaration {
    #[resolve_field(parent_variant = Field)]
    pub parent_type: WithSpan<EntityName>,
    #[resolve_field(parent_variant = Field)]
    pub client_field_name: WithSpan<ClientFieldName>,
    #[resolve_field]
    pub variable_definitions: Option<WithSpan<VariableDeclarationList>>,
    #[resolve_field(parent_variant = Field)]
    pub selection_set: WithSpan<SelectionSet>,
}
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct ClientFieldDeclaration {
    #[resolve_field(parent_variant = Field)]
    pub parent_type: WithSpan<EntityName>,
    #[resolve_field(parent_variant = Field)]
    pub client_field_name: WithSpan<ClientFieldName>,
    #[resolve_field]
    pub variable_definitions: Option<WithSpan<VariableDeclarationList>>,
    #[resolve_field]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field(parent_variant = Field)]
    pub selection_set: WithSpan<SelectionSet>,
}
```

## The parser

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub(crate) fn consume_description(cursor: &mut ItemCursor<'_>) -> Option<WithSpan<Description>> {
    let span = cursor
        .consume_token_if(NonBracketTokenKind::StringLiteral)
        .or_else(|| cursor.consume_token_if(NonBracketTokenKind::BlockStringLiteral))?;
    cursor
        .token_text(span)
        .intern()
        .to::<Description>()
        .with_span(span)
        .wrap_some()
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let variable_definitions = consume_variable_declaration_list(cursor, push_error);
    let description = consume_description(cursor);
    let selection_set = require_selection_set(cursor, push_error)?;
```

A malformed string token is not consumed here and surfaces as the found token of the selection-set expectation. A description in any other position is an ordinary unexpected token there.

## The resolution surface

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    Description(DescriptionPath<'a>),
```

The expansion follows `EntityName`.

## Tests

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn a_single_line_description_parses_with_its_quotes() {
        let text = "field Query.Foo \"the home route\" { bar }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let description = as_field(parse.reference())
            .description
            .as_ref()
            .expect("the fixture carries a description");
        assert_eq!(description.location, span_of(text, "\"the home route\""));
        assert_eq!(
            description.item,
            "\"the home route\"".intern().to()
        );
    }

    #[test]
    fn a_block_string_description_spans_lines_without_splitting_the_chunk() {
        let text = "field Query.Foo($id: ID!) \"\"\"\n  the home\n  route\n\"\"\" { bar }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let description = as_field(parse.reference())
            .description
            .as_ref()
            .expect("the fixture carries a description");
        assert_eq!(
            description.location,
            span_of(text, "\"\"\"\n  the home\n  route\n\"\"\"")
        );
        assert!(as_field(parse.reference()).variable_definitions.is_some());
    }

    #[test]
    fn a_description_after_the_selection_set_is_leftover() {
        let text = "field Query.Foo { bar } \"too late\"";
        let (parse, errors) = parsed(text);
        as_field(parse.reference());
        assert!(first_slot(parse.reference()).extra_tokens.is_some());
        assert_eq!(
            errors,
            expected(
                Expectation::EndOfDeclaration,
                Found::Token(NonBracketTokenKind::StringLiteral)
            )
            .with_span(span_of(text, "\"too late\""))
            .wrap_vec(),
        );
    }

    #[test]
    fn an_entrypoint_carries_no_description() {
        let text = "entrypoint Query.foo \"nope\"";
        let (parse, errors) = parsed(text);
        as_entrypoint(parse.reference());
        assert_eq!(
            errors,
            expected(
                Expectation::EndOfDeclaration,
                Found::Token(NonBracketTokenKind::StringLiteral)
            )
            .with_span(span_of(text, "\"nope\""))
            .wrap_vec(),
        );
    }

    #[test]
    fn a_description_resolves_to_its_leaf() {
        let text = "field Query.Foo \"the home route\" { bar }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "home")) {
            IsographResolutionNode::Description(description) => {
                assert_eq!(
                    description.parent.inner.client_field_name.location,
                    span_of(text, "Foo")
                );
            }
            node => panic!("expected the description leaf, got {node:?}"),
        }
    }
```

## Landing checklist

1. The `Description` type, the `ClientFieldDeclaration` and `parse_field` changes, the resolution-node variant, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
