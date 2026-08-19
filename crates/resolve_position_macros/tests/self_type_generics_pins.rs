#![expect(dead_code)]

use prelude::Postfix;
use resolve_position::{PositionResolutionPath, ResolvePosition};
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan, WithSpanPostfix};

#[derive(Debug)]
enum TestResolvedNode<'a> {
    ListA(PathA<'a>),
    ListB(PathB<'a>),
    SlotA(SlotAPath<'a>),
    SlotB(SlotBPath<'a>),
    ChildA(PositionResolutionPath<&'a ChildA, SlotAPath<'a>>),
    ExtraA(PositionResolutionPath<&'a ExtraA, SlotAPath<'a>>),
    ChildB(PositionResolutionPath<&'a ChildB, SlotBPath<'a>>),
    ExtraB(PositionResolutionPath<&'a ExtraB, SlotBPath<'a>>),
}

impl<'a> From<SlotAPath<'a>> for TestResolvedNode<'a> {
    fn from(path: SlotAPath<'a>) -> Self {
        TestResolvedNode::SlotA(path)
    }
}

impl<'a> From<SlotBPath<'a>> for TestResolvedNode<'a> {
    fn from(path: SlotBPath<'a>) -> Self {
        TestResolvedNode::SlotB(path)
    }
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = TestResolvedNode<'a>)]
struct ListA(#[resolve_field] Vec<WithSpan<Slot<ChildA, ExtraA>>>);

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = TestResolvedNode<'a>)]
struct ListB(#[resolve_field] Vec<WithSpan<Slot<ChildB, ExtraB>>>);

type PathA<'a> = PositionResolutionPath<&'a ListA, ()>;
type PathB<'a> = PositionResolutionPath<&'a ListB, ()>;
type SlotAPath<'a> = PositionResolutionPath<&'a Slot<ChildA, ExtraA>, PathA<'a>>;
type SlotBPath<'a> = PositionResolutionPath<&'a Slot<ChildB, ExtraB>, PathB<'a>>;

#[derive(Debug, ResolvePosition)]
#[resolve_position(
    resolved_node = TestResolvedNode<'a>,
    on_unmatched_span = from_path,
    self_type_generics = [
        (<ChildA, ExtraA>, PathA<'a>),
        (<ChildB, ExtraB>, PathB<'a>),
    ]
)]
struct Slot<T, E> {
    #[resolve_field]
    item: Option<WithSpan<T>>,
    #[resolve_field]
    extra_tokens: Option<WithSpan<E>>,
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = SlotAPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct ChildA;

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = SlotAPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct ExtraA;

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = SlotBPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct ChildB;

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = SlotBPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct ExtraB;

#[test]
fn pin_a_item_and_gap() {
    let list = ListA(
        Slot {
            item: ChildA.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: ExtraA.with_span(Span::new(6, 8)).wrap_some(),
        }
        .with_span(Span::new(0, 8))
        .wrap_vec(),
    );
    match list.resolve((), Span::new(1, 2)) {
        TestResolvedNode::ChildA(path) => {
            assert!(std::ptr::eq(path.parent.inner, list.0[0].item.reference()));
        }
        node => panic!("expected ChildA, got {node:?}"),
    }
    match list.resolve((), Span::new(4, 5)) {
        TestResolvedNode::SlotA(_) => {}
        node => panic!("expected SlotA, got {node:?}"),
    }
}

#[test]
fn pin_b_item_and_gap() {
    let list = ListB(
        Slot {
            item: ChildB.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: ExtraB.with_span(Span::new(6, 8)).wrap_some(),
        }
        .with_span(Span::new(0, 8))
        .wrap_vec(),
    );
    match list.resolve((), Span::new(1, 2)) {
        TestResolvedNode::ChildB(path) => {
            assert!(std::ptr::eq(path.parent.inner, list.0[0].item.reference()));
        }
        node => panic!("expected ChildB, got {node:?}"),
    }
    match list.resolve((), Span::new(4, 5)) {
        TestResolvedNode::SlotB(_) => {}
        node => panic!("expected SlotB, got {node:?}"),
    }
}
