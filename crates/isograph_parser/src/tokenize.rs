use logos::Logos;
use span::{WithSpan, WithSpanPostfix};

use crate::IsographLangTokenKind;
use prelude::Postfix;

/// Tokenize one literal: every token with its span, in order, ending at the end of the input
/// rather than with an `EndOfFile` token. The tokenizer skips spaces (line breaks are
/// tokens; the bracket matcher captures the ones at the literal's start and at a closed
/// group's interior's start), so consecutive tokens' spans need not touch.
pub fn tokenize(literal: &str) -> Vec<WithSpan<IsographLangTokenKind>> {
    let mut lexer = IsographLangTokenKind::lexer(literal);
    let mut tokens = Vec::new();
    while let Some(kind) = lexer.next() {
        tokens.push(kind.with_span(lexer.span().to()));
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::tokenize;
    use crate::IsographLangTokenKind;
    use span::Span;

    #[test]
    fn tokens_carry_their_spans() {
        let tokens = tokenize("field Query.Foo");
        let kinds: Vec<_> = tokens.iter().map(|token| token.item).collect();
        assert_eq!(
            kinds,
            vec![
                IsographLangTokenKind::Identifier,
                IsographLangTokenKind::Identifier,
                IsographLangTokenKind::Period,
                IsographLangTokenKind::Identifier,
            ]
        );
        assert_eq!(tokens[0].location, Span::new(0, 5));
        assert_eq!(tokens[3].location, Span::new(12, 15));
    }

    #[test]
    fn spaces_produce_no_token_and_line_breaks_produce_one_each() {
        assert_eq!(tokenize("   \t "), vec![]);
        let tokens = tokenize("  \n\r\n ");
        let kinds: Vec<_> = tokens.iter().map(|token| token.item).collect();
        assert_eq!(
            kinds,
            vec![
                IsographLangTokenKind::LineBreak,
                IsographLangTokenKind::LineBreak,
            ]
        );
        assert_eq!(tokens[0].location, Span::new(2, 3));
        assert_eq!(tokens[1].location, Span::new(3, 5));
    }

    #[test]
    fn punctuation_and_sigils_are_their_kinds() {
        let tokens = tokenize("$@:=!,");
        let kinds: Vec<_> = tokens.iter().map(|token| token.item).collect();
        assert_eq!(
            kinds,
            vec![
                IsographLangTokenKind::Dollar,
                IsographLangTokenKind::At,
                IsographLangTokenKind::Colon,
                IsographLangTokenKind::Equals,
                IsographLangTokenKind::Exclamation,
                IsographLangTokenKind::Comma,
            ]
        );
    }

    #[test]
    fn a_string_and_a_block_string_are_their_kinds() {
        assert_eq!(
            tokenize("\"hi\"")[0].item,
            IsographLangTokenKind::StringLiteral
        );
        assert_eq!(
            tokenize("\"\"")[0].item,
            IsographLangTokenKind::StringLiteral
        );
        assert_eq!(
            tokenize("\"\"\"hi\"\"\"")[0].item,
            IsographLangTokenKind::BlockStringLiteral
        );
    }

    #[test]
    fn integers_are_their_kind() {
        for text in ["0", "-0", "42", "-7", "9223372036854775807"] {
            assert_eq!(
                tokenize(text)[0].item,
                IsographLangTokenKind::IntegerLiteral,
                "for literal {text:?}"
            );
        }
        assert_eq!(
            tokenize("1.5")[0].item,
            IsographLangTokenKind::IntegerLiteral
        );
    }

    #[test]
    fn brackets_are_their_kinds() {
        let kinds: Vec<_> = tokenize("(){}[]").iter().map(|token| token.item).collect();
        assert_eq!(
            kinds,
            vec![
                IsographLangTokenKind::OpenParenthesis,
                IsographLangTokenKind::CloseParenthesis,
                IsographLangTokenKind::OpenBrace,
                IsographLangTokenKind::CloseBrace,
                IsographLangTokenKind::OpenBracket,
                IsographLangTokenKind::CloseBracket,
            ]
        );
    }

    #[test]
    fn number_and_string_errors_are_their_kinds() {
        assert_eq!(
            tokenize("01")[0].item,
            IsographLangTokenKind::ErrorNumberLiteralLeadingZero
        );
        assert_eq!(
            tokenize(".5")[0].item,
            IsographLangTokenKind::ErrorFloatLiteralMissingZero
        );
        assert_eq!(
            tokenize("1.")[0].item,
            IsographLangTokenKind::ErrorNumberLiteralTrailingInvalid
        );
        assert_eq!(tokenize("1e2")[0].item, IsographLangTokenKind::Error);
        assert_eq!(tokenize("-")[0].item, IsographLangTokenKind::Error);
        let unterminated = tokenize("\"unterminated");
        assert_eq!(unterminated[0].item, IsographLangTokenKind::Error);
        assert_eq!(unterminated[0].location, Span::new(0, 1));
        assert_eq!(unterminated[1].item, IsographLangTokenKind::Identifier);
        let block = tokenize("\"\"\"unterminated");
        assert_eq!(block[0].item, IsographLangTokenKind::Error);
        assert_eq!(block[1].item, IsographLangTokenKind::Identifier);
    }
}
