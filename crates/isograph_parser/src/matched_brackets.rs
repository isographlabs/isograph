use prelude::Postfix;
use resolve_position_macros::ResolvePosition;
use safe_peekable::{IntoSafePeekable, SafePeekable};
use scoped_stack::Stack;
use span::{Span, WithSpan, WithSpanPostfix};
use thiserror::Error;

use crate::{
    BracketKind, BracketToken, ChunkContentItemParent, ChunkedGroupPath, IsographLangTokenKind,
    IsographResolutionNode, NonBracketTokenKind, SplitToken,
};

/// One level: the whole literal at the root, a group's interior below.
#[derive(Debug, PartialEq, Eq)]
pub struct MatchedBrackets(pub Vec<WithSpan<BracketItem>>);

#[derive(Debug, PartialEq, Eq)]
pub enum BracketItem {
    Raw(NonBracketToken),
    Bracketed(Bracketed),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Bracketed {
    pub opening: WithSpan<OpenBracket>,
    /// The wrapping `WithSpan`'s span runs from the opening's end to the closing's start.
    pub children: WithSpan<MatchedBrackets>,
    pub closing: WithSpan<CloseBracket>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkContentItemParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NonBracketToken(pub NonBracketTokenKind);

/// An opening bracket: a group's own opening.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkedGroupPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct OpenBracket(pub BracketKind);

/// A closing bracket: a group's own closing.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkedGroupPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct CloseBracket(pub BracketKind);

/// The matcher's errors, returned beside the tree, in source order. The tree cannot
/// represent them: each cut its level at its position.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum BracketError {
    /// An open bracket whose close never came.
    #[error("Unclosed {}", .0.item.0)]
    UnmatchedOpen(WithSpan<OpenBracket>),
    /// A close bracket no enclosing group owns.
    #[error("Unexpected {}", .0.item.0)]
    UnmatchedClose(WithSpan<CloseBracket>),
}

type TokenStream = SafePeekable<std::vec::IntoIter<WithSpan<IsographLangTokenKind>>>;

/// Line breaks at the start of the items an opening leads are captured by that opening:
/// dropped as insignificant whitespace, like the spaces the tokenizer skips. The
/// literal's start always leads its items, and a group's opening leads them once the
/// group closes; an unclosed group yields nothing, so no other case exists.
fn strip_captured_line_breaks(items: &mut Vec<WithSpan<BracketItem>>) {
    let captured = items
        .iter()
        .position(|item| {
            !matches!(
                item.item,
                BracketItem::Raw(NonBracketToken(NonBracketTokenKind::LineBreak))
            )
        })
        .unwrap_or(items.len());
    items.drain(..captured);
}

/// The root's span is the whole literal, leading and trailing whitespace included, which
/// the tokens alone do not record; hence the length parameter.
pub(crate) fn match_brackets(
    tokens: Vec<WithSpan<IsographLangTokenKind>>,
    literal_length: u32,
) -> (WithSpan<MatchedBrackets>, Vec<BracketError>) {
    let mut tokens = tokens.into_iter().safe_peekable();
    // The kind of every group the level being parsed sits inside, innermost last. The
    // stack exists to classify a close that does not close the innermost group: in
    // `foo { bar ) }`, no enclosing group is a parenthesis, so the `)` is unmatched,
    // while a brace is on the stack, so the `}` closes the group.
    let mut enclosing_stack = Stack::new();
    let mut errors = Vec::new();
    let mut items = parse_bracket_items(&mut tokens, &mut enclosing_stack, &mut |e| errors.push(e));
    strip_captured_line_breaks(&mut items);
    errors.sort_by_key(|error| match error {
        BracketError::UnmatchedOpen(open) => open.location.start,
        BracketError::UnmatchedClose(close) => close.location.start,
    });
    (
        MatchedBrackets(items).with_span(Span::new(0, literal_length)),
        errors,
    )
}

/// What parsing a group produced. An unclosed group records its error inside
/// `parse_bracketed` and yields nothing: its opening and everything it would have
/// contained are the cut of the caller's level.
enum ParsedGroup {
    Closed(Bracketed),
    Unclosed,
}

/// Whether a level is still emitting items, or was cut at its first unmatched bracket:
/// from `Cut` on, tokens are consumed and diagnosed but produce nothing.
enum Emission {
    Emitting,
    Cut,
}

fn parse_bracket_items(
    tokens: &mut TokenStream,
    enclosing_stack: &mut Stack<BracketKind>,
    emit: &mut impl FnMut(BracketError),
) -> Vec<WithSpan<BracketItem>> {
    let mut items = Vec::new();
    let mut emission = Emission::Emitting;
    while let Some(peek) = tokens.peek() {
        match SplitToken::from(peek.view().item) {
            SplitToken::NonBracket(kind) => {
                let token = peek.commit();
                if let Emission::Emitting = emission {
                    items.push(BracketItem::Raw(NonBracketToken(kind)).with_span(token.location));
                }
            }
            SplitToken::Bracket(BracketToken::Open(kind)) => {
                let token = peek.commit();
                let opening = OpenBracket(kind).with_span(token.location);
                match parse_bracketed(tokens, enclosing_stack, emit, opening) {
                    ParsedGroup::Closed(group) => {
                        if let Emission::Emitting = emission {
                            let span = Span::join(group.opening.location, group.closing.location);
                            items.push(BracketItem::Bracketed(group).with_span(span));
                        }
                    }
                    ParsedGroup::Unclosed => {
                        emission = Emission::Cut;
                    }
                }
            }
            SplitToken::Bracket(BracketToken::Close(kind)) => {
                if enclosing_stack.all().contains(kind.reference()) {
                    // Some enclosing group owns this close. Dropping the peek leaves it
                    // unconsumed for its owner; every group between here and the owner
                    // reports unclosed.
                    break;
                }
                let token = peek.commit();
                emit(BracketError::UnmatchedClose(
                    CloseBracket(kind).with_span(token.location),
                ));
                emission = Emission::Cut;
            }
        }
    }
    items
}

/// One group, whose opening the caller already consumed. Its own close closes it; at a
/// close an enclosing group owns, or at the end of the tokens, it never closes, and it
/// yields nothing but its error.
fn parse_bracketed(
    tokens: &mut TokenStream,
    enclosing_stack: &mut Stack<BracketKind>,
    emit: &mut impl FnMut(BracketError),
    opening: WithSpan<OpenBracket>,
) -> ParsedGroup {
    let mut children = enclosing_stack.with_pushed(opening.item.0, |enclosing_stack| {
        parse_bracket_items(tokens, enclosing_stack, emit)
    });
    match tokens.peek() {
        Some(peek)
            if SplitToken::from(peek.view().item)
                == SplitToken::Bracket(BracketToken::Close(opening.item.0)) =>
        {
            let token = peek.commit();
            let closing = CloseBracket(opening.item.0).with_span(token.location);
            let interior = Span::between(opening.location, closing.location);
            strip_captured_line_breaks(&mut children);
            ParsedGroup::Closed(Bracketed {
                opening,
                children: MatchedBrackets(children).with_span(interior),
                closing,
            })
        }
        _ => {
            emit(BracketError::UnmatchedOpen(opening));
            ParsedGroup::Unclosed
        }
    }
}

#[cfg(test)]
mod tests {
    use prelude::Postfix;

