use common_lang_types::Span;

/// This module defines a trait [`ResolvePosition`], which is used to convert a
/// mouse or keyboard cursor position (given by a [`Span`]) to a
/// [`ResolvedNode`](ResolvePosition::ResolvedNode), which is (by
/// convention) an enum of possible items where the cursor could be sitting.
/// (Indeed, in the actual implementation of `ResolvePosition` in the
/// `isograph_lang_types` crate has an enum for `ResolvedNode`.)
///
/// Each item in this `ResolvedNode` enum contains (again, by convention)
/// a `Path<&'a Node, NodeParent<'a>>` for a given Node, typically aliased as
/// NodePath. The [`Path`] struct contains a reference to the Node and an owned
/// parent struct, which is (again by convention) either an enum of possible
/// parent paths, or simply the parent path directly. The final item will have a
/// parent of `()`, indicating that it is a root item.
///
/// As an example, consider the isograph literal `field Query.Foo { id }`,
/// with the cursor sitting atop of `id`. In this case, we would construct a
/// data structure roughly akin to:
///
/// ```txt
/// ResolvedNode::ScalarSelectableName(ScalarSelectableNamePath {
///   inner: &ScalarSelectableName,
///   parent: ClientScalarSelectionPath {
///     item: &ClientScalarSelection,
///     parent: ClientScalarSelectionParent::ClientFieldDefinition(
///       ClientFieldDefinitionPath {
///         inner: &ClientFieldDefinition,
///         parent: (),
///       }
///     )
///   }
/// })
/// ```
///
/// Let's examine this. The `ScalarSelectableNamePath` has an inner field,
/// pointing to the innermost identifier we are hovering on. Such an identifier
/// can only show up in one place, as the name of a client scalar field.
/// (Aliases are different types!) So, the parent is the ClientScalarSelectionPath
/// directly.
///
/// A ClientScalarSelection can have multiple parents, though — for example, it
/// can appear in the selection set of a linked field or at the top level of
/// a client field, as is the case with this `id` field. Hence, the parent field
/// is an enum, `ClientScalarSelectionParent`. In our case, the parent is a
/// `ClientFieldDefinitionPath`, which has no parent, so its parent field is
/// `()`.
///
/// ## Implementation details
///
/// In order to implement `ResolvePosition`, we define the `ResolvedNode`` enum,
/// and for each AST node:
/// - define a parent path enum, or alias the parent path as `[NODE_TYPE]Parent`
///   (if there is only one type of parent).
/// - define a `[NODE_TYPE]Path` alias, containing `WithSpan<NODE_TYPE>` and the
///   parent type, and
/// - implement the `ResolvePosition` trait for the AST node itself.
///
/// The actual logic lives in the implementation of `ResolvePosition::resolve`
/// for each AST node. The implementation must behave as follows:
/// - It must check whether each subnode contains the position, most often by
///   calling `field.span.contains(position)`. When the child node containing the
///   position is discovered, it must return the result of calling `.resolve()`
///   on that child.
/// - If no child contains the position, then the node must assume that it is
///   the leaf node and returns its own variant of `ResolvedNode`.

#[derive(Debug)]
pub struct PositionResolutionPath<Inner, Parent> {
    pub inner: Inner,
    pub parent: Parent,
}

pub trait ResolvePosition: Sized {
    type Parent<'a>
    where
        Self: 'a;
    type ResolvedNode<'a>
    where
        Self: 'a;

    /// Called when we are sure that the node contains the cursor. i.e. the parent must check
    /// self.field.span.contains(position) before calling .resolve().
    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: Span) -> Self::ResolvedNode<'a>;

    fn path<'a>(
        &'a self,
        parent: Self::Parent<'a>,
    ) -> PositionResolutionPath<&'a Self, Self::Parent<'a>> {
        PositionResolutionPath {
            inner: self,
            parent,
        }
    }
}

