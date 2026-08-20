# peek-then-parse: `parse_non_constant_value`

`parse_non_constant_value` peeks, matches, `commit`s, then calls a parse function with the cursor (free after `commit`) and the committed item. `CursorPeek` is unchanged: `commit` returns the item.

`$ ident`: the match arm commits `$`, then `parse_variable_name(cursor)` requires the identifier. parse-variables.md adds `require_variable_name`, which requires `$` then calls `parse_variable_name`.

A brace group is not committed in the match. The peek is dropped and `consume_group_if` takes it.

No AST type, path alias, or `IsographResolutionNode` variant changes.

## `item.text(cursor)` / `item.interned(cursor)`

The item holds the span; the cursor holds the source. `WithSpan` is a foreign type, so these are a local trait.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
pub(crate) trait ItemTokenText {
    fn text<'a>(&self, cursor: &ItemCursor<'a>) -> &'a str;
    fn interned<'a, T: From<intern::string_key::StringKey>>(
        &self,
        cursor: &ItemCursor<'a>,
    ) -> WithSpan<T>;
}

impl ItemTokenText for WithSpan<ChunkContentItem> {
    fn text<'a>(&self, cursor: &ItemCursor<'a>) -> &'a str {
        &cursor.text()[self.location.as_usize_range()]
    }

    fn interned<'a, T: From<intern::string_key::StringKey>>(
        &self,
        cursor: &ItemCursor<'a>,
    ) -> WithSpan<T> {
        self.text(cursor)
            .intern()
            .to::<T>()
            .with_span(self.location)
    }
}
```

`ItemCursor::text` is used here; drop `#[cfg_attr(not(test), expect(dead_code))]`.

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
) -> Result<WithSpan<VariableNameWrapper>, WithSpan<ParseError>> {
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Variable)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    name.interned().map(VariableNameWrapper).wrap_ok()
}

fn parse_string_literal(
    cursor: &ItemCursor<'_>,
    item: &WithSpan<ChunkContentItem>,
) -> StringLiteralValueWrapper {
    item.interned(cursor).map(StringLiteralValueWrapper).item
}

fn parse_integer_value(
    cursor: &ItemCursor<'_>,
    item: &WithSpan<ChunkContentItem>,
) -> Result<IntegerValue, WithSpan<ParseError>> {
    match item.text(cursor).parse() {
        Ok(value) => IntegerValue(value).wrap_ok(),
        Err(_) => ParseError::IntegerDoesNotFitI64
            .with_span(item.location)
            .wrap_err(),
    }
}

fn parse_boolean_or_null(
    cursor: &ItemCursor<'_>,
    item: &WithSpan<ChunkContentItem>,
) -> Result<NonConstantValue, WithSpan<ParseError>> {
    match item.text(cursor) {
        "true" => NonConstantValue::Boolean(BooleanValue(Boolean::True)).wrap_ok(),
        "false" => NonConstantValue::Boolean(BooleanValue(Boolean::False)).wrap_ok(),
        "null" => NonConstantValue::Null(NullValue).wrap_ok(),
        _ => ParseError::expected(
            Expectation::Value,
            Found::Token(NonBracketTokenKind::Identifier),
        )
        .with_span(item.location)
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
                    peek.commit(SemanticToken::Variable);
                    return NonConstantValue::Variable(VariableUse(parse_variable_name(cursor)?))
                        .wrap_ok();
                }
                ChunkContentItem::NonBracket(NonBracketToken(
                    NonBracketTokenKind::StringLiteral,
                )) => {
                    let item = peek.commit(SemanticToken::String);
                    return NonConstantValue::String(parse_string_literal(cursor, item)).wrap_ok();
                }
                ChunkContentItem::NonBracket(NonBracketToken(
                    NonBracketTokenKind::IntegerLiteral,
                )) => {
                    let item = peek.commit(SemanticToken::Integer);
                    return NonConstantValue::Integer(parse_integer_value(cursor, item)?).wrap_ok();
                }
                ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::Identifier)) => {
                    let item = peek.commit(SemanticToken::BooleanOrNull);
                    return parse_boolean_or_null(cursor, item);
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

Token arms `commit` then parse with the cursor and the item. `{ ... }` is `consume_group_if` after the peek is dropped.

## Tests

Existing value tests. Behavior is unchanged.

## Landing checklist

1. `ItemTokenText`, `parse_variable_name`, the token parse functions, the new `parse_non_constant_value` body. `cargo test -p isograph_parser` passes.
2. Move this doc to refactors/past.
