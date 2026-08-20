use prelude::Postfix;
use resolve_position::{PositionResolutionPath, ResolvePosition};
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan, WithSpanPostfix};

#[derive(Debug)]
enum TestResolvedNode<'a> {
    List(ParentPath<'a>),
    Child(ChildPath<'a>),
    Unparsed(UnparsedPath<'a>),
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = TestResolvedNode<'a>)]
struct List(#[resolve_field] Vec<WithSpan<Slot<Child>>>);

type ParentPath<'a> = PositionResolutionPath<&'a List, ()>;

#[derive(Debug, ResolvePosition)]
#[resolve_position(
    parent_type = <T as ResolvePosition>::Parent<'a>,
    resolved_node = <T as ResolvePosition>::ResolvedNode<'a>
)]
enum Slot<T: ResolvePosition>
where
    for<'a> SlotUnparsedParent<'a>: From<<T as ResolvePosition>::Parent<'a>>,
    for<'a> Unparsed: ResolvePosition<ResolvedNode<'a> = <T as ResolvePosition>::ResolvedNode<'a>>,
{
    Parsed(Parsed<T>),
    Unparsed(#[parent_from] Unparsed),
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(
    parent_type = <T as ResolvePosition>::Parent<'a>,
    resolved_node = <T as ResolvePosition>::ResolvedNode<'a>
)]
struct Parsed<T: ResolvePosition>(#[resolve_field(transparent)] T);

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = ParentPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct Child;

type ChildPath<'a> = PositionResolutionPath<&'a Child, ParentPath<'a>>;

#[derive(Debug, ResolvePosition)]
#[resolve_position(
    parent_type = SlotUnparsedParent<'a>,
    resolved_node = TestResolvedNode<'a>
)]
struct Unparsed;

type UnparsedPath<'a> = PositionResolutionPath<&'a Unparsed, SlotUnparsedParent<'a>>;

#[derive(Debug)]
enum SlotUnparsedParent<'a> {
    List(ParentPath<'a>),
}

impl<'a> From<ParentPath<'a>> for SlotUnparsedParent<'a> {
    fn from(path: ParentPath<'a>) -> Self {
        SlotUnparsedParent::List(path)
    }
}

#[test]
fn a_parsed_child_resolves_to_that_child_with_the_list_path() {
    let list = List(
        Slot::<Child>::Parsed(Parsed(Child))
            .with_span(Span::new(0, 4))
            .wrap_vec(),
    );

    match list.resolve((), Span::new(1, 2)) {
        TestResolvedNode::Child(path) => {
            assert!(std::ptr::eq(path.parent.inner, list.reference()));
        }
        node => panic!("expected the child, got {node:?}"),
    }
}

#[test]
fn a_position_outside_the_slot_resolves_to_the_list() {
    let list = List(
        Slot::<Child>::Parsed(Parsed(Child))
            .with_span(Span::new(0, 4))
            .wrap_vec(),
    );

    match list.resolve((), Span::new(10, 11)) {
        TestResolvedNode::List(path) => {
            assert!(std::ptr::eq(path.inner, list.reference()));
        }
        node => panic!("expected the list, got {node:?}"),
    }
}

#[test]
fn an_unparsed_child_resolves_through_from_the_list_path() {
    let list = List(
        Slot::<Child>::Unparsed(Unparsed)
            .with_span(Span::new(0, 4))
            .wrap_vec(),
    );

    match list.resolve((), Span::new(1, 2)) {
        TestResolvedNode::Unparsed(path) => match path.parent {
            SlotUnparsedParent::List(list_path) => {
                assert!(std::ptr::eq(list_path.inner, list.reference()));
            }
        },
        node => panic!("expected the unparsed item, got {node:?}"),
    }
}
