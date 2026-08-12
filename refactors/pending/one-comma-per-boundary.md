# one-comma-per-boundary: a comma must follow an item

A prefactor to the series parsing-plan.md orders. One change to the bracket matcher and one to the chunking pass establish one invariant: an empty chunk exists exactly when a comma has no item before it, and its boundary starts with that comma. An empty chunk is therefore unambiguously a parse error to the grammar stage, with no boundary inspection and no first-chunk special case.

1. Grouping captures line breaks with the opening bracket: after the matcher consumes an opening, it also consumes the line breaks directly after it, and the literal's start gets the same treatment for the root level. Captured line breaks are consumed like the spaces the tokenizer already drops; they appear in no level, so `foo {\n}` and a literal opening with a line break produce no separator at all, and a position on one resolves to the interior level exactly as a space there does. The opening's span stays the bare bracket.
2. The chunking pass's boundary phase stops before a second comma, so every `ChunkSeparator` holds any number of line breaks and at most one comma: the boundary grammar is `line-break+` or `line-break* comma line-break*`. The refused comma opens the next chunk, which is empty when nothing sits between the commas.

A leading comma (`{, bar }`, `{,}`) also opens an empty chunk: capture consumes only line breaks, so a comma with no item before it always lands as the first boundary token of an empty chunk, whether at a level's start or between two commas.

Both passes stay infallible, and `MatchedBrackets`, `ChunkedLevel`, and `ChunkSeparator` keep their shapes.

## The bracket matcher's capture

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// Line breaks directly after an opening bracket, or at the literal's start, are
/// captured: consumed with the bracket as insignificant whitespace, like the spaces the
/// tokenizer drops. Line breaks anywhere else remain ordinary tokens.
fn capture_line_breaks(tokens: &mut TokenStream) {
    while let Some(peek) = tokens.peek() {
        if peek.view().item != IsographLangTokenKind::LineBreak {
            break;
        }
        peek.commit();
    }
}
```

`match_brackets` captures at the literal's start. Before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
    let mut tokens = tokens.into_iter().safe_peekable();
    // The kind of every group the level being parsed sits inside, innermost last. The
```

After:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
    let mut tokens = tokens.into_iter().safe_peekable();
    capture_line_breaks(&mut tokens);
    // The kind of every group the level being parsed sits inside, innermost last. The
```

`parse_bracketed` captures right after the opening its caller consumed. Before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
    let children = enclosing_stack.with_pushed(opening.item.0, |enclosing_stack| {
        parse_items(tokens, enclosing_stack)
    });
```

After:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
    capture_line_breaks(tokens);
    let children = enclosing_stack.with_pushed(opening.item.0, |enclosing_stack| {
        parse_items(tokens, enclosing_stack)
    });
