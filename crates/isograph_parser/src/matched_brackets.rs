use std::fmt;
use std::iter::Peekable;

use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan};

use crate::{BracketKind, BracketToken, IsographLangTokenKind, NonBracketTokenKind, SplitToken};

/// The types one matched-brackets tree holds: what a run between brackets is, and what the
/// two bracket errors carry. A pipeline stage is an implementor, and a pass that changes any
/// of these changes all of them at once, through `try_map` (refactors/past/error-refinement.md).
/// The slots carry these bounds so the tree types' derives compile.
pub trait TreeContents {
    /// What a run between brackets is: the `Inner` run of lexed tokens out of the matcher,
    /// parsed nodes later.
    type Inner: fmt::Debug + PartialEq + Eq;
    /// What a stray close carries: `CloseBracket` while bracket errors are representable,
    /// `Infallible` once refined.
    type StrayClose: fmt::Debug + PartialEq + Eq;
}

/// The stage `match_brackets` produces: its runs hold lexed tokens, and the tree can carry
/// both bracket errors.
#[derive(Debug, PartialEq, Eq)]
pub struct BracketsMatched;

impl TreeContents for BracketsMatched {
    type Inner = Inner;
    type StrayClose = CloseBracket;
}

/// A maximal run of non-bracket tokens between brackets.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = BracketItemParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct Inner(pub Vec<WithSpan<NonBracketTokenKind>>);

/// A group's opening bracket.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = OpenBracketParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct OpenBracket(pub BracketKind);

/// A close bracket token.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = BracketItemParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct CloseBracket(pub BracketKind);

/// One isograph literal with its brackets matched. Spans live on the `WithSpan` wrapping
/// each item.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = (),
    resolved_node = ResolvedBracketNode<'a>,
    self_type_generics = <BracketsMatched>
)]
pub struct MatchedBrackets<TContents: TreeContents>(
    #[resolve_field] pub Vec<WithSpan<BracketItem<TContents>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = BracketItemParent<'a>,
    resolved_node = ResolvedBracketNode<'a>,
    self_type_generics = <BracketsMatched>
)]
pub enum BracketItem<TContents: TreeContents> {
    Inner(TContents::Inner),
    Bracketed(Bracketed<TContents>),
    StrayClose(TContents::StrayClose),
}

/// An open bracket, its children, and its close. The wrapping `WithSpan`'s span runs from
/// the start of the opening to the end of a real closing, or to the end of the last child
/// when the closing is `None`.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = BracketItemParent<'a>,
    resolved_node = ResolvedBracketNode<'a>,
    self_type_generics = <BracketsMatched>
)]
pub struct Bracketed<TContents: TreeContents> {
    #[resolve_field]
    pub opening: WithSpan<OpenBracket>,
    #[resolve_field]
    pub children: Vec<WithSpan<BracketItem<TContents>>>,
    /// The close the author typed, or `None` for a group that never got its close and was
    /// forced to end: at the close bracket an enclosing group owns, or at the end of the
    /// tokens. A `None` group is an invalid section.
    #[resolve_field]
    pub closing: Option<WithSpan<CloseBracket>>,
}

