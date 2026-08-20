# parse-name-colon: `name : rhs`

`parse_name_colon_value` hardcodes the rhs as `parse_non_constant_value`. This doc splits out `:` + rhs and makes the name form take the rhs parser. parse-variables.md's `$name: Type` is `$` + identifier, then this `:` + rhs.

No AST type, path alias, or `IsographResolutionNode` variant changes. A selection alias is `consume_token_if(Colon)`, not this form.

## Before

```rust
// from crates/isograph_parser/src/arguments.rs
fn parse_name_colon_value<N: From<intern::string_key::StringKey>>(
    cursor: &mut ItemCursor<'_>,
    name_token: SemanticToken,
    missing_name: Expectation,
) -> Result<(WithSpan<N>, WithSpan<NonConstantValue>), WithSpan<ParseError>> {
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, name_token)
        .map_err(|()| cursor.expected(missing_name))?;
    cursor
        .require_token(NonBracketTokenKind::Colon, SemanticToken::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Colon)))?;
    let value = parse_non_constant_value(cursor)?;
    (name.interned(), value).wrap_ok()
}

fn parse_argument(
    cursor: &mut ItemCursor<'_>,
) -> Result<SelectionFieldArgument, WithSpan<ParseError>> {
    let (name, value) =
        parse_name_colon_value(cursor, SemanticToken::Argument, Expectation::Argument)?;
    SelectionFieldArgument {
        name: name.map(FieldArgumentNameWrapper),
        value,
    }
    .wrap_ok()
}

fn parse_object_entry(cursor: &mut ItemCursor<'_>) -> Result<ObjectEntry, WithSpan<ParseError>> {
    let (name, value) =
        parse_name_colon_value(cursor, SemanticToken::ObjectKey, Expectation::ObjectEntry)?;
    ObjectEntry {
        name: name.map(ValueKeyNameWrapper),
        value,
    }
    .wrap_ok()
}
```

## After

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn parse_colon_rhs<V>(
    cursor: &mut ItemCursor<'_>,
    parse_rhs: impl FnOnce(&mut ItemCursor<'_>) -> Result<WithSpan<V>, WithSpan<ParseError>>,
) -> Result<WithSpan<V>, WithSpan<ParseError>> {
    cursor
        .require_token(NonBracketTokenKind::Colon, SemanticToken::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Colon)))?;
    parse_rhs(cursor)
}

fn parse_name_colon<N, V>(
    cursor: &mut ItemCursor<'_>,
    name_token: SemanticToken,
    missing_name: Expectation,
    parse_rhs: impl FnOnce(&mut ItemCursor<'_>) -> Result<WithSpan<V>, WithSpan<ParseError>>,
) -> Result<(WithSpan<N>, WithSpan<V>), WithSpan<ParseError>>
where
    N: From<intern::string_key::StringKey>,
{
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, name_token)
        .map_err(|()| cursor.expected(missing_name))?;
    let rhs = parse_colon_rhs(cursor, parse_rhs)?;
    (name.interned(), rhs).wrap_ok()
}

fn parse_argument(
    cursor: &mut ItemCursor<'_>,
) -> Result<SelectionFieldArgument, WithSpan<ParseError>> {
    let (name, value) = parse_name_colon(
        cursor,
        SemanticToken::Argument,
        Expectation::Argument,
        parse_non_constant_value,
    )?;
    SelectionFieldArgument {
        name: name.map(FieldArgumentNameWrapper),
        value,
    }
    .wrap_ok()
}

fn parse_object_entry(cursor: &mut ItemCursor<'_>) -> Result<ObjectEntry, WithSpan<ParseError>> {
    let (name, value) = parse_name_colon(
        cursor,
        SemanticToken::ObjectKey,
        Expectation::ObjectEntry,
        parse_non_constant_value,
    )?;
    ObjectEntry {
        name: name.map(ValueKeyNameWrapper),
        value,
    }
    .wrap_ok()
}
```

`parse_colon_rhs` is `pub(crate)` for parse-variables.md. `parse_name_colon` stays in this module. `N` is the inner lang type; callers `.map` the wrapper.

## Tests

Existing argument and object-entry tests. Behavior is unchanged.

## Landing checklist

1. `parse_colon_rhs`, `parse_name_colon`, the two call sites. `cargo test -p isograph_parser` passes.
2. Move this doc to refactors/past.
