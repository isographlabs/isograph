use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use safe_peekable::{IntoSafePeekable, SafePeekable};
use scoped_stack::Stack;
use span::{Span, WithSpan};

use crate::{BracketKind, BracketToken, IsographLangTokenKind, NonBracketTokenKind, SplitToken};

/// One level: the whole literal at the root, a group's interior below.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = MatchedBracketsParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct MatchedBrackets(#[resolve_field] pub Vec<WithSpan<BracketItem>>);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = MatchedBracketsPath<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub enum BracketItem {
    Raw(RawToken),
    Bracketed(Bracketed),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = MatchedBracketsPath<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct Bracketed {
    #[resolve_field(parent_variant = Matched)]
    pub opening: WithSpan<OpenBracket>,
    /// The wrapping `WithSpan`'s span runs from the opening's end to the closing's start.
    #[resolve_field(parent_variant = Interior)]
    pub children: WithSpan<MatchedBrackets>,
    #[resolve_field(parent_variant = Matched)]
    pub closing: WithSpan<CloseBracket>,
}

/// A token that is not part of any structure; matched brackets are structure, never
/// raw, so a bracket token here is unmatched.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = MatchedBracketsPath<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub enum RawToken {
    NonBracket(NonBracketToken),
    Open(#[resolve_field(parent_variant = Unmatched)] OpenBracket),
    Close(#[resolve_field(parent_variant = Unmatched)] CloseBracket),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = MatchedBracketsPath<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct NonBracketToken(pub NonBracketTokenKind);

/// An opening bracket; its parent says whether it is a group's own opening or
/// unmatched in a level.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = BracketTokenParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct OpenBracket(pub BracketKind);

/// A closing bracket; its parent says whether it is a group's own closing or
/// unmatched in a level.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = BracketTokenParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct CloseBracket(pub BracketKind);

#[derive(Debug)]
pub enum ResolvedBracketNode<'a> {
    MatchedBrackets(MatchedBracketsPath<'a>),
    /// This will be resolved for spans that contains one of the opening/closing brace
    /// and part of the inside, e.g. "{ ba" in "foo { bar }". Single-character spans
    /// will never resolve to this.
    Bracketed(BracketedPath<'a>),
    NonBracketToken(NonBracketTokenPath<'a>),
    OpenBracket(OpenBracketPath<'a>),
    CloseBracket(CloseBracketPath<'a>),
}

#[derive(Debug)]
pub enum MatchedBracketsParent<'a> {
    Root,
    Interior(Box<BracketedPath<'a>>),
}

pub type MatchedBracketsPath<'a> =
    PositionResolutionPath<&'a MatchedBrackets, MatchedBracketsParent<'a>>;

pub type BracketedPath<'a> = PositionResolutionPath<&'a Bracketed, MatchedBracketsPath<'a>>;

pub type NonBracketTokenPath<'a> =
    PositionResolutionPath<&'a NonBracketToken, MatchedBracketsPath<'a>>;

/// Shared by `OpenBracket` and `CloseBracket`: a bracket token is a group's own
/// opening or closing, or unmatched and raw in a level.
#[derive(Debug)]
pub enum BracketTokenParent<'a> {
    Matched(BracketedPath<'a>),
    Unmatched(MatchedBracketsPath<'a>),
}

pub type OpenBracketPath<'a> = PositionResolutionPath<&'a OpenBracket, BracketTokenParent<'a>>;
pub type CloseBracketPath<'a> = PositionResolutionPath<&'a CloseBracket, BracketTokenParent<'a>>;

#[derive(Debug, PartialEq, Eq)]
pub enum BracketError {
    /// An open bracket that sits raw in a level: its group never closed.
    UnmatchedOpen(WithSpan<OpenBracket>),
    /// A close bracket that sits raw in a level: no open of its kind was waiting.
    UnmatchedClose(WithSpan<CloseBracket>),
}

impl MatchedBrackets {
    /// Every unmatched bracket under this level, in source order.
    pub fn errors(&self) -> Vec<BracketError> {
        let mut errors = Vec::new();
        collect_errors(self, &mut errors);
        errors
    }
}

fn collect_errors(level: &MatchedBrackets, errors: &mut Vec<BracketError>) {
    for item in &level.0 {
        match &item.item {
            BracketItem::Raw(RawToken::NonBracket(_)) => {}
            BracketItem::Raw(RawToken::Open(open)) => {
                errors.push(BracketError::UnmatchedOpen(WithSpan::new(
                    *open,
                    item.location,
                )));
            }
            BracketItem::Raw(RawToken::Close(close)) => {
                errors.push(BracketError::UnmatchedClose(WithSpan::new(
                    *close,
                    item.location,
                )));
            }
            BracketItem::Bracketed(group) => collect_errors(&group.children.item, errors),
        }
    }
}

type TokenStream = SafePeekable<std::vec::IntoIter<WithSpan<IsographLangTokenKind>>>;

/// The root's span is the whole literal, leading and trailing whitespace included, which
/// the tokens alone do not record; hence the length parameter.
pub fn match_brackets(
    tokens: Vec<WithSpan<IsographLangTokenKind>>,
    literal_length: u32,
) -> WithSpan<MatchedBrackets> {
    let mut tokens = tokens.into_iter().safe_peekable();
    // The kind of every group the level being parsed sits inside, innermost last. The
    // stack exists to classify a close that does not close the innermost group: in
    // `foo { bar ) }`, no enclosing group is a parenthesis, so the `)` is a stray raw
    // token, while a brace is on the stack, so the `}` closes the group.
    let mut enclosing_stack = Stack::new();
    WithSpan::new(
        MatchedBrackets(parse_items(&mut tokens, &mut enclosing_stack)),
        Span::new(0, literal_length),
    )
}

/// What parsing a group produced: the group closed for real, or it never got its close,
/// in which case the caller stores the opening as a raw item and the children as its
/// siblings.
enum ParsedGroup {
    Closed(Bracketed),
    Unclosed(UnclosedGroup),
}

struct UnclosedGroup {
    opening: WithSpan<OpenBracket>,
    children: Vec<WithSpan<BracketItem>>,
}

fn parse_items(
    tokens: &mut TokenStream,
    enclosing_stack: &mut Stack<BracketKind>,
) -> Vec<WithSpan<BracketItem>> {
    let mut items = Vec::new();
    while let Some(peek) = tokens.peek() {
        let token = *peek.view();
        match SplitToken::from(token.item) {
            SplitToken::NonBracket(kind) => {
                peek.commit();
                items.push(WithSpan::new(
                    BracketItem::Raw(RawToken::NonBracket(NonBracketToken(kind))),
                    token.location,
                ));
            }
            SplitToken::Bracket(BracketToken::Open(kind)) => {
                peek.commit();
                let opening = WithSpan::new(OpenBracket(kind), token.location);
                match parse_bracketed(tokens, enclosing_stack, opening) {
                    ParsedGroup::Closed(group) => {
                        let span =
                            Span::join(group.opening.location, group.closing.location);
                        items.push(WithSpan::new(BracketItem::Bracketed(group), span));
                    }
                    ParsedGroup::Unclosed(UnclosedGroup { opening, children }) => {
                        items.push(WithSpan::new(
                            BracketItem::Raw(RawToken::Open(opening.item)),
                            opening.location,
                        ));
                        items.extend(children);
                    }
                }
            }
            SplitToken::Bracket(BracketToken::Close(kind)) => {
                if enclosing_stack.all().contains(&kind) {
                    // Some enclosing group owns this close. Dropping the peek leaves it
                    // unconsumed, which is what takes apart every group between here and
                    // its owner.
                    break;
                }
                peek.commit();
                items.push(WithSpan::new(
                    BracketItem::Raw(RawToken::Close(CloseBracket(kind))),
                    token.location,
                ));
            }
        }
    }
    items
}

/// One group, whose opening the caller already consumed. Its own close closes it; at a
/// close an enclosing group owns, or at the end of the tokens, it never closes.
fn parse_bracketed(
    tokens: &mut TokenStream,
    enclosing_stack: &mut Stack<BracketKind>,
    opening: WithSpan<OpenBracket>,
) -> ParsedGroup {
    let children = enclosing_stack.with_pushed(opening.item.0, |enclosing_stack| {
        parse_items(tokens, enclosing_stack)
    });

    // parse_items stopped at this group's own close, at a close an enclosing group
    // owns, or at the end of the tokens; only the first is consumed. In `foo { ( }`,
    // the paren's items stop at the `}` because the brace owns it, and the paren
    // refusing it here leaves it in the stream for the brace, which consumes it one
    // level up as its own. A close is consumed only by the group it closes. Dropping
    // the peek leaves the stopping close for its owner.
    match tokens.peek() {
        Some(peek)
            if SplitToken::from(peek.view().item)
                == SplitToken::Bracket(BracketToken::Close(opening.item.0)) =>
        {
            let token = peek.commit();
            let closing = WithSpan::new(CloseBracket(opening.item.0), token.location);
            let interior = Span::between(opening.location, closing.location);
            ParsedGroup::Closed(Bracketed {
                opening,
                children: WithSpan::new(MatchedBrackets(children), interior),
                closing,
            })
        }
        _ => ParsedGroup::Unclosed(UnclosedGroup { opening, children }),
    }
}

#[cfg(test)]
mod tests {
    use resolve_position::ResolvePosition;

    use super::*;
    use crate::tokenize;
    use BracketKind::{Brace, Parenthesis};

    fn tree(literal: &str) -> WithSpan<MatchedBrackets> {
        match_brackets(tokenize(literal), literal.len() as u32)
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

    fn raw(items: &[WithSpan<BracketItem>], index: usize) -> RawToken {
        match &items[index].item {
            BracketItem::Raw(token) => *token,
            item => panic!("expected a raw token at {index}, got {item:?}"),
        }
    }

    fn group(items: &[WithSpan<BracketItem>], index: usize) -> &Bracketed {
        match &items[index].item {
            BracketItem::Bracketed(group) => group,
            item => panic!("expected a group at {index}, got {item:?}"),
        }
    }

    #[test]
    fn text_outside_any_bracket_is_raw_items() {
        let tree = tree("field Query.Foo");
        assert_eq!(tree.item.0.len(), 4);
        for index in 0..4 {
            assert!(matches!(raw(&tree.item.0, index), RawToken::NonBracket(_)));
        }
        assert_eq!(tree.item.errors(), vec![]);
    }

    #[test]
    fn balanced_input_nests_as_typed() {
        let text = "field Query.Foo { bar(arg: [1, 2]) { id } }";
        let tree = tree(text);
        let brace = group(&tree.item.0, 4);
        assert_eq!(brace.opening.item.0, Brace);
        let brace_anchor = span_of(text, "{ bar");
        assert_eq!(
            brace.opening.location,
            Span::new(brace_anchor.start, brace_anchor.start + 1)
        );
        let parenthesis = group(&brace.children.item.0, 1);
        assert_eq!(parenthesis.opening.item.0, Parenthesis);
        let square = group(&parenthesis.children.item.0, 2);
        assert_eq!(square.opening.item.0, BracketKind::Bracket);
        assert_eq!(tree.item.errors(), vec![]);
    }

    #[test]
    fn a_stray_close_is_a_raw_item_inside_the_brace() {
        let text = "foo { ) }";
        let tree = tree(text);
        let brace = group(&tree.item.0, 1);
        assert_eq!(
            raw(&brace.children.item.0, 0),
            RawToken::Close(CloseBracket(Parenthesis))
        );
        match tree.item.errors().as_slice() {
            [BracketError::UnmatchedClose(close)] => {
                assert_eq!(close.location, span_of(text, ")"));
            }
            errors => panic!("expected exactly the unmatched close, got {errors:?}"),
        }
    }

    #[test]
    fn an_unclosed_open_is_a_raw_item_inside_the_brace() {
        let text = "foo { ( }";
        let tree = tree(text);
        let brace = group(&tree.item.0, 1);
        assert_eq!(
            raw(&brace.children.item.0, 0),
            RawToken::Open(OpenBracket(Parenthesis))
        );
        match tree.item.errors().as_slice() {
            [BracketError::UnmatchedOpen(open)] => {
                assert_eq!(open.location, span_of(text, "("));
            }
            errors => panic!("expected exactly the unmatched open, got {errors:?}"),
        }
    }

    #[test]
    fn crossing_junk_leaks_past_the_early_close() {
        let text = "foo { (}) }";
        let tree = tree(text);
        assert_eq!(tree.item.0.len(), 4);
        let brace = group(&tree.item.0, 1);
        assert_eq!(
            raw(&brace.children.item.0, 0),
            RawToken::Open(OpenBracket(Parenthesis))
        );
        assert_eq!(raw(&tree.item.0, 2), RawToken::Close(CloseBracket(Parenthesis)));
        assert_eq!(raw(&tree.item.0, 3), RawToken::Close(CloseBracket(Brace)));
        match tree.item.errors().as_slice() {
            [
                BracketError::UnmatchedOpen(open),
                BracketError::UnmatchedClose(parenthesis),
                BracketError::UnmatchedClose(brace_close),
            ] => {
                assert_eq!(open.location, span_of(text, "("));
                assert_eq!(parenthesis.location, span_of(text, ")"));
                let tail = span_of(text, ") }");
                assert_eq!(brace_close.location, Span::new(tail.end - 1, tail.end));
            }
            errors => panic!("expected three unmatched brackets, got {errors:?}"),
        }
    }

    #[test]
    fn an_extra_close_after_the_balanced_brace_is_raw_at_the_top() {
        let text = "foo { ( } }";
        let tree = tree(text);
        assert_eq!(tree.item.0.len(), 3);
        let brace = group(&tree.item.0, 1);
        assert_eq!(
            raw(&brace.children.item.0, 0),
            RawToken::Open(OpenBracket(Parenthesis))
        );
        assert_eq!(raw(&tree.item.0, 2), RawToken::Close(CloseBracket(Brace)));
    }

    #[test]
    fn an_unclosed_open_inside_a_matched_brace_is_the_only_error() {
        let text = "foo { bar(a: }";
        let tree = tree(text);
        assert_eq!(tree.item.0.len(), 2);
        let brace = group(&tree.item.0, 1);
        assert_eq!(brace.opening.item.0, Brace);
        assert_eq!(
            raw(&brace.children.item.0, 1),
            RawToken::Open(OpenBracket(Parenthesis))
        );
        match tree.item.errors().as_slice() {
            [BracketError::UnmatchedOpen(open)] => {
                assert_eq!(open.location, span_of(text, "("));
            }
            errors => panic!("expected exactly the unmatched open, got {errors:?}"),
        }
    }

    #[test]
    fn the_close_pairs_with_the_nearest_open() {
        let text = "a { b { c }";
        let tree = tree(text);
        assert_eq!(tree.item.0.len(), 4);
        assert_eq!(raw(&tree.item.0, 1), RawToken::Open(OpenBracket(Brace)));
        let inner = group(&tree.item.0, 3);
        assert!(matches!(
            raw(&inner.children.item.0, 0),
            RawToken::NonBracket(_)
        ));
        match tree.item.errors().as_slice() {
            [BracketError::UnmatchedOpen(open)] => {
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
        let tree = tree("{ name: \"a}\" }");
        let brace = group(&tree.item.0, 0);
        assert_eq!(brace.children.item.0.len(), 3);
        assert_eq!(tree.item.errors(), vec![]);
    }

    #[test]
    fn a_wrong_kind_close_inside_a_matched_pair_is_raw() {
        let text = "( } )";
        let tree = tree(text);
        let parenthesis = group(&tree.item.0, 0);
        assert_eq!(
            raw(&parenthesis.children.item.0, 0),
            RawToken::Close(CloseBracket(Brace))
        );
        match tree.item.errors().as_slice() {
            [BracketError::UnmatchedClose(close)] => {
                assert_eq!(close.location, span_of(text, "}"));
            }
            errors => panic!("expected exactly the unmatched close, got {errors:?}"),
        }
    }

    #[test]
    fn spans_cover_the_literal_and_each_interior() {
        let text = "  { a }  ";
        let tree = tree(text);
        assert_eq!(tree.location, Span::from_usize(0, text.len()));
        let brace = group(&tree.item.0, 0);
        assert_eq!(
            brace.children.location,
            Span::new(span_of(text, "{").end, span_of(text, "}").start)
        );
    }

    #[test]
    fn an_unmatched_open_resolves_with_the_level_as_parent() {
        let text = "foo { ( }";
        let tree = tree(text);
        match tree.resolve(MatchedBracketsParent::Root, span_of(text, "(")) {
            ResolvedBracketNode::OpenBracket(open) => {
                assert_eq!(open.inner.0, Parenthesis);
                match open.parent {
                    BracketTokenParent::Unmatched(level) => {
                        assert!(matches!(
                            level.parent,
                            MatchedBracketsParent::Interior(_)
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
        let tree = tree(text);
        match tree.resolve(MatchedBracketsParent::Root, span_of(text, "}")) {
            ResolvedBracketNode::CloseBracket(close) => {
                assert_eq!(close.inner.0, Brace);
                match close.parent {
                    BracketTokenParent::Unmatched(level) => {
                        assert!(matches!(level.parent, MatchedBracketsParent::Root));
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
        let tree = tree(text);
        match tree.resolve(MatchedBracketsParent::Root, span_of(text, "{")) {
            ResolvedBracketNode::OpenBracket(open) => match open.parent {
                BracketTokenParent::Matched(group) => {
                    assert_eq!(group.inner.closing.item.0, Brace);
                }
                parent => panic!("expected a matched open, got {parent:?}"),
            },
            node => panic!("expected the open bracket leaf, got {node:?}"),
        }
        match tree.resolve(MatchedBracketsParent::Root, span_of(text, "}")) {
            ResolvedBracketNode::CloseBracket(close) => match close.parent {
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
        let tree = tree(text);
        let straddle = Span::new(span_of(text, "{").start, span_of(text, "bar").end);
        match tree.resolve(MatchedBracketsParent::Root, straddle) {
            ResolvedBracketNode::Bracketed(group) => {
                assert_eq!(group.inner.opening.item.0, Brace);
            }
            node => panic!("expected the group leaf, got {node:?}"),
        }
    }

    #[test]
    fn an_ordinary_token_resolves_to_its_own_leaf() {
        let text = "foo { bar }";
        let tree = tree(text);
        match tree.resolve(MatchedBracketsParent::Root, span_of(text, "bar")) {
            ResolvedBracketNode::NonBracketToken(token) => {
                assert_eq!(token.inner.0, NonBracketTokenKind::Identifier);
                assert!(matches!(
                    token.parent.parent,
                    MatchedBracketsParent::Interior(_)
                ));
            }
            node => panic!("expected the token leaf, got {node:?}"),
        }
    }

    #[test]
    fn whitespace_resolves_to_its_level() {
        let text = "foo { bar }";
        let tree = tree(text);
        let gap = Span::new(span_of(text, "foo").end, span_of(text, "{").start);
        match tree.resolve(MatchedBracketsParent::Root, gap) {
            ResolvedBracketNode::MatchedBrackets(level) => {
                assert!(matches!(level.parent, MatchedBracketsParent::Root));
            }
            node => panic!("expected the root level, got {node:?}"),
        }
    }
}
