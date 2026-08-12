use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use safe_peekable::{IntoSafePeekable, SafePeekable};
use span::{Span, WithSpan};

use crate::{
    BracketItem, Bracketed, CloseBracket, IsographResolutionNode, MatchedBrackets, NonBracketToken,
    NonBracketTokenKind, OpenBracket, RawToken,
};

/// One level of the chunk tree: the whole literal at the root, a group's interior
/// below — the same role `MatchedBrackets` has on the bracket tree. Its chunks, in
/// order, and nothing else. Line breaks at a level's start were captured by the opening
/// bracket (or the literal's start) and never arrive here, so a chunk with no contents
/// is always a comma no item precedes, holding that comma as its boundary's first
/// token.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkedLevelParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ChunkedLevel(#[resolve_field] pub Vec<WithSpan<Chunk>>);

/// A maximal separator-free run of a level's items — tokens, groups, unmatched
/// brackets, anything — plus the boundary that ended it when one did: line breaks and
/// at most one comma, a second comma ending the boundary as well. The chunk-parsing
/// pass consumes it as one unit. Every chunk but a level's last has a trailing
/// separator by construction; the last's is the optional trailing delimiter.
/// The wrapping `WithSpan`'s span runs from the first part's start to the last part's
/// end.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkedLevelPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct Chunk {
    #[resolve_field]
    pub contents: Vec<WithSpan<ChunkContentItem>>,
    #[resolve_field]
    pub trailing_separator: Option<WithSpan<ChunkSeparator>>,
}

/// What a chunk holds: every non-separator item of its level, groups included.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum ChunkContentItem {
    NonBracket(NonBracketToken),
    UnmatchedOpen(#[resolve_field(parent_variant = Unmatched)] OpenBracket),
    UnmatchedClose(#[resolve_field(parent_variant = Unmatched)] CloseBracket),
    Group(ChunkedGroup),
}

/// A matched group re-chunked: the bracket tree's opening and closing are kept, and the
/// interior is a `ChunkedLevel` — same layout as `Bracketed` / `MatchedBrackets`.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ChunkedGroup {
    #[resolve_field(parent_variant = Matched)]
    pub opening: WithSpan<OpenBracket>,
    /// The wrapping `WithSpan`'s span runs from the opening's end to the closing's start.
    #[resolve_field(parent_variant = Interior)]
    pub children: WithSpan<ChunkedLevel>,
    #[resolve_field(parent_variant = Matched)]
    pub closing: WithSpan<CloseBracket>,
}

/// The boundary that ended its chunk: its line-break tokens and at most one comma, in
/// order. A second comma is never absorbed; it opens the next chunk's boundary. Its
/// tokens are not resolution leaves; a position on any of them answers the separator.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ChunkSeparator(pub Vec<WithSpan<SeparatorToken>>);

/// The two token kinds a separator boundary can hold.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SeparatorToken {
    Comma,
    LineBreak,
}

#[derive(Debug)]
pub enum ChunkedLevelParent<'a> {
    Root,
    Interior(Box<ChunkedGroupPath<'a>>),
}

pub type ChunkedLevelPath<'a> = PositionResolutionPath<&'a ChunkedLevel, ChunkedLevelParent<'a>>;

pub type ChunkPath<'a> = PositionResolutionPath<&'a Chunk, ChunkedLevelPath<'a>>;

pub type ChunkedGroupPath<'a> = PositionResolutionPath<&'a ChunkedGroup, ChunkPath<'a>>;

pub type ChunkSeparatorPath<'a> = PositionResolutionPath<&'a ChunkSeparator, ChunkPath<'a>>;

pub type NonBracketTokenPath<'a> = PositionResolutionPath<&'a NonBracketToken, ChunkPath<'a>>;

