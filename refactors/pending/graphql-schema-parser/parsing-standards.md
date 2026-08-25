# GraphQL schema parser: standards

Requires iso parser consume (`require_token` / `TokenText` / `spanning` / `expected`) as the primitive to copy. Feature work in `order.md` calls the functions listed here. If an implementation disagrees with this doc, the same review amends the doc or changes the code.

Origin of the lexer and parse functions: isograph `crates/graphql_schema_parser` (`peekable_lexer.rs`, `parse_schema.rs`, `description.rs`) and `relay-crates/graphql-syntax/src/relay_lexer.rs` `TokenKind`. Origin of consume: i2 `ItemCursor::require_token` / `consume_token_if` / `spanning` / `expected` / `TokenText` (copied into this crate).

Delta from origin PeekableLexer: `WithSpan` (no `TextSource` / `offset` / `WithEmbeddedLocation`); first error is `Result<T, WithSpan<ParseError>>` (`thiserror`); `require_token` takes a `GraphQLSemanticToken` and returns `Result<T, ()>` so the caller writes `map_err(|()| lexer.expected(...))`; skip set is whitespace including newlines, comma, and `#` to end of line; `parse_token` takes `extras.error_token` so `ErrorUnterminatedString` and siblings are the found kind (origin PeekableLexer leaves extras unread). Delta from origin `TokenKind`: kinds the schema grammar does not produce (`Dollar`, `Period`, `PeriodPeriod`, `Spread`, `Empty`) are omitted; `strum::Display` with origin's messages; no serde. Delta from origin errors: structured `ParseError` / `Expectation` / `Found` instead of `Diagnostic` format strings; `require_matching_identifier` mismatch is `Expectation::Keyword` for both a non-identifier and a wrong identifier (origin has two messages).

## Entry

```rust
// from crates/graphql_schema_parser/src/lib.rs
pub fn parse_schema(text: &str) -> Result<WithSpan<TypeSystemDocument>, WithSpan<ParseError>>

pub fn parse_schema_extensions(
    text: &str,
) -> Result<WithSpan<TypeSystemExtensionDocument>, WithSpan<ParseError>>

fn parse_schema_inner(
    text: &str,
    tokens: &mut Vec<WithSpan<GraphQLSemanticToken>>,
) -> Result<WithSpan<TypeSystemDocument>, WithSpan<ParseError>>

fn parse_schema_extensions_inner(
    text: &str,
    tokens: &mut Vec<WithSpan<GraphQLSemanticToken>>,
) -> Result<WithSpan<TypeSystemExtensionDocument>, WithSpan<ParseError>>

#[cfg(test)]
pub fn parse_schema_with_tokens(
    text: &str,
) -> Result<(WithSpan<TypeSystemDocument>, Vec<WithSpan<GraphQLSemanticToken>>), WithSpan<ParseError>>

#[cfg(test)]
pub fn parse_schema_extensions_with_tokens(
    text: &str,
) -> Result<
    (
        WithSpan<TypeSystemExtensionDocument>,
        Vec<WithSpan<GraphQLSemanticToken>>,
    ),
    WithSpan<ParseError>,
>
```

`text` is the file. `parse_schema` / `parse_schema_extensions` construct an empty tokens vec and call the inner function. The inner function constructs `PeekableLexer::new(text, tokens)` and `spanning` around the definition loop. The wrapper span is that spanning span (empty file: zero-width at 0). The first error stops the parse. Callers are the unit tests in this crate: they call `parse_schema` / `parse_schema_extensions`. Semantic-token tests call the `#[cfg(test)]` helpers.

Logos lives on `PeekableLexer`. There is no `tokenize() -> Vec` that the grammar then walks.

## Lexer skip

```rust
// from crates/graphql_schema_parser/src/token_kind.rs
#[regex(r"[ \t\r\n\f,\ufeff]+|#[^\n\r]*", logos::skip)]
```