    use super::*;
    use crate::{parsed_items::span_of, tokenize};
    use BracketKind::{Brace, Parenthesis};

    fn tree(literal: &str) -> (WithSpan<MatchedBrackets>, Vec<BracketError>) {
        match_brackets(tokenize(literal), literal.len() as u32)
    }

    /// The tree of a fixture that must produce no bracket errors; failing loudly here
    /// keeps a well-formed test from silently exercising a cut tree.
    fn well_formed(literal: &str) -> WithSpan<MatchedBrackets> {
        let (tree, errors) = tree(literal);
        assert_eq!(errors, vec![]);
        tree
    }

    fn raw(items: &[WithSpan<BracketItem>], index: usize) -> NonBracketToken {
        match items[index].item.reference() {
            BracketItem::Raw(token) => *token,
            item => panic!("expected a raw token at {index}, got {item:?}"),
        }
    }

    fn group(items: &[WithSpan<BracketItem>], index: usize) -> &Bracketed {
        match items[index].item.reference() {
            BracketItem::Bracketed(group) => group,
            item => panic!("expected a group at {index}, got {item:?}"),
        }
    }

    #[test]
    fn unmatched_open_displays_the_kind() {
        let err =
            BracketError::UnmatchedOpen(OpenBracket(BracketKind::Brace).with_span(Span::new(0, 1)));
        assert_eq!(err.to_string(), "Unclosed '{'");
    }

