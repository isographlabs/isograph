use resolve_position::{PositionResolutionPath, ResolvePosition};
use span::{Span, WithSpan};

use crate::{BracketItem, BracketKind, Bracketed, Closing, MatchedBrackets, TreeContents};

/// Every node a position can resolve to while only brackets are matched. Once later passes
/// add their nodes, the full isograph path enum replaces this one.
#[derive(Debug)]
pub enum ResolvedBracketNode<'a, TContents: TreeContents> {
    /// A position outside every item.
    MatchedBrackets(MatchedBracketsPath<'a, TContents>),
    /// A position on whitespace inside a group: on none of its children and on neither of
    /// its brackets.
    Bracketed(BracketedPath<'a, TContents>),
    /// A position in a run of non-bracket tokens.
    Inner(InnerPath<'a, TContents>),
    /// A position on a group's opening bracket.
    OpenBracket(OpenBracketPath<'a, TContents>),
    /// A position on a group's real closing bracket.
    MatchedClose(MatchedClosePath<'a, TContents>),
    /// A position on a close bracket no open of its kind was waiting for.
    UnmatchedClose(UnmatchedClosePath<'a, TContents>),
}

pub type MatchedBracketsPath<'a, TContents> =
    PositionResolutionPath<&'a MatchedBrackets<TContents>, ()>;

/// Everything a `BracketItem` can sit inside.
#[derive(Debug)]
pub enum BracketItemParent<'a, TContents: TreeContents> {
    MatchedBrackets(MatchedBracketsPath<'a, TContents>),
    Bracketed(Box<BracketedPath<'a, TContents>>),
}

pub type InnerPath<'a, TContents> = PositionResolutionPath<
    &'a <TContents as TreeContents>::Inner,
    BracketItemParent<'a, TContents>,
>;
pub type BracketedPath<'a, TContents> =
    PositionResolutionPath<&'a Bracketed<TContents>, BracketItemParent<'a, TContents>>;
pub type OpenBracketPath<'a, TContents> =
    PositionResolutionPath<&'a WithSpan<BracketKind>, Box<BracketedPath<'a, TContents>>>;
pub type MatchedClosePath<'a, TContents> =
    PositionResolutionPath<&'a Span, Box<BracketedPath<'a, TContents>>>;
pub type UnmatchedClosePath<'a, TContents> = PositionResolutionPath<
    &'a <TContents as TreeContents>::Stray,
    BracketItemParent<'a, TContents>,
>;

impl<TContents: TreeContents> ResolvePosition for MatchedBrackets<TContents> {
    type Parent<'a>
        = ()
    where
        Self: 'a;
    type ResolvedNode<'a>
        = ResolvedBracketNode<'a, TContents>
    where
        Self: 'a;

    fn resolve<'a>(&'a self, parent: (), position: Span) -> ResolvedBracketNode<'a, TContents> {
        match containing_child(&self.0, position) {
            Some(child) => {
                let parent = BracketItemParent::MatchedBrackets(self.path(parent));
                resolve_child(child, parent, position)
            }
            None => ResolvedBracketNode::MatchedBrackets(self.path(parent)),
        }
    }
}

impl<TContents: TreeContents> ResolvePosition for Bracketed<TContents> {
    type Parent<'a>
        = BracketItemParent<'a, TContents>
    where
        Self: 'a;
    type ResolvedNode<'a>
        = ResolvedBracketNode<'a, TContents>
    where
        Self: 'a;

    fn resolve<'a>(
        &'a self,
        parent: BracketItemParent<'a, TContents>,
        position: Span,
    ) -> ResolvedBracketNode<'a, TContents> {
        if self.opening.location.contains(position) {
            return ResolvedBracketNode::OpenBracket(PositionResolutionPath {
                inner: &self.opening,
                parent: Box::new(self.path(parent)),
            });
        }
        if let Closing::Real(close) = &self.closing {
            if close.contains(position) {
                return ResolvedBracketNode::MatchedClose(PositionResolutionPath {
                    inner: close,
                    parent: Box::new(self.path(parent)),
                });
            }
        }
        match containing_child(&self.children, position) {
            Some(child) => {
                let parent = BracketItemParent::Bracketed(Box::new(self.path(parent)));
                resolve_child(child, parent, position)
            }
            None => ResolvedBracketNode::Bracketed(self.path(parent)),
        }
    }
}

/// The first item whose span contains the position.
fn containing_child<'a, TContents: TreeContents>(
    items: &'a [WithSpan<BracketItem<TContents>>],
    position: Span,
) -> Option<&'a WithSpan<BracketItem<TContents>>> {
    items.iter().find(|item| item.location.contains(position))
}

/// Resolve into an item already known to contain the position.
fn resolve_child<'a, TContents: TreeContents>(
    child: &'a WithSpan<BracketItem<TContents>>,
    parent: BracketItemParent<'a, TContents>,
    position: Span,
) -> ResolvedBracketNode<'a, TContents> {
    match &child.item {
        BracketItem::Inner(inner) => {
            ResolvedBracketNode::Inner(PositionResolutionPath { inner, parent })
        }
        BracketItem::Bracketed(bracketed) => bracketed.resolve(parent, position),
        BracketItem::StrayClose(stray) => {
            ResolvedBracketNode::UnmatchedClose(PositionResolutionPath {
                inner: stray,
                parent,
            })
        }
    }
}