Copied from `relay-crates/graphql-syntax/src/relay_lexer.rs`. Spaces, tabs, CR, LF, form feed, comma, BOM, and `#` to end of line are not tokens.

## `TokenKind`

Copied from `relay-crates/graphql-syntax/src/relay_lexer.rs` for the kinds below, including `TokenKindExtras`, `StringToken`, `BlockStringToken`, `lex_string`, and `lex_block_string`. `lex_block_string`'s `BlockStringToken::Error` arm is the same as `Other` (origin is `unreachable!()`).

```rust
// from crates/graphql_schema_parser/src/token_kind.rs
#[derive(Default)]
pub struct TokenKindExtras {
    pub(crate) error_token: Option<TokenKind>,
}

#[derive(Logos, Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, strum::Display)]
#[logos(extras = TokenKindExtras)]
pub enum TokenKind {
    #[regex(r"[ \t\r\n\f,\ufeff]+|#[^\n\r]*", logos::skip)]
    #[error]
    #[strum(to_string = "error")]
    Error,

    #[strum(to_string = "unterminated string")]
    ErrorUnterminatedString,
    #[strum(to_string = "unsupported character in string")]
    ErrorUnsupportedStringCharacter,
    #[strum(to_string = "unterminated block string")]
    ErrorUnterminatedBlockString,

    #[token("&")]
    #[strum(to_string = "ampersand ('&')")]
    Ampersand,

    #[token("@")]
    #[strum(to_string = "at ('@')")]
    At,

    #[token("}")]
    #[strum(to_string = "closing brace ('}}')")]
    CloseBrace,

    #[token("]")]
    #[strum(to_string = "closing bracket (']')")]
    CloseBracket,

    #[token(")")]
    #[strum(to_string = "closing paren (')')")]
    CloseParen,

    #[token(":")]
    #[strum(to_string = "colon (':')")]
    Colon,

    #[strum(to_string = "end of file")]
    EndOfFile,

    #[token("=")]
    #[strum(to_string = "equals ('=')")]
    Equals,

    #[token("!")]
    #[strum(to_string = "exclamation mark ('!')")]
    Exclamation,

    #[regex("-?(0|[1-9][0-9]*)(\\.[0-9]+[eE][+-]?[0-9]+|\\.[0-9]+|[eE][+-]?[0-9]+)")]
    #[strum(to_string = "floating point value (e.g. '3.14')")]
    FloatLiteral,

    #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
    #[strum(to_string = "non-variable identifier (e.g. 'x' or 'Foo')")]
    Identifier,

    #[regex("-?(0|[1-9][0-9]*)")]
    #[strum(to_string = "integer value (e.g. '0' or '42')")]
    IntegerLiteral,

    #[regex("-?0[0-9]+(\\.[0-9]+[eE][+-]?[0-9]+|\\.[0-9]+|[eE][+-]?[0-9]+)?")]
    #[strum(to_string = "unsupported number (int or float) literal")]
    ErrorNumberLiteralLeadingZero,

    #[regex("-?(0|[1-9][0-9]*)(\\.[0-9]+[eE][+-]?[0-9]+|\\.[0-9]+|[eE][+-]?[0-9]+)?[.a-zA-Z_]")]
    #[strum(to_string = "unsupported number (int or float) literal")]
    ErrorNumberLiteralTrailingInvalid,

    #[regex("-?(\\.[0-9]+[eE][+-]?[0-9]+|\\.[0-9]+)")]
    #[strum(to_string = "unsupported number (int or float) literal")]
    ErrorFloatLiteralMissingZero,

    #[token("{")]
    #[strum(to_string = "open brace ('{{')")]
    OpenBrace,

    #[token("[")]
    #[strum(to_string = "open bracket ('[')")]
    OpenBracket,

    #[token("(")]
    #[strum(to_string = "open parenthesis ('(')")]
    OpenParen,

    #[token("|")]
    #[strum(to_string = "pipe ('|')")]
    Pipe,

    #[token("\"", lex_string)]
    #[strum(to_string = "string literal (e.g. '\"...\"')")]
    StringLiteral,

    #[token("\"\"\"", lex_block_string)]
    #[strum(to_string = "block string (e.g. '\"\"\"hi\"\"\"')")]
    BlockStringLiteral,
}
```

