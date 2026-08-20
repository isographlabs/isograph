#![expect(dead_code)]

use prelude::Postfix;
use resolve_position::{PositionResolutionPath, ResolvePosition};
use resolve_position_macros::ResolvePosition;
use span::{Span, WithGenericLocation, WithOptionalSpan, WithSpanPostfix};

#[derive(Debug)]
enum TestResolvedNode<'a> {
    Container(ContainerPath<'a>),
    Child(ChildPath<'a>),
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = TestResolvedNode<'a>)]
struct Container(#[resolve_field] Vec<WithOptionalSpan<Child>>);

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = ContainerPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct Child;

type ContainerPath<'a> = PositionResolutionPath<&'a Container, ()>;
type ChildPath<'a> = PositionResolutionPath<&'a Child, ContainerPath<'a>>;

#[test]
fn a_none_location_is_not_entered() {
    let container = Container(vec![
        WithGenericLocation::new(Child, None),
        Child
            .with_span(Span::new(0, 4))
            .map_location(|span| span.wrap_some()),
    ]);
    match container.resolve((), Span::new(1, 2)) {
        TestResolvedNode::Child(_) => {}
        node => panic!("expected the spanned child, got {node:?}"),
    }
}

#[test]
fn a_none_location_does_not_match_a_zero_width_position() {
    let container = Container(WithGenericLocation::new(Child, None).wrap_vec());
    match container.resolve((), Span::new(0, 0)) {
        TestResolvedNode::Container(_) => {}
        node => panic!("expected the container, got {node:?}"),
    }
}
