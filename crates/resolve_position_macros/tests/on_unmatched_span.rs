use prelude::Postfix;
use resolve_position::{PositionResolutionPath, ResolvePosition};
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan, WithSpanPostfix};

#[derive(Debug)]
enum TestResolvedNode<'a> {
    List(ParentPath<'a>),
    Slot(SlotPath<'a>),
    Child(ChildPath<'a>),
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(
    parent_type = ParentPath<'a>,
    resolved_node = TestResolvedNode<'a>,
    on_unmatched_span = from_path
)]
struct Slot {
    #[resolve_field]
    item: Option<WithSpan<Child>>,
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(
    parent_type = (),
    resolved_node = TestResolvedNode<'a>,
    on_unmatched_span = struct_name
)]
struct List(#[resolve_field] Vec<WithSpan<Slot>>);

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = SlotPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct Child;

type ParentPath<'a> = PositionResolutionPath<&'a List, ()>;

type SlotPath<'a> = PositionResolutionPath<&'a Slot, ParentPath<'a>>;

type ChildPath<'a> = PositionResolutionPath<&'a Child, SlotPath<'a>>;

impl<'a> From<SlotPath<'a>> for TestResolvedNode<'a> {
    fn from(path: SlotPath<'a>) -> Self {
        TestResolvedNode::Slot(path)
    }
}

#[test]
fn from_path_unmatched_span_uses_the_from_impl() {
    let list = List(
        Slot {
            item: Child.with_span(Span::new(0, 2)).wrap_some(),
        }
        .with_span(Span::new(0, 8))
        .wrap_vec(),
    );

    match list.resolve((), Span::new(4, 5)) {
        TestResolvedNode::Slot(path) => {
            assert!(path.inner.item.is_some());
        }
        node => panic!("expected the slot, got {node:?}"),
    }
}

#[test]
fn from_path_matched_field_descends_into_the_child() {
    let list = List(
        Slot {
            item: Child.with_span(Span::new(0, 4)).wrap_some(),
        }
        .with_span(Span::new(0, 4))
        .wrap_vec(),
    );

    match list.resolve((), Span::new(1, 2)) {
        TestResolvedNode::Child(path) => {
            assert!(std::ptr::eq(path.parent.parent.inner, list.reference()));
        }
        node => panic!("expected the child, got {node:?}"),
    }
}

#[test]
fn struct_name_unmatched_span_uses_the_struct_variant() {
    let list = List(vec![]);

    match list.resolve((), Span::new(0, 1)) {
        TestResolvedNode::List(path) => {
            assert!(std::ptr::eq(path.inner, list.reference()));
        }
        node => panic!("expected the list, got {node:?}"),
    }
}
