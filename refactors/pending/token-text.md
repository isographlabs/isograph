# token-text: source slice on a consumed token

`consume_token_if` and `require_token` return `TokenText`. The source slice is `token_text` on that value. An interned name is `interned` on that value: intern, `From<StringKey>`, and the token's span. `ItemCursor::text` is still the whole literal.

## Change 1: `TokenText`

```rust
// from crates/isograph_parser/src/chunk_stream.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct TokenText<'a> {
    pub location: Span,
    text: &'a str,
}

impl<'a> TokenText<'a> {
    pub(crate) fn token_text(self) -> &'a str {
        self.text
    }

    pub(crate) fn interned<T: From<intern::string_key::StringKey>>(self) -> WithSpan<T> {
        self.text.intern().to::<T>().with_span(self.location)
    }
}
```

`text` is the slice of `ItemCursor`'s source at `location`. `'a` is that source, not the cursor borrow. `interned` needs `use intern::string_key::Intern` in `chunk_stream.rs`. A parser wrapper that implements `From<StringKey>` is `token.interned()`. A parser wrapper that does not is `token.interned().map(ArgumentName)`, and `interned` infers the inner lang type.

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    pub(crate) fn consume_token_if(
        &mut self,
        kind: NonBracketTokenKind,
        token: SemanticToken,
    ) -> Option<Span> {
        let peek = self.peek()?;
        match peek.view().item.reference() {
            ChunkContentItem::NonBracket(found) if found.0 == kind => {}
            _ => return None,
        }
        peek.commit(token).location.wrap_some()
    }

    pub(crate) fn require_token(
        &mut self,
        kind: NonBracketTokenKind,
        token: SemanticToken,
    ) -> Result<Span, ()> {
        self.consume_token_if(kind, token).ok_or(())
    }

    pub(crate) fn token_text(&self, span: Span) -> &'a str {
        &self.text[span.as_usize_range()]
    }
```

After:

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
        let location = peek.commit(token).location;
        TokenText {
            location,
            text: &self.text[location.as_usize_range()],
        }
        .wrap_some()
    }

    pub(crate) fn require_token(
        &mut self,
        kind: NonBracketTokenKind,
        token: SemanticToken,
    ) -> Result<TokenText<'a>, ()> {
        self.consume_token_if(kind, token).ok_or(())
    }
```

`require_token` is still `consume_token_if` or `Err(())`. The caller still maps `Err` with `expected`.

## Change 2: `parse_iso_literal`

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_iso_literal_item(
    cursor: &mut ItemCursor<'_>,
) -> Result<IsoLiteralItem, WithSpan<ParseError>> {
    let keyword = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Keyword)
        .map_err(|()| cursor.expected(Expectation::DeclarationKeyword))?;
    match cursor.token_text(keyword) {
        "entrypoint" => IsoLiteralItem::Entrypoint(parse_entrypoint(cursor)?).wrap_ok(),
        "field" | "pointer" => ParseError::UnsupportedDeclarationType
            .with_span(keyword)
            .wrap_err(),
        _ => ParseError::expected(
            Expectation::DeclarationKeyword,
            Found::Token(NonBracketTokenKind::Identifier),
        )
        .with_span(keyword)
        .wrap_err(),
    }
}

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
        parent_type: cursor
            .token_text(parent_type)
            .intern()
            .to::<EntityName>()
            .with_span(parent_type),
        client_field_name: cursor
            .token_text(client_field_name)
            .intern()
            .to::<ClientFieldName>()
            .with_span(client_field_name),
    }
    .wrap_ok()
}
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_iso_literal_item(
    cursor: &mut ItemCursor<'_>,
) -> Result<IsoLiteralItem, WithSpan<ParseError>> {
    let keyword = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Keyword)
        .map_err(|()| cursor.expected(Expectation::DeclarationKeyword))?;
    match keyword.token_text() {
        "entrypoint" => IsoLiteralItem::Entrypoint(parse_entrypoint(cursor)?).wrap_ok(),
        "field" | "pointer" => ParseError::UnsupportedDeclarationType
            .with_span(keyword.location)
            .wrap_err(),
        _ => ParseError::expected(
            Expectation::DeclarationKeyword,
            Found::Token(NonBracketTokenKind::Identifier),
        )
        .with_span(keyword.location)
        .wrap_err(),
    }
}

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
        parent_type: parent_type.interned(),
        client_field_name: client_field_name.interned(),
    }
    .wrap_ok()
}
```

