use span::Span;

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
