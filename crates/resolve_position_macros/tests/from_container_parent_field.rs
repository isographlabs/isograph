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
    ChildB(PositionResolutionPath<&'a ChildB, SlotBPath<'a>>),
    Extra(PositionResolutionPath<&'a Extra, ExtraParent<'a>>),
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
struct ListA(#[resolve_field] Vec<WithSpan<Slot<ChildA, Extra>>>);

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = TestResolvedNode<'a>)]
struct ListB(#[resolve_field] Vec<WithSpan<Slot<ChildB, Extra>>>);

type PathA<'a> = PositionResolutionPath<&'a ListA, ()>;
type PathB<'a> = PositionResolutionPath<&'a ListB, ()>;
type SlotAPath<'a> = PositionResolutionPath<&'a Slot<ChildA, Extra>, PathA<'a>>;
type SlotBPath<'a> = PositionResolutionPath<&'a Slot<ChildB, Extra>, PathB<'a>>;

#[derive(Debug, ResolvePosition)]
#[resolve_position(
    resolved_node = TestResolvedNode<'a>,
    on_unmatched_span = from_path,
    self_type_generics = [
        (<ChildA, Extra>, PathA<'a>),
        (<ChildB, Extra>, PathB<'a>),
    ]
)]
struct Slot<T, E> {
    #[resolve_field]
    item: Option<WithSpan<T>>,
    #[resolve_field]
    #[from_container_parent]
    extra_tokens: Option<WithSpan<E>>,
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = SlotAPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct ChildA;

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = SlotBPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct ChildB;

#[derive(Debug)]
enum ExtraParent<'a> {
    SlotA(SlotAPath<'a>),
    SlotB(SlotBPath<'a>),
}

impl<'a> From<SlotAPath<'a>> for ExtraParent<'a> {
    fn from(path: SlotAPath<'a>) -> Self {
        ExtraParent::SlotA(path)
    }
}

impl<'a> From<SlotBPath<'a>> for ExtraParent<'a> {
    fn from(path: SlotBPath<'a>) -> Self {
        ExtraParent::SlotB(path)
    }
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = ExtraParent<'a>, resolved_node = TestResolvedNode<'a>)]
struct Extra;

#[test]
fn leftover_parent_is_the_slot_path_through_from() {
    let list = ListA(
        Slot {
            item: ChildA.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: Extra.with_span(Span::new(6, 8)).wrap_some(),
        }
        .with_span(Span::new(0, 8))
        .wrap_vec(),
    );
    match list.resolve((), Span::new(6, 7)) {
        TestResolvedNode::Extra(path) => match path.parent {
            ExtraParent::SlotA(slot) => {
                assert!(std::ptr::eq(slot.inner, list.0[0].item.reference()));
            }
            parent => panic!("expected ExtraParent::SlotA, got {parent:?}"),
        },
        node => panic!("expected Extra, got {node:?}"),
    }
    match list.resolve((), Span::new(4, 5)) {
        TestResolvedNode::SlotA(_) => {}
        node => panic!("expected SlotA, got {node:?}"),
    }
}