`lex_string` / `lex_block_string` / `StringToken` / `BlockStringToken` are copied from `relay-crates/graphql-syntax/src/relay_lexer.rs` with `TokenKind` in place of that file's name. `strum` `}}` / `{{` emit a single brace.

## `TokenText`

Copied into this crate from iso `chunk_stream.rs`. Same methods, including `exclude_ends`.

```rust
// from crates/graphql_schema_parser/src/peekable_lexer.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct TokenText<'a> {
    pub location: Span,
    text: &'a str,
}

impl<'a> TokenText<'a> {
    pub(crate) fn text(self) -> &'a str {
        &self.text[self.location.as_usize_range()]
    }

    pub(crate) fn interned<T: From<intern::string_key::StringKey>>(self) -> WithSpan<T> {
        self.text().intern().to::<T>().with_span(self.location)
    }

    pub(crate) fn exclude_ends(self, n: u32) -> TokenText<'a> {
        TokenText {
            location: Span::new(self.location.start + n, self.location.end - n),
            text: self.text,
        }
    }
}
```

`text()` slices `source` at `location`. `interned` interns `text()` then `WithSpan<T>` at `location`. Identifiers use `interned`. A string or block-string token's span includes the quotes; the interned value is the interior (`exclude_ends(1).text()` for `StringLiteral`; `clean_block_string_literal(token.text())` for `BlockStringLiteral`, which slices the quotes itself).

## `PeekableLexer`

Logos is the tokenizer. `parse_token` maps iterator end to `EndOfFile` and takes `extras.error_token`.

```rust
// from crates/graphql_schema_parser/src/peekable_lexer.rs
pub(crate) struct PeekableLexer<'source> {
    current: WithSpan<TokenKind>,
    lexer: logos::Lexer<'source, TokenKind>,
    source: &'source str,
    end_index_of_last_parsed_token: u32,
    tokens: &'source mut Vec<WithSpan<GraphQLSemanticToken>>,
}

impl<'source> PeekableLexer<'source> {
    pub fn new(
        source: &'source str,
        tokens: &'source mut Vec<WithSpan<GraphQLSemanticToken>>,
    ) -> Self;

    fn parse_token(&mut self) -> WithSpan<TokenKind>;

    pub fn peek(&self) -> WithSpan<TokenKind>;
    pub fn reached_eof(&self) -> bool;
    pub fn source(&self, span: Span) -> &'source str;

    pub fn spanning<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, WithSpan<ParseError>>,
    ) -> Result<WithSpan<T>, WithSpan<ParseError>>;

    pub fn expected(&self, expected: Expectation) -> WithSpan<ParseError>;

    pub fn consume_token_if(
        &mut self,
        kind: TokenKind,
        token: GraphQLSemanticToken,
    ) -> Option<TokenText<'source>>;

    pub fn require_token(
        &mut self,
        kind: TokenKind,
        token: GraphQLSemanticToken,
    ) -> Result<TokenText<'source>, ()>;

    pub fn consume_matching_identifier(
        &mut self,
        identifier: &'static str,
        token: GraphQLSemanticToken,
    ) -> Option<TokenText<'source>>;

    pub fn require_matching_identifier(
        &mut self,
        identifier: &'static str,
        token: GraphQLSemanticToken,
    ) -> Result<TokenText<'source>, ()>;
}
```

`new` constructs a dummy `EndOfFile` at `Span::new(0, 0)` then `parse_token` to the first real token, same as origin `PeekableLexer::new`. The dummy is not recorded.

