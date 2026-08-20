# peek-then-parse: `parse_non_constant_value`

`parse_non_constant_value` peeks once and passes that peek to a parse function.

```
let Some(peek) = cursor.peek() else { ... };
match peek.view().item.reference() {
    Dollar => parse_variable_name(peek),
    ...
}
```

`CursorPeek::commit` returns the cursor so the parse function can keep reading. `$ ident` after a peeked `$` is `parse_variable_name(peek)`. parse-variables.md adds `require_variable_name` for a declaration that did not peek.

No AST type, path alias, or `IsographResolutionNode` variant changes.

## `CursorPeek`

Before, `CursorPeek` borrows the iterator peek, `previous_end`, and `tokens` as separate fields. `commit` returns the item. A parse function that takes the peek cannot require a later token.

After, `CursorPeek` holds the cursor. `view` is the next item. `commit` records the semantic token, advances, and returns the cursor with the item.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
pub(crate) struct CursorPeek<'c, 'a> {
    cursor: &'c mut ItemCursor<'a>,
    item: &'a WithSpan<ChunkContentItem>,
}

impl<'a> ItemCursor<'a> {
    pub(crate) fn peek(&mut self) -> Option<CursorPeek<'_, 'a>> {
        let item = self.items.peek()?.view().dereference();
        CursorPeek {
            cursor: self,
            item,
        }
        .wrap_some()
    }
}

impl<'c, 'a> CursorPeek<'c, 'a> {
    pub(crate) fn view(&self) -> &'a WithSpan<ChunkContentItem> {
        self.item
    }

    pub(crate) fn commit(
        self,
        token: SemanticToken,
    ) -> (&'c mut ItemCursor<'a>, &'a WithSpan<ChunkContentItem>) {
        let item = self.cursor.items.peek().map(|peek| peek.commit());
        let item = match item {
            Some(item) => item,
            None => self.item,
        };
        self.cursor.previous_end = item.location.end;
        let span = match item.item.reference() {
            ChunkContentItem::NonBracket(_) => item.location,
            ChunkContentItem::Group(group) => group.opening.location,
        };
        self.cursor.tokens.push(token.with_span(span));
        (self.cursor, item)
    }

    pub(crate) fn into_cursor(self) -> &'c mut ItemCursor<'a> {
        self.cursor
    }
}
```

The `None` arm of `commit` is the case `peek()` already established is not next. `consume_token_if` / `consume_group_if` use the returned cursor; they can no longer use `self` until `peek` is consumed.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    pub(crate) fn consume_token_if(
        &mut self,
        kind: NonBracketTokenKind,
        token: SemanticToken,
    ) -> Option<TokenText<'a>> {
        let peek = self.peek()?;
        match peek.view().item.reference() {
            ChunkContentItem::NonBracket(found) if found.0 == kind => {}
            _ => return None,
        }
        let (cursor, item) = peek.commit(token);
        TokenText {
            location: item.location,
            text: cursor.text,
        }
        .wrap_some()
    }
```

`WithSpan` is a foreign type, so these are a local trait. The item holds the span; the cursor holds the source.

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

`consume_group_if` is the same shape: match `peek.view()`, `commit`, then `RecordGroupClose` on the returned cursor.

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

`ChunkContentItem` and `NonBracketToken` join the `use crate` list in arguments.rs. `CursorPeek` is imported from `chunk_stream`.

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn parse_variable_name(
    peek: CursorPeek<'_, '_>,
) -> Result<WithSpan<VariableNameWrapper>, WithSpan<ParseError>> {
    let (cursor, _) = peek.commit(SemanticToken::Variable);
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Variable)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    name.interned().map(VariableNameWrapper).wrap_ok()
}

fn parse_string_literal(peek: CursorPeek<'_, '_>) -> StringLiteralValueWrapper {
    let (cursor, item) = peek.commit(SemanticToken::String);
    item.interned(cursor)
        .map(StringLiteralValueWrapper)
        .item
}

fn parse_integer_value(
    peek: CursorPeek<'_, '_>,
) -> Result<IntegerValue, WithSpan<ParseError>> {
    let (cursor, item) = peek.commit(SemanticToken::Integer);
    match item.text(cursor).parse() {
        Ok(value) => IntegerValue(value).wrap_ok(),
        Err(_) => ParseError::IntegerDoesNotFitI64
            .with_span(item.location)
            .wrap_err(),
    }
}

fn parse_boolean_or_null(
    peek: CursorPeek<'_, '_>,
) -> Result<NonConstantValue, WithSpan<ParseError>> {
    let (cursor, item) = peek.commit(SemanticToken::BooleanOrNull);
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
    peek: CursorPeek<'_, '_>,
) -> Result<ObjectLiteral, WithSpan<ParseError>> {
    let item = peek.view();
    let ChunkContentItem::Group(group) = item.item.reference() else {
        return peek.into_cursor().expected(Expectation::Value).wrap_err();
    };
    let children = group.children.reference();
    let closing = group.closing.location;
    let (cursor, _) = peek.commit(SemanticToken::Brace);
    let mut close = RecordGroupClose {
        cursor,
        closing,
        token: SemanticToken::Brace,
    };
    ObjectLiteral(children.item.parse_each_chunk(
        close.cursor(),
        Expectation::Separator(BracketKind::Brace),
        parse_object_entry,
    ))
    .wrap_ok()
}

pub(crate) fn parse_non_constant_value(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<NonConstantValue>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        let Some(peek) = cursor.peek() else {
            return cursor.expected(Expectation::Value).wrap_err();
        };
        match peek.view().item.reference() {
            ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::Dollar)) => {
                return NonConstantValue::Variable(VariableUse(parse_variable_name(peek)?))
                    .wrap_ok();
            }
            ChunkContentItem::NonBracket(NonBracketToken(
                NonBracketTokenKind::StringLiteral,
            )) => {
                return NonConstantValue::String(parse_string_literal(peek)).wrap_ok();
            }
            ChunkContentItem::NonBracket(NonBracketToken(
                NonBracketTokenKind::IntegerLiteral,
            )) => {
                return NonConstantValue::Integer(parse_integer_value(peek)?).wrap_ok();
            }
            ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::Identifier)) => {
                return parse_boolean_or_null(peek);
            }
            ChunkContentItem::Group(group)
                if group.opening.item.0 == BracketKind::Brace =>
            {
                return NonConstantValue::Object(parse_object_literal(peek)?).wrap_ok();
            }
            _ => {}
        }
        cursor.expected(Expectation::Value).wrap_err()
    })
}
```

`RecordGroupClose` is `pub(crate)` so `parse_object_literal` can record the close. `ItemCursor::text` is used from `text` / `interned`; drop `#[cfg_attr(not(test), expect(dead_code))]`.

## Tests

Existing value tests. Behavior is unchanged.

## Landing checklist

1. `CursorPeek` holds the cursor; `commit` returns `(cursor, item)`. `consume_token_if` / `consume_group_if` use that. `parse_variable_name(peek)` and the other value parse functions. `parse_non_constant_value` peeks once and matches. `cargo test -p isograph_parser` passes.
2. Move this doc to refactors/past.
