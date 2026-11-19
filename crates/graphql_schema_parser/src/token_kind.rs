use logos::{Lexer, Logos};
use prelude::Postfix;

#[derive(Default)]
pub struct TokenKindExtras {
    pub(crate) error_token: Option<GraphQLTokenKind>,
}

#[derive(Logos, Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, derive_more::Display)]
#[logos(extras = TokenKindExtras)]
pub enum GraphQLTokenKind {
    #[regex(r"[ \t\r\n,\ufeff]+|#[^\r\n]*", logos::skip)]
    #[error]
    #[display("error")]
    Error,

    #[display("unterminated string")]
    ErrorUnterminatedString,
    #[display("unsupported character in string")]
    ErrorUnsupportedStringCharacter,
    #[display("unterminated block string")]
    ErrorUnterminatedBlockString,

    #[token("&")]
    #[display("ampersand ('&')")]
    Ampersand,
    #[token("@")]
    #[display("at symbol ('@')")]
    At,
    #[token("}")]
    #[display("closing brace ('}}')")]
    CloseBrace,
    #[token("]")]
    #[display("closing bracket (']')")]
    CloseBracket,
    #[token(")")]
    #[display("closing parenthesis (')')")]
    CloseParenthesis,
    #[token(":")]
    #[display("colon (':')")]
    Colon,
    #[token("=")]
    #[display("equals ('=')")]
    Equals,
    #[token("!")]
    #[display("exclamation mark ('!')")]
    Exclamation,

    #[regex("-?(0|[1-9][0-9]*)(\\.[0-9]+[eE][+-]?[0-9]+|\\.[0-9]+|[eE][+-]?[0-9]+)")]
    #[display("floating point value (e.g. '3.14')")]
    FloatLiteral,

    #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
    #[display("non-variable identifier (e.g. 'x' or 'Foo')")]
    Identifier,

    #[regex("-?(0|[1-9][0-9]*)")]
    #[display("integer value (e.g. '0' or '42')")]
    IntegerLiteral,
    #[regex("-?0[0-9]+(\\.[0-9]+[eE][+-]?[0-9]+|\\.[0-9]+|[eE][+-]?[0-9]+)?")]
    #[display("unsupported number (int or float) literal")]
    ErrorNumberLiteralLeadingZero,
    #[regex("-?(0|[1-9][0-9]*)(\\.[0-9]+[eE][+-]?[0-9]+|\\.[0-9]+|[eE][+-]?[0-9]+)?[.a-zA-Z_]")]
    #[display("unsupported number (int or float) literal")]
    ErrorNumberLiteralTrailingInvalid,
    #[regex("-?(\\.[0-9]+[eE][+-]?[0-9]+|\\.[0-9]+)")]
    #[display("unsupported number (int or float) literal")]
    ErrorFloatLiteralMissingZero,

    #[token("{")]
    #[display("open brace ('{{')")]
    OpenBrace,
    #[token("[")]
    #[display("open bracket ('[')")]
    OpenBracket,
    #[token("(")]
    #[display("open parenthesis ('(')")]
    OpenParenthesis,
    #[token("|")]
    #[display("pipe ('|')")]
    Pipe,

    #[token("\"", lex_string)]
    #[display("string literal (e.g. '\"...\"')")]
    StringLiteral,
    #[token("\"\"\"", lex_block_string)]
    #[display("block string (e.g. '\"\"\"hi\"\"\"')")]
    BlockStringLiteral,
}

#[derive(Logos, Debug)]
pub enum StringToken {
    #[error]
    Error,

    #[token("\\\"")]
    EscapedQuote,
    #[token("\\\\")]
    EscapedBackslash,
    #[token("\\/")]
    EscapedSlash,
    #[token("\\b")]
    EscapedBackspace,
    #[token("\\f")]
    EscapedFormFeed,
    #[token("\\n")]
    EscapedLineFeed,
    #[token("\\r")]
    EscapedCarriageReturn,
    #[token("\\t")]
    EscapedTab,