`parse_token`:

```rust
// from crates/graphql_schema_parser/src/peekable_lexer.rs
fn parse_token(&mut self) -> WithSpan<TokenKind> {
    let kind = match self.lexer.next() {
        Some(kind) => self.lexer.extras.error_token.take().unwrap_or(kind),
        None => TokenKind::EndOfFile,
    };
    self.end_index_of_last_parsed_token = self.current.location.end;
    let span: Span = self.lexer.span().into();
    std::mem::replace(&mut self.current, kind.with_span(span))
}
```

`None` is end of input, mapped to `EndOfFile`. `take().unwrap_or(kind)` is extras if the string callback stored one. At iterator end the span is logos' span (origin `lexer_span()`).

`peek` returns `current`. `reached_eof` is `current.item == TokenKind::EndOfFile`. `source` is `&self.source[span.as_usize_range()]` (no origin `offset`).

```rust
// from crates/graphql_schema_parser/src/peekable_lexer.rs
pub fn consume_token_if(
    &mut self,
    kind: TokenKind,
    token: GraphQLSemanticToken,
) -> Option<TokenText<'source>> {
    if self.peek().item != kind {
        return None;
    }
    let consumed = self.parse_token();
    self.tokens.push(token.with_span(consumed.location));
    TokenText {
        location: consumed.location,
        text: self.source,
    }
    .wrap_some()
}

pub fn require_token(
    &mut self,
    kind: TokenKind,
    token: GraphQLSemanticToken,
) -> Result<TokenText<'source>, ()> {
    self.consume_token_if(kind, token).ok_or(())
}

pub fn consume_matching_identifier(
    &mut self,
    identifier: &'static str,
    token: GraphQLSemanticToken,
) -> Option<TokenText<'source>> {
    if self.peek().item != TokenKind::Identifier {
        return None;
    }
    let location = self.peek().location;
    if self.source(location) != identifier {
        return None;
    }
    self.consume_token_if(TokenKind::Identifier, token)
}

pub fn require_matching_identifier(
    &mut self,
    identifier: &'static str,
    token: GraphQLSemanticToken,
) -> Result<TokenText<'source>, ()> {
    self.consume_matching_identifier(identifier, token).ok_or(())
}

pub fn expected(&self, expected: Expectation) -> WithSpan<ParseError> {
    ParseError::Expected(ExpectedFound {
        expected,
        found: Found::Token(self.peek().item),
    })
    .with_span(self.peek().location)
}

pub fn spanning<T>(
    &mut self,
    parse: impl FnOnce(&mut Self) -> Result<T, WithSpan<ParseError>>,
) -> Result<WithSpan<T>, WithSpan<ParseError>> {
    let start = self.current.location.start;
    let value = parse(self)?;
    value
        .with_span(Span::new(start, self.end_index_of_last_parsed_token))
        .wrap_ok()
}
```

The caller writes `lexer.require_token(kind, token).map_err(|()| lexer.expected(Expectation::Token(kind)))?`. `require_matching_identifier` mismatch (non-identifier or wrong identifier) is `map_err(|()| lexer.expected(Expectation::Keyword(identifier)))`. Origin has two messages for those cases; this is one.

A failed require records nothing. `spanning` start is `current.location.start` before the closure; end is `end_index_of_last_parsed_token` after success. On error the error's span is unchanged. A no-consume success is a zero-width span at that start.

A failed require records nothing.

## Groups

