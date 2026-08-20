# `WithOptionalSpan`

A value that does not exist in the source has no span. `WithSpan<T>` cannot say that. `WithOptionalSpan<T>` is `WithGenericLocation<T, Option<Span>>`. `Some(span)` is the source range. `None` is not in the text.

`()` as a location is a tree with no positions at all (spanless parsing). That is not this.

Does not depend on type-annotation-union.md. type-annotation-union.md depends on this. Origin of the listings: type-annotation-union.md Change 1, moved here.

AGENTS.md and parsing-standards.md already require this wrapper. This change is the alias and the derive walk.

One change.

```rust
// from crates/span/src/lib.rs
/// An item that may have no source span.
pub type WithOptionalSpan<T> = WithGenericLocation<T, Option<Span>>;
```

Before: no alias. `drop_location` still produces `WithGenericLocation<T, ()>`.

Do not write a second `ResolvePosition` impl for `WithGenericLocation<T, Option<Span>>`. The blanket already covers every `TLocation`:

```rust
// from crates/resolve_position/src/lib.rs
impl<T: ResolvePosition, TLocation> ResolvePosition for WithGenericLocation<T, TLocation> {
    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: Span) -> Self::ResolvedNode<'a> {
        self.item.resolve(parent, position)
    }
}
```

The parent checks the location, then calls `resolve`. `WithSpan` is `location.contains(position)`. `WithLocation` (file-qualified, kept chain) is already `if let Some(span) = location.span()`. `WithOptionalSpan` is the same check on `Option<Span>`:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
enum ResolveFieldInfoType {
    WithSpan(syn::Type),
    WithLocation(syn::Type),
    WithEmbeddedLocation(syn::Type),
    WithOptionalSpan(syn::Type),
    GraphQLTypeAnnotation(syn::Type),
}
```

Before: no `WithOptionalSpan` variant.

`parse_resolve_field_type` gains `"WithOptionalSpan" => handle_case(..., ResolveFieldInfoType::WithOptionalSpan)`. The expected-type error lists `WithOptionalSpan`. `resolve_field_inner_type` matches it with the other wrappers.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
            ResolveFieldInfoType::WithOptionalSpan(inner_type) => {
                let new_parent = new_parent_expr(parent_construction, inner_type);
                quote! {
                    if let Some(span) = #field_expr.location {
                        if span.contains(position) {
                            let new_parent = #new_parent;
                            return #field_expr.item.resolve(new_parent, position);
                        }
                    }
                }
            }
```

`None` does not contain a position. The item is never `resolve`d.

Who calls: type-annotation-union.md's `UnionTypeAnnotation` variants vec. Existing `WithSpan` / `WithLocation` / `WithEmbeddedLocation` sites are unchanged.

```rust
// from crates/resolve_position_macros/tests/optional_span.rs
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
        Child.with_span(Span::new(0, 4)).map_location(|span| span.wrap_some()),
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
```

`map_location(|span| span.wrap_some())` turns `WithSpan<Child>` into `WithOptionalSpan<Child>`. The `None` row does not steal a zero-width position.
