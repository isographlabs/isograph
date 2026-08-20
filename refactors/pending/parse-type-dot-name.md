# parse-type-dot-name: `Type.name`

Entrypoint and field declarations both parse `Identifier . Identifier`. optional-to.md keeps that header and adds optional `to Type` after it. This doc extracts the shared form. No AST type, path alias, or `IsographResolutionNode` variant changes.

The parent type is `EntityNameWrapper`. The name is generic `N: From<StringKey>`; callers `.map` the wrapper (`ClientScalarSelectableNameWrapper`).

Lands after parse-variables.md, before optional-to.md.

## Before

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_entrypoint(
    cursor: &mut ItemCursor<'_>,
) -> Result<EntrypointDeclaration, WithSpan<ParseError>> {
    let parent_type = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Type)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Period, SemanticToken::Period)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Period)))?;
    let client_field_name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    EntrypointDeclaration {
        parent_type: parent_type.interned().map(EntityNameWrapper),
        client_field_name: client_field_name
            .interned()
            .map(ClientScalarSelectableNameWrapper),
    }
    .wrap_ok()
}

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
    let description = consume_description(cursor);
    let selection_set = require_selection_set(cursor)?;
    ClientFieldDeclaration {
        parent_type: parent_type.interned().map(EntityNameWrapper),
        client_field_name: client_field_name
            .interned()
            .map(ClientScalarSelectableNameWrapper),
        description,
        selection_set,
    }
    .wrap_ok()
}
```

## After

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_type_dot_name<N: From<intern::string_key::StringKey>>(
    cursor: &mut ItemCursor<'_>,
) -> Result<(WithSpan<EntityNameWrapper>, WithSpan<N>), WithSpan<ParseError>> {
    let parent_type = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Type)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Period, SemanticToken::Period)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Period)))?;
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    (
        parent_type.interned().map(EntityNameWrapper),
        name.interned(),
    )
    .wrap_ok()
}

fn parse_entrypoint(
    cursor: &mut ItemCursor<'_>,
) -> Result<EntrypointDeclaration, WithSpan<ParseError>> {
    let (parent_type, client_field_name) = parse_type_dot_name(cursor)?;
    EntrypointDeclaration {
        parent_type,
        client_field_name: client_field_name.map(ClientScalarSelectableNameWrapper),
    }
    .wrap_ok()
}

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

`parse_type_dot_name` is `pub(crate)` so `parse_entrypoint` and `parse_field` share it. `N` is the inner lang type; callers `.map` the wrapper. `variable_definitions` is parse-variables.md's, between the name and the description.

## Tests

Existing entrypoint and field tests. Behavior is unchanged.

## Landing checklist

1. `parse_type_dot_name`, the two call sites. `cargo test -p isograph_parser` passes.
2. Move this doc to refactors/past.