## Change 3: `parsing-standards.md`

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
impl<'a> ItemCursor<'a> {
    pub(crate) fn peek(&mut self) -> Option<CursorPeek<'_, 'a>>;
    pub(crate) fn consume_token_if(
        &mut self,
        kind: NonBracketTokenKind,
        token: SemanticToken,
    ) -> Option<Span>;
    pub(crate) fn consume_group_if(
        &mut self,
        kind: BracketKind,
        token: SemanticToken,
    ) -> Option<WithSpan<&'a ChunkedGroup>>;
    pub(crate) fn expected(&mut self, expected: Expectation) -> WithSpan<ParseError>;
    pub(crate) fn require_token(
        &mut self,
        kind: NonBracketTokenKind,
        token: SemanticToken,
    ) -> Result<Span, ()>;
    pub(crate) fn require_group(
        &mut self,
        kind: BracketKind,
        token: SemanticToken,
    ) -> Result<WithSpan<&'a ChunkedGroup>, ()>;
    pub(crate) fn record_group_close(&mut self, group: &ChunkedGroup, token: SemanticToken);
    pub(crate) fn report_error(&mut self, error: WithSpan<ParseError>);
    pub(crate) fn stream_chunk<'c>(&'c mut self, chunk: &'c Chunk) -> ChunkStream<'c>;
    pub(crate) fn text(&self) -> &'a str;
    pub(crate) fn token_text(&self, span: Span) -> &'a str;
    pub(crate) fn spanning<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, WithSpan<ParseError>>,
    ) -> Result<WithSpan<T>, WithSpan<ParseError>>;
}
```

- Leaf: the `Span` from `require_token` or `consume_token_if`, or the `WithSpan` from `require_group` or `consume_group_if`.
- `token_text` is `&self.text[span.as_usize_range()]`. A name in the tree is an interned string key (`token_text(span).intern().to::<EntityName>()`). The converted scalar is the `i64`. The wrapper span is location only.
- Keyword / boolean / null text: `token_text` after an identifier
- Integer conversion: `token_text(span).parse()` on an `IntegerLiteral` span
- Interned name: `token_text(span).intern().to::<EntityName>()`

After:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct TokenText<'a> {
    pub location: Span,
    text: &'a str,
}

impl<'a> TokenText<'a> {
    pub(crate) fn token_text(self) -> &'a str;
    pub(crate) fn interned<T: From<intern::string_key::StringKey>>(self) -> WithSpan<T>;
}

impl<'a> ItemCursor<'a> {
    pub(crate) fn peek(&mut self) -> Option<CursorPeek<'_, 'a>>;
    pub(crate) fn consume_token_if(
        &mut self,
        kind: NonBracketTokenKind,
        token: SemanticToken,
    ) -> Option<TokenText<'a>>;
    pub(crate) fn consume_group_if(
        &mut self,
        kind: BracketKind,
        token: SemanticToken,
    ) -> Option<WithSpan<&'a ChunkedGroup>>;
    pub(crate) fn expected(&mut self, expected: Expectation) -> WithSpan<ParseError>;
    pub(crate) fn require_token(
        &mut self,
        kind: NonBracketTokenKind,
        token: SemanticToken,
    ) -> Result<TokenText<'a>, ()>;
    pub(crate) fn require_group(
        &mut self,
        kind: BracketKind,
        token: SemanticToken,
    ) -> Result<WithSpan<&'a ChunkedGroup>, ()>;
    pub(crate) fn record_group_close(&mut self, group: &ChunkedGroup, token: SemanticToken);
    pub(crate) fn report_error(&mut self, error: WithSpan<ParseError>);
    pub(crate) fn stream_chunk<'c>(&'c mut self, chunk: &'c Chunk) -> ChunkStream<'c>;
    pub(crate) fn text(&self) -> &'a str;
    pub(crate) fn spanning<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, WithSpan<ParseError>>,
    ) -> Result<WithSpan<T>, WithSpan<ParseError>>;
}
```

- Leaf: the `location` of the `TokenText` from `require_token` or `consume_token_if`, or the `WithSpan` from `require_group` or `consume_group_if`.
- `token_text` is the source slice on that `TokenText`. A name in the tree is `token.interned()`. The converted scalar is the `i64`. The wrapper span is location only.
- Keyword / boolean / null text: `token.token_text()` after an identifier
- Integer conversion: `token.token_text().parse()` on an `IntegerLiteral` token
- Interned name: `token.interned()` when the wrapper implements `From<StringKey>`; `token.interned().map(ArgumentName)` when it does not

The `parse_value` ladder and the interned-name sentence in that file:

```rust
// from crates/isograph_parser/src/arguments.rs
            return NonConstantValue::Variable(VariableUse(
                name.interned().map(VariableNameWrapper),
            ))
            .wrap_ok();
        }
        if let Some(span) =
            cursor.consume_token_if(NonBracketTokenKind::StringLiteral, SemanticToken::String)
        {
            return NonConstantValue::String(span.interned().map(StringValue).item)
                .wrap_ok();
        }
        if let Some(span) = cursor
            .consume_token_if(NonBracketTokenKind::IntegerLiteral, SemanticToken::Integer)
        {
            let value = match span.token_text().parse() {
                Ok(value) => value,
                Err(_) => {
                    return ParseError::IntegerDoesNotFitI64.with_span(span.location).wrap_err();
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
```

