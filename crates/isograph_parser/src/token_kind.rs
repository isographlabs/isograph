use std::fmt;

use logos::{Lexer, Logos};
use prelude::Postfix;

#[derive(Default)]
pub struct TokenKindExtras {
    pub(crate) error_token: Option<IsographLangTokenKind>,
}

#[derive(Logos, Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[logos(extras = TokenKindExtras)]
pub enum IsographLangTokenKind {
    // TODO don't skip comments and spaces, since we want to auto-format etc
    #[regex(r"[ \t\f\ufeff]+", logos::skip)]
    #[error]
    Error,

    /// One line break: `\r\n`, `\n`, or `\r`. A blank line is two of these.
    #[regex(r"\r\n|\n|\r")]
    LineBreak,

    ErrorUnterminatedString,
    ErrorUnsupportedStringCharacter,
    ErrorUnterminatedBlockString,

    // Valid tokens
    #[token("@")]
    At,
    #[token("}")]
    CloseBrace,

    #[token("]")]
    CloseBracket,
    #[token(")")]
    CloseParenthesis,
    #[token(":")]
    Colon,
    #[token("$")]
    Dollar,
    EndOfFile,

    #[token("=")]
    Equals,

    #[token("!")]
    Exclamation,

    // IntegerPart:    -?(0|[1-9][0-9]*)
    // FractionalPart: \\.[0-9]+
    // ExponentPart:   [eE][+-]?[0-9]+
    // #[regex("-?(0|[1-9][0-9]*)(\\.[0-9]+[eE][+-]?[0-9]+|\\.[0-9]+|[eE][+-]?[0-9]+)")]
    // FloatLiteral,
    #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,

    #[regex("-?(0|[1-9][0-9]*)")]
    IntegerLiteral,
    #[regex("-?0[0-9]+(\\.[0-9]+[eE][+-]?[0-9]+|\\.[0-9]+|[eE][+-]?[0-9]+)?")]
    ErrorNumberLiteralLeadingZero,

    #[regex("-?(0|[1-9][0-9]*)(\\.[0-9]+[eE][+-]?[0-9]+|\\.[0-9]+|[eE][+-]?[0-9]+)?[.a-zA-Z_]")]
    ErrorNumberLiteralTrailingInvalid,

    #[regex("-?(\\.[0-9]+[eE][+-]?[0-9]+|\\.[0-9]+)")]
    ErrorFloatLiteralMissingZero,

    #[token("{")]
    OpenBrace,

    #[token("[")]
    OpenBracket,
    #[token("(")]
    OpenParenthesis,
    #[token(".")]
    Period,
    // #[token("..")]
    // PeriodPeriod,

    // #[token("|")]
    // Pipe,

    // #[token("...")]
    // Spread,

    // Comments
    // #[regex("#[^\n\r]*")]
    // SingleLineComment,
    // Whitespace
    #[token(",")]
    Comma,

    #[token("\"", lex_string)]
    StringLiteral,

    #[token("\"\"\"", lex_block_string)]
    BlockStringLiteral,
}

#[derive(Logos, Debug)]
pub enum StringToken {
    #[error]
    Error,

    #[regex(r#"\\["\\/bfnrt]"#)]
    EscapedCharacter,

    #[regex(r#"\\u[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]"#)]
    EscapedUnicode,

    #[token("\"")]
    Quote,

    #[regex(r#"\n|\r|\r\n"#)]
    LineTerminator,

    #[regex(r#"[\u0009\u0020\u0021\u0023-\u005B\u005D-\uFFFF]+"#)]
    StringCharacters,
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

fn lex_string(lexer: &mut Lexer<'_, IsographLangTokenKind>) -> bool {
    let remainder = lexer.remainder();
    let mut string_lexer = StringToken::lexer(remainder);
    let mut first_error: Option<IsographLangTokenKind> = None;
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
                    first_error.unwrap_or(IsographLangTokenKind::ErrorUnterminatedString),
                );
            }
            StringToken::EscapedCharacter
            | StringToken::EscapedUnicode
            | StringToken::StringCharacters => {}
            StringToken::Error => {
                if first_error.is_none() {
                    first_error =
                        IsographLangTokenKind::ErrorUnsupportedStringCharacter.wrap_some();
                }
            }
        }
    }
    fail_with(
        lexer,
        lexer.remainder().len(),
        first_error.unwrap_or(IsographLangTokenKind::ErrorUnterminatedString),
    )
}

