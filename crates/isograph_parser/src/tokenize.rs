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
        let kind = lexer.extras.error_token.take().unwrap_or(kind);
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
    }

    #[test]
    fn floats_are_their_kind() {
        for text in ["1.5", "12.34", "0.0", "-1.5", "1e2", "1.5e2"] {
            let tokens = tokenize(text);
            assert_eq!(tokens.len(), 1, "for literal {text:?}");
            assert_eq!(
                tokens[0].item,
                IsographLangTokenKind::FloatLiteral,
                "for literal {text:?}"
            );
        }
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
        assert_eq!(tokenize("-")[0].item, IsographLangTokenKind::Error);
        let unterminated = tokenize("\"unterminated");
        assert_eq!(unterminated.len(), 1);
        assert_eq!(
            unterminated[0].item,
            IsographLangTokenKind::ErrorUnterminatedString
        );
        assert_eq!(unterminated[0].location, Span::new(0, 13));
        let block = tokenize("\"\"\"unterminated");
        assert_eq!(block.len(), 1);
        assert_eq!(
            block[0].item,
            IsographLangTokenKind::ErrorUnterminatedBlockString
        );
        assert_eq!(block[0].location, Span::new(0, 15));
        let unsupported = tokenize("\"\\x\"");
        assert_eq!(unsupported.len(), 1);
        assert_eq!(
            unsupported[0].item,
            IsographLangTokenKind::ErrorUnsupportedStringCharacter
        );
        assert_eq!(unsupported[0].location, Span::new(0, 4));
        let unterminated_at_newline = tokenize("\"hi\n");
        assert_eq!(unterminated_at_newline.len(), 2);
        assert_eq!(
            unterminated_at_newline[0].item,
            IsographLangTokenKind::ErrorUnterminatedString
        );
        assert_eq!(unterminated_at_newline[0].location, Span::new(0, 3));
        assert_eq!(
            unterminated_at_newline[1].item,
            IsographLangTokenKind::LineBreak
        );
    }
}