/// Every node a position can resolve to while only brackets are matched. Once later passes
/// add their nodes, the full isograph path enum replaces this one. A position on whitespace
/// inside a group resolves to the group.
#[derive(Debug)]
pub enum ResolvedBracketNode<'a> {
    MatchedBrackets(MatchedBracketsPath<'a>),
    Bracketed(BracketedPath<'a>),
    Inner(InnerPath<'a>),
    OpenBracket(OpenBracketPath<'a>),
    CloseBracket(CloseBracketPath<'a>),
}

pub type MatchedBracketsPath<'a> = PositionResolutionPath<&'a MatchedBrackets<BracketsMatched>, ()>;

/// Everything a `BracketItem` can sit inside.
#[derive(Debug)]
pub enum BracketItemParent<'a> {
    MatchedBrackets(MatchedBracketsPath<'a>),
    Bracketed(Box<BracketedPath<'a>>),
}

pub type BracketedPath<'a> =
    PositionResolutionPath<&'a Bracketed<BracketsMatched>, BracketItemParent<'a>>;
pub type InnerPath<'a> = PositionResolutionPath<&'a Inner, BracketItemParent<'a>>;

/// The one place an opening bracket can sit: its group.
#[derive(Debug)]
pub enum OpenBracketParent<'a> {
    Bracketed(Box<BracketedPath<'a>>),
}

pub type OpenBracketPath<'a> = PositionResolutionPath<&'a OpenBracket, OpenBracketParent<'a>>;
pub type CloseBracketPath<'a> = PositionResolutionPath<&'a CloseBracket, BracketItemParent<'a>>;

#[derive(Debug, PartialEq, Eq)]
pub enum BracketError {
    /// A close bracket no open of its kind was waiting for.
    UnexpectedClose(WithSpan<CloseBracket>),
    /// A group that never got its close.
    Unclosed(WithSpan<UnclosedGroup>),
}

/// The opening bracket of a group that never got its close. The wrapping `WithSpan`'s span
/// is the whole group; its end is where the close should have been.
#[derive(Debug, PartialEq, Eq)]
pub struct UnclosedGroup(pub WithSpan<OpenBracket>);

impl<TContents> MatchedBrackets<TContents>
where
    TContents: TreeContents<StrayClose = CloseBracket>,
{
    /// Every error the pass produced, in source order of the position each error starts at.
    /// The list is empty iff every bracket matched.
    pub fn errors(&self) -> Vec<BracketError> {
        let mut errors = Vec::new();
        collect_errors(&self.0, &mut errors);
        errors
    }
}

fn collect_errors<TContents>(
    items: &[WithSpan<BracketItem<TContents>>],
    errors: &mut Vec<BracketError>,
) where
    TContents: TreeContents<StrayClose = CloseBracket>,
{
    for item in items {
        match &item.item {
            BracketItem::Inner(_) => {}
            BracketItem::StrayClose(stray) => {
                errors.push(BracketError::UnexpectedClose(WithSpan::new(
                    *stray,
                    item.location,
                )));
            }
            BracketItem::Bracketed(bracketed) => {
                if bracketed.closing.is_none() {
                    errors.push(BracketError::Unclosed(WithSpan::new(
                        UnclosedGroup(bracketed.opening),
                        item.location,
                    )));
                }
                collect_errors(&bracketed.children, errors);
            }
        }
    }
}

type TokenStream = Peekable<std::vec::IntoIter<WithSpan<IsographLangTokenKind>>>;

pub fn match_brackets(
    tokens: Vec<WithSpan<IsographLangTokenKind>>,
) -> MatchedBrackets<BracketsMatched> {
    let mut tokens = tokens.into_iter().peekable();
    let mut enclosing = Vec::new();
    let items = parse_items(&mut tokens, &mut enclosing);
    MatchedBrackets(items)
}

/// Parse items until a close bracket some enclosing group owns, or the end of the tokens.
///
/// `enclosing` is the kind of every group this level sits inside, innermost last, the group
/// being parsed included; it is how a close bracket with no open of its kind anywhere is
/// recognized as stray rather than left to end this level.
fn parse_items(
    tokens: &mut TokenStream,
    enclosing: &mut Vec<BracketKind>,
) -> Vec<WithSpan<BracketItem<BracketsMatched>>> {
    let mut items = Vec::new();
    let mut run: Vec<WithSpan<NonBracketTokenKind>> = Vec::new();
    while let Some(&token) = tokens.peek() {
        match SplitToken::from(token.item) {
            SplitToken::NonBracket(kind) => {
                tokens.next();
                run.push(WithSpan::new(kind, token.location));
            }
            SplitToken::Bracket(BracketToken::Open(kind)) => {
                flush_run(&mut items, &mut run);
                tokens.next();
                items.push(parse_bracketed(
                    tokens,
                    enclosing,
                    WithSpan::new(OpenBracket(kind), token.location),
                ));
            }
            SplitToken::Bracket(BracketToken::Close(kind)) => {
                if enclosing.contains(&kind) {
                    // Some enclosing group owns this close. Leaving it unconsumed is what
                    // synthetically closes every group between here and its owner.
                    break;
                }
                flush_run(&mut items, &mut run);
                tokens.next();
                items.push(WithSpan::new(
                    BracketItem::StrayClose(CloseBracket(kind)),
                    token.location,
                ));
            }
        }
    }
    flush_run(&mut items, &mut run);
    items
}

/// End the run in progress, if any, into one `Inner` item spanning its first token's start
/// to its last token's end.
fn flush_run(
    items: &mut Vec<WithSpan<BracketItem<BracketsMatched>>>,
    run: &mut Vec<WithSpan<NonBracketTokenKind>>,
) {
    let span = match (run.first(), run.last()) {
        (Some(first), Some(last)) => Span::join(first.location, last.location),
        _ => return,
    };
    items.push(WithSpan::new(
        BracketItem::Inner(Inner(std::mem::take(run))),
        span,
    ));
}

/// One group, whose opening the caller already consumed. This parses the children, then
/// looks at the one token that stopped them: the group's own close, which it consumes into
/// the closing, or a close an enclosing group owns, which it leaves alone while ending this
/// group where it stands.
fn parse_bracketed(
    tokens: &mut TokenStream,
    enclosing: &mut Vec<BracketKind>,
    opening: WithSpan<OpenBracket>,
) -> WithSpan<BracketItem<BracketsMatched>> {
    enclosing.push(opening.item.0);
    let children = parse_items(tokens, enclosing);
    enclosing.pop();

    let (closing, end) = match tokens.peek() {
        Some(&token)
            if SplitToken::from(token.item)
                == SplitToken::Bracket(BracketToken::Close(opening.item.0)) =>
        {
            tokens.next();
            (
                Some(WithSpan::new(CloseBracket(opening.item.0), token.location)),
                token.location.end,
            )
        }
        // The group was forced to end: at a close an enclosing group owns, or at the end
        // of the tokens. It ends after its last child, or right after the opening when
        // there is none.
        _ => (
            None,
            children
                .last()
                .map_or(opening.location.end, |last| last.location.end),
        ),
    };

    let span = Span::new(opening.location.start, end);
    WithSpan::new(
        BracketItem::Bracketed(Bracketed {
            opening,
            children,
            closing,
        }),
        span,
    )
}

impl<TFrom: TreeContents> MatchedBrackets<TFrom> {
    /// Cross the tree to another stage, mapping every slot fallibly. The result is either
    /// the whole crossed tree or every refusal, in source order.
    pub fn try_map<TTo: TreeContents, TError>(
        self,
        map_inner: &mut impl FnMut(WithSpan<TFrom::Inner>) -> Result<TTo::Inner, TError>,
        map_stray_close: &mut impl FnMut(WithSpan<TFrom::StrayClose>) -> Result<TTo::StrayClose, TError>,
    ) -> Result<MatchedBrackets<TTo>, Vec<TError>> {
        let mut errors = Vec::new();
        let items = try_map_items(self.0, &mut errors, map_inner, map_stray_close);
        if errors.is_empty() {
            Ok(MatchedBrackets(items))
        } else {
            Err(errors)
        }
    }
}

fn try_map_items<TFrom, TTo, TError>(
    items: Vec<WithSpan<BracketItem<TFrom>>>,
    errors: &mut Vec<TError>,
    map_inner: &mut impl FnMut(WithSpan<TFrom::Inner>) -> Result<TTo::Inner, TError>,
    map_stray_close: &mut impl FnMut(WithSpan<TFrom::StrayClose>) -> Result<TTo::StrayClose, TError>,
) -> Vec<WithSpan<BracketItem<TTo>>>
where
    TFrom: TreeContents,
    TTo: TreeContents,
{
    let mut mapped = Vec::new();
    for with_span in items {
        let WithSpan {
            item,
            location: span,
        } = with_span;
        match item {
            BracketItem::Inner(inner) => match map_inner(WithSpan::new(inner, span)) {
                Ok(inner) => mapped.push(WithSpan::new(BracketItem::Inner(inner), span)),
                Err(e) => errors.push(e),
            },
            BracketItem::StrayClose(stray) => match map_stray_close(WithSpan::new(stray, span)) {
                Ok(stray) => mapped.push(WithSpan::new(BracketItem::StrayClose(stray), span)),
                Err(e) => errors.push(e),
            },
            BracketItem::Bracketed(Bracketed {
                opening,
                children,
                closing,
            }) => {
                let children = try_map_items(children, errors, map_inner, map_stray_close);
                mapped.push(WithSpan::new(
                    BracketItem::Bracketed(Bracketed {
                        opening,
                        children,
                        closing,
                    }),
                    span,
                ));
            }
        }
    }
    mapped
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use resolve_position::ResolvePosition;

    use super::*;
    use crate::tokenize;
    use BracketKind::{Brace, Bracket, Parenthesis};

    fn tree(literal: &str) -> MatchedBrackets<BracketsMatched> {
        match_brackets(tokenize(literal))
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

    fn resolved<'a>(
        tree: &'a MatchedBrackets<BracketsMatched>,
        text: &str,
        pattern: &str,
    ) -> ResolvedBracketNode<'a> {
        tree.resolve((), span_of(text, pattern))
    }

    fn run(node: ResolvedBracketNode<'_>) -> InnerPath<'_> {
        match node {
            ResolvedBracketNode::Inner(run) => run,
            node => panic!("expected a run, got {node:?}"),
        }
    }

    fn open_bracket(node: ResolvedBracketNode<'_>) -> OpenBracketPath<'_> {
        match node {
            ResolvedBracketNode::OpenBracket(open) => open,
            node => panic!("expected an open bracket, got {node:?}"),
        }
    }

    fn close_bracket(node: ResolvedBracketNode<'_>) -> CloseBracketPath<'_> {
        match node {
            ResolvedBracketNode::CloseBracket(close) => close,
            node => panic!("expected a close bracket, got {node:?}"),
        }
    }

    fn group_leaf(node: ResolvedBracketNode<'_>) -> BracketedPath<'_> {
        match node {
            ResolvedBracketNode::Bracketed(group) => group,
            node => panic!("expected a group, got {node:?}"),
        }
    }

    fn enclosing_group(parent: BracketItemParent<'_>) -> BracketedPath<'_> {
        match parent {
            BracketItemParent::Bracketed(group) => *group,
            parent => panic!("expected an enclosing group, got {parent:?}"),
        }
    }

    fn assert_root(parent: BracketItemParent<'_>) {
        assert!(matches!(parent, BracketItemParent::MatchedBrackets(_)));
    }

    fn assert_balanced(group: &BracketedPath<'_>, kind: BracketKind) {
        assert_eq!(group.inner.opening.item.0, kind);
        assert!(group.inner.closing.is_some());
    }

    fn assert_unbalanced(group: &BracketedPath<'_>, kind: BracketKind) {
        assert_eq!(group.inner.opening.item.0, kind);
        assert!(group.inner.closing.is_none());
    }

    #[test]
    fn an_unbracketed_run_sits_at_the_top_level() {
        let text = "field Query.Foo";
        let tree = tree(text);
        assert_root(run(resolved(&tree, text, "Query")).parent);
        assert_eq!(tree.errors(), vec![]);
    }

    #[test]
    fn balanced_input_nests_as_typed() {
        let text = "field Query.Foo { bar(arg: [1, 2]) { id } }";
        let tree = tree(text);
        // `1` sits in the `[...]` inside the `(...)` inside the outer `{...}`.
        let square_group = enclosing_group(run(resolved(&tree, text, "1")).parent);
        assert_balanced(&square_group, Bracket);
        let parenthesis_group = enclosing_group(square_group.parent);
        assert_balanced(&parenthesis_group, Parenthesis);
        let brace_group = enclosing_group(parenthesis_group.parent);
        assert_balanced(&brace_group, Brace);
        assert_root(brace_group.parent);
        // `id` sits in the inner `{...}` inside the outer `{...}`.
        let inner_brace_group = enclosing_group(run(resolved(&tree, text, "id")).parent);
        assert_balanced(&inner_brace_group, Brace);
        let outer_brace_group = enclosing_group(inner_brace_group.parent);
        assert_balanced(&outer_brace_group, Brace);
        assert_root(outer_brace_group.parent);
        assert_eq!(tree.errors(), vec![]);
    }

    #[test]
    fn a_wrong_kind_close_leaves_only_the_paren_unbalanced() {
        let text = "field Query.Foo { bar( }";
        let tree = tree(text);
        let open = open_bracket(resolved(&tree, text, "("));
        assert_eq!(open.inner.0, Parenthesis);
        let OpenBracketParent::Bracketed(parenthesis_group) = open.parent;
        assert_unbalanced(&parenthesis_group, Parenthesis);
        let brace_group = enclosing_group(parenthesis_group.parent);
        assert_balanced(&brace_group, Brace);
        assert_root(brace_group.parent);
        assert_balanced(
            &enclosing_group(run(resolved(&tree, text, "bar")).parent),
            Brace,
        );
        assert_root(run(resolved(&tree, text, "Query")).parent);
        // The whitespace between `(` and `}` (byte 22) sits outside the childless paren
        // group, so it resolves to the enclosing brace group.
        assert_balanced(&group_leaf(tree.resolve((), Span::new(22, 23))), Brace);
        match tree.errors().as_slice() {
            [BracketError::Unclosed(unclosed)] => {
                assert_eq!(unclosed.item.0.location, span_of(text, "("));
                // The childless group's span is its opening alone.
                assert_eq!(unclosed.location, span_of(text, "("));
            }
            errors => panic!("expected exactly the unclosed paren, got {errors:?}"),
        }
    }

    #[test]
    fn wrong_kind_opens_close_synthetically_and_nest() {
        let text = "{ ( [ }";
        let tree = tree(text);
        let OpenBracketParent::Bracketed(brace_group) =
            open_bracket(resolved(&tree, text, "{")).parent;
        assert_balanced(&brace_group, Brace);
        assert_root(brace_group.parent);
        let OpenBracketParent::Bracketed(square_group) =
            open_bracket(resolved(&tree, text, "[")).parent;
        assert_unbalanced(&square_group, Bracket);
        let parenthesis_group = enclosing_group(square_group.parent);
        assert_unbalanced(&parenthesis_group, Parenthesis);
        let outer_group = enclosing_group(parenthesis_group.parent);
        assert_balanced(&outer_group, Brace);
        assert_root(outer_group.parent);
        match tree.errors().as_slice() {
            [
                BracketError::Unclosed(paren),
                BracketError::Unclosed(square),
            ] => {
                assert_eq!(paren.item.0.location, span_of(text, "("));
                assert_eq!(square.item.0.location, span_of(text, "["));
                // The paren group reaches its last child, the `[` group.
                assert_eq!(
                    paren.location,
                    Span::join(span_of(text, "("), span_of(text, "[")),
                );
            }
            errors => panic!("expected the paren then the bracket, got {errors:?}"),
        }
    }

    #[test]
    fn a_stray_close_is_one_token_inside_the_balanced_brace() {
        let text = "{ foo ) bar }";
        let tree = tree(text);
        assert_balanced(
            &enclosing_group(run(resolved(&tree, text, "foo")).parent),
            Brace,
        );
        let stray = close_bracket(resolved(&tree, text, ")"));
        assert_eq!(stray.inner.0, Parenthesis);
        let brace_group = enclosing_group(stray.parent);
        assert_balanced(&brace_group, Brace);
        assert_root(brace_group.parent);
        assert_balanced(
            &enclosing_group(run(resolved(&tree, text, "bar")).parent),
            Brace,
        );
        match tree.errors().as_slice() {
            [BracketError::UnexpectedClose(stray)] => {
                assert_eq!(stray.location, span_of(text, ")"));
                assert_eq!(stray.item.0, Parenthesis);
            }
            errors => panic!("expected exactly the stray close, got {errors:?}"),
        }
    }

    #[test]
    fn a_stray_close_does_not_end_a_different_kind() {
        let text = "( } )";
        let tree = tree(text);
        // The paren pair still matches around the stray `}`.
        let OpenBracketParent::Bracketed(parenthesis_group) =
            open_bracket(resolved(&tree, text, "(")).parent;
        assert_balanced(&parenthesis_group, Parenthesis);
        assert_root(parenthesis_group.parent);
        let stray = close_bracket(resolved(&tree, text, "}"));
        assert_eq!(stray.inner.0, Brace);
        assert_balanced(&enclosing_group(stray.parent), Parenthesis);
        match tree.errors().as_slice() {
            [BracketError::UnexpectedClose(stray)] => {
                assert_eq!(stray.location, span_of(text, "}"));
                assert_eq!(stray.item.0, Brace);
            }
            errors => panic!("expected exactly the stray close, got {errors:?}"),
        }
    }

    #[test]
    fn crossing_pairs_produce_two_errors_in_source_order() {
        let text = "( { ) }";
        let tree = tree(text);
        let OpenBracketParent::Bracketed(parenthesis_group) =
            open_bracket(resolved(&tree, text, "(")).parent;
        assert_balanced(&parenthesis_group, Parenthesis);
        assert_root(parenthesis_group.parent);
        let OpenBracketParent::Bracketed(brace_group) =
            open_bracket(resolved(&tree, text, "{")).parent;
        assert_unbalanced(&brace_group, Brace);
        assert_balanced(&enclosing_group(brace_group.parent), Parenthesis);
        // The trailing `}` is a stray brace close at the top level: its `{` was consumed
        // inside the paren group.
        let stray = close_bracket(resolved(&tree, text, "}"));
        assert_eq!(stray.inner.0, Brace);
        assert_root(stray.parent);
        match tree.errors().as_slice() {
            [
                BracketError::Unclosed(brace),
                BracketError::UnexpectedClose(stray),
            ] => {
                assert_eq!(brace.item.0.location, span_of(text, "{"));
                assert_eq!(stray.location, span_of(text, "}"));
            }
            errors => panic!("expected the unclosed brace then the stray close, got {errors:?}"),
        }
    }

    #[test]
    fn the_close_pairs_with_the_nearest_open() {
        let text = "a {\n  b {\n    c\n}\n";
        let tree = tree(text);
        assert_root(run(resolved(&tree, text, "a")).parent);
        // `c` sits in a run inside b's balanced brace group, which sits inside a's
        // unbalanced brace group.
        let b_group = enclosing_group(run(resolved(&tree, text, "c")).parent);
        assert_balanced(&b_group, Brace);
        let a_group = enclosing_group(b_group.parent);
        assert_unbalanced(&a_group, Brace);
        assert_root(a_group.parent);
        // The one `}` is b's: a position on it answers the close, with b's balanced group
        // as its parent.
        let close = close_bracket(resolved(&tree, text, "}"));
        assert_eq!(close.inner.0, Brace);
        let close_group = enclosing_group(close.parent);
        assert_balanced(&close_group, Brace);
        assert_unbalanced(&enclosing_group(close_group.parent), Brace);
        match tree.errors().as_slice() {
            [BracketError::Unclosed(unclosed)] => {
                // The unclosed group is the outer one: it contains `b`, which b's own
                // group does not.
                assert!(unclosed.location.contains(span_of(text, "b")));
            }
            errors => panic!("expected exactly the outer unclosed brace, got {errors:?}"),
        }
    }

    #[test]
    fn an_empty_group_is_balanced_and_childless() {
        let text = "a {}";
        let tree = tree(text);
        let OpenBracketParent::Bracketed(brace_group) =
            open_bracket(resolved(&tree, text, "{")).parent;
        assert_balanced(&brace_group, Brace);
        assert_root(brace_group.parent);
        assert!(brace_group.inner.children.is_empty());
        assert_eq!(tree.errors(), vec![]);
    }

    #[test]
    fn brackets_inside_strings_are_not_structural() {
        let text = "{ name: \"a}\" }";
        let tree = tree(text);
        let brace_group = enclosing_group(run(resolved(&tree, text, "\"a}\"")).parent);
        assert_balanced(&brace_group, Brace);
        assert_root(brace_group.parent);
        assert_eq!(tree.errors(), vec![]);
    }

    #[test]
    fn content_after_an_unclosed_open_sits_inside_the_unbalanced_group() {
        let text = "a {\n  b\n";
        let tree = tree(text);
        let brace_group = enclosing_group(run(resolved(&tree, text, "b")).parent);
        assert_unbalanced(&brace_group, Brace);
        assert_root(brace_group.parent);
        match tree.errors().as_slice() {
            [BracketError::Unclosed(unclosed)] => {
                assert_eq!(unclosed.item.0.location, span_of(text, "{"));
            }
            errors => panic!("expected exactly the unclosed brace, got {errors:?}"),
        }
    }

    #[test]
    fn a_stray_close_at_the_top_level_is_a_leaf_of_the_root() {
        let text = "a }";
        let tree = tree(text);
        assert_root(run(resolved(&tree, text, "a")).parent);
        let stray = close_bracket(resolved(&tree, text, "}"));
        assert_eq!(stray.inner.0, Brace);
        assert_root(stray.parent);
        match tree.errors().as_slice() {
            [BracketError::UnexpectedClose(stray)] => {
                assert_eq!(stray.location, span_of(text, "}"));
                assert_eq!(stray.item.0, Brace);
            }
            errors => panic!("expected exactly the stray close, got {errors:?}"),
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct BracketsMatchedNoErrors;

    impl TreeContents for BracketsMatchedNoErrors {
        type Inner = Inner;
        type StrayClose = Infallible;
    }

    fn refine(
        tree: MatchedBrackets<BracketsMatched>,
    ) -> Result<MatchedBrackets<BracketsMatchedNoErrors>, Vec<BracketError>> {
        tree.try_map(&mut |tokens| Ok(tokens.item), &mut |stray| {
            Err(BracketError::UnexpectedClose(stray))
        })
    }

    #[test]
    fn a_clean_tree_refines() {
        assert!(refine(tree("field Query.Foo { bar(arg: [1, 2]) { id } }")).is_ok());
    }
}
