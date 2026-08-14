use logos::Logos;
use span::WithSpan;

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
        tokens.push(WithSpan::new(kind, lexer.span().to()));
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
}
