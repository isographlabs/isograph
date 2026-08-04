use logos::Logos;
use span::{Span, WithSpan};

use crate::IsographLangTokenKind;

/// Tokenize one literal: every token with its span, in order, ending at the end of the input
/// rather than with an `EndOfFile` token. The tokenizer skips whitespace, so consecutive
/// tokens' spans need not touch.
pub fn tokenize(literal: &str) -> Vec<WithSpan<IsographLangTokenKind>> {
    let mut lexer = IsographLangTokenKind::lexer(literal);
    let mut tokens = Vec::new();
    while let Some(kind) = lexer.next() {
        tokens.push(WithSpan::new(kind, lexer.span().into()));
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
        assert_eq!(tokens[0].span, Span::new(0, 5));
        assert_eq!(tokens[3].span, Span::new(12, 15));
    }

    #[test]
    fn whitespace_produces_no_token_and_no_eof_token_is_appended() {
        assert_eq!(tokenize("   \n\t "), vec![]);
    }
}