fn fail_with(
    lexer: &mut Lexer<'_, IsographLangTokenKind>,
    n: usize,
    kind: IsographLangTokenKind,
) -> bool {
    lexer.bump(n);
    lexer.extras.error_token = kind.wrap_some();
    false
}

impl fmt::Display for IsographLangTokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            // IsographLangTokenKind::Ampersand => "ampersand ('&')",
            IsographLangTokenKind::At => "at symbol ('@')",
            IsographLangTokenKind::CloseBrace => "closing brace ('}')",
            IsographLangTokenKind::CloseBracket => "closing bracket (']')",
            IsographLangTokenKind::CloseParenthesis => "closing parenthesis (')')",
            // IsographLangTokenKind::Colon => "colon (':')",
            IsographLangTokenKind::Dollar => "dollar ('$')",
            IsographLangTokenKind::EndOfFile => "end of file",
            IsographLangTokenKind::Equals => "equals ('=')",
            IsographLangTokenKind::Exclamation => "exclamation mark ('!')",
            // IsographLangTokenKind::FloatLiteral => "floating point value (e.g. '3.14')",
            IsographLangTokenKind::Identifier => "non-variable identifier (e.g. 'x' or 'Foo')",
            IsographLangTokenKind::LineBreak => "line break",
            IsographLangTokenKind::IntegerLiteral => "integer value (e.g. '0' or '42')",
            IsographLangTokenKind::OpenBrace => "open brace ('{')",
            IsographLangTokenKind::OpenBracket => "open bracket ('[')",
            IsographLangTokenKind::OpenParenthesis => "open parenthesis ('(')",
            IsographLangTokenKind::Period => "period ('.')",
            // IsographLangTokenKind::PeriodPeriod => "double period ('..')",
            // IsographLangTokenKind::Pipe => "pipe ('|')",
            // IsographLangTokenKind::Spread => "spread ('...')",
            IsographLangTokenKind::BlockStringLiteral => "block string (e.g. '\"\"\"hi\"\"\"')",
            IsographLangTokenKind::Error => "error",
            IsographLangTokenKind::ErrorFloatLiteralMissingZero => {
                "unsupported number (int or float) literal"
            }
            IsographLangTokenKind::ErrorNumberLiteralLeadingZero => {
                "unsupported number (int or float) literal"
            }
            IsographLangTokenKind::ErrorNumberLiteralTrailingInvalid => {
                "unsupported number (int or float) literal"
            }
            IsographLangTokenKind::Comma => "comma (',')",
            IsographLangTokenKind::Colon => "colon (':')",
            IsographLangTokenKind::StringLiteral => "string literal (e.g. '\"...\"')",
            IsographLangTokenKind::ErrorUnterminatedString => "unterminated string",
            IsographLangTokenKind::ErrorUnsupportedStringCharacter => {
                "unsupported character in string"
            }
            IsographLangTokenKind::ErrorUnterminatedBlockString => "unterminated block string",
            // IsographLangTokenKind::Empty => "missing expected kind",
        };
        f.write_str(message)
    }
}

fn lex_block_string(lexer: &mut Lexer<'_, IsographLangTokenKind>) -> bool {
    let remainder = lexer.remainder();
    let mut string_lexer = BlockStringToken::lexer(remainder);
    while let Some(string_token) = string_lexer.next() {
        match string_token {
            BlockStringToken::TripleQuote => {
                lexer.bump(string_lexer.span().end);
                return true;
            }
            BlockStringToken::EscapedTripleQuote | BlockStringToken::Other => {}
            BlockStringToken::Error => unreachable!(),
        }
    }
    fail_with(
        lexer,
        lexer.remainder().len(),
        IsographLangTokenKind::ErrorUnterminatedBlockString,
    )
}