Origin `parse_optional_enclosed_items` and `parse_optional_fields`: if the open token is missing, empty `Vec`. If open is present, parse one item (required), then `while consume_token_if(close).is_none() { parse another item }`. The close is consumed by the successful `consume_token_if`. There is no `require_token(close)` after the loop. An unclosed group fails as the next item parse (at EOF, typically `Expected Token(Identifier), found end of file`). Empty `{}` after a present open fails as the first item parse (`Expected Token(Identifier), found closing brace ('}')`).

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
fn parse_optional_enclosed_items<T>(
    lexer: &mut PeekableLexer<'_>,
    open: TokenKind,
    close: TokenKind,
    open_token: GraphQLSemanticToken,
    close_token: GraphQLSemanticToken,
    mut parse: impl FnMut(&mut PeekableLexer<'_>) -> Result<WithSpan<T>, WithSpan<ParseError>>,
) -> Result<Vec<WithSpan<T>>, WithSpan<ParseError>> {
    if lexer.consume_token_if(open, open_token).is_none() {
        return Vec::new().wrap_ok();
    }
    let first = parse(lexer)?;
    let mut items = first.wrap_vec();
    while lexer.consume_token_if(close, close_token).is_none() {
        items.push(parse(lexer)?);
    }
    items.wrap_ok()
}
```

Delta: origin inlines this loop in `parse_optional_fields` as well; `parse_optional_fields` calls this helper.

Constant lists and constant objects allow empty. After a present open, loop `while consume_token_if(close).is_none() { parse item }` with no required first item. `[]` and `{}` are empty vecs. `parse_optional_constant_arguments` uses the required-first-item helper (origin: at least one argument if `(` is present; empty `()` errors).

Required braces (schema definition body): `require_token(OpenBrace, Brace)?` then a required first item then the same while-close loop.

`!` after a type is consumed (`consume_token_if(Exclamation, Bang)`), not a resolve leaf. A position on `!` resolves to the `NonNull` annotation.

## Errors

```rust
// from crates/graphql_schema_parser/src/parse_error.rs
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ParseError {
    #[error("{0}")]
    Expected(ExpectedFound),
    #[error("This integer does not fit in a 64-bit signed integer.")]
    IntegerDoesNotFitI64,
    #[error("This float literal is not a valid float.")]
    InvalidFloat,
    #[error("Enum values cannot be true, false or null.")]
    EnumValueCannotBeTrueFalseOrNull,
    #[error(
        "Root operation types (query, subscription or mutation) cannot be defined twice in a schema definition."
    )]
    DuplicateRootOperation,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, derive_more::Display)]
