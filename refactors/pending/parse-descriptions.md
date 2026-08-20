# parse-descriptions: descriptions on declarations

A field declaration may carry a description between its variable definitions and its selection set. parse-pointers.md reuses the slot. Entrypoints carry none.

## The grammar this doc accepts

```
field <Identifier> . <Identifier> [<paren group>] [<description>] <brace group>
```

A description is one `StringLiteral` token (`"..."`) or one `BlockStringLiteral` token (`"""..."""`). A block string is a single token whatever it contains, line breaks included, so a multi-line description never splits the chunk. The tree stores the interned source slice, quotes included. Unquoting and block-string dedenting are derivations a consumer computes from that key.

Origin: `parse_optional_description` in `crates/isograph_lang_parser/src/description.rs` and `Description` in `crates/isograph_lang_types/src/string_key_wrappers.rs`. Delta: i2 does not unquote or dedent; isograph stores the cleaned inner text. i2 function name is `consume_description` (cursor `consume_*` convention).

## The types

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ClientFieldDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct Description(common_lang_types::DescriptionValue);

pub type DescriptionPath<'a> =
    PositionResolutionPath<&'a Description, ClientFieldDeclarationPath<'a>>;
```

parse-pointers.md converts the parent to `DescriptionParent` when the second parent arrives.

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
    pub variable_definitions: Option<WithSpan<VariableDeclarationList>>,
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
    pub variable_definitions: Option<WithSpan<VariableDeclarationList>>,
    #[resolve_field]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(ClientFieldDeclaration)]
    pub selection_set: WithSpan<SelectionSet>,
}
```

## The parser

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub(crate) fn consume_description(cursor: &mut ItemCursor<'_>) -> Option<WithSpan<Description>> {
    let span = cursor
        .consume_token_if(NonBracketTokenKind::StringLiteral, SemanticToken::String)
        .or_else(|| {
            cursor.consume_token_if(
                NonBracketTokenKind::BlockStringLiteral,
                SemanticToken::String,
            )
        })?;
    span.interned().map(Description).wrap_some()
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let variable_definitions = consume_variable_declaration_list(cursor);
    let description = consume_description(cursor);
    let selection_set = require_selection_set(cursor)?;
```

A malformed string token is not consumed here and surfaces as the found token of the selection-set expectation. A description in any other position is an ordinary unexpected token there.

## The resolution surface

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    Description(DescriptionPath<'a>),
```

The expansion follows `EntityNameWrapper`.

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
            Description("\"the home route\"".intern().to())
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