/// Shared by `OpenBracket` and `CloseBracket`: a bracket token is a group's own
/// opening or closing, or unmatched content of a chunk.
#[derive(Debug)]
pub enum BracketTokenParent<'a> {
    Matched(ChunkedGroupPath<'a>),
    Unmatched(ChunkPath<'a>),
}

pub type OpenBracketPath<'a> = PositionResolutionPath<&'a OpenBracket, BracketTokenParent<'a>>;
pub type CloseBracketPath<'a> = PositionResolutionPath<&'a CloseBracket, BracketTokenParent<'a>>;

type LevelItems<'a> = SafePeekable<std::slice::Iter<'a, WithSpan<BracketItem>>>;

/// Chunk a matched-brackets tree. The pass is infallible. Every raw token lands in a
/// chunk, and no grammar is checked.
pub fn chunk(tree: &WithSpan<MatchedBrackets>) -> WithSpan<ChunkedLevel> {
    WithSpan::new(chunk_level(&tree.item), tree.location)
}

fn chunk_level(level: &MatchedBrackets) -> ChunkedLevel {
    let mut items = level.0.iter().safe_peekable();
    let mut out = Vec::new();
    while items.peek().is_some() {
        out.push(absorb_chunk(&mut items));
    }
    ChunkedLevel(out)
}

fn separator_token(kind: NonBracketTokenKind) -> Option<SeparatorToken> {
    match kind {
        NonBracketTokenKind::Comma => Some(SeparatorToken::Comma),
        NonBracketTokenKind::LineBreak => Some(SeparatorToken::LineBreak),
        _ => None,
    }
}

fn separator_of(item: &WithSpan<BracketItem>) -> Option<SeparatorToken> {
    match &item.item {
        BracketItem::Raw(RawToken::NonBracket(token)) => separator_token(token.0),
        _ => None,
    }
}

/// One chunk from a nonempty stream: the content phase, then the boundary phase.
/// Whichever phase matches the first item consumes it — every item is either content
/// or a separator — so the chunk has at least one part and its span exists; the
/// boundary phase stops before a second comma, and absorbs at least the comma it
/// starts at when the chunk opens on one. Group interiors recurse in the content
/// phase's `Bracketed` arm.
fn absorb_chunk(items: &mut LevelItems<'_>) -> WithSpan<Chunk> {
    let mut contents = Vec::new();
    while let Some(peek) = items.peek() {
        let content_item = match &peek.view().item {
            BracketItem::Raw(RawToken::NonBracket(token)) => {
                if separator_token(token.0).is_some() {
                    // A separator ends the content phase.
                    break;
                }
                ChunkContentItem::NonBracket(*token)
            }
            BracketItem::Raw(RawToken::Open(open)) => ChunkContentItem::UnmatchedOpen(*open),
            BracketItem::Raw(RawToken::Close(close)) => ChunkContentItem::UnmatchedClose(*close),
            BracketItem::Bracketed(group) => ChunkContentItem::Group(chunk_group(group)),
        };
        let item = peek.commit();
        contents.push(WithSpan::new(content_item, item.location));
    }

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

    let separator_location = separators.iter().map(|s| s.location).reduce(Span::join);
    let span = contents
        .iter()
        .map(|c| c.location)
        .chain(separator_location)
        .reduce(Span::join)
        .expect("absorb_chunk requires a nonempty stream");
    let trailing_separator =
        separator_location.map(|location| WithSpan::new(ChunkSeparator(separators), location));
    WithSpan::new(
        Chunk {
            contents,
            trailing_separator,
        },
        span,
    )
}

fn chunk_group(group: &Bracketed) -> ChunkedGroup {
    ChunkedGroup {
        opening: group.opening,
        children: WithSpan::new(chunk_level(&group.children.item), group.children.location),
        closing: group.closing,
    }
}

#[cfg(test)]
mod tests {
    use resolve_position::ResolvePosition;

    use super::*;
    use crate::{
        match_brackets, tokenize, BracketError, BracketKind, NonBracketTokenKind, OpenBracket,
        CloseBracket,
    };
    use BracketKind::{Brace, Parenthesis};

    fn tree(literal: &str) -> WithSpan<MatchedBrackets> {
        match_brackets(tokenize(literal), literal.len() as u32)
    }

    fn chunked(literal: &str) -> WithSpan<ChunkedLevel> {
        chunk(&tree(literal))
    }

    /// The span of `pattern`, which must occur exactly once in `text`: an anchor an edit
    /// cannot silently shift, and one that fails loudly when it stops being unique.
    fn span_of(text: &str, pattern: &str) -> Span {
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

    fn chunk_span(chunk: &Chunk) -> Span {
        chunk
            .contents
            .iter()
            .map(|c| c.location)
            .chain(chunk.trailing_separator.as_ref().map(|s| s.location))
            .reduce(Span::join)
            .expect("a chunk has at least one part")
    }

    fn render_chunk<'a>(literal: &'a str, chunk: &Chunk) -> &'a str {
        let span = chunk_span(chunk);
        &literal[span.start as usize..span.end as usize]
    }

    fn content_item(chunk: &Chunk, index: usize) -> &ChunkContentItem {
        &chunk.contents[index].item
    }

    fn as_group(item: &ChunkContentItem) -> &ChunkedGroup {
        match item {
            ChunkContentItem::Group(group) => group,
            other => panic!("expected a group, got {other:?}"),
        }
    }

    fn as_non_bracket(item: &ChunkContentItem) -> NonBracketToken {
        match item {
            ChunkContentItem::NonBracket(token) => *token,
            other => panic!("expected a non-bracket token, got {other:?}"),
        }
    }

    fn as_unmatched_open(item: &ChunkContentItem) -> OpenBracket {
        match item {
            ChunkContentItem::UnmatchedOpen(open) => *open,
            other => panic!("expected an unmatched open, got {other:?}"),
        }
    }

    fn as_unmatched_close(item: &ChunkContentItem) -> CloseBracket {
        match item {
            ChunkContentItem::UnmatchedClose(close) => *close,
            other => panic!("expected an unmatched close, got {other:?}"),
        }
    }

    fn separator_kinds(separator: &ChunkSeparator) -> Vec<SeparatorToken> {
        separator.0.iter().map(|s| s.item).collect()
    }

    #[test]
    fn a_selection_set_splits_on_commas_and_line_breaks() {
        let text = "foo { bar, baz\nqux }";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 1);
        let top = &tree.item.0[0].item;
        assert_eq!(top.contents.len(), 2);
        assert_eq!(
            as_non_bracket(content_item(top, 0)).0,
            NonBracketTokenKind::Identifier
        );
        let brace = as_group(content_item(top, 1));
        assert_eq!(brace.opening.item.0, Brace);
        assert!(top.trailing_separator.is_none());

        let interior = &brace.children.item.0;
        assert_eq!(interior.len(), 3);
        assert_eq!(
            as_non_bracket(content_item(&interior[0].item, 0)).0,
            NonBracketTokenKind::Identifier
        );
        assert_eq!(
            separator_kinds(&interior[0].item.trailing_separator.as_ref().unwrap().item),
            vec![SeparatorToken::Comma]
        );
        assert_eq!(
            separator_kinds(&interior[1].item.trailing_separator.as_ref().unwrap().item),
            vec![SeparatorToken::LineBreak]
        );
        assert!(interior[2].item.trailing_separator.is_none());
        assert_eq!(render_chunk(text, &interior[0].item), "bar,");
        assert_eq!(render_chunk(text, &interior[1].item), "baz\n");
        assert_eq!(render_chunk(text, &interior[2].item), "qux");
    }

    #[test]
    fn multiple_groups_share_one_chunk_and_empty_interiors_are_zero_chunks() {
        let text = "foo { } { }";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 1);
        let top = &tree.item.0[0].item;
        assert_eq!(top.contents.len(), 3);
        assert_eq!(
            as_non_bracket(content_item(top, 0)).0,
            NonBracketTokenKind::Identifier
        );
        let first = as_group(content_item(top, 1));
        let second = as_group(content_item(top, 2));
        assert_eq!(first.children.item.0.len(), 0);
        assert_eq!(second.children.item.0.len(), 0);
        assert!(top.trailing_separator.is_none());
    }

    #[test]
    fn an_empty_brace_group_has_an_empty_interior_level() {
        let text = "{}";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 1);
        let top = &tree.item.0[0].item;
        assert_eq!(top.contents.len(), 1);
        let brace = as_group(content_item(top, 0));
        assert_eq!(brace.children.item.0.len(), 0);
        assert_eq!(
            brace.children.location,
            Span::new(span_of(text, "{").end, span_of(text, "}").start)
        );
    }

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
    fn a_demoted_brace_leaves_its_line_break_as_a_boundary() {
        let text = "foo {\n bar";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 2);
        assert_eq!(render_chunk(text, &tree.item.0[0].item), "foo {\n");
        assert_eq!(render_chunk(text, &tree.item.0[1].item), "bar");
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

    #[test]
    fn non_separator_tokens_stay_in_one_chunk() {
        let text = "bar, baz watttt, qux";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 3);
        let middle = &tree.item.0[1].item;
        assert_eq!(middle.contents.len(), 2);
        assert_eq!(render_chunk(text, middle), "baz watttt,");
    }

    #[test]
    fn a_line_break_before_a_group_splits_the_field_from_its_selection_set() {
        let text = "foo\n{ bar }";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 2);
        let first = &tree.item.0[0].item;
        assert_eq!(first.contents.len(), 1);
        assert_eq!(
            separator_kinds(&first.trailing_separator.as_ref().unwrap().item),
            vec![SeparatorToken::LineBreak]
        );
        let second = &tree.item.0[1].item;
        assert_eq!(second.contents.len(), 1);
        as_group(content_item(second, 0));
        assert!(second.trailing_separator.is_none());
    }

    #[test]
    fn an_unmatched_close_rides_inside_a_chunk_and_errors_stay_on_the_bracket_tree() {
        let text = "a ) b";
        let brackets = tree(text);
        let tree = chunk(&brackets);
        assert_eq!(tree.item.0.len(), 1);
        let top = &tree.item.0[0].item;
        assert_eq!(top.contents.len(), 3);
        assert_eq!(
            as_unmatched_close(content_item(top, 1)),
            CloseBracket(Parenthesis)
        );
        match brackets.item.errors().as_slice() {
            [BracketError::UnmatchedClose(close)] => {
                assert_eq!(close.location, span_of(text, ")"));
            }
            errors => panic!("expected exactly the unmatched close, got {errors:?}"),
        }
    }

    #[test]
    fn an_unclosed_brace_demotes_to_raw_items_at_the_top() {
        let text = "foo { bar";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 1);
        let top = &tree.item.0[0].item;
        assert_eq!(top.contents.len(), 3);
        assert_eq!(
            as_non_bracket(content_item(top, 0)).0,
            NonBracketTokenKind::Identifier
        );
        assert_eq!(
            as_unmatched_open(content_item(top, 1)),
            OpenBracket(Brace)
        );
        assert_eq!(
            as_non_bracket(content_item(top, 2)).0,
            NonBracketTokenKind::Identifier
        );
    }

    #[test]
    fn whitespace_only_and_empty_literals_are_empty_levels() {
        for text in ["   ", "", "\n\n"] {
            let tree = chunked(text);
            assert_eq!(tree.item.0.len(), 0);
            assert_eq!(tree.location, Span::from_usize(0, text.len()));
        }
    }

    #[test]
    fn chunk_spans_are_tight_to_their_parts() {
        let text = "foo { bar, baz }";
        let tree = chunked(text);
        let top = &tree.item.0[0];
        assert_eq!(top.location, chunk_span(&top.item));
        assert_eq!(
            top.location,
            Span::new(span_of(text, "foo").start, span_of(text, "}").end)
        );
        let interior = &as_group(content_item(&top.item, 1)).children.item.0;
        assert_eq!(interior[0].location, span_of(text, "bar,"));
        assert_eq!(
            interior[1].location,
            Span::new(span_of(text, "baz").start, span_of(text, "baz").end)
        );
    }

    #[test]
    fn an_unmatched_open_resolves_with_the_host_chunk_as_parent() {
        let text = "foo { ( }";
        let tree = chunked(text);
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "(")) {
            IsographResolutionNode::OpenBracket(open) => {
                assert_eq!(open.inner.0, Parenthesis);
                match open.parent {
                    BracketTokenParent::Unmatched(chunk_path) => {
                        assert!(matches!(
                            chunk_path.parent.parent,
                            ChunkedLevelParent::Interior(_)
                        ));
                    }
                    parent => panic!("expected an unmatched open, got {parent:?}"),
                }
            }
            node => panic!("expected the open bracket leaf, got {node:?}"),
        }
    }

    #[test]
    fn an_unmatched_close_at_the_root_resolves_with_the_root_level() {
        let text = "a }";
        let tree = chunked(text);
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "}")) {
            IsographResolutionNode::CloseBracket(close) => {
                assert_eq!(close.inner.0, Brace);
                match close.parent {
                    BracketTokenParent::Unmatched(chunk_path) => {
                        assert!(matches!(
                            chunk_path.parent.parent,
                            ChunkedLevelParent::Root
                        ));
                    }
                    parent => panic!("expected an unmatched close, got {parent:?}"),
                }
            }
            node => panic!("expected the close bracket leaf, got {node:?}"),
        }
    }

    #[test]
    fn a_matched_pair_resolves_with_its_group_as_parent() {
        let text = "foo { bar }";
        let tree = chunked(text);
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "{")) {
            IsographResolutionNode::OpenBracket(open) => match open.parent {
                BracketTokenParent::Matched(group) => {
                    assert_eq!(group.inner.closing.item.0, Brace);
                    assert_eq!(render_chunk(text, group.parent.inner), "foo { bar }");
                }
                parent => panic!("expected a matched open, got {parent:?}"),
            },
            node => panic!("expected the open bracket leaf, got {node:?}"),
        }
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "}")) {
            IsographResolutionNode::CloseBracket(close) => match close.parent {
                BracketTokenParent::Matched(group) => {
                    assert_eq!(group.inner.opening.item.0, Brace);
                }
                parent => panic!("expected a matched close, got {parent:?}"),
            },
            node => panic!("expected the close bracket leaf, got {node:?}"),
        }
    }

    #[test]
    fn a_span_straddling_a_groups_own_parts_resolves_to_the_group() {
        let text = "foo { bar }";
        let tree = chunked(text);
        let straddle = Span::new(span_of(text, "{").start, span_of(text, "bar").end);
        match tree.resolve(ChunkedLevelParent::Root, straddle) {
            IsographResolutionNode::ChunkedGroup(group) => {
                assert_eq!(group.inner.opening.item.0, Brace);
            }
            node => panic!("expected the group leaf, got {node:?}"),
        }
    }

    #[test]
    fn an_ordinary_token_resolves_to_its_own_leaf() {
        let text = "foo { bar }";
        let tree = chunked(text);
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "bar")) {
            IsographResolutionNode::NonBracketToken(token) => {
                assert_eq!(token.inner.0, NonBracketTokenKind::Identifier);
                assert!(matches!(
                    token.parent.parent.parent,
                    ChunkedLevelParent::Interior(_)
                ));
            }
            node => panic!("expected the token leaf, got {node:?}"),
        }
    }

    #[test]
    fn whitespace_inside_a_chunk_resolves_to_the_chunk() {
        let text = "foo { bar }";
        let tree = chunked(text);
        let gap = Span::new(span_of(text, "foo").end, span_of(text, "{").start);
        match tree.resolve(ChunkedLevelParent::Root, gap) {
            IsographResolutionNode::Chunk(chunk_path) => {
                assert_eq!(render_chunk(text, chunk_path.inner), "foo { bar }");
            }
            node => panic!("expected the chunk, got {node:?}"),
        }
    }

    #[test]
    fn resolution_walks_ancestry_against_source_text() {
        let text = "foo { bar, baz }";
        let tree = chunked(text);

        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "bar")) {
            IsographResolutionNode::NonBracketToken(token) => {
                assert_eq!(token.inner.0, NonBracketTokenKind::Identifier);
                assert_eq!(render_chunk(text, token.parent.inner), "bar,");
                match &token.parent.parent.parent {
                    ChunkedLevelParent::Interior(group) => {
                        assert_eq!(render_chunk(text, group.parent.inner), "foo { bar, baz }");
                    }
                    parent => panic!("expected an interior level, got {parent:?}"),
                }
            }
            node => panic!("expected the token leaf, got {node:?}"),
        }

        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "{")) {
            IsographResolutionNode::OpenBracket(open) => match open.parent {
                BracketTokenParent::Matched(group) => {
                    assert_eq!(render_chunk(text, group.parent.inner), "foo { bar, baz }");
                }
                parent => panic!("expected a matched open, got {parent:?}"),
            },
            node => panic!("expected the open bracket leaf, got {node:?}"),
        }

        match tree.resolve(ChunkedLevelParent::Root, span_of(text, ",")) {
            IsographResolutionNode::ChunkSeparator(separator) => {
                assert_eq!(render_chunk(text, separator.parent.inner), "bar,");
                assert_eq!(
                    as_non_bracket(content_item(separator.parent.inner, 0)).0,
                    NonBracketTokenKind::Identifier
                );
            }
            node => panic!("expected the separator, got {node:?}"),
        }

        let gap_in_chunk = Span::new(span_of(text, "foo").end, span_of(text, "{").start);
        match tree.resolve(ChunkedLevelParent::Root, gap_in_chunk) {
            IsographResolutionNode::Chunk(chunk_path) => {
                assert_eq!(render_chunk(text, chunk_path.inner), "foo { bar, baz }");
            }
            node => panic!("expected the chunk, got {node:?}"),
        }

        let gap_between = Span::new(span_of(text, "bar,").end, span_of(text, "baz").start);
        match tree.resolve(ChunkedLevelParent::Root, gap_between) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Interior(_)));
            }
            node => panic!("expected the interior level, got {node:?}"),
        }

        let text_ws = " foo { bar, baz }";
        let tree_ws = chunked(text_ws);
        let leading_ws = Span::new(0, span_of(text_ws, "foo").start);
        match tree_ws.resolve(ChunkedLevelParent::Root, leading_ws) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Root));
            }
            node => panic!("expected the root level, got {node:?}"),
        }
    }

    #[test]
    fn an_empty_interior_resolves_to_the_interior_level() {
        // A zero-width interior (`foo {}`) has no position that is not also the opening's
        // end / closing's start under Span::contains, so the case that can be asked is a
        // space inside the braces: the level has zero chunks and answers itself.
        let text = "foo { }";
        let tree = chunked(text);
        let space = Span::new(span_of(text, "{").end, span_of(text, "}").start);
        match tree.resolve(ChunkedLevelParent::Root, space) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Interior(_)));
            }
            node => panic!("expected the interior level, got {node:?}"),
        }
    }

    #[test]
    fn whitespace_only_resolves_to_the_root_level() {
        let text = "   ";
        let tree = chunked(text);
        match tree.resolve(ChunkedLevelParent::Root, Span::new(1, 2)) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Root));
            }
            node => panic!("expected the root level, got {node:?}"),
        }
        let empty = chunked("");
        match empty.resolve(ChunkedLevelParent::Root, Span::new(0, 0)) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Root));
            }
            node => panic!("expected the root level, got {node:?}"),
        }
    }

    #[test]
    fn an_unmatched_close_resolves_inside_its_host_chunk() {
        let text = "a ) b";
        let tree = chunked(text);
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, ")")) {
            IsographResolutionNode::CloseBracket(close) => match close.parent {
                BracketTokenParent::Unmatched(chunk_path) => {
                    assert_eq!(render_chunk(text, chunk_path.inner), "a ) b");
                }
                parent => panic!("expected an unmatched close, got {parent:?}"),
            },
            node => panic!("expected the close bracket leaf, got {node:?}"),
        }
    }
}