    #[test]
    fn unmatched_close_displays_the_kind() {
        let err = BracketError::UnmatchedClose(
            CloseBracket(BracketKind::Parenthesis).with_span(Span::new(0, 1)),
        );
        assert_eq!(err.to_string(), "Unexpected '('");
    }

    #[test]
    fn text_outside_any_bracket_is_raw_items() {
        let tree = well_formed("field Query.Foo");
        assert_eq!(tree.item.0.len(), 4);
        for index in 0..4 {
            raw(tree.item.0.reference(), index);
        }
    }

    #[test]
    fn balanced_input_nests_as_typed() {
        let text = "field Query.Foo { bar(arg: [1, 2]) { id } }";
        let tree = well_formed(text);
        let brace = group(tree.item.0.reference(), 4);
        assert_eq!(brace.opening.item.0, Brace);
        let brace_anchor = span_of(text, "{ bar");
        assert_eq!(
            brace.opening.location,
            Span::new(brace_anchor.start, brace_anchor.start + 1)
        );
        let parenthesis = group(brace.children.item.0.reference(), 1);
        assert_eq!(parenthesis.opening.item.0, Parenthesis);
        let square = group(parenthesis.children.item.0.reference(), 2);
        assert_eq!(square.opening.item.0, BracketKind::Bracket);
    }

    #[test]
    fn a_stray_close_is_a_raw_item_inside_the_brace() {
        let text = "{ foo, bar) }";
        let (tree, errors) = tree(text);
        assert_eq!(tree.item.0.len(), 1);
        let brace = group(tree.item.0.reference(), 0);
        assert_eq!(brace.opening.item.0, Brace);
        assert_eq!(brace.opening.location, span_of(text, "{"));
        assert_eq!(brace.closing.item.0, Brace);
        assert_eq!(brace.closing.location, span_of(text, "}"));
        assert_eq!(brace.children.item.0.len(), 3);
        assert_eq!(
            raw(brace.children.item.0.reference(), 0),
            NonBracketToken(NonBracketTokenKind::Identifier)
        );
        assert_eq!(brace.children.item.0[0].location, span_of(text, "foo"));
        assert_eq!(
            raw(brace.children.item.0.reference(), 1),
            NonBracketToken(NonBracketTokenKind::Comma)
        );
        assert_eq!(brace.children.item.0[1].location, span_of(text, ","));
        assert_eq!(
            raw(brace.children.item.0.reference(), 2),
            NonBracketToken(NonBracketTokenKind::Identifier)
        );
        assert_eq!(brace.children.item.0[2].location, span_of(text, "bar"));
        match errors.as_slice() {
            [BracketError::UnmatchedClose(close)] => {
                assert_eq!(close.item.0, Parenthesis);
                assert_eq!(close.location, span_of(text, ")"));
            }
            errors => panic!("expected exactly the unmatched close, got {errors:?}"),
        }
    }

