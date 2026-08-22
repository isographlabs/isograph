use prelude::Postfix;
use span::{Span, WithSpan};

use crate::assert_semantic_tokens::assert_semantic_tokens;
use crate::chunk_stream::ItemCursor;
use crate::{
    AstError, CommaWithoutItem, Expectation, IsographSemanticToken, Slot, UnparsedChunkItems,
    chunk_level, match_brackets, tokenize,
};

pub(crate) type ParsedItems<P> = (
    Vec<WithSpan<Slot<P, UnparsedChunkItems>>>,
    Vec<WithSpan<AstError>>,
    Vec<CommaWithoutItem>,
);

pub(crate) fn parsed_items<P>(
    text: &str,
    leftover: Expectation,
    parse_item: impl Fn(&mut ItemCursor<'_>) -> Result<P, WithSpan<AstError>>,
    expected_tokens: &[(IsographSemanticToken, &str)],
) -> ParsedItems<P> {
    let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
    assert!(bracket_errors.is_empty(), "for literal {text:?}");
    let mut comma_errors = Vec::new();
    let tree = chunk_level(brackets.item.reference(), &mut comma_errors);
    let mut errors = Vec::new();
    let mut tokens = Vec::new();
    let dummy = {
        let (brackets, bracket_errors) = match_brackets(tokenize("x"), 1);
        assert!(bracket_errors.is_empty());
        let mut dummy_commas = Vec::new();
        chunk_level(brackets.item.reference(), &mut dummy_commas)
    };
    let mut parent = dummy.0[0].item.stream(text, &mut tokens, &mut errors);
    let items = tree.parse_each_chunk(parent.cursor(), leftover, parse_item);
    assert_semantic_tokens(text, &tokens, expected_tokens);
    (items, errors, comma_errors)
}

/// The span of `pattern`, which must occur exactly once in `text`: an anchor an edit
/// cannot silently shift, and one that fails loudly when it stops being unique.
pub(crate) fn span_of(text: &str, pattern: &str) -> Span {
    let mut occurrences = text.match_indices(pattern);
    let (offset, _) = occurrences
        .next()
        .expect("the pattern the test anchors on occurs in the literal");
    assert!(
        occurrences.next().is_none(),
        "the pattern the test anchors on occurs exactly once in the literal"
    );
    Span::from_usize(offset, offset + pattern.len())
}
