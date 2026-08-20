# token-kind-zst: proof tokens in `NonBracketTokenKind`

Deferred. Not in the grammar-stage order.

Each `NonBracketTokenKind` variant carries a ZST. Matching `Dollar(dollar)` yields a `Dollar` you can only get from a token. `parse_variable_name` and the other value parse functions take that proof.

The ZST field is private. Construction is only in this module, via `From<IsographLangTokenKind>` and the associated constants that `consume_token_if` / `require_token` pass. Outside this module, a `Dollar` comes from a match.

No AST type, path alias, or `IsographResolutionNode` variant changes.

## Types

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Error(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ErrorUnterminatedString(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ErrorUnsupportedStringCharacter(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ErrorUnterminatedBlockString(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct At(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Colon(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Dollar(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EndOfFile(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Equals(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Exclamation(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Identifier(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct IntegerLiteral(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct LineBreak(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ErrorNumberLiteralLeadingZero(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ErrorNumberLiteralTrailingInvalid(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ErrorFloatLiteralMissingZero(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Period(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Comma(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct StringLiteral(());
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BlockStringLiteral(());

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum NonBracketTokenKind {
    Error(Error),
    ErrorUnterminatedString(ErrorUnterminatedString),
    ErrorUnsupportedStringCharacter(ErrorUnsupportedStringCharacter),
    ErrorUnterminatedBlockString(ErrorUnterminatedBlockString),
    At(At),
    Colon(Colon),
    Dollar(Dollar),
    EndOfFile(EndOfFile),
    Equals(Equals),
    Exclamation(Exclamation),
    Identifier(Identifier),
    IntegerLiteral(IntegerLiteral),
    LineBreak(LineBreak),
    ErrorNumberLiteralLeadingZero(ErrorNumberLiteralLeadingZero),
    ErrorNumberLiteralTrailingInvalid(ErrorNumberLiteralTrailingInvalid),
    ErrorFloatLiteralMissingZero(ErrorFloatLiteralMissingZero),
    Period(Period),
    Comma(Comma),
    StringLiteral(StringLiteral),
    BlockStringLiteral(BlockStringLiteral),
}

impl NonBracketTokenKind {
    pub const ERROR: Self = Self::Error(Error(()));
    pub const ERROR_UNTERMINATED_STRING: Self =
        Self::ErrorUnterminatedString(ErrorUnterminatedString(()));
    pub const ERROR_UNSUPPORTED_STRING_CHARACTER: Self =
        Self::ErrorUnsupportedStringCharacter(ErrorUnsupportedStringCharacter(()));
    pub const ERROR_UNTERMINATED_BLOCK_STRING: Self =
        Self::ErrorUnterminatedBlockString(ErrorUnterminatedBlockString(()));
    pub const AT: Self = Self::At(At(()));
    pub const COLON: Self = Self::Colon(Colon(()));
    pub const DOLLAR: Self = Self::Dollar(Dollar(()));
    pub const END_OF_FILE: Self = Self::EndOfFile(EndOfFile(()));
    pub const EQUALS: Self = Self::Equals(Equals(()));
    pub const EXCLAMATION: Self = Self::Exclamation(Exclamation(()));
    pub const IDENTIFIER: Self = Self::Identifier(Identifier(()));
    pub const INTEGER_LITERAL: Self = Self::IntegerLiteral(IntegerLiteral(()));
    pub const LINE_BREAK: Self = Self::LineBreak(LineBreak(()));
    pub const ERROR_NUMBER_LITERAL_LEADING_ZERO: Self =
        Self::ErrorNumberLiteralLeadingZero(ErrorNumberLiteralLeadingZero(()));
    pub const ERROR_NUMBER_LITERAL_TRAILING_INVALID: Self =
        Self::ErrorNumberLiteralTrailingInvalid(ErrorNumberLiteralTrailingInvalid(()));
    pub const ERROR_FLOAT_LITERAL_MISSING_ZERO: Self =
        Self::ErrorFloatLiteralMissingZero(ErrorFloatLiteralMissingZero(()));
    pub const PERIOD: Self = Self::Period(Period(()));
    pub const COMMA: Self = Self::Comma(Comma(()));
    pub const STRING_LITERAL: Self = Self::StringLiteral(StringLiteral(()));
    pub const BLOCK_STRING_LITERAL: Self = Self::BlockStringLiteral(BlockStringLiteral(()));
}
```

`consume_token_if` / `require_token` / `Expectation::Token` take the constants (`NonBracketTokenKind::DOLLAR`). A match that needs the proof binds the payload (`NonBracketTokenKind::Dollar(dollar)`).

## `From` conversions

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
            IsographLangTokenKind::Error => SplitToken::NonBracket(NonBracketTokenKind::ERROR),
            IsographLangTokenKind::ErrorUnterminatedString => {
                SplitToken::NonBracket(NonBracketTokenKind::ERROR_UNTERMINATED_STRING)
            }
            IsographLangTokenKind::ErrorUnsupportedStringCharacter => {
                SplitToken::NonBracket(NonBracketTokenKind::ERROR_UNSUPPORTED_STRING_CHARACTER)
            }
            IsographLangTokenKind::ErrorUnterminatedBlockString => {
                SplitToken::NonBracket(NonBracketTokenKind::ERROR_UNTERMINATED_BLOCK_STRING)
            }
            IsographLangTokenKind::At => SplitToken::NonBracket(NonBracketTokenKind::AT),
            IsographLangTokenKind::Colon => SplitToken::NonBracket(NonBracketTokenKind::COLON),
            IsographLangTokenKind::Dollar => SplitToken::NonBracket(NonBracketTokenKind::DOLLAR),
            IsographLangTokenKind::EndOfFile => {
                SplitToken::NonBracket(NonBracketTokenKind::END_OF_FILE)
            }
            IsographLangTokenKind::Equals => SplitToken::NonBracket(NonBracketTokenKind::EQUALS),
            IsographLangTokenKind::Exclamation => {
                SplitToken::NonBracket(NonBracketTokenKind::EXCLAMATION)
            }
            IsographLangTokenKind::Identifier => {
                SplitToken::NonBracket(NonBracketTokenKind::IDENTIFIER)
            }
            IsographLangTokenKind::IntegerLiteral => {
                SplitToken::NonBracket(NonBracketTokenKind::INTEGER_LITERAL)
            }
            IsographLangTokenKind::LineBreak => {
                SplitToken::NonBracket(NonBracketTokenKind::LINE_BREAK)
            }
            IsographLangTokenKind::ErrorNumberLiteralLeadingZero => {
                SplitToken::NonBracket(NonBracketTokenKind::ERROR_NUMBER_LITERAL_LEADING_ZERO)
            }
            IsographLangTokenKind::ErrorNumberLiteralTrailingInvalid => {
                SplitToken::NonBracket(NonBracketTokenKind::ERROR_NUMBER_LITERAL_TRAILING_INVALID)
            }
            IsographLangTokenKind::ErrorFloatLiteralMissingZero => {
                SplitToken::NonBracket(NonBracketTokenKind::ERROR_FLOAT_LITERAL_MISSING_ZERO)
            }
            IsographLangTokenKind::Period => SplitToken::NonBracket(NonBracketTokenKind::PERIOD),
            IsographLangTokenKind::Comma => SplitToken::NonBracket(NonBracketTokenKind::COMMA),
            IsographLangTokenKind::StringLiteral => {
                SplitToken::NonBracket(NonBracketTokenKind::STRING_LITERAL)
            }
            IsographLangTokenKind::BlockStringLiteral => {
                SplitToken::NonBracket(NonBracketTokenKind::BLOCK_STRING_LITERAL)
            }
```

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
            NonBracketTokenKind::Error(_) => IsographLangTokenKind::Error,
            NonBracketTokenKind::ErrorUnterminatedString(_) => {
                IsographLangTokenKind::ErrorUnterminatedString
            }
            NonBracketTokenKind::ErrorUnsupportedStringCharacter(_) => {
                IsographLangTokenKind::ErrorUnsupportedStringCharacter
            }
            NonBracketTokenKind::ErrorUnterminatedBlockString(_) => {
                IsographLangTokenKind::ErrorUnterminatedBlockString
            }
            NonBracketTokenKind::At(_) => IsographLangTokenKind::At,
            NonBracketTokenKind::Colon(_) => IsographLangTokenKind::Colon,
            NonBracketTokenKind::Dollar(_) => IsographLangTokenKind::Dollar,
            NonBracketTokenKind::EndOfFile(_) => IsographLangTokenKind::EndOfFile,
            NonBracketTokenKind::Equals(_) => IsographLangTokenKind::Equals,
            NonBracketTokenKind::Exclamation(_) => IsographLangTokenKind::Exclamation,
            NonBracketTokenKind::Identifier(_) => IsographLangTokenKind::Identifier,
            NonBracketTokenKind::IntegerLiteral(_) => IsographLangTokenKind::IntegerLiteral,
            NonBracketTokenKind::LineBreak(_) => IsographLangTokenKind::LineBreak,
            NonBracketTokenKind::ErrorNumberLiteralLeadingZero(_) => {
                IsographLangTokenKind::ErrorNumberLiteralLeadingZero
            }
            NonBracketTokenKind::ErrorNumberLiteralTrailingInvalid(_) => {
                IsographLangTokenKind::ErrorNumberLiteralTrailingInvalid
            }
            NonBracketTokenKind::ErrorFloatLiteralMissingZero(_) => {
                IsographLangTokenKind::ErrorFloatLiteralMissingZero
            }
            NonBracketTokenKind::Period(_) => IsographLangTokenKind::Period,
            NonBracketTokenKind::Comma(_) => IsographLangTokenKind::Comma,
            NonBracketTokenKind::StringLiteral(_) => IsographLangTokenKind::StringLiteral,
            NonBracketTokenKind::BlockStringLiteral(_) => IsographLangTokenKind::BlockStringLiteral,
```

## parse functions

Before (peek-then-parse.md): `parse_variable_name(cursor, missing_dollar)`, `parse_string_literal(cursor)`, `parse_integer_value(cursor)`, `parse_boolean_or_null(cursor)`.

After:

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn parse_variable_name(
    cursor: &mut ItemCursor<'_>,
    _dollar: Dollar,
) -> Result<WithSpan<VariableNameWrapper>, WithSpan<ParseError>> {
    cursor
        .require_token(NonBracketTokenKind::DOLLAR, SemanticToken::Variable)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::DOLLAR)))?;
    let name = cursor
        .require_token(NonBracketTokenKind::IDENTIFIER, SemanticToken::Variable)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::IDENTIFIER)))?;
    name.interned().map(VariableNameWrapper).wrap_ok()
}

fn parse_string_literal(
    cursor: &mut ItemCursor<'_>,
    _string_literal: StringLiteral,
) -> Result<StringLiteralValueWrapper, WithSpan<ParseError>> {
    let span = cursor
        .require_token(NonBracketTokenKind::STRING_LITERAL, SemanticToken::String)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::STRING_LITERAL)))?;
    span.interned().map(StringLiteralValueWrapper).item.wrap_ok()
}

fn parse_integer_value(
    cursor: &mut ItemCursor<'_>,
    _integer_literal: IntegerLiteral,
) -> Result<IntegerValue, WithSpan<ParseError>> {
    let span = cursor
        .require_token(
            NonBracketTokenKind::INTEGER_LITERAL,
            SemanticToken::Integer,
        )
        .map_err(|()| {
            cursor.expected(Expectation::Token(NonBracketTokenKind::INTEGER_LITERAL))
        })?;
    match span.text().parse() {
        Ok(value) => IntegerValue(value).wrap_ok(),
        Err(_) => ParseError::IntegerDoesNotFitI64
            .with_span(span.location)
            .wrap_err(),
    }
}

fn parse_boolean_or_null(
    cursor: &mut ItemCursor<'_>,
    _identifier: Identifier,
) -> Result<NonConstantValue, WithSpan<ParseError>> {
    let span = cursor
        .require_token(NonBracketTokenKind::IDENTIFIER, SemanticToken::BooleanOrNull)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::IDENTIFIER)))?;
    match span.text() {
        "true" => NonConstantValue::Boolean(BooleanValue(Boolean::True)).wrap_ok(),
        "false" => NonConstantValue::Boolean(BooleanValue(Boolean::False)).wrap_ok(),
        "null" => NonConstantValue::Null(NullValue).wrap_ok(),
        _ => ParseError::expected(
            Expectation::Value,
            Found::Token(NonBracketTokenKind::IDENTIFIER),
        )
        .with_span(span.location)
        .wrap_err(),
    }
}
```

`parse_object_literal` is unchanged: a brace group is `ChunkContentItem::Group`, not a `NonBracketTokenKind`.

## Dispatch

```rust
// from crates/isograph_parser/src/arguments.rs
                ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::Dollar(
                    dollar,
                ))) => {
                    drop(peek);
                    return NonConstantValue::Variable(VariableUse(parse_variable_name(
                        cursor, dollar,
                    )?))
                    .wrap_ok();
                }
                ChunkContentItem::NonBracket(NonBracketToken(
                    NonBracketTokenKind::StringLiteral(string_literal),
                )) => {
                    drop(peek);
                    return NonConstantValue::String(parse_string_literal(cursor, string_literal)?)
                        .wrap_ok();
                }
                ChunkContentItem::NonBracket(NonBracketToken(
                    NonBracketTokenKind::IntegerLiteral(integer_literal),
                )) => {
                    drop(peek);
                    return NonConstantValue::Integer(parse_integer_value(
                        cursor,
                        integer_literal,
                    )?)
                    .wrap_ok();
                }
                ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::Identifier(
                    identifier,
                ))) => {
                    drop(peek);
                    return parse_boolean_or_null(cursor, identifier);
                }
```

parse-variables.md's `parse_name_colon` lhs is `parse_variable_name(cursor, Expectation::VariableDeclarationOrUsage)`. This doc peeks there to bind `Dollar(dollar)` and pass it; anything else is `Expectation::VariableDeclarationOrUsage`.

```rust
// from crates/isograph_parser/src/variables.rs
        |cursor| match cursor.peek() {
            Some(peek) => match peek.view().item.reference() {
                ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::Dollar(
                    dollar,
                ))) => {
                    drop(peek);
                    parse_variable_name(cursor, dollar)
                }
                _ => cursor
                    .expected(Expectation::VariableDeclarationOrUsage)
                    .wrap_err(),
            },
            None => cursor
                .expected(Expectation::VariableDeclarationOrUsage)
                .wrap_err(),
        },
```

Every other `NonBracketTokenKind::Identifier` (unit) in consume/require/`Expectation::Token` becomes `NonBracketTokenKind::IDENTIFIER`. Same for the other variants.

## Tests

Existing token-kind and value tests. A test that `parse_variable_name` cannot be called without a `Dollar` is the type system.

## Landing checklist

1. The ZST structs, `NonBracketTokenKind` payloads, associated constants, both `From` impls, consume/require/`Expectation::Token` sites, the parse function proofs, the dispatch and declaration lhs. `cargo test -p isograph_parser` passes.
2. Move this doc to refactors/past.
