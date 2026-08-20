# peek-then-parse: `parse_non_constant_value`

Passing the peek into `parse_*` would be the better API. It needs `CursorPeek` to yield the cursor after `commit`, which makes `consume_token_if` / `consume_group_if` worse. This doc does not change `CursorPeek`.

`parse_non_constant_value` peeks to choose a form, drops the peek without `commit`, then calls a parse function that requires its first token. That `require_token` / `require_group` is the assert that the item is still what the peek saw.

`$ ident` is `parse_variable_name(cursor, missing_dollar)`: require `$`, then the identifier. The value arm passes `Expectation::Token(Dollar)`. parse-variables.md passes `Expectation::VariableDeclarationOrUsage`.

No AST type, path alias, or `IsographResolutionNode` variant changes.

## Before

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn parse_non_constant_value(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<NonConstantValue>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if cursor
            .consume_token_if(NonBracketTokenKind::Dollar, SemanticToken::Variable)
            .is_some()
        {
            let name = cursor
                .require_token(NonBracketTokenKind::Identifier, SemanticToken::Variable)
                .map_err(|()| {
                    cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier))
                })?;
            return NonConstantValue::Variable(VariableUse(
                name.interned().map(VariableNameWrapper),
            ))
            .wrap_ok();
        }
        if let Some(span) =
            cursor.consume_token_if(NonBracketTokenKind::StringLiteral, SemanticToken::String)
        {
            return NonConstantValue::String(span.interned().map(StringLiteralValueWrapper).item)
                .wrap_ok();
        }
        if let Some(span) =
            cursor.consume_token_if(NonBracketTokenKind::IntegerLiteral, SemanticToken::Integer)
        {
            let value = match span.text().parse() {
                Ok(value) => value,
                Err(_) => {
                    return ParseError::IntegerDoesNotFitI64
                        .with_span(span.location)
                        .wrap_err();
                }
            };
            return NonConstantValue::Integer(IntegerValue(value)).wrap_ok();
        }
        if let Some(span) = cursor.consume_token_if(
            NonBracketTokenKind::Identifier,
            SemanticToken::BooleanOrNull,
        ) {
            return match span.text() {
                "true" => NonConstantValue::Boolean(BooleanValue(Boolean::True)).wrap_ok(),
                "false" => NonConstantValue::Boolean(BooleanValue(Boolean::False)).wrap_ok(),
                "null" => NonConstantValue::Null(NullValue).wrap_ok(),
                _ => ParseError::expected(
                    Expectation::Value,
                    Found::Token(NonBracketTokenKind::Identifier),
                )
                .with_span(span.location)
                .wrap_err(),
            };
        }
        if let Some(object) = cursor.consume_group_if(
            BracketKind::Brace,
            SemanticToken::Brace,
            |cursor, children| {
                ObjectLiteral(children.item.parse_each_chunk(
                    cursor,
                    Expectation::Separator(BracketKind::Brace),
                    parse_object_entry,
                ))
            },
        ) {
            return NonConstantValue::Object(object.item).wrap_ok();
        }
        cursor.expected(Expectation::Value).wrap_err()
    })
}
```

## After

`ChunkContentItem` and `NonBracketToken` join the `use crate` list in arguments.rs.

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn parse_variable_name(
    cursor: &mut ItemCursor<'_>,
    missing_dollar: Expectation,
) -> Result<WithSpan<VariableNameWrapper>, WithSpan<ParseError>> {
    cursor
        .require_token(NonBracketTokenKind::Dollar, SemanticToken::Variable)
        .map_err(|()| cursor.expected(missing_dollar))?;
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Variable)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    name.interned().map(VariableNameWrapper).wrap_ok()
}

fn parse_string_literal(
    cursor: &mut ItemCursor<'_>,
) -> Result<StringLiteralValueWrapper, WithSpan<ParseError>> {
    let span = cursor
        .require_token(NonBracketTokenKind::StringLiteral, SemanticToken::String)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::StringLiteral)))?;
    span.interned().map(StringLiteralValueWrapper).item.wrap_ok()
}

fn parse_integer_value(
    cursor: &mut ItemCursor<'_>,
) -> Result<IntegerValue, WithSpan<ParseError>> {
    let span = cursor
        .require_token(NonBracketTokenKind::IntegerLiteral, SemanticToken::Integer)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::IntegerLiteral)))?;
    match span.text().parse() {
        Ok(value) => IntegerValue(value).wrap_ok(),
        Err(_) => ParseError::IntegerDoesNotFitI64
            .with_span(span.location)
            .wrap_err(),
    }
}

fn parse_boolean_or_null(
    cursor: &mut ItemCursor<'_>,
) -> Result<NonConstantValue, WithSpan<ParseError>> {
    let span = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::BooleanOrNull)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    match span.text() {
        "true" => NonConstantValue::Boolean(BooleanValue(Boolean::True)).wrap_ok(),
        "false" => NonConstantValue::Boolean(BooleanValue(Boolean::False)).wrap_ok(),
        "null" => NonConstantValue::Null(NullValue).wrap_ok(),
        _ => ParseError::expected(
            Expectation::Value,
            Found::Token(NonBracketTokenKind::Identifier),
        )
        .with_span(span.location)
        .wrap_err(),
    }
}

fn parse_object_literal(
    cursor: &mut ItemCursor<'_>,
) -> Result<ObjectLiteral, WithSpan<ParseError>> {
    let object = cursor
        .require_group(
            BracketKind::Brace,
            SemanticToken::Brace,
            |cursor, children| {
                ObjectLiteral(children.item.parse_each_chunk(
                    cursor,
                    Expectation::Separator(BracketKind::Brace),
                    parse_object_entry,
                ))
            },
        )
        .map_err(|()| cursor.expected(Expectation::Value))?;
    object.item.wrap_ok()
}

pub(crate) fn parse_non_constant_value(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<NonConstantValue>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if let Some(peek) = cursor.peek() {
            match peek.view().item.reference() {
                ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::Dollar)) => {
                    return NonConstantValue::Variable(VariableUse(parse_variable_name(
                        cursor,
                        Expectation::Token(NonBracketTokenKind::Dollar),
                    )?))
                    .wrap_ok();
                }
                ChunkContentItem::NonBracket(NonBracketToken(
                    NonBracketTokenKind::StringLiteral,
                )) => {
                    return NonConstantValue::String(parse_string_literal(cursor)?).wrap_ok();
                }
                ChunkContentItem::NonBracket(NonBracketToken(
                    NonBracketTokenKind::IntegerLiteral,
                )) => {
                    return NonConstantValue::Integer(parse_integer_value(cursor)?).wrap_ok();
                }
                ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::Identifier)) => {
                    return parse_boolean_or_null(cursor);
                }
                ChunkContentItem::Group(group)
                    if group.opening.item.0 == BracketKind::Brace =>
                {
                    return NonConstantValue::Object(parse_object_literal(cursor)?).wrap_ok();
                }
                _ => {}
            }
        }
        cursor.expected(Expectation::Value).wrap_err()
    })
}
```

The peek is not committed. Each `parse_*` requires its first token. If the match-arm borrow of `peek` outlives the `parse_*` call, copy the kind off `view()` first so the peek is dropped.

## Tests

Existing value tests. Behavior is unchanged.

## Landing checklist

1. `parse_variable_name`, the other value parse functions, the new `parse_non_constant_value` body. `cargo test -p isograph_parser` passes.
2. Move this doc to refactors/past.