    #[test]
    fn an_unclosed_open_is_a_raw_item_inside_the_brace() {
        let text = "foo { ( }";
        let (tree, errors) = tree(text);
        assert_eq!(tree.item.0.len(), 2);
        assert_eq!(
            raw(tree.item.0.reference(), 0),
            NonBracketToken(NonBracketTokenKind::Identifier)
        );
        assert_eq!(tree.item.0[0].location, span_of(text, "foo"));
        let brace = group(tree.item.0.reference(), 1);
        assert_eq!(brace.opening.item.0, Brace);
        assert_eq!(brace.opening.location, span_of(text, "{"));
        assert_eq!(brace.closing.item.0, Brace);
        assert_eq!(brace.closing.location, span_of(text, "}"));
        assert_eq!(brace.children.item.0.len(), 0);
        match errors.as_slice() {
            [BracketError::UnmatchedOpen(open)] => {
                assert_eq!(open.item.0, Parenthesis);
                assert_eq!(open.location, span_of(text, "("));
            }
            errors => panic!("expected exactly the unmatched open, got {errors:?}"),
        }
    }

    #[test]
    fn crossing_junk_leaks_past_the_early_close() {
        let text = "foo { (} )";
        let (tree, errors) = tree(text);
        assert_eq!(tree.item.0.len(), 2);
        assert_eq!(
            raw(tree.item.0.reference(), 0),
            NonBracketToken(NonBracketTokenKind::Identifier)
        );
        assert_eq!(tree.item.0[0].location, span_of(text, "foo"));
        let brace = group(tree.item.0.reference(), 1);
        assert_eq!(brace.opening.item.0, Brace);
        assert_eq!(brace.opening.location, span_of(text, "{"));
        assert_eq!(brace.closing.item.0, Brace);
        assert_eq!(brace.closing.location, span_of(text, "}"));
        assert_eq!(brace.children.item.0.len(), 0);
        match errors.as_slice() {
            [
                BracketError::UnmatchedOpen(open),
                BracketError::UnmatchedClose(close),
            ] => {
                assert_eq!(open.item.0, Parenthesis);
                assert_eq!(open.location, span_of(text, "("));
                assert_eq!(close.item.0, Parenthesis);
                assert_eq!(close.location, span_of(text, ")"));
            }
            errors => panic!("expected unmatched open then unmatched close, got {errors:?}"),
        }
    }

    #[test]
    fn an_extra_close_after_the_balanced_brace_is_raw_at_the_top() {
        let text = "foo { ( } }";
        let (tree, errors) = tree(text);
        assert_eq!(tree.item.0.len(), 2);
        assert_eq!(
            raw(tree.item.0.reference(), 0),
            NonBracketToken(NonBracketTokenKind::Identifier)
        );
        assert_eq!(tree.item.0[0].location, span_of(text, "foo"));
        let brace = group(tree.item.0.reference(), 1);
        assert_eq!(brace.opening.item.0, Brace);
        assert_eq!(brace.children.item.0.len(), 0);
        match errors.as_slice() {
            [
                BracketError::UnmatchedOpen(open),
                BracketError::UnmatchedClose(close),
            ] => {
                assert_eq!(open.item.0, Parenthesis);
                assert_eq!(open.location, span_of(text, "("));
                assert_eq!(close.item.0, Brace);
                let tail = span_of(text, "} }");
                assert_eq!(close.location, Span::new(tail.end - 1, tail.end));
            }
            errors => panic!("expected unmatched open then unmatched close, got {errors:?}"),
        }
    }

    #[test]
    fn an_unclosed_open_inside_a_matched_brace_is_the_only_error() {
        let text = "foo { bar(a: }";
        let (tree, errors) = tree(text);
        assert_eq!(tree.item.0.len(), 2);
        assert_eq!(
            raw(tree.item.0.reference(), 0),
            NonBracketToken(NonBracketTokenKind::Identifier)
        );
        assert_eq!(tree.item.0[0].location, span_of(text, "foo"));
        let brace = group(tree.item.0.reference(), 1);
        assert_eq!(brace.opening.item.0, Brace);
        assert_eq!(brace.opening.location, span_of(text, "{"));
        assert_eq!(brace.closing.item.0, Brace);
        assert_eq!(brace.closing.location, span_of(text, "}"));
        assert_eq!(brace.children.item.0.len(), 1);
        assert_eq!(
            raw(brace.children.item.0.reference(), 0),
            NonBracketToken(NonBracketTokenKind::Identifier)
        );
        assert_eq!(brace.children.item.0[0].location, span_of(text, "bar"));
        match errors.as_slice() {
            [BracketError::UnmatchedOpen(open)] => {
                assert_eq!(open.item.0, Parenthesis);
                assert_eq!(open.location, span_of(text, "("));
            }
            errors => panic!("expected exactly the unmatched open, got {errors:?}"),
        }
    }