#[display("Expected {expected}, found {found}.")]
pub struct ExpectedFound {
    pub expected: Expectation,
    pub found: Found,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Expectation {
    Token(TokenKind),
    Keyword(&'static str),
    TypeAnnotation,
    ConstantValue,
    Definition,
    ExtensionBody,
    DirectiveLocation,
    RootOperation,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, derive_more::Display)]
pub enum Found {
    #[display("{0}")]
    Token(TokenKind),
}
```

```rust
// from crates/graphql_schema_parser/src/parse_error.rs
impl std::fmt::Display for Expectation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Expectation::Token(kind) => write!(f, "{kind}"),
            Expectation::Keyword(word) => write!(f, "the keyword `{word}`"),
            Expectation::TypeAnnotation => {
                write!(f, "a type, like 'String', 'String!', or '[String]'")
            }
            Expectation::ConstantValue => write!(f, "a constant value"),
            Expectation::Definition => {
                write!(f, "scalar, type, interface, union, enum, input, schema or directive")
            }
            Expectation::ExtensionBody => write!(f, "type"),
            Expectation::DirectiveLocation => write!(f, "a directive location"),
            Expectation::RootOperation => write!(f, "query, mutation or subscription"),
        }
    }
}
```

`Expectation` does not implement `std::error::Error`.

`Found::Token(TokenKind::EndOfFile)` displays `end of file`.

Do not `match` a `Result` to bind success and convert the error. `let value = lexer.require_token(kind, token).map_err(|()| lexer.expected(Expectation::Token(kind)))?;`.

Delta from origin messages: origin `parse_root_operation_type` says `Expected schema, mutation or subscription`; we use `RootOperation`. Origin `parse_type_system_definition` lists `extend` in the definition message; `Definition` does not. Origin `parse_type_system_extension` lists every definition keyword; `ExtensionBody` is `type`. Origin integer/float errors include the raw lexeme; integer overflow is `IntegerDoesNotFitI64`. `InvalidFloat` is the `Err` arm of `str::parse::<f64>()`; logos's float regex does not currently produce a token that fails that parse (overflow is `inf`). The arm exists because `parse` is `Result`.

## Descriptions and string constants

`parse_optional_description` (origin `description.rs`): `consume_token_if(StringLiteral, Description)` then interior via `exclude_ends(1).text()`, intern as `DescriptionValue`, wrap `Description`, span is the full token including quotes. Else `consume_token_if(BlockStringLiteral, Description)` then `clean_block_string_literal(token.text())` (origin, takes the quoted source and slices 3 off each end), intern as `DescriptionValue`, same full-token span. Else `None`.

Constant strings are `StringLiteral` only. A `BlockStringLiteral` in a constant-value position is `Expectation::ConstantValue`. The interned `StringLiteralValue` is the interior; quotes are not unescaped (`"a\\nb"` is backslash + `n`). Span is the full token including quotes.

`clean_block_string_literal` and its helpers (`get_common_indent`, `line_is_whitespace`, `is_not_whitespace`) are copied from origin `description.rs`.

## Semantic tokens

`require_token` / `require_matching_identifier` record on success. `consume_*` records on success. A failed require records nothing.

```rust
// from crates/graphql_schema_parser/src/semantic_token.rs
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum GraphQLSemanticToken {
    Keyword,
    Type,
    FieldName,
    Argument,
    DirectiveName,
    EnumValue,
    Description,
    Integer,
    Float,
    String,
    BooleanOrNull,
    Colon,
    Equals,
    Pipe,
    Ampersand,
    Bang,
    Brace,
    Parenthesis,
    Bracket,
}
```

Call sites:

- `Keyword`: `type`, `scalar`, `interface`, `union`, `enum`, `input`, `directive`, `schema`, `extend`, `implements`, `on`, `repeatable`, `query` / `mutation` / `subscription` in a schema definition, and directive locations (`FIELD`, `INPUT_OBJECT`, …)
- `Type`: type names (`Pet`, `ID`, `String`, implements names, union members, schema operation types)
- `FieldName`: field names
- `Argument`: input-value names and constant-object keys
- `DirectiveName`: `@` and the directive identifier
- `EnumValue`: enum value names
- `Description`: string or block string used as a description
- `Integer` / `Float` / `String`: those literals as constant values
- `BooleanOrNull`: `true` / `false` / `null`
- `Colon` / `Equals` / `Pipe` / `Ampersand` / `Bang`
- `Brace` / `Parenthesis` / `Bracket`: open and close share the token

An `Error` kind at peek is `found`; it is not consumed and not recorded.

## Constant-value dispatch

Origin `parse_constant_value` tries each production via `ControlFlow`. Match `peek().item`:

- `IntegerLiteral` → parse i64, `IntegerDoesNotFitI64` on failure
- `FloatLiteral` → `require_token(FloatLiteral, Float)`, `text().parse::<f64>()`, `InvalidFloat` at the token span on `Err`
- `StringLiteral` → interior interned as `StringLiteralValue`
- `Identifier` → `true` / `false` / `null` / else enum
- `OpenBracket` → list (empty allowed)
- `OpenBrace` → object (empty allowed)
- else → `lexer.expected(Expectation::ConstantValue)`

Type annotations match peek `Identifier` (named, then optional `!`) or `OpenBracket` (list, then optional `!`); else `Expectation::TypeAnnotation`. Origin uses `ControlFlow` for these two as well.

## Tests

Facts about the result. No snapshot files. Degenerate: empty file, unknown keyword, unclosed `{`, `type` with no name, empty `{}` after a present open, empty `[]` / `{}` as constants, optional `@` on a directive definition.
