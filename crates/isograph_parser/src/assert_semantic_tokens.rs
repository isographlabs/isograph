use span::{Span, WithSpan, WithSpanPostfix};

use crate::IsographSemanticToken;

pub(crate) fn assert_semantic_tokens(
    text: &str,
    actual: &[WithSpan<IsographSemanticToken>],
    expected: &[(IsographSemanticToken, &str)],
) {
    let mut search_from = 0usize;
    let mut expected_tokens = Vec::with_capacity(expected.len());
    for &(role, pattern) in expected {
        let span = loop {
            let offset = text[search_from..]
                .find(pattern)
                .expect("the expected lexeme occurs as a recorded token after the previous token");
            let start = search_from + offset;
            let end = start + pattern.len();
            let span = Span::from_usize(start, end);
            search_from = end;
            if actual.iter().any(|token| token.location == span) {
                break span;
            }
        };
        expected_tokens.push(role.with_span(span));
    }
    assert_eq!(
        actual,
        expected_tokens.as_slice(),
        "for literal {text:?}, actual {:?}, expected {:?}",
        displayed(text, actual),
        displayed(text, &expected_tokens),
    );
}

fn displayed(
    text: &str,
    tokens: &[WithSpan<IsographSemanticToken>],
) -> Vec<(IsographSemanticToken, String)> {
    tokens
        .iter()
        .map(|token| (token.item, text[token.location.as_usize_range()].to_owned()))
        .collect()
}

mod tests {
    use super::*;

    #[test]
    fn sequential_search_skips_an_unrecorded_occurrence() {
        let text = "Pet!!";
        let actual = [
            IsographSemanticToken::GraphQLTypeName.with_span(Span::from_usize(0, 3)),
            IsographSemanticToken::Content.with_span(Span::from_usize(4, 5)),
        ];
        assert_semantic_tokens(
            text,
            actual.as_slice(),
            &[
                (IsographSemanticToken::GraphQLTypeName, "Pet"),
                (IsographSemanticToken::Content, "!"),
            ],
        );
    }

    #[test]
    fn sequential_search_keeps_source_order_when_both_occurrences_are_recorded() {
        let text = "aa";
        let actual = [
            IsographSemanticToken::Content.with_span(Span::from_usize(0, 1)),
            IsographSemanticToken::Content.with_span(Span::from_usize(1, 2)),
        ];
        assert_semantic_tokens(
            text,
            actual.as_slice(),
            &[
                (IsographSemanticToken::Content, "a"),
                (IsographSemanticToken::Content, "a"),
            ],
        );
    }

    #[test]
    fn sequential_search_accepts_an_empty_expected_list_when_nothing_was_recorded() {
        assert_semantic_tokens("", &[], &[]);
    }
}
