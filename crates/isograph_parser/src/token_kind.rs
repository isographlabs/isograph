use logos::{Lexer, Logos};
use prelude::Postfix;

#[derive(Default)]
pub struct TokenKindExtras {
    pub(crate) error_token: Option<IsographLangTokenKind>,
}

#[derive(Logos, Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, strum::Display)]
#[logos(extras = TokenKindExtras)]
pub enum IsographLangTokenKind {
    // TODO don't skip comments and spaces, since we want to auto-format etc
    #[regex(r"[ \t\f\ufeff]+", logos::skip)]
    #[error]
    #[strum(to_string = "error")]
    Error,

    /// One line break: `\r\n`, `\n`, or `\r`. A blank line is two of these.
    #[regex(r"\r\n|\n|\r")]
    #[strum(to_string = "line break")]
    LineBreak,

    #[strum(to_string = "unterminated string")]
    ErrorUnterminatedString,
    #[strum(to_string = "unsupported character in string")]
    ErrorUnsupportedStringCharacter,
    #[strum(to_string = "unterminated block string")]
    ErrorUnterminatedBlockString,

    // Valid tokens
    #[token("@")]
    #[strum(to_string = "at symbol ('@')")]
    At,
    #[token("}")]
    #[strum(to_string = "closing brace ('}}')")]
    CloseBrace,

    #[token("]")]
    #[strum(to_string = "closing bracket (']')")]
    CloseBracket,
    #[token(")")]
    #[strum(to_string = "closing parenthesis (')')")]
    CloseParenthesis,
    #[token(":")]
    #[strum(to_string = "colon (':')")]
    Colon,
    #[token("$")]
    #[strum(to_string = "dollar ('$')")]
    Dollar,
    #[strum(to_string = "end of file")]
    EndOfFile,

    #[token("=")]
    #[strum(to_string = "equals ('=')")]
    Equals,

    #[token("!")]
    #[strum(to_string = "exclamation mark ('!')")]
    Exclamation,

    // IntegerPart:    -?(0|[1-9][0-9]*)
    // FractionalPart: \\.[0-9]+
    // ExponentPart:   [eE][+-]?[0-9]+
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
    OpenParenthesis,
    #[token(".")]
    #[strum(to_string = "period ('.')")]
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
    #[strum(to_string = "comma (',')")]
    Comma,

    #[token("\"", lex_string)]
    #[strum(to_string = "string literal (e.g. '\"...\"')")]
    StringLiteral,

    #[token("\"\"\"", lex_block_string)]
    #[strum(to_string = "block string (e.g. '\"\"\"hi\"\"\"')")]
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

fn lex_block_string(lexer: &mut Lexer<'_, IsographLangTokenKind>) -> bool {
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
        IsographLangTokenKind::ErrorUnterminatedBlockString,
    )
}