#[cfg(test)]
mod test {
    #![allow(unused)]

    use std::ops::ControlFlow;

    use crate::{PositionResolutionPath, ResolvePosition};
    use common_lang_types::{Span, WithSpan};

    #[derive(Debug)]
    enum TestResolvedNode<'a> {
        Parent(ParentPath<'a>),
        Child(ChildPath<'a>),
        SelfContained(SelfContainedPath<'a>),
    }

    #[derive(Debug)]
    struct Parent {
        children: Vec<WithSpan<Child>>,
    }

    #[derive(Debug)]
    struct Child {
        children: Vec<WithSpan<Child>>,
    }

    type ParentPath<'a> = PositionResolutionPath<&'a Parent, ()>;

    #[derive(Debug)]
    enum ChildParent<'a> {
        Parent(ParentPath<'a>),
        Child(ChildPath<'a>),
    }

    type ChildPath<'a> = PositionResolutionPath<&'a Child, Box<ChildParent<'a>>>;

    #[derive(Debug)]
    struct SelfContained {}

    type SelfContainedPath<'a> = PositionResolutionPath<&'a WithSpan<SelfContained>, ()>;

    impl ResolvePosition for Parent {
        type Parent<'a> = ();

        type ResolvedNode<'a> = TestResolvedNode<'a>;

        fn resolve<'a>(
            &'a self,
            parent: Self::Parent<'a>,
            position: Span,
        ) -> Self::ResolvedNode<'a> {
            for child in self.children.iter() {
                if child.span.contains(position) {
                    let parent = <Child as ResolvePosition>::Parent::Parent(self.path(parent));
                    return child.item.resolve(parent, position);
                }
            }

            return Self::ResolvedNode::Parent(self.path(parent));
        }
    }

    impl ResolvePosition for Child {
        type Parent<'a> = ChildParent<'a>;

        type ResolvedNode<'a> = TestResolvedNode<'a>;

        fn resolve<'a>(
            &'a self,
            mut parent: Self::Parent<'a>,
            position: Span,
        ) -> Self::ResolvedNode<'a> {
            for child in self.children.iter() {
                if child.span.contains(position) {
                    let parent = <Child as ResolvePosition>::Parent::Child(self.path(parent));
                    return child.item.resolve(parent, position);
                }
            }

            return Self::ResolvedNode::Child(self.path(parent));
        }
    }

    #[test]
    fn resolve_no_children_inside() {
        let item = Parent { children: vec![] };

        let result = item.resolve((), Span::new(0, 0));

        assert!(matches!(result, TestResolvedNode::Parent(_)));
    }

    #[test]
    fn resolve_outside() {
        let item = Parent { children: vec![] };

        let result = item.resolve((), Span::new(100, 100));

        // It will still "match", even if we made a mistake and didn't check the span.
        assert!(matches!(result, TestResolvedNode::Parent(_)));
    }

    #[test]
    fn resolve_parent_with_children() {
        let item = Parent {
            children: vec![WithSpan::new(Child { children: vec![] }, Span::new(0, 5))],
        };

        let result = item.resolve((), Span::new(0, 4));

        match result {
            TestResolvedNode::Child(child) => {
                assert!(matches!(*child.parent, ChildParent::Parent(_)));
            }
            _ => {
                panic!("Unexpected variant")
            }
        };
    }

    #[test]
    fn resolve_parent_with_nested_children() {
        let item = Parent {
            children: vec![WithSpan::new(
                Child {
                    children: vec![WithSpan::new(Child { children: vec![] }, Span::new(0, 3))],
                },
                Span::new(0, 5),
            )],
        };

        let result = item.resolve((), Span::new(0, 2));

        match result {
            TestResolvedNode::Child(child) => {
                assert!(matches!(*child.parent, ChildParent::Child(_)));
            }
            _ => {
                panic!("Unexpected variant")
            }
        };
    }
}
