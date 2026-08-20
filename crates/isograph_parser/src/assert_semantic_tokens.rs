use span::{Span, WithSpan, WithSpanPostfix};

use crate::SemanticToken;

pub(crate) fn assert_semantic_tokens(
    text: &str,
    actual: &[WithSpan<SemanticToken>],
    expected: &[(SemanticToken, &str)],
) {
    let mut search_from = 0usize;
    let mut expected_tokens = Vec::with_capacity(expected.len());
    for &(role, pattern) in expected {
        let offset = text[search_from..]
            .find(pattern)
            .expect("the expected lexeme occurs in the fixture after the previous token");
        let start = search_from + offset;
        let end = start + pattern.len();
        expected_tokens.push(role.with_span(Span::from_usize(start, end)));
        search_from = end;
    }
    assert_eq!(
        actual,
        expected_tokens.as_slice(),
        "for literal {text:?}, actual {:?}, expected {:?}",
        displayed(text, actual),
        displayed(text, &expected_tokens),
    );
}

fn displayed(text: &str, tokens: &[WithSpan<SemanticToken>]) -> Vec<(SemanticToken, String)> {
    tokens
        .iter()
        .map(|token| (token.item, text[token.location.as_usize_range()].to_owned()))
        .collect()
}