    #[regex(
        r#"\\u[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]"#,
        fixed_width_code_unit
    )]
    EscapedUnicode(u16),

    #[regex(r#"\\u\{[0-9A-Fa-f]+\}"#, variable_width_escape)]
    EscapedVariableWidthUnicode(VariableWidthEscape),

    #[token("\"")]
    Quote,

    #[regex(r#"\n|\r|\r\n"#)]
    LineTerminator,

    #[regex(r#"[^"\\\r\n]+"#)]
    StringCharacters,
}

/// The value of a `\u{...}` escape.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VariableWidthEscape {
    Scalar(char),
    NotAScalar,
}

fn fixed_width_code_unit(lexer: &mut Lexer<'_, StringToken>) -> Option<u16> {
    u16::from_str_radix(&lexer.slice()[2..], 16).ok()
}

fn variable_width_escape(lexer: &mut Lexer<'_, StringToken>) -> VariableWidthEscape {
    let slice = lexer.slice();
    u32::from_str_radix(&slice[3..slice.len() - 1], 16)
        .ok()
        .and_then(char::from_u32)
        .map_or(VariableWidthEscape::NotAScalar, VariableWidthEscape::Scalar)
}

#[derive(Logos, Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BlockStringToken {
    #[error]
    Error,

    #[token("\\\"\"\"")]
    EscapedTripleQuote,

    #[token("\"\"\"")]
    TripleQuote,

    #[regex(r#"[\u0009\u000A\u000D\u0020-\uFFFF]"#)]
    Other,
}

fn lex_string(lexer: &mut Lexer<'_, GraphQLTokenKind>) -> bool {
    let remainder = lexer.remainder();
    let mut string_lexer = StringToken::lexer(remainder);
    let mut first_error: Option<GraphQLTokenKind> = None;
    while let Some(string_token) = string_lexer.next() {
        match string_token {
            StringToken::Quote => {
                lexer.bump(string_lexer.span().end);
                return match first_error {
                    None => true,
                    Some(kind) => {
                        lexer.extras.error_token = kind.wrap_some();
                        false
                    }
                };
            }
            StringToken::LineTerminator => {
                return fail_with(
                    lexer,
                    string_lexer.span().start,
                    first_error.unwrap_or(GraphQLTokenKind::ErrorUnterminatedString),
                );
            }
            StringToken::EscapedQuote
            | StringToken::EscapedBackslash
            | StringToken::EscapedSlash
            | StringToken::EscapedBackspace
            | StringToken::EscapedFormFeed
            | StringToken::EscapedLineFeed
            | StringToken::EscapedCarriageReturn
            | StringToken::EscapedTab
            | StringToken::EscapedUnicode(_)
            | StringToken::EscapedVariableWidthUnicode(_)
            | StringToken::StringCharacters => {}
            StringToken::Error => {
                if first_error.is_none() {
                    first_error = GraphQLTokenKind::ErrorUnsupportedStringCharacter.wrap_some();
                }
            }
        }
    }
    fail_with(
        lexer,
        lexer.remainder().len(),
        first_error.unwrap_or(GraphQLTokenKind::ErrorUnterminatedString),
    )
}

fn fail_with(lexer: &mut Lexer<'_, GraphQLTokenKind>, n: usize, kind: GraphQLTokenKind) -> bool {
    lexer.bump(n);
    lexer.extras.error_token = kind.wrap_some();
    false
}

fn lex_block_string(lexer: &mut Lexer<'_, GraphQLTokenKind>) -> bool {
    let remainder = lexer.remainder();
    let mut string_lexer = BlockStringToken::lexer(remainder);
    while let Some(string_token) = string_lexer.next() {
        match string_token {
            BlockStringToken::TripleQuote => {
                lexer.bump(string_lexer.span().end);
                return true;
            }
            BlockStringToken::EscapedTripleQuote
            | BlockStringToken::Other
            | BlockStringToken::Error => {}
        }
    }
    fail_with(
        lexer,
        lexer.remainder().len(),
        GraphQLTokenKind::ErrorUnterminatedBlockString,
    )
}