    #[test]
    fn the_close_pairs_with_the_nearest_open() {
        let text = "a { b { c }";
        let (tree, errors) = tree(text);
        assert_eq!(tree.item.0.len(), 1);
        assert_eq!(
            raw(tree.item.0.reference(), 0),
            NonBracketToken(NonBracketTokenKind::Identifier)
        );
        assert_eq!(tree.item.0[0].location, span_of(text, "a"));
        match errors.as_slice() {
            [BracketError::UnmatchedOpen(open)] => {
                assert_eq!(open.item.0, Brace);
                let open_anchor = span_of(text, "{ b");
                assert_eq!(
                    open.location,
                    Span::new(open_anchor.start, open_anchor.start + 1)
                );
            }
            errors => panic!("expected exactly the unmatched open, got {errors:?}"),
        }
    }

    #[test]
    fn brackets_inside_strings_are_not_structural() {
        let tree = well_formed("{ name: \"a}\" }");
        let brace = group(tree.item.0.reference(), 0);
        assert_eq!(brace.children.item.0.len(), 3);
    }

    #[test]
    fn a_wrong_kind_close_inside_a_matched_pair_is_raw() {
        let text = "( } )";
        let (tree, errors) = tree(text);
        assert_eq!(tree.item.0.len(), 1);
        let parenthesis = group(tree.item.0.reference(), 0);
        assert_eq!(parenthesis.opening.item.0, Parenthesis);
        assert_eq!(parenthesis.opening.location, span_of(text, "("));
        assert_eq!(parenthesis.closing.item.0, Parenthesis);
        assert_eq!(parenthesis.closing.location, span_of(text, ")"));
        assert_eq!(parenthesis.children.item.0.len(), 0);
        match errors.as_slice() {
            [BracketError::UnmatchedClose(close)] => {
                assert_eq!(close.item.0, Brace);
                assert_eq!(close.location, span_of(text, "}"));
            }
            errors => panic!("expected exactly the unmatched close, got {errors:?}"),
        }
    }

    #[test]
    fn spans_cover_the_literal_and_each_interior() {
        let text = "  { a }  ";
        let tree = well_formed(text);
        assert_eq!(tree.location, Span::from_usize(0, text.len()));
        let brace = group(tree.item.0.reference(), 0);
        assert_eq!(
            brace.children.location,
            Span::new(span_of(text, "{").end, span_of(text, "}").start)
        );
    }

    #[test]
    fn a_closed_groups_opening_captures_the_line_breaks_after_it() {
        let text = "foo {\n\n bar\n}";
        let tree = well_formed(text);
        let brace = group(tree.item.0.reference(), 1);
        assert_eq!(brace.children.item.0.len(), 2);
        assert_eq!(
            raw(brace.children.item.0.reference(), 0),
            NonBracketToken(NonBracketTokenKind::Identifier)
        );
        assert_eq!(
            raw(brace.children.item.0.reference(), 1),
            NonBracketToken(NonBracketTokenKind::LineBreak)
        );
    }

    #[test]
    fn the_literal_start_captures_its_line_breaks() {
        let text = "\n\nfoo";
        let tree = well_formed(text);
        assert_eq!(tree.item.0.len(), 1);
        assert_eq!(
            raw(tree.item.0.reference(), 0),
            NonBracketToken(NonBracketTokenKind::Identifier)
        );
        assert_eq!(tree.location, Span::from_usize(0, text.len()));
    }
}
