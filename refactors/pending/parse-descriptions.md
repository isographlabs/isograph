# parse-descriptions: descriptions on declarations

Fifth doc of the series parsing-plan.md orders, after parse-variables.md. A field declaration may carry a description between its variable definitions and its selection set; parse-pointers.md reuses the slot. Entrypoints carry none, as upstream.

## The grammar this doc accepts

```
field <Identifier> . <Identifier> [<paren group>] [<description>] <brace group>
```

A description is one `StringLiteral` token (`"..."`) or one `BlockStringLiteral` token (`"""..."""`). A block string is a single token whatever it contains, line breaks included, so a multi-line description never splits the chunk. The AST stores the token's span, quotes included; unquoting and block-string dedenting are derivations a consumer computes from the span, not parse work.

## The types

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
/// A declaration's description: one string or block-string token, quotes included in
/// the span.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ClientFieldDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct Description;

pub type DescriptionPath<'a> = PositionResolutionPath<&'a Description, ClientFieldDeclarationPath<'a>>;
```

parse-pointers.md converts the parent to an enum when the second parent arrives. `ClientFieldDeclaration` gains the slot:

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
    #[resolve_field]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field(parent_variant = Field)]
    pub selection_set: WithSpan<SelectionSet>,
}
```

## The parser

```rust
// from crates/isograph_parser/src/chunk_stream.rs
impl<'a> ItemCursor<'a> {
    pub(crate) fn consume_token_if_any(&mut self, kinds: &[NonBracketTokenKind]) -> Option<Span> {
        /* parsing-standards.md */
    }
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub(crate) fn consume_description(cursor: &mut ItemCursor<'_>) -> Option<WithSpan<Description>> {
    let span = cursor.consume_token_if_any(&[
        NonBracketTokenKind::StringLiteral,
        NonBracketTokenKind::BlockStringLiteral,
    ])?;
    Some(WithSpan::new(Description, span))
}
```

`parse_field`, between the variable definitions and the selection set:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let variable_definitions = consume_variable_declaration_list(cursor);
    let description = consume_description(cursor);
    let selection_set = require_selection_set(cursor)?;
```

No error paths are added: a malformed string token (`ErrorUnterminatedString` and kin) is not consumed here and surfaces as the found token of the selection-set expectation, and a description in any other position is an ordinary unexpected token there.

## The resolution surface

`IsographResolutionNode` gains:

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    Description(DescriptionPath<'a>),
```

The expansion follows `EntityName`'s fieldless-leaf pattern with the names substituted.

## Tests

Extending the parse_iso_literal.rs test module.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs (test module)
    #[test]
    fn a_single_line_description_parses_with_its_quotes() {
        let text = "field Query.Foo \"the home route\" { bar }";
        let parse = parsed(text);
        assert_eq!(parse.item.errors(), vec![]);
        let description = as_field(&parse).description.as_ref().expect("the fixture carries a description");
        assert_eq!(description.location, span_of(text, "\"the home route\""));
    }

    #[test]
    fn a_block_string_description_spans_lines_without_splitting_the_chunk() {
        let text = "field Query.Foo($id: ID!) \"\"\"\n  the home\n  route\n\"\"\" { bar }";
        let parse = parsed(text);
        assert_eq!(parse.item.errors(), vec![]);
        let description = as_field(&parse).description.as_ref().expect("the fixture carries a description");
        assert_eq!(description.location, span_of(text, "\"\"\"\n  the home\n  route\n\"\"\""));
        assert!(as_field(&parse).variable_definitions.is_some());
    }

    #[test]
    fn a_description_after_the_selection_set_is_leftover() {
        let text = "field Query.Foo { bar } \"too late\"";
        assert_unparsed(
            text,
            expected(Expectation::EndOfDeclaration, Found::Token(NonBracketTokenKind::StringLiteral)),
            span_of(text, "\"too late\""),
        );
    }

    #[test]
    fn an_entrypoint_carries_no_description() {
        let text = "entrypoint Query.foo \"nope\"";
        assert_unparsed(
            text,
            expected(Expectation::EndOfDeclaration, Found::Token(NonBracketTokenKind::StringLiteral)),
            span_of(text, "\"nope\""),
        );
    }

    #[test]
    fn a_description_resolves_to_its_leaf() {
        let text = "field Query.Foo \"the home route\" { bar }";
        let parse = parsed(text);
        match parse.resolve((), span_of(text, "home")) {
            IsographResolutionNode::Description(description) => {
                assert_eq!(description.parent.inner.client_field_name.location, span_of(text, "Foo"));
            }
            node => panic!("expected the description leaf, got {node:?}"),
        }
    }
```

## Landing checklist

1. The `Description` type, the `ClientFieldDeclaration` and `parse_field` changes, the resolution-node variant, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
