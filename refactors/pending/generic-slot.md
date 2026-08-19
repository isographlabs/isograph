# generic-slot: one impl for every `Slot<T, E>`

Lands after path-parent.md.

`Slot` is used at the root and in every list. One pinned impl cannot cover `Slot<Selection, UnparsedChunkItems>`. Drop `self_type_generics`. Both fields stay bare `#[resolve_field]`. The `Slot` is in the path for `item`, leftover, and the gap, the same way a level is in the path for `{ foo bar }`.

`Slot::Parent` is `<T::Parent as PathParent>::Parent`. `T::Parent` is the path to this `Slot`. That path's `parent` is the list (`IsoLiteralParsePath` at the root, later `SelectionSetPath`). Live `IsoLiteralItem` and `UnparsedChunkItems` already have `parent_type = SlotPath`. They stay that way.

`IsographResolutionNode` has one variant per `Slot<T, E>`. The root is `IsoLiteralSlot(IsoLiteralSlotPath<'a>)`. A later list adds `SelectionSlot(SelectionSlotPath<'a>)`. Live `SlotPath` is this alias renamed. There is no `IsographResolutionNode::Slot`.

Leftover span stays tight to the leftover tokens. The space after `foo` in `entrypoint Query.foo bar` is that gap.

## After

```rust
// from crates/isograph_parser/src/chunk.rs
/// One parse attempt: an item plus leftover tokens in the same chunk.
///
/// `resolve` receives the list parent (`IsoLiteralParsePath` at the root, later
/// `SelectionSetPath`). That value is `<T::Parent as PathParent>::Parent`. `T::Parent`
/// is the path to this `Slot`, so the list parent is that path's `parent` field.
/// The owned `Slot` does not carry a lifetime.
///
/// Walk, given that list parent:
/// - Position in `item`: bare `#[resolve_field]` passes `self.path(parent)`, a path
///   to this `Slot`. `T::Parent` is that path. `Slot` is a path segment.
/// - Position in `extra_tokens`: the same `self.path(parent)`. `E::Parent` is that
///   path (`UnparsedChunkItems` at the root, same as live).
/// - Position in the slot span but in neither field: `on_unmatched_span = from_path`
///   returns `self.path(parent).to()`. Each `Slot<T, E>` supplies a `From` that
///   builds its `ResolvedNode` variant (`IsoLiteralSlot`, later `SelectionSlot`).
///
/// `Parent` and `ResolvedNode` are GATs (`type Parent<'a> where Self: 'a`). A
/// path holds `&'a` the node; that `'a` is the resolve borrow of the tree, not
/// a lifetime stored in `Slot`.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = <<T as ResolvePosition>::Parent<'a> as PathParent>::Parent,
    resolved_node = <T as ResolvePosition>::ResolvedNode<'a>,
    on_unmatched_span = from_path
)]
pub struct Slot<T: ResolvePosition, E: ResolvePosition> {
    /// `Some` when the form parsed.
    #[resolve_field]
    pub item: Option<WithSpan<T>>,
    /// Unread or failed tokens after the item. Span is tight to those tokens.
    #[resolve_field]
    pub extra_tokens: Option<WithSpan<E>>,
}
```

`chunk.rs` imports `PathParent` from `resolve_position`. `Slot::resolve` is called with the list parent. Both fields pass `self.path(parent)` into the child. Hover on `Query` is `entrypoint -> slot -> literal`. Hover on leftover `bar` is the leftover token through `UnparsedChunkItems` through the same `Slot` path. Hover on the space after `foo` is the `Slot`.

`on_unmatched_span = from_path` means the no-hit arm is `return self.path(parent).to()`, not `ResolvedNode::Slot(...)`. Each `Slot<T, E>` writes a `From` into `T::ResolvedNode` that builds its own variant. At the root that is `IsographResolutionNode`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub type IsoLiteralSlotPath<'a> = PositionResolutionPath<
    &'a Slot<IsoLiteralItem, UnparsedChunkItems>,
    IsoLiteralParsePath<'a>,
>;

impl<'a> From<IsoLiteralSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: IsoLiteralSlotPath<'a>) -> Self {
        IsographResolutionNode::IsoLiteralSlot(path)
    }
}
```

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    IsoLiteralSlot(IsoLiteralSlotPath<'a>),
```

Live `SlotPath` is `IsoLiteralSlotPath`. `IsoLiteralItem`, `EntrypointDeclaration`, and `UnparsedChunkItems` keep `parent_type = IsoLiteralSlotPath<'a>` (today `SlotPath`). `UnparsedChunkItemsPath` stays `PositionResolutionPath<&'a UnparsedChunkItems, IsoLiteralSlotPath<'a>>`.

A later list that stores a `Slot` adds a `ResolvedNode` variant whose payload is that `Slot<T, E>`'s path, a `From` into `IsographResolutionNode`, and the same `parent_type =` that list's slot path on its item type. `IsoLiteralSlotPath` is never an enum of other lists' slots.

A later list that reuses `UnparsedChunkItems` cannot keep `parent_type = IsoLiteralSlotPath`. That leftover parent becomes an enum of those slot paths. Not this doc.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[resolve_position(parent_type = IsoLiteralSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
}

#[resolve_position(parent_type = IsoLiteralSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntrypointDeclaration { /* fields unchanged */ }

pub type EntrypointDeclarationPath<'a> =
    PositionResolutionPath<&'a EntrypointDeclaration, IsoLiteralSlotPath<'a>>;
```

Hover that matches `IsoLiteralSlot` reads `path.inner.item`; `None` is a noop.

## Before

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = IsoLiteralParsePath<'a>,
    resolved_node = IsographResolutionNode<'a>,
    self_type_generics = <IsoLiteralItem, UnparsedChunkItems>
)]
pub struct Slot<T, E> {
    #[resolve_field]
    pub item: Option<WithSpan<T>>,
    #[resolve_field]
    pub extra_tokens: Option<WithSpan<E>>,
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub type SlotPath<'a> =
    PositionResolutionPath<&'a Slot<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>>;

#[resolve_position(parent_type = SlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
}

#[resolve_position(parent_type = SlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntrypointDeclaration { /* ... */ }

pub type UnparsedChunkItemsPath<'a> = PositionResolutionPath<&'a UnparsedChunkItems, SlotPath<'a>>;

pub type EntrypointDeclarationPath<'a> =
    PositionResolutionPath<&'a EntrypointDeclaration, SlotPath<'a>>;
```

```rust
// from crates/isograph_parser/src/chunk.rs
#[resolve_position(parent_type = SlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct UnparsedChunkItems(
    #[resolve_field]
    #[parent_variant(Unparsed)]
    pub NonEmpty<WithSpan<ChunkContentItem>>,
);
```

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    Slot(SlotPath<'a>),
```

## Change 1: extra predicates on the associated types

Live `handle_data_struct` emits only `where Self: 'a` on both associated types. A generic `Slot<T, E>` body does not type-check without more predicates. `for<'a> ...` on the impl is `'static` on rustc 1.97. The predicates go on `type Parent<'a>` and `type ResolvedNode<'a>` for that `'a`. `handle_data_enum` is unchanged.

The generated `resolve` body, with both fields bare `#[resolve_field]`:

```
return item.item.resolve(self.path(parent), position);
return extra.item.resolve(self.path(parent), position);
return self.path(parent).to();
```

`self.path(parent)` is `PositionResolutionPath<&'a Slot<T, E>, Slot::Parent<'a>>`. `item.resolve` wants `T::Parent`. `extra.resolve` wants `E::Parent`. The miss arm `.to()` wants `From` into `#resolved_node`.

On `type Parent<'a>`:

- `Self: 'a`
- If `#parent_type` is `<X as Trait>::Assoc`, then `X: Trait`. For `Slot` that is `<T as ResolvePosition>::Parent<'a>: PathParent`. Live `parent_type = IsoLiteralParsePath<'a>` has no `as`, so no bound.

On `type ResolvedNode<'a>`:

- `Self: 'a`
- Each non-transparent resolve field `F`: `F: ResolvePosition<ResolvedNode<'a> = #resolved_node>`
- Each `ContainerPath` field `F`: `F: ResolvePosition<Parent<'a> = PositionResolutionPath<&'a #struct_name #ty_generics, #parent_type>>`
- The `from_path` unmatched arm: `#resolved_node: From<PositionResolutionPath<&'a #struct_name #ty_generics, #parent_type>>`

`EntrypointDeclaration` gets `EntityName::Parent = Path<&EntrypointDeclaration, SlotPath>` from the `ContainerPath` rule. Rustc already knew that from the concrete types. Live pinned `Slot` is still `self_type_generics` and `struct_name`, so this change adds concrete `IsoLiteralItem` / `UnparsedChunkItems` `ResolvedNode` equality and `Parent = IsoLiteralSlotPath`. No `from_path` bound yet. No `PathParent` bound: live `parent_type` is `IsoLiteralParsePath<'a>`, not a `PathParent` projection.

The derive does not name `T` and `E`. `F` is the inner type of a resolve field. A later `Singleton<T, E>` with `ContainerPath` fields gets the same `Parent` / `ResolvedNode` equalities from its fields. Different parameter names do not change the derive. A new parent construction that needs a new proof would.

Origin: `handle_data_struct` in `crates/resolve_position_macros/src/resolve_position_macro.rs`. Delta: `split_for_impl` moves above the unmatched arm so `ty_generics` is in scope. `type Parent<'a>` uses `parent_predicates`. `type ResolvedNode<'a>` uses `resolved_node_predicates`. The `from_path` arm also pushes the path `From`. Omitted and `struct_name` come first in the match; live has `from_path`, then `None`, then `struct_name`.

Before:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
            type Parent<'a>
                = #parent_type
            where
                Self: 'a;
            type ResolvedNode<'a>
                = #resolved_node
            where
                Self: 'a;
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        match on_unmatched_span.reference() {
            Some(ident) if ident == "from_path" => quote! {
                return self.path(parent).to();
            },
            None => quote! {
                return Self::ResolvedNode::#struct_name(self.path(parent).to());
            },
            Some(ident) if ident == "struct_name" => quote! {
                return Self::ResolvedNode::#struct_name(self.path(parent).to());
            },
            Some(ident) => Error::new_spanned(
                ident,
                "expected `on_unmatched_span = from_path` or `struct_name`",
            )
            .to_compile_error(),
        }
```

After. `split_for_impl` is unchanged and sits above this. `has_transparent` is the live `any` named once.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
    let mut parent_predicates = quote!(Self: 'a).wrap_vec();
    if let Some(bound) = qself_trait_bound(parent_type.reference()) {
        parent_predicates.push(bound);
    }

    let mut resolved_node_predicates = field_resolved_node_predicates(
        field_infos.reference(),
        resolved_node.reference(),
        parent_type.reference(),
        struct_name.reference(),
        ty_generics.reference(),
    );

    let unmatched = if has_transparent {
        quote!()
    } else {
        match on_unmatched_span.reference() {
            None => quote! {
                return Self::ResolvedNode::#struct_name(self.path(parent).to());
            },
            Some(ident) if ident == "struct_name" => quote! {
                return Self::ResolvedNode::#struct_name(self.path(parent).to());
            },
            Some(ident) if ident == "from_path" => {
                resolved_node_predicates.push(from_path_predicate(
                    resolved_node.reference(),
                    struct_name.reference(),
                    ty_generics.reference(),
                    parent_type.reference(),
                ));
                quote! {
                    return self.path(parent).to();
                }
            }
            Some(ident) => Error::new_spanned(
                ident,
                "expected `on_unmatched_span = from_path` or `struct_name`",
            )
            .to_compile_error(),
        }
    };
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
            type Parent<'a>
                = #parent_type
            where
                #(#parent_predicates),*;
            type ResolvedNode<'a>
                = #resolved_node
            where
                #(#resolved_node_predicates),*;
```

`<X as Trait>::Assoc` is a syn `Type::Path` with `QSelf`. `qself.ty` is `X`. `qself.position` is the index of `Assoc` in the path. The segments before that are `Trait`.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
fn qself_trait_bound(parent_type: &syn::Type) -> Option<proc_macro2::TokenStream> {
    let syn::Type::Path(type_path) = parent_type else {
        return None;
    };
    let qself = type_path.qself.as_ref()?;
    let Some(_) = qself.as_token else {
        return None;
    };
    let mut trait_path = type_path.path.clone();
    trait_path.segments = type_path
        .path
        .segments
        .iter()
        .take(qself.position)
        .cloned()
        .collect();
    let inner = qself.ty.reference();
    quote!(#inner: #trait_path).wrap_some()
}
```

`Option` / `Vec` / `NonEmpty` unwrap to the inner `WithSpan` type, same as the resolve walk. `transparent` fields are skipped.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
fn resolve_field_inner_type(wrapper: &ResolveFieldInfoTypeWrapper) -> Option<&syn::Type> {
    match wrapper {
        ResolveFieldInfoTypeWrapper::None(inner) => match (**inner).reference() {
            ResolveFieldInfoType::WithSpan(inner_type)
            | ResolveFieldInfoType::WithLocation(inner_type)
            | ResolveFieldInfoType::WithEmbeddedLocation(inner_type)
            | ResolveFieldInfoType::GraphQLTypeAnnotation(inner_type) => inner_type.wrap_some(),
        },
        ResolveFieldInfoTypeWrapper::IteratorWrapper(inner) => resolve_field_inner_type(inner),
        ResolveFieldInfoTypeWrapper::Transparent(_) => None,
    }
}

fn field_resolved_node_predicates(
    field_infos: &[ResolveFieldInfo],
    resolved_node: &syn::Type,
    parent_type: &syn::Type,
    struct_name: &syn::Ident,
    ty_generics: &proc_macro2::TokenStream,
) -> Vec<proc_macro2::TokenStream> {
    let mut predicates = quote!(Self: 'a).wrap_vec();
    for info in field_infos {
        let Some(inner_type) = resolve_field_inner_type(info.field_type.reference()) else {
            continue;
        };
        predicates.push(quote! {
            #inner_type: ::resolve_position::ResolvePosition<
                ResolvedNode<'a> = #resolved_node
            >
        });
        if matches!(info.parent_construction, ParentConstruction::ContainerPath) {
            predicates.push(quote! {
                #inner_type: ::resolve_position::ResolvePosition<
                    Parent<'a> = ::resolve_position::PositionResolutionPath<
                        &'a #struct_name #ty_generics,
                        #parent_type
                    >
                >
            });
        }
    }
    predicates
}

fn from_path_predicate(
    resolved_node: &syn::Type,
    struct_name: &syn::Ident,
    ty_generics: &proc_macro2::TokenStream,
    parent_type: &syn::Type,
) -> proc_macro2::TokenStream {
    quote! {
        #resolved_node: ::std::convert::From<
            ::resolve_position::PositionResolutionPath<&'a #struct_name #ty_generics, #parent_type>
        >
    }
}
```

Live pinned `Slot` after this change, unmatched still `struct_name`:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for Slot<IsoLiteralItem, UnparsedChunkItems> {
    type Parent<'a>
        = IsoLiteralParsePath<'a>
    where
        Self: 'a;
    type ResolvedNode<'a>
        = IsographResolutionNode<'a>
    where
        Self: 'a,
        IsoLiteralItem: ::resolve_position::ResolvePosition<
            ResolvedNode<'a> = IsographResolutionNode<'a>
        >,
        IsoLiteralItem: ::resolve_position::ResolvePosition<
            Parent<'a> = ::resolve_position::PositionResolutionPath<
                &'a Slot<IsoLiteralItem, UnparsedChunkItems>,
                IsoLiteralParsePath<'a>
            >
        >,
        UnparsedChunkItems: ::resolve_position::ResolvePosition<
            ResolvedNode<'a> = IsographResolutionNode<'a>
        >,
        UnparsedChunkItems: ::resolve_position::ResolvePosition<
            Parent<'a> = ::resolve_position::PositionResolutionPath<
                &'a Slot<IsoLiteralItem, UnparsedChunkItems>,
                IsoLiteralParsePath<'a>
            >
        >;
```

Those `Parent` equalities are live `SlotPath`. `cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.

## Change 2: generic `Slot`

The After listings. Generated expansion:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl<T: ResolvePosition, E: ResolvePosition> ::resolve_position::ResolvePosition for Slot<T, E> {
    type Parent<'a>
        = <<T as ::resolve_position::ResolvePosition>::Parent<'a> as ::resolve_position::PathParent>::Parent
    where
        Self: 'a,
        <T as ::resolve_position::ResolvePosition>::Parent<'a>: ::resolve_position::PathParent;
    type ResolvedNode<'a>
        = <T as ::resolve_position::ResolvePosition>::ResolvedNode<'a>
    where
        Self: 'a,
        T: ::resolve_position::ResolvePosition<
            ResolvedNode<'a> = <T as ::resolve_position::ResolvePosition>::ResolvedNode<'a>
        >,
        T: ::resolve_position::ResolvePosition<
            Parent<'a> = ::resolve_position::PositionResolutionPath<
                &'a Slot<T, E>,
                <<T as ::resolve_position::ResolvePosition>::Parent<'a> as ::resolve_position::PathParent>::Parent
            >
        >,
        E: ::resolve_position::ResolvePosition<
            ResolvedNode<'a> = <T as ::resolve_position::ResolvePosition>::ResolvedNode<'a>
        >,
        E: ::resolve_position::ResolvePosition<
            Parent<'a> = ::resolve_position::PositionResolutionPath<
                &'a Slot<T, E>,
                <<T as ::resolve_position::ResolvePosition>::Parent<'a> as ::resolve_position::PathParent>::Parent
            >
        >,
        <T as ::resolve_position::ResolvePosition>::ResolvedNode<'a>: ::std::convert::From<
            ::resolve_position::PositionResolutionPath<
                &'a Slot<T, E>,
                <<T as ::resolve_position::ResolvePosition>::Parent<'a> as ::resolve_position::PathParent>::Parent
            >,
        >;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        for item in self.item.iter() {
            if item.location.contains(position) {
                let new_parent = self.path(parent);
                return item.item.resolve(new_parent, position);
            }
        }
        for item in self.extra_tokens.iter() {
            if item.location.contains(position) {
                let new_parent = self.path(parent);
                return item.item.resolve(new_parent, position);
            }
        }
        return self.path(parent).to();
    }
}
```

`split_for_impl` still keeps `T: ResolvePosition, E: ResolvePosition`. `where Self: 'a` on the associated types makes `parent_type = <<T as ResolvePosition>::Parent<'a> as PathParent>::Parent` legal.

Take `entrypoint Query.foo bar`:

- `item` span is `entrypoint Query.foo`
- leftover span is tight to `bar`
- the space after `foo` is in the slot span and in neither field

`Query` answers through `EntrypointDeclaration` with parent `IsoLiteralSlotPath`. `bar` answers the leftover token; `UnparsedChunkItems`'s parent is `IsoLiteralSlotPath`. The space answers `IsographResolutionNode::IsoLiteralSlot(path)` with `path.inner: &Slot<IsoLiteralItem, UnparsedChunkItems>`. `{ item: None, extra_tokens: None }` has no field hits, so the same unmatched-span arm answers `IsoLiteralSlot`.

### Macro test

```rust
// from crates/resolve_position_macros/tests/path_parent_slot.rs
use prelude::Postfix;
use resolve_position::{PathParent, PositionResolutionPath, ResolvePosition};
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan, WithSpanPostfix};

#[derive(Debug)]
enum TestResolvedNode<'a> {
    List(ParentPath<'a>),
    Slot(SlotPath<'a>),
    Child(ChildPath<'a>),
    Extra(ExtraPath<'a>),
}

impl<'a> From<SlotPath<'a>> for TestResolvedNode<'a> {
    fn from(path: SlotPath<'a>) -> Self {
        TestResolvedNode::Slot(path)
    }
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = TestResolvedNode<'a>)]
struct List(#[resolve_field] Vec<WithSpan<Slot<Child, Extra>>>);

type ParentPath<'a> = PositionResolutionPath<&'a List, ()>;

type SlotPath<'a> = PositionResolutionPath<&'a Slot<Child, Extra>, ParentPath<'a>>;

#[derive(Debug, ResolvePosition)]
#[resolve_position(
    parent_type = <<T as ResolvePosition>::Parent<'a> as PathParent>::Parent,
    resolved_node = <T as ResolvePosition>::ResolvedNode<'a>,
    on_unmatched_span = from_path
)]
struct Slot<T: ResolvePosition, E: ResolvePosition> {
    #[resolve_field]
    item: Option<WithSpan<T>>,
    #[resolve_field]
    extra_tokens: Option<WithSpan<E>>,
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = SlotPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct Child;

type ChildPath<'a> = PositionResolutionPath<&'a Child, SlotPath<'a>>;

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = SlotPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct Extra;

type ExtraPath<'a> = PositionResolutionPath<&'a Extra, SlotPath<'a>>;

fn slot_list(slot: Slot<Child, Extra>, span: Span) -> List {
    List(slot.with_span(span).wrap_vec())
}

#[test]
fn an_item_resolves_to_the_child_with_the_slot_path() {
    let list = slot_list(
        Slot {
            item: Child.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: None,
        },
        Span::new(0, 4),
    );

    match list.resolve((), Span::new(1, 2)) {
        TestResolvedNode::Child(path) => {
            assert!(std::ptr::eq(path.parent.inner, list.0[0].item.reference()));
        }
        node => panic!("expected the child, got {node:?}"),
    }
}

#[test]
fn leftover_resolves_with_the_slot_path() {
    let list = slot_list(
        Slot {
            item: Child.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: Extra.with_span(Span::new(6, 8)).wrap_some(),
        },
        Span::new(0, 8),
    );

    match list.resolve((), Span::new(6, 7)) {
        TestResolvedNode::Extra(path) => {
            assert!(std::ptr::eq(path.parent.inner, list.0[0].item.reference()));
        }
        node => panic!("expected leftover, got {node:?}"),
    }
}

#[test]
fn a_gap_after_the_item_resolves_to_the_slot() {
    let list = slot_list(
        Slot {
            item: Child.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: Extra.with_span(Span::new(6, 8)).wrap_some(),
        },
        Span::new(0, 8),
    );

    match list.resolve((), Span::new(4, 5)) {
        TestResolvedNode::Slot(path) => {
            assert!(path.inner.item.is_some());
            assert!(path.inner.extra_tokens.is_some());
        }
        node => panic!("expected the slot, got {node:?}"),
    }
}

#[test]
fn an_empty_slot_resolves_to_the_slot() {
    let list = slot_list(
        Slot {
            item: None,
            extra_tokens: None,
        },
        Span::new(0, 4),
    );

    match list.resolve((), Span::new(1, 2)) {
        TestResolvedNode::Slot(path) => {
            assert!(path.inner.item.is_none());
            assert!(path.inner.extra_tokens.is_none());
        }
        node => panic!("expected the slot, got {node:?}"),
    }
}

#[test]
fn a_position_outside_the_slot_resolves_to_the_list() {
    let list = slot_list(
        Slot {
            item: Child.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: None,
        },
        Span::new(0, 4),
    );

    match list.resolve((), Span::new(10, 11)) {
        TestResolvedNode::List(path) => {
            assert!(std::ptr::eq(path.inner, list.reference()));
        }
        node => panic!("expected the list, got {node:?}"),
    }
}
```

The test `Slot` has no `where` clause. Change 1's predicates on the associated types are what make it compile. `List.0` is the vec field; the first test compares `path.parent.inner` to that `Slot`.

### Parser tests

Entrypoint tests keep passing. `names_resolve_to_their_leaves_and_the_rest_to_the_declaration` still resolves `Query` / `foo` / `entrypoint` / `.` through `item`; those paths still go through `IsoLiteralSlotPath`. A leftover token still resolves to `NonBracketToken`. `token.parent` inside leftover is `ChunkContentItemParent::Unparsed`; that path's parent is `IsoLiteralSlotPath`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn a_gap_after_the_item_resolves_to_the_slot() {
        let text = "entrypoint Query.foo bar";
        let (parse, _) = parsed(text);
        let gap = Span::new(span_of(text, "foo").end, span_of(text, "bar").start);
        match parse.resolve((), gap) {
            IsographResolutionNode::IsoLiteralSlot(path) => {
                assert!(path.inner.item.is_some());
                assert!(path.inner.extra_tokens.is_some());
            }
            node => panic!("expected IsoLiteralSlot, got {node:?}"),
        }
    }
```

`leftover_after_an_entrypoint_resolves_to_the_leftover_token` stays.

`cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.

## Landing checklist

1. Change 1: extra predicates on `type Parent<'a>` and `type ResolvedNode<'a>`, `qself_trait_bound`, `ContainerPath` parent equality, unmatched match order. `cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.
2. Change 2: generic `Slot`, `From<IsoLiteralSlotPath> for IsographResolutionNode`, `IsoLiteralSlot` replaces `Slot`, `SlotPath` renamed, `path_parent_slot.rs`, the parser tests above. Leftover span is unchanged. Item and leftover parents stay the slot path. `cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.
3. Move this doc to refactors/past.
