# peek-then-parse: `parse_non_constant_value`

`parse_non_constant_value` peeks, commits, and passes the peeked `TokenText` to a parse function. `$ ident` after a peeked `$` is `parse_variable_name`. parse-variables.md adds `require_variable_name` for a declaration that did not peek.

No AST type, path alias, or `IsographResolutionNode` variant changes.

## `ItemCursor::token_text`

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    pub(crate) fn token_text(&self, location: Span) -> TokenText<'a> {
        TokenText {
            location,
            text: self.text,
        }
    }
```

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
            let value = match span.token_text().parse() {
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
            return match span.token_text() {
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

`ChunkContentItem`, `NonBracketToken`, `ChunkedLevel`, and `TokenText` join the `use` lists in arguments.rs.

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn parse_variable_name(
    cursor: &mut ItemCursor<'_>,
    _dollar: TokenText<'_>,
) -> Result<WithSpan<VariableNameWrapper>, WithSpan<ParseError>> {
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Variable)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    name.interned().map(VariableNameWrapper).wrap_ok()
}

fn parse_string_literal(span: TokenText<'_>) -> StringLiteralValueWrapper {
    span.interned().map(StringLiteralValueWrapper).item
}

fn parse_integer_value(span: TokenText<'_>) -> Result<IntegerValue, WithSpan<ParseError>> {
    match span.token_text().parse() {
        Ok(value) => IntegerValue(value).wrap_ok(),
        Err(_) => ParseError::IntegerDoesNotFitI64
            .with_span(span.location)
            .wrap_err(),
    }
}

fn parse_boolean_or_null(
    span: TokenText<'_>,
) -> Result<NonConstantValue, WithSpan<ParseError>> {
    match span.token_text() {
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
    children: &WithSpan<ChunkedLevel>,
) -> ObjectLiteral {
    ObjectLiteral(children.item.parse_each_chunk(
        cursor,
        Expectation::Separator(BracketKind::Brace),
        parse_object_entry,
    ))
}

pub(crate) fn parse_non_constant_value(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<NonConstantValue>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if let Some(peek) = cursor.peek() {
            match peek.view().item.reference() {
                ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::Dollar)) => {
                    let dollar =
                        cursor.token_text(peek.commit(SemanticToken::Variable).location);
                    return NonConstantValue::Variable(VariableUse(parse_variable_name(
                        cursor, dollar,
                    )?))
                    .wrap_ok();
                }
                ChunkContentItem::NonBracket(NonBracketToken(
                    NonBracketTokenKind::StringLiteral,
                )) => {
                    let span = cursor.token_text(peek.commit(SemanticToken::String).location);
                    return NonConstantValue::String(parse_string_literal(span)).wrap_ok();
                }
                ChunkContentItem::NonBracket(NonBracketToken(
                    NonBracketTokenKind::IntegerLiteral,
                )) => {
                    let span = cursor.token_text(peek.commit(SemanticToken::Integer).location);
                    return NonConstantValue::Integer(parse_integer_value(span)?).wrap_ok();
                }
                ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::Identifier)) => {
                    let span =
                        cursor.token_text(peek.commit(SemanticToken::BooleanOrNull).location);
                    return parse_boolean_or_null(span);
                }
                _ => {}
            }
        }
        if let Some(object) = cursor.consume_group_if(
            BracketKind::Brace,
            SemanticToken::Brace,
            parse_object_literal,
        ) {
            return NonConstantValue::Object(object.item).wrap_ok();
        }
        cursor.expected(Expectation::Value).wrap_err()
    })
}
```

A token arm commits the peek and passes `TokenText`. `{ ... }` is `consume_group_if`; `parse_object_literal` receives the peeked group's children. The `_` arm drops the peek uncommitted so `consume_group_if` can take a brace group.

## Tests

Existing value tests. Behavior is unchanged.

## Landing checklist

1. `ItemCursor::token_text`, `parse_variable_name`, the token parse functions, the new `parse_non_constant_value` body. `cargo test -p isograph_parser` passes.
2. Move this doc to refactors/past.
