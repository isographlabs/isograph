use logos::Logos;
use prelude::Postfix;
use span::{WithSpan, WithSpanPostfix};

use crate::GraphQLTokenKind;

#[cfg_attr(not(test), expect(dead_code))]
pub(crate) fn tokenize(text: &str) -> Vec<WithSpan<GraphQLTokenKind>> {
    let mut lexer = GraphQLTokenKind::lexer(text);
    let mut tokens = Vec::new();
    while let Some(kind) = lexer.next() {
        let kind = lexer.extras.error_token.take().unwrap_or(kind);
        tokens.push(kind.with_span(lexer.span().to()));
    }
    tokens
}

#[cfg(test)]
mod tests {
    use span::Span;

    use super::tokenize;
    use crate::GraphQLTokenKind;

    fn kinds(text: &str) -> Vec<GraphQLTokenKind> {
        tokenize(text).iter().map(|token| token.item).collect()
    }

    fn single(text: &str) -> GraphQLTokenKind {
        let tokens = kinds(text);
        assert_eq!(tokens.len(), 1, "for text {text:?}");
        tokens[0]
    }

    #[test]
    fn a_type_definition_is_its_tokens() {
        assert_eq!(
            kinds("type Pet { id: ID! }"),
            vec![
                GraphQLTokenKind::Identifier,
                GraphQLTokenKind::Identifier,
                GraphQLTokenKind::OpenBrace,
                GraphQLTokenKind::Identifier,
                GraphQLTokenKind::Colon,
                GraphQLTokenKind::Identifier,
                GraphQLTokenKind::Exclamation,
                GraphQLTokenKind::CloseBrace,
            ]
        );
    }

    #[test]
    fn commas_are_ignored() {
        assert_eq!(
            kinds("a, b"),
            vec![GraphQLTokenKind::Identifier, GraphQLTokenKind::Identifier]
        );
    }

    #[test]
    fn a_comment_is_ignored() {
        let tokens = tokenize("# comment\ntype");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].item, GraphQLTokenKind::Identifier);
        assert_eq!(tokens[0].location, Span::new(10, 14));
    }

    #[test]
    fn a_byte_order_mark_is_ignored() {
        assert_eq!(single("\u{feff}type"), GraphQLTokenKind::Identifier);
    }

    #[test]
    fn every_line_terminator_separates() {
        for text in ["a\rb", "a\r\nb"] {
            assert_eq!(
                kinds(text),
                vec![GraphQLTokenKind::Identifier, GraphQLTokenKind::Identifier],
                "for text {text:?}"
            );
        }
    }

    #[test]
    fn a_form_feed_is_an_error() {
        assert_eq!(single("\u{000C}"), GraphQLTokenKind::Error);
    }

    #[test]
    fn empty_and_whitespace_text_have_no_tokens() {
        assert_eq!(kinds(""), vec![]);
        assert_eq!(kinds("  \t\n"), vec![]);
    }

    #[test]
    fn punctuators_are_their_kinds() {
        assert_eq!(
            kinds("& | = @ : ! ( ) [ ] { }"),
            vec![
                GraphQLTokenKind::Ampersand,
                GraphQLTokenKind::Pipe,
                GraphQLTokenKind::Equals,
                GraphQLTokenKind::At,
                GraphQLTokenKind::Colon,
                GraphQLTokenKind::Exclamation,
                GraphQLTokenKind::OpenParenthesis,
                GraphQLTokenKind::CloseParenthesis,
                GraphQLTokenKind::OpenBracket,
                GraphQLTokenKind::CloseBracket,
                GraphQLTokenKind::OpenBrace,
                GraphQLTokenKind::CloseBrace,
            ]
        );
    }

    #[test]
    fn executable_punctuators_are_errors() {
        for text in ["$", ".", "..."] {
            let tokens = kinds(text);
            assert!(!tokens.is_empty(), "for text {text:?}");
            assert!(
                tokens.iter().all(|kind| *kind == GraphQLTokenKind::Error),
                "for text {text:?}"
            );
        }
    }

    #[test]
    fn numbers_are_their_kinds() {
        for text in ["0", "-0", "42", "9223372036854775807"] {
            assert_eq!(
                single(text),
                GraphQLTokenKind::IntegerLiteral,
                "for text {text:?}"
            );
        }
        for text in ["1.5", "-1.5", "1e2", "1.5E-2"] {
            assert_eq!(
                single(text),
                GraphQLTokenKind::FloatLiteral,
                "for text {text:?}"
            );
        }
    }

    #[test]
    fn invalid_numbers_are_their_error_kinds() {
        assert_eq!(
            single("01"),
            GraphQLTokenKind::ErrorNumberLiteralLeadingZero
        );
        assert_eq!(single(".5"), GraphQLTokenKind::ErrorFloatLiteralMissingZero);
        for text in ["1.", "1a", "0x"] {
            assert_eq!(
                single(text),
                GraphQLTokenKind::ErrorNumberLiteralTrailingInvalid,
                "for text {text:?}"
            );
        }
    }

    #[test]
    fn strings_and_block_strings_are_their_kinds() {
        assert_eq!(single("\"hi\""), GraphQLTokenKind::StringLiteral);
        assert_eq!(single("\"\""), GraphQLTokenKind::StringLiteral);
        assert_eq!(
            single("\"\"\"hi\"\"\""),
            GraphQLTokenKind::BlockStringLiteral
        );
        let empty_block = tokenize("\"\"\"\"\"\"");
        assert_eq!(empty_block.len(), 1);
        assert_eq!(empty_block[0].item, GraphQLTokenKind::BlockStringLiteral);
        assert_eq!(empty_block[0].location, Span::new(0, 6));
    }

    #[test]
    fn unicode_in_strings_lexes() {
        for text in [
            "\"\\u{1F4A9}\"",
            "\"\\uD83D\\uDCA9\"",
            "\"\u{1F4A9}\"",
            "\"a\u{0001}b\"",
        ] {
            assert_eq!(
                single(text),
                GraphQLTokenKind::StringLiteral,
                "for text {text:?}"
            );
        }
    }

    #[test]
    fn invalid_strings_are_their_error_kinds() {
        let unsupported = tokenize("\"\\x\"");
        assert_eq!(unsupported.len(), 1);
        assert_eq!(
            unsupported[0].item,
            GraphQLTokenKind::ErrorUnsupportedStringCharacter
        );
        assert_eq!(unsupported[0].location, Span::new(0, 4));

        let unterminated = tokenize("\"unterminated");
        assert_eq!(unterminated.len(), 1);
        assert_eq!(
            unterminated[0].item,
            GraphQLTokenKind::ErrorUnterminatedString
        );
        assert_eq!(unterminated[0].location, Span::new(0, 13));

        let at_line_break = tokenize("\"hi\n");
        assert_eq!(at_line_break.len(), 1);
        assert_eq!(
            at_line_break[0].item,
            GraphQLTokenKind::ErrorUnterminatedString
        );
        assert_eq!(at_line_break[0].location, Span::new(0, 3));

        let block = tokenize("\"\"\"unterminated");
        assert_eq!(block.len(), 1);
        assert_eq!(
            block[0].item,
            GraphQLTokenKind::ErrorUnterminatedBlockString
        );
        assert_eq!(block[0].location, Span::new(0, 15));
    }

    #[test]
    fn a_block_string_holds_control_and_supplementary_characters() {
        assert_eq!(
            single("\"\"\"\u{0000}\u{1F4A9}\"\"\""),
            GraphQLTokenKind::BlockStringLiteral
        );
    }
}
