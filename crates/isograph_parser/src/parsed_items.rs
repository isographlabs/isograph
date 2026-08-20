use prelude::Postfix;
use span::{Span, WithSpan};

use crate::chunk_stream::ItemCursor;
use crate::{
    AstError, CommaWithoutItem, Expectation, SemanticToken, Slot, UnparsedChunkItems, chunk,
    match_brackets, tokenize,
};

pub(crate) type ParsedItems<P> = (
    Vec<WithSpan<Slot<P, UnparsedChunkItems>>>,
    Vec<WithSpan<AstError>>,
    Vec<CommaWithoutItem>,
    Vec<WithSpan<SemanticToken>>,
);

pub(crate) fn parsed_items<P>(
    text: &str,
    leftover: Expectation,
    parse_item: impl Fn(&mut ItemCursor<'_>) -> Result<P, WithSpan<AstError>>,
) -> ParsedItems<P> {
    let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
    assert!(bracket_errors.is_empty(), "for literal {text:?}");
    let (tree, comma_errors) = chunk(brackets.reference());
    let mut errors = Vec::new();
    let mut tokens = Vec::new();
    let dummy = {
        let (brackets, bracket_errors) = match_brackets(tokenize("x"), 1);
        assert!(bracket_errors.is_empty());
        let (tree, comma_errors) = chunk(brackets.reference());
        assert_eq!(comma_errors, vec![]);
        tree
    };
    let mut parent = dummy.item.0[0].item.stream(text, &mut tokens, &mut errors);
    let items = tree
        .item
        .parse_each_chunk(parent.cursor(), leftover, parse_item);
    (items, errors, comma_errors, tokens)
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
