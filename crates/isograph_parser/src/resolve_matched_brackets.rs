use resolve_position::{PositionResolutionPath, ResolvePosition};
use span::{Span, WithSpan};

use crate::{BracketItem, Bracketed, MatchedBrackets, TreeContents};

#[derive(Debug)]
pub enum ResolvedBracketNode<'a, TContents: TreeContents> {
    MatchedBrackets(MatchedBracketsPath<'a, TContents>),
    Inner(InnerPath<'a, TContents>),
    Bracketed(BracketedPath<'a, TContents>),
    StrayClose(StrayClosePath<'a, TContents>),
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
pub type StrayClosePath<'a, TContents> = PositionResolutionPath<
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
    items.iter().find(|item| item.span.contains(position))
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
        BracketItem::StrayClose(stray) => ResolvedBracketNode::StrayClose(PositionResolutionPath {
            inner: stray,
            parent,
        }),
    }
}

