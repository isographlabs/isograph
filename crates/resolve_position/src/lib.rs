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
///   inner: &WithSpan<ScalarSelectableName>,
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
/// In order to implement `ResolvePosition`, we define the outermost enum, and
/// for each AST node:
/// - define a parent path enum, or alias the parent path as `[NODE_TYPE]Parent`.
/// - define a `[NODE_TYPE]Path` alias, containing `WithSpan<NODE_TYPE>` and the
///   parent type, and
/// - implement the `ResolvePosition` trait for the AST node itself.
///
/// The actual logic lives in the implementation of `ResolvePosition::resolve`
/// for each AST node. The implementation must behave as follows:
/// - It must call `.contains()` on each of its child nodes. When the child node
///   containing the position is discovered, it must return the result of calling
///   `.resolve()` on that child.
/// - If no child's `.contains()` method returns true (i.e. no child contains the
///   position), then the node must assume that it is the laf node and returns its
///   own variant of `ResolvedNode`.
///
/// This approach is not ideal: there is an implicit (i.e. not enforced by the
/// compiler) contract that each parent node must check the result of `.contains()`
/// on a child before calling that child's `.resolve()` method. Ideally, `.resolve()`
/// could return an `Option`. However, calling `.resolve()` moves the parent into
/// the child's method, meaning that we cannot try out multiple potential children.
/// Hence, the alternative of calling `.contains()` followed by `.resolve()`.
///
/// NOTE: I believe this can be improved by return a `ControlFlow` from `resolve()`.

#[derive(Debug)]
pub struct Path<Inner, Parent> {
    pub inner: Inner,
    pub parent: Parent,
}

pub trait ResolvePosition: Sized {
    type Parent;
    type ResolvedNode<'a>
    where
        Self: 'a;

    fn resolve<'a>(&'a self, parent: Self::Parent, position: Span) -> Self::ResolvedNode<'a>;
    fn contains(&self, position: Span) -> bool;
    fn path(&self, parent: Self::Parent) -> Path<&Self, Self::Parent> {
        Path {
            inner: self,
            parent,
        }
    }
}