```

Capture survives demotion: in `foo {\nbar`, the brace captures the line break before the group turns out unclosed, so the demoted items are `foo`, `{`, `bar` with no separator between them. The captured line break belongs to the opening either way.

## The chunking pass's boundary phase

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
    let mut separators = Vec::new();
    while let Some(peek) = items.peek() {
        let Some(separator) = separator_of(peek.view()) else {
            break;
        };
        let item = peek.commit();
        separators.push(WithSpan::new(separator, item.location));
    }
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
    let mut separators: Vec<WithSpan<SeparatorToken>> = Vec::new();
    while let Some(peek) = items.peek() {
        let Some(separator) = separator_of(peek.view()) else {
            break;
        };
        if separator == SeparatorToken::Comma
            && separators.iter().any(|absorbed| absorbed.item == SeparatorToken::Comma)
        {
            // The second comma opens the next chunk's boundary.
            break;
        }
        let item = peek.commit();
        separators.push(WithSpan::new(separator, item.location));
    }
```

The loop in `chunk_level` still always advances: a chunk that starts at a refused or leading comma has an empty separator list, so its boundary phase absorbs that comma as its first token. That token is the chunk's first part, which is why an empty chunk's boundary always opens with its comma.

## Doc comments

`tokenize`, before:

```rust
// from crates/isograph_parser/src/tokenize.rs
/// Tokenize one literal: every token with its span, in order, ending at the end of the input
/// rather than with an `EndOfFile` token. The tokenizer skips spaces (line breaks are
/// tokens), so consecutive tokens' spans need not touch.
```

After:

```rust
// from crates/isograph_parser/src/tokenize.rs
/// Tokenize one literal: every token with its span, in order, ending at the end of the input
/// rather than with an `EndOfFile` token. The tokenizer skips spaces (line breaks are
/// tokens; the bracket matcher captures the ones after an opening bracket), so
/// consecutive tokens' spans need not touch.
```

`ChunkedLevel`, before:

```rust
// from crates/isograph_parser/src/chunk.rs
/// One level of the chunk tree: the whole literal at the root, a group's interior
/// below — the same role `MatchedBrackets` has on the bracket tree. Its chunks, in
/// order, and nothing else. A level that opens with separators holds them in a first
/// chunk with no contents: an admittedly suboptimal encoding, accepted as the price of
/// a level being a plain vec of one uniform chunk shape.
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
/// One level of the chunk tree: the whole literal at the root, a group's interior
/// below — the same role `MatchedBrackets` has on the bracket tree. Its chunks, in
/// order, and nothing else. Line breaks at a level's start were captured by the opening
/// bracket (or the literal's start) and never arrive here, so a chunk with no contents
/// is always a comma no item precedes, holding that comma as its boundary's first
/// token.
```

`Chunk`, before:

```rust
// from crates/isograph_parser/src/chunk.rs
/// A maximal separator-free run of a level's items — tokens, groups, unmatched
/// brackets, anything — plus the separator run that ended it when one did. The
/// chunk-parsing pass consumes it as one unit. Every chunk but a level's last has a
/// trailing separator by construction; the last's is the optional trailing delimiter.
/// The wrapping `WithSpan`'s span runs from the first part's start to the last part's
/// end.
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
/// A maximal separator-free run of a level's items — tokens, groups, unmatched
/// brackets, anything — plus the boundary that ended it when one did: line breaks and
/// at most one comma, a second comma ending the boundary as well. The chunk-parsing
/// pass consumes it as one unit. Every chunk but a level's last has a trailing
/// separator by construction; the last's is the optional trailing delimiter.
/// The wrapping `WithSpan`'s span runs from the first part's start to the last part's
/// end.
```

`ChunkSeparator`, before:

```rust
// from crates/isograph_parser/src/chunk.rs
/// The boundary that ended its chunk, holding every comma and line-break token it
/// absorbed, in order. Its tokens are not resolution leaves; a position on any of them
/// answers the separator.
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
/// The boundary that ended its chunk: its line-break tokens and at most one comma, in
/// order. A second comma is never absorbed; it opens the next chunk's boundary. Its
/// tokens are not resolution leaves; a position on any of them answers the separator.
```

`absorb_chunk`, before:

```rust
// from crates/isograph_parser/src/chunk.rs
/// One chunk from a nonempty stream: the content phase, then the boundary phase.
/// Whichever phase matches the first item consumes it — every item is either content
/// or a separator — so the chunk has at least one part and its span exists. Group
/// interiors recurse in the content phase's `Bracketed` arm.
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
/// One chunk from a nonempty stream: the content phase, then the boundary phase.
/// Whichever phase matches the first item consumes it — every item is either content
/// or a separator — so the chunk has at least one part and its span exists; the
/// boundary phase stops before a second comma, and absorbs at least the comma it
/// starts at when the chunk opens on one. Group interiors recurse in the content
/// phase's `Bracketed` arm.
```

## Tests

New matched_brackets.rs tests pin the capture at both call sites:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
    #[test]
    fn an_opening_bracket_captures_the_line_breaks_after_it() {
        let text = "foo {\n\n bar\n}";
        let tree = tree(text);
        let brace = group(&tree.item.0, 1);
        assert_eq!(brace.children.item.0.len(), 2);
        assert!(matches!(
            raw(&brace.children.item.0, 0),
            RawToken::NonBracket(NonBracketToken(NonBracketTokenKind::Identifier))
        ));
        assert!(matches!(
            raw(&brace.children.item.0, 1),
            RawToken::NonBracket(NonBracketToken(NonBracketTokenKind::LineBreak))
        ));
    }

    #[test]
    fn the_literal_start_captures_its_line_breaks() {
        let text = "\n\nfoo";
        let tree = tree(text);
        assert_eq!(tree.item.0.len(), 1);
        assert!(matches!(
            raw(&tree.item.0, 0),
            RawToken::NonBracket(NonBracketToken(NonBracketTokenKind::Identifier))
        ));
        assert_eq!(tree.location, Span::from_usize(0, text.len()));
    }
```

In chunk.rs, the multi-separator tail of `commas_and_line_breaks_are_equivalent_separators` asserted that `a,\n\n,b` produces two chunks with a four-token boundary; that input now produces three. The test keeps its equivalence half and drops the tail:

```rust
// from crates/isograph_parser/src/chunk.rs
    #[test]
    fn commas_and_line_breaks_are_equivalent_separators() {
        let comma = chunked("a, b");
        let linebreak = chunked("a\nb");
        assert_eq!(comma.item.0.len(), 2);
        assert_eq!(linebreak.item.0.len(), 2);
        assert_eq!(
            separator_kinds(&comma.item.0[0].item.trailing_separator.as_ref().unwrap().item),
            vec![SeparatorToken::Comma]
        );
        assert_eq!(
            separator_kinds(
                &linebreak.item.0[0].item.trailing_separator.as_ref().unwrap().item
            ),
            vec![SeparatorToken::LineBreak]
        );
        assert!(comma.item.0[1].item.trailing_separator.is_none());
        assert!(linebreak.item.0[1].item.trailing_separator.is_none());
    }
```

`leading_separators_make_an_empty_first_chunk` (`\n, a, b,\n`) described the old encoding and is replaced by one test per side of the new invariant:

```rust
// from crates/isograph_parser/src/chunk.rs
    #[test]
    fn captured_line_breaks_make_no_chunk() {
        let text = "\n\na, b\n";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 2);
        assert_eq!(render_chunk(text, &tree.item.0[0].item), "a,");
        assert_eq!(tree.location, Span::from_usize(0, text.len()));

        let interior = "foo {\n bar\n}";
        let tree = chunked(interior);
        let brace = as_group(content_item(&tree.item.0[0].item, 1));
        assert_eq!(brace.children.item.0.len(), 1);
        assert_eq!(render_chunk(interior, &brace.children.item.0[0].item), "bar\n");
    }

    #[test]
    fn a_comma_before_the_first_item_opens_an_empty_chunk() {
        let text = "\n, a";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 2);
        let empty = &tree.item.0[0];
        assert!(empty.item.contents.is_empty());
        assert_eq!(
            separator_kinds(&empty.item.trailing_separator.as_ref().unwrap().item),
            vec![SeparatorToken::Comma]
        );
        assert_eq!(empty.location, span_of(text, ","));
        assert_eq!(render_chunk(text, &tree.item.0[1].item), "a");
    }

    #[test]
    fn a_second_comma_ends_the_boundary_and_leaves_an_empty_chunk() {
        let text = "a,\n\n,b";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 3);
        let first = &tree.item.0[0].item;
        assert_eq!(render_chunk(text, first), "a,\n\n");
        assert_eq!(
            separator_kinds(&first.trailing_separator.as_ref().unwrap().item),
            vec![
                SeparatorToken::Comma,
                SeparatorToken::LineBreak,
                SeparatorToken::LineBreak,
            ]
        );
        let middle = &tree.item.0[1];
        assert!(middle.item.contents.is_empty());
        assert_eq!(
            separator_kinds(&middle.item.trailing_separator.as_ref().unwrap().item),
            vec![SeparatorToken::Comma]
        );
        let second_comma = span_of(text, ",b");
        assert_eq!(middle.location, Span::new(second_comma.start, second_comma.start + 1));
        assert_eq!(render_chunk(text, &tree.item.0[2].item), "b");
    }

    #[test]
    fn doubled_leading_commas_leave_two_empty_chunks() {
        let text = ",,a";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 3);
        assert!(tree.item.0[0].item.contents.is_empty());
        assert!(tree.item.0[1].item.contents.is_empty());
        assert_eq!(render_chunk(text, &tree.item.0[2].item), "a");
    }

    #[test]
    fn a_captured_line_break_resolves_to_its_level() {
        let text = "\nfoo {\n bar }";
        let tree = chunked(text);
        match tree.resolve(ChunkedLevelParent::Root, Span::new(0, 1)) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Root));
            }
            node => panic!("expected the root level, got {node:?}"),
        }
        let opening = span_of(text, "{\n");
        match tree.resolve(ChunkedLevelParent::Root, Span::new(opening.start + 1, opening.end)) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Interior(_)));
            }
            node => panic!("expected the interior level, got {node:?}"),
        }
    }
```

`whitespace_only_and_empty_literals_are_empty_levels` gains `"\n\n"` beside `"   "` and `""`: all three produce zero chunks. Every other test is untouched; a line break between items (`a_line_break_before_a_group_splits_the_field_from_its_selection_set`) is still a boundary, since capture applies only to an opening bracket and the literal's start.

## Landing checklist

1. The capture in matched_brackets.rs, the boundary-phase change in chunk.rs, the comment updates, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past. parse-entrypoint.md builds on the invariant this doc lands: an empty chunk is always an error, and its boundary's first token is its comma.
