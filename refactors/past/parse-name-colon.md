# parse-name-colon: lhs, colon, rhs

`parse_name_colon` takes an lhs parser and an rhs parser. It runs lhs, requires `:`, runs rhs. The lhs is whatever the caller passes: an identifier, `$` then an identifier, or another form. This doc's call sites pass an interned identifier and `parse_non_constant_value`. parse-variables.md passes a `$ ident` parser and `parse_type_annotation`.

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
pub(crate) fn parse_name_colon<L, R>(
    cursor: &mut ItemCursor<'_>,
    parse_lhs: impl FnOnce(&mut ItemCursor<'_>) -> Result<L, WithSpan<ParseError>>,
    parse_rhs: impl FnOnce(&mut ItemCursor<'_>) -> Result<R, WithSpan<ParseError>>,
) -> Result<(L, R), WithSpan<ParseError>> {
    let lhs = parse_lhs(cursor)?;
    cursor
        .require_token(NonBracketTokenKind::Colon, SemanticToken::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Colon)))?;
    let rhs = parse_rhs(cursor)?;
    (lhs, rhs).wrap_ok()
}

fn require_interned_identifier<N: From<intern::string_key::StringKey>>(
    cursor: &mut ItemCursor<'_>,
    name_token: SemanticToken,
    missing_name: Expectation,
) -> Result<WithSpan<N>, WithSpan<ParseError>> {
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, name_token)
        .map_err(|()| cursor.expected(missing_name))?;
    name.interned().wrap_ok()
}

fn parse_argument(
    cursor: &mut ItemCursor<'_>,
) -> Result<SelectionFieldArgument, WithSpan<ParseError>> {
    let (name, value) = parse_name_colon(
        cursor,
        |cursor| require_interned_identifier(cursor, SemanticToken::Argument, Expectation::Argument),
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
        |cursor| {
            require_interned_identifier(cursor, SemanticToken::ObjectKey, Expectation::ObjectEntry)
        },
        parse_non_constant_value,
    )?;
    ObjectEntry {
        name: name.map(ValueKeyNameWrapper),
        value,
    }
    .wrap_ok()
}
```

`parse_name_colon` is `pub(crate)` for parse-variables.md. `require_interned_identifier` stays in this module. `N` is the inner lang type; callers `.map` the wrapper.

parse-variables.md's lhs is `require_variable_name` (`$` then identifier):

```rust
// from crates/isograph_parser/src/variables.rs
    let (name, type_) = parse_name_colon(
        cursor,
        |cursor| require_variable_name(cursor, Expectation::VariableDeclarationOrUsage),
        parse_type_annotation,
    )?;
```

## Tests

Existing argument and object-entry tests. Behavior is unchanged.

## Landing checklist

1. `parse_name_colon`, `require_interned_identifier`, the two call sites. `cargo test -p isograph_parser` passes.
2. Move this doc to refactors/past.