Parser wrappers that do not implement `From<StringKey>` construct `name.interned().map(VariableNameWrapper)`. The integer arm is `token.token_text().parse()`; that token is the one `consume_token_if(IntegerLiteral)` just returned.

## Change 4: tests

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    use super::{ChunkStream, TokenText};
```

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    fn token_text<'a>(text: &'a str, pattern: &str) -> TokenText<'a> {
        let location = span_of(text, pattern);
        TokenText {
            location,
            text: &text[location.as_usize_range()],
        }
    }
```

Every `assert_eq` whose right-hand side is a `Span` from `consume_token_if` / `require_token` uses `token_text(text, pattern)` instead of `span_of(text, pattern)`.

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    fn consume_token_if_matches_the_next_identifier_and_skips_the_wrong_kind() {
        let text = "foo bar";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            span_of(text, "foo").wrap_some(),
        );
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Period, SemanticToken::Period),
            None
        );
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            span_of(text, "bar").wrap_some(),
        );
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            None
        );
    }
```

After:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    fn consume_token_if_matches_the_next_identifier_and_skips_the_wrong_kind() {
        let text = "foo bar";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            token_text(text, "foo").wrap_some(),
        );
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Period, SemanticToken::Period),
            None
        );
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            token_text(text, "bar").wrap_some(),
        );
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            None
        );
    }
```

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            span_of(text, "foo").wrap_some(),
        );
```

in `consume_group_if_matches_a_brace_group_and_skips_a_token_or_the_wrong_bracket`. After: `token_text(text, "foo").wrap_some()`.

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
        assert_eq!(
            cursor.require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            span_of(text, "foo").wrap_ok(),
        );
```

in `require_token_and_require_group_are_consume_or_err`. After: `token_text(text, "foo").wrap_ok()`. The `().wrap_err()` arms stay.

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
        assert_eq!(
            stream
                .cursor()
                .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            span_of(text, "foo").wrap_some(),
        );
```

in `spanning_does_not_advance_when_the_closure_errors_without_consuming`. After: `token_text(text, "foo").wrap_some()`.

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    fn token_text_is_the_source_slice() {
        let text = "foo bar";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        let foo = cursor
            .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
            .expect("foo is present");
        assert_eq!(cursor.token_text(foo), "foo");
        assert_eq!(cursor.text(), text);
    }
```

After:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    fn token_text_is_the_source_slice() {
        let text = "foo bar";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        let foo = cursor
            .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
            .expect("foo is present");
        assert_eq!(foo.token_text(), "foo");
        assert_eq!(foo.location, span_of(text, "foo"));
        assert_eq!(cursor.text(), text);
    }
```

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
                assert_eq!(
                    stream
                        .cursor()
                        .require_token(NonBracketTokenKind::Identifier, token),
                    span_of(text, text).wrap_ok(),
                    "for literal {text:?}",
                );
```

in `require_token_records_the_role_the_caller_passed`. After: `token_text(text, text).wrap_ok()`.

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
            assert_eq!(
                cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
                span_of(text, "alias").wrap_some(),
            );
            assert_eq!(
                cursor.consume_token_if(NonBracketTokenKind::Colon, SemanticToken::Colon),
                span_of(text, ":").wrap_some(),
            );
            assert_eq!(
                cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
                span_of(text, "name").wrap_some(),
            );
```

in `alias_colon_name_records_field_name_colon_field_name`. After: `token_text(text, "alias")`, `token_text(text, ":")`, `token_text(text, "name")`.

Before:

```rust
// from crates/isograph_parser/src/chunk_stream.rs
            assert_eq!(
                stream
                    .cursor()
                    .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
                span_of(text, "foo").wrap_some(),
            );
```

in `peek_without_commit_records_nothing`. After: `token_text(text, "foo").wrap_some()`.

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
    fn parse_identifier(cursor: &mut ItemCursor<'_>) -> Result<Span, WithSpan<ParseError>> {
        cursor
            .require_token(Identifier, SemanticToken::FieldName)
            .map_err(|()| cursor.expected(Expectation::Token(Identifier)))
    }
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
    fn parse_identifier(cursor: &mut ItemCursor<'_>) -> Result<Span, WithSpan<ParseError>> {
        cursor
            .require_token(Identifier, SemanticToken::FieldName)
            .map_err(|()| cursor.expected(Expectation::Token(Identifier)))
            .map(|token| token.location)
    }
```

`Slot<Span, UnparsedChunkItems>` and the span assertions on those slots stay.

## Landing checklist

1. `TokenText` with `token_text` and `interned`, `consume_token_if` / `require_token` return it, `parse_iso_literal` uses `interned` for names, the tests, `parsing-standards.md`. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
