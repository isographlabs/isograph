# generic-slot: one impl for every `Slot<T, E>`

Lands after resolve-position-on-unmatched-span.md (refactors/past).

`Slot` is used at the root and in every list. One pinned impl cannot cover `Slot<P, UnparsedChunkItems>` for a later list item `P`. Drop `self_type_generics`. Both fields use `from_container_parent`. A position in a field skips `Slot` in the path. A position in the slot span but in neither field answers that `Slot<T, E>`'s `ResolvedNode` variant, including `{ item: None, extra_tokens: None }`.

`IsographResolutionNode` has one variant per `Slot<T, E>`. The root is `IsoLiteralSlot(IsoLiteralSlotPath<'a>)`. A later list adds `SelectionSlot(SelectionSlotPath<'a>)`. `IsoLiteralSlotPath` is `PositionResolutionPath<&'a Slot<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>>`. Live `SlotPath` is this alias renamed. There is no `IsographResolutionNode::Slot`.

Leftover span stays tight to the leftover tokens. The space after `foo` in `entrypoint Query.foo bar` is that gap.

## After

```rust
// from crates/isograph_parser/src/chunk.rs
/// One parse attempt: an item plus leftover tokens in the same chunk.
///
/// `resolve` receives the list parent (`IsoLiteralParsePath` at the root, later
/// `SelectionSetPath`, and so on). That parent is `T::Parent`, so `parent_type`
/// is `T::Parent` and the owned tree does not carry a lifetime.
///
/// Walk, given that parent:
/// - Position in `item`: `from_container_parent` passes `From::from(parent)` as `T::Parent`
///   (identity). `Slot` is not a path segment.
/// - Position in `extra_tokens`: `from_container_parent` passes `From::from(parent)` as
///   `E::Parent` (wrap, `UnparsedChunkItemsParent::Literal` at the root).
/// - Position in the slot span but in neither field: `on_unmatched_span = from_path`
///   returns `self.path(parent).to()`. Each `Slot<T, E>` supplies a `From` that
///   builds its `ResolvedNode` variant (`IsoLiteralSlot`, later `SelectionSlot`).
///
/// `Parent` and `ResolvedNode` are GATs (`type Parent<'a> where Self: 'a`). A
/// path holds `&'a` the node; that `'a` is the resolve borrow of the tree, not
/// a lifetime stored in `Slot`.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = <T as ResolvePosition>::Parent<'a>,
    resolved_node = <T as ResolvePosition>::ResolvedNode<'a>,
    on_unmatched_span = from_path
)]
pub struct Slot<T: ResolvePosition, E: ResolvePosition> {
    /// `Some` when the form parsed.
    #[resolve_field]
    #[from_container_parent]
    pub item: Option<WithSpan<T>>,
    /// Unread or failed tokens after the item. Span is tight to those tokens.
    #[resolve_field]
    #[from_container_parent]
    pub extra_tokens: Option<WithSpan<E>>,
}
```

`Slot::Parent` is `T::Parent`. `Slot::ResolvedNode` is `T::ResolvedNode`. Both fields emit `From::from(parent)`. `on_unmatched_span = from_path` means the no-hit arm is `return self.path(parent).to()`, not `ResolvedNode::Slot(...)`. Each `Slot<T, E>` writes a `From` into `T::ResolvedNode` that builds its own variant. At the root that is `IsographResolutionNode`.

At the root, `T` is `IsoLiteralItem` and `E` is `UnparsedChunkItems`. After this doc, `IsoLiteralItem::Parent` is `IsoLiteralParsePath`. The generated leftover arm is:

```
let new_parent = From::from(parent);
return extra.item.resolve(new_parent, position);
```

`parent` is `IsoLiteralParsePath`. `UnparsedChunkItems::resolve` wants `UnparsedChunkItems::Parent`, which is `UnparsedChunkItemsParent`. `From::from` is the wrap `UnparsedChunkItemsParent::Literal(parent)`. The compiler requires `UnparsedChunkItemsParent: From<IsoLiteralParsePath>`.

The item arm is the same `From::from(parent)`, but the target is `IsoLiteralItem::Parent`, which is `IsoLiteralParsePath`. That is `From<P> for P`. No impl to write.

The leftover `From` bound is that fact for any `T` / `E`: `E::Parent: From<T::Parent>`. Leftover `resolve` returns `E::ResolvedNode<'a>`; `Slot::resolve` returns `T::ResolvedNode<'a>`. Those are the same type for this `'a`. The unmatched-span arm: `self.path(parent)` is `PositionResolutionPath<&Slot<T, E>, T::Parent>`. `.to()` requires `T::ResolvedNode: From<that path>`. Change 2 and change 3 put those predicates on `type ResolvedNode<'a>`.

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

A list that stores a `Slot` adds a `ResolvedNode` variant whose payload is that `Slot<T, E>`'s path, a `From` into `IsographResolutionNode`, an `UnparsedChunkItemsParent` variant, and a `From` into that. `IsoLiteralSlotPath` is never an enum of other lists' slots.

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug)]
pub enum UnparsedChunkItemsParent<'a> {
    Literal(IsoLiteralParsePath<'a>),
}

#[resolve_position(parent_type = UnparsedChunkItemsParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct UnparsedChunkItems(
    #[resolve_field]
    #[parent_variant(Unparsed)]
    pub NonEmpty<WithSpan<ChunkContentItem>>,
);

pub type UnparsedChunkItemsPath<'a> =
    PositionResolutionPath<&'a UnparsedChunkItems, UnparsedChunkItemsParent<'a>>;

impl<'a> From<IsoLiteralParsePath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(parent: IsoLiteralParsePath<'a>) -> Self {
        UnparsedChunkItemsParent::Literal(parent)
    }
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[resolve_position(parent_type = IsoLiteralParsePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
}

#[resolve_position(parent_type = IsoLiteralParsePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntrypointDeclaration { /* fields unchanged */ }

pub type EntrypointDeclarationPath<'a> =
    PositionResolutionPath<&'a EntrypointDeclaration, IsoLiteralParsePath<'a>>;
```

`UnparsedChunkItemsPath` moves from `parse_iso_literal.rs` to `chunk.rs`. `IsoLiteralItem` and `EntrypointDeclaration` no longer use `SlotPath` as `parent_type`. Hover that matches `IsoLiteralSlot` reads `path.inner.item`; `None` is a noop.

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
// from crates/isograph_parser/src/chunk.rs
#[resolve_position(parent_type = SlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct UnparsedChunkItems(
    #[resolve_field]
    #[parent_variant(Unparsed)]
    pub NonEmpty<WithSpan<ChunkContentItem>>,
);
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[resolve_position(parent_type = SlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
}

#[resolve_position(parent_type = SlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntrypointDeclaration { /* ... */ }

pub type UnparsedChunkItemsPath<'a> = PositionResolutionPath<&'a UnparsedChunkItems, SlotPath<'a>>;

pub type SlotPath<'a> =
    PositionResolutionPath<&'a Slot<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>>;

pub type EntrypointDeclarationPath<'a> =
    PositionResolutionPath<&'a EntrypointDeclaration, SlotPath<'a>>;
```

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    Slot(SlotPath<'a>),
```

## Change 1: rename `parent_from` to `from_container_parent`

Every `parent_from` ident and string in `crates/resolve_position_macros/src/lib.rs`, `crates/resolve_position_macros/src/resolve_position_macro.rs`, and `crates/resolve_position_macros/tests/generic_slot.rs` becomes `from_container_parent`. `parse_parent_from` becomes `parse_from_container_parent`. Behavior is unchanged: enum payloads still emit `From::from(parent)`; a struct field with the attribute still errors, now as "`#[from_container_parent]` is an enum-payload attribute".

Origin: those three files as they stand. Delta: the rename.

```rust
// from crates/resolve_position_macros/src/lib.rs
        from_container_parent,
        parent_variant,
        resolve_field,
        resolve_position,
        self_type_generics
```

```rust
// from crates/resolve_position_macros/tests/generic_slot.rs
    Unparsed(#[from_container_parent] Unparsed),
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
fn parse_from_container_parent(attr: &syn::Attribute) -> Result<(), proc_macro2::TokenStream> {
    match attr.meta.reference() {
        syn::Meta::Path(_) => ().wrap_ok(),
        _ => Error::new_spanned(attr, "expected `#[from_container_parent]`")
            .to_compile_error()
            .wrap_err(),
    }
}
```

`cargo test -p resolve_position_macros` passes.

## Change 2: extra predicates on `type ResolvedNode<'a>`

The generated `resolve` body returns a field's `resolve`, or on a miss `self.path(parent).to()`. The function's return type is `Self::ResolvedNode<'a>`, which is `#resolved_node`.

```
return item.item.resolve(new_parent, position);
return extra.item.resolve(new_parent, position);
return self.path(parent).to();
```

Those three expressions must be `#resolved_node`. On a concrete struct rustc checks the concrete associated types. On `Slot<T, E>` it cannot, unless the impl states the facts. Five of them; 3 lands in change 3, 5 is not a bound.

1. `T: ResolvePosition<ResolvedNode<'a> = #resolved_node>`

`item.resolve` returns `T::ResolvedNode<'a>`. `Slot::resolve` returns `#resolved_node`. Those are the same type. At the root `#resolved_node` is `T::ResolvedNode`, so this is identity. The derive still writes it: the inner type is `T`, not a special case.

2. `E: ResolvePosition<ResolvedNode<'a> = #resolved_node>`

Same return-type identity for leftover. `extra.resolve` returns `E::ResolvedNode<'a>`. That must be `T::ResolvedNode<'a>`. At the root both are `IsographResolutionNode`. A `Slot<Foo, Bar>` where `Bar` resolves to a different enum does not compile.

3. `E::Parent<'a>: From<T::Parent<'a>>`

Leftover does `From::from(parent)` with `parent: Slot::Parent`, which is `T::Parent`. Change 3 adds this when the field is `from_container_parent`. Change 2 does not emit it.

4. `#resolved_node: From<PositionResolutionPath<&'a Slot<T, E>, T::Parent<'a>>>`

The unmatched arm is `self.path(parent).to()`. `parent` is `Slot::Parent<'a>`, which is `T::Parent<'a>`. `path` builds `&Slot<T, E>` plus that parent. `.to()` is `From` into `#resolved_node`. Each `Slot<T, E>` writes that `From`. At the root the path type is `IsoLiteralSlotPath` and the impl builds `IsographResolutionNode::IsoLiteralSlot`. A later `Slot<Selection, UnparsedChunkItems>` writes a different impl that builds `SelectionSlot`. The generic body never names the variant. Syntactically the bound is on `#resolved_node`. The input type contains `T` and `E`, so instantiating `Slot<Foo, Bar>` without that `From` does not compile.

5. There is no `From<PositionResolutionPath<&'a Slot<T, E>, E::Parent<'a>>>`.

`path` is always called with `Self::Parent`, which is `T::Parent`. Leftover never builds a `Slot` path. Leftover converts `T::Parent` into `E::Parent` (bound 3) and calls `E::resolve`. The leftover leaf's path is `UnparsedChunkItemsPath`: `&UnparsedChunkItems` plus `E::Parent`. Bound 3 is that conversion. It does not produce a `Slot` path whose parent is `E::Parent`. That path type does not occur.

```
// leftover
let new_parent = From::from(parent);
return extra.item.resolve(new_parent, position);

// unmatched
return self.path(parent).to();
```

`for<'a> E: ResolvePosition<ResolvedNode<'a> = T::ResolvedNode<'a>>` on the impl is `E: 'static` on rustc 1.97. The predicates go on `type ResolvedNode<'a>` for that `'a`. `fn resolve` returns `Self::ResolvedNode<'a>`, so projecting that GAT brings them into scope for the body. `handle_data_enum` is unchanged.

The derive does not name `T` and `E`. `F` is the inner type of a resolve field, the same type `generate_resolve_code` already passes to `new_parent_expr`. `#resolved_node` and `#parent_type` are the attribute values. `#struct_name #ty_generics` is `Self` on the impl. A later `Singleton<T, E>` with resolve fields and `from_path` gets 1, 2, and 4 from its own fields. Different parameter names do not change the derive.

The walk changes when the generated body gains a new kind of expression that needs a proof. Change 3 is that: `From::from(parent)` adds bound 3. A new unmatched arm would add another predicate. A struct that only uses `#[resolve_field]` and `struct_name` does not need a derive edit.

Origin: `handle_data_struct` in `crates/resolve_position_macros/src/resolve_position_macro.rs`. Delta: `split_for_impl` moves above the unmatched arm so `ty_generics` is in scope for bound 4. `type ResolvedNode<'a>` uses `field_resolved_node_predicates`. The `from_path` arm also pushes `from_path_predicate`. Omitted and `struct_name` come first in the match; live has `from_path`, then `None`, then `struct_name`.

Before, the GAT and the live match:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
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
    let mut resolved_node_predicates =
        field_resolved_node_predicates(field_infos.reference(), resolved_node.reference());

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
            type ResolvedNode<'a>
                = #resolved_node
            where
                #(#resolved_node_predicates),*;
```

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

`Option` / `Vec` / `NonEmpty` unwrap to the inner `WithSpan` type, same as the resolve walk. `transparent` fields are skipped. Tautological equalities stay (bound 1 on `T` when `#resolved_node` is `T::ResolvedNode`).

Live pinned `Slot` after this change, `self_type_generics` still in place, unmatched still `struct_name`. Bounds 1 and 2, concrete. No bound 4:

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
        UnparsedChunkItems: ::resolve_position::ResolvePosition<
            ResolvedNode<'a> = IsographResolutionNode<'a>
        >;
```

`on_unmatched_span.rs` `Slot` is concrete, `from_path`. Bound 1 for `Child`, bound 4 for `TestResolvedNode<'a>: From<PositionResolutionPath<&'a Slot, ParentPath<'a>>>`. The `From` impl in that file is bound 4.

`cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.

## Change 3: `from_container_parent` on a struct field

`#[resolve_field]` + `#[from_container_parent]` on a struct field is accepted. Emission is `From::from(parent)`. `ParentConstruction` gains `FromContainerParent`. `field_resolved_node_predicates` gains the leftover `Parent: From` predicate for those fields.

`on_unmatched_span = from_path` is shipped (refactors/past/resolve-position-on-unmatched-span.md). This doc uses it.

Today `ParentConstruction` has no `FromContainerParent`: enum payloads emit `From::from` directly, and a struct field with `#[from_container_parent]` is an error. This change puts `FromContainerParent` on struct fields.

Origin: `ParentConstruction`, `get_resolve_field_info`, `new_parent_expr`, and `field_resolved_node_predicates` after change 2. Delta: the variant, the struct-field accept path, the `new_parent_expr` arm, and the `From` predicate.

Before:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
enum ParentConstruction {
    ContainerPath,
    EnumVariant(syn::Ident),
    Transparent,
}
```

After:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
enum ParentConstruction {
    ContainerPath,
    EnumVariant(syn::Ident),
    /// `#[resolve_field]` + `#[from_container_parent]`: the child's `Parent` is `From` the
    /// container's `Parent`.
    FromContainerParent,
    Transparent,
}
```

Before:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
            if let Some(attr) = from_container_parent {
                parse_from_container_parent(attr)?;
                return Error::new_spanned(attr, "`#[from_container_parent]` is an enum-payload attribute")
                    .to_compile_error()
                    .wrap_err();
            }
            match parent_variant {
                Some(attr) => ParentConstruction::EnumVariant(parse_parent_variant(attr)?),
                None => ParentConstruction::ContainerPath,
            }
```

After:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
            if let Some(attr) = from_container_parent {
                parse_from_container_parent(attr)?;
                ParentConstruction::FromContainerParent
            } else {
                match parent_variant {
                    Some(attr) => ParentConstruction::EnumVariant(parse_parent_variant(attr)?),
                    None => ParentConstruction::ContainerPath,
                }
            }
```

`Option<WithSpan<T>>` is already a legal field type. `new_parent_expr` gains a `FromContainerParent` arm:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        ParentConstruction::FromContainerParent => quote!(::std::convert::From::from(parent)),
        ParentConstruction::Transparent => {
            Error::new_spanned(inner_type, "`transparent` does not build a field parent")
                .to_compile_error()
        }
```

Origin: `field_resolved_node_predicates` after change 2. Delta: `parent_type` parameter; `FromContainerParent` pushes a `Parent: From` predicate.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
fn field_resolved_node_predicates(
    field_infos: &[ResolveFieldInfo],
    resolved_node: &syn::Type,
    parent_type: &syn::Type,
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
        if matches!(
            info.parent_construction,
            ParentConstruction::FromContainerParent
        ) {
            predicates.push(quote! {
                <#inner_type as ::resolve_position::ResolvePosition>::Parent<'a>:
                    ::std::convert::From<#parent_type>
            });
        }
    }
    predicates
}
```

The change 2 call becomes `field_resolved_node_predicates(field_infos.reference(), resolved_node.reference(), parent_type.reference())`.

### Test

```rust
// from crates/resolve_position_macros/tests/from_container_parent_struct.rs
use prelude::Postfix;
use resolve_position::{PositionResolutionPath, ResolvePosition};
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan, WithSpanPostfix};

#[derive(Debug)]
enum TestResolvedNode<'a> {
    List(ParentPath<'a>),
    Slot(PositionResolutionPath<&'a Slot<Child, Extra>, ParentPath<'a>>),
    Child(ChildPath<'a>),
    Extra(ExtraPath<'a>),
}

impl<'a> From<PositionResolutionPath<&'a Slot<Child, Extra>, ParentPath<'a>>> for TestResolvedNode<'a> {
    fn from(path: PositionResolutionPath<&'a Slot<Child, Extra>, ParentPath<'a>>) -> Self {
        TestResolvedNode::Slot(path)
    }
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = TestResolvedNode<'a>)]
struct List(#[resolve_field] Vec<WithSpan<Slot<Child, Extra>>>);

type ParentPath<'a> = PositionResolutionPath<&'a List, ()>;

#[derive(Debug, ResolvePosition)]
#[resolve_position(
    parent_type = <T as ResolvePosition>::Parent<'a>,
    resolved_node = <T as ResolvePosition>::ResolvedNode<'a>,
    on_unmatched_span = from_path
)]
struct Slot<T: ResolvePosition, E: ResolvePosition> {
    #[resolve_field]
    #[from_container_parent]
    item: Option<WithSpan<T>>,
    #[resolve_field]
    #[from_container_parent]
    extra_tokens: Option<WithSpan<E>>,
}

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = ParentPath<'a>, resolved_node = TestResolvedNode<'a>)]
struct Child;

type ChildPath<'a> = PositionResolutionPath<&'a Child, ParentPath<'a>>;

#[derive(Debug, ResolvePosition)]
#[resolve_position(parent_type = ExtraParent<'a>, resolved_node = TestResolvedNode<'a>)]
struct Extra;

type ExtraPath<'a> = PositionResolutionPath<&'a Extra, ExtraParent<'a>>;

#[derive(Debug)]
enum ExtraParent<'a> {
    List(ParentPath<'a>),
}

impl<'a> From<ParentPath<'a>> for ExtraParent<'a> {
    fn from(path: ParentPath<'a>) -> Self {
        ExtraParent::List(path)
    }
}

fn slot_list(slot: Slot<Child, Extra>, span: Span) -> List {
    List(slot.with_span(span).wrap_vec())
}

#[test]
fn an_item_resolves_to_the_child_with_the_list_path() {
    let list = slot_list(
        Slot {
            item: Child.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: None,
        },
        Span::new(0, 4),
    );

    match list.resolve((), Span::new(1, 2)) {
        TestResolvedNode::Child(path) => {
            assert!(std::ptr::eq(path.parent.inner, list.reference()));
        }
        node => panic!("expected the child, got {node:?}"),
    }
}

#[test]
fn leftover_resolves_through_from_the_list_path() {
    let list = slot_list(
        Slot {
            item: Child.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: Extra.with_span(Span::new(6, 8)).wrap_some(),
        },
        Span::new(0, 8),
    );

    match list.resolve((), Span::new(6, 7)) {
        TestResolvedNode::Extra(path) => match path.parent {
            ExtraParent::List(list_path) => {
                assert!(std::ptr::eq(list_path.inner, list.reference()));
            }
        },
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

The test `Slot` has no `where` clause. Change 2's predicates on `type ResolvedNode<'a>` are what make it compile. `cargo test -p resolve_position_macros` passes.

## Change 4: generic `Slot`

The After listings. Generated expansion, field order `item` then `extra_tokens`, then the `from_path` predicate:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl<T: ResolvePosition, E: ResolvePosition> ::resolve_position::ResolvePosition for Slot<T, E> {
    type Parent<'a>
        = <T as ::resolve_position::ResolvePosition>::Parent<'a>
    where
        Self: 'a;
    type ResolvedNode<'a>
        = <T as ::resolve_position::ResolvePosition>::ResolvedNode<'a>
    where
        Self: 'a,
        T: ::resolve_position::ResolvePosition<
            ResolvedNode<'a> = <T as ::resolve_position::ResolvePosition>::ResolvedNode<'a>
        >,
        <T as ::resolve_position::ResolvePosition>::Parent<'a>:
            ::std::convert::From<<T as ::resolve_position::ResolvePosition>::Parent<'a>>,
        E: ::resolve_position::ResolvePosition<
            ResolvedNode<'a> = <T as ::resolve_position::ResolvePosition>::ResolvedNode<'a>
        >,
        <E as ::resolve_position::ResolvePosition>::Parent<'a>:
            ::std::convert::From<<T as ::resolve_position::ResolvePosition>::Parent<'a>>,
        <T as ::resolve_position::ResolvePosition>::ResolvedNode<'a>: ::std::convert::From<
            ::resolve_position::PositionResolutionPath<
                &'a Slot<T, E>,
                <T as ::resolve_position::ResolvePosition>::Parent<'a>,
            >,
        >;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        for item in self.item.iter() {
            if item.location.contains(position) {
                let new_parent = ::std::convert::From::from(parent);
                return item.item.resolve(new_parent, position);
            }
        }
        for item in self.extra_tokens.iter() {
            if item.location.contains(position) {
                let new_parent = ::std::convert::From::from(parent);
                return item.item.resolve(new_parent, position);
            }
        }
        return self.path(parent).to();
    }
}
```

`split_for_impl` still keeps `T: ResolvePosition, E: ResolvePosition`. `where Self: 'a` on the associated types makes `parent_type = <T as ResolvePosition>::Parent<'a>` legal.

Take `entrypoint Query.foo bar`:

- `item` span is `entrypoint Query.foo`
- leftover span is tight to `bar`
- the space after `foo` is in the slot span and in neither field

That space answers `IsographResolutionNode::IsoLiteralSlot(path)` with `path.inner: &Slot<IsoLiteralItem, UnparsedChunkItems>`. A position on `bar` answers the token. `{ item: None, extra_tokens: None }` has no field hits, so the same unmatched-span arm answers `IsoLiteralSlot`.

### Parser tests

Entrypoint tests keep passing. A leftover token still resolves to `NonBracketToken`. `names_resolve_to_their_leaves_and_the_rest_to_the_declaration` still resolves `Query` / `foo` / `entrypoint` / `.` the same way: those positions are in `item`, so the path does not go through `Slot`. `token.parent` inside leftover is `ChunkContentItemParent::Unparsed`; that path's parent is `UnparsedChunkItemsParent::Literal`.

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

    #[test]
    fn leftover_token_parent_is_the_literal() {
        let text = "entrypoint Query.foo bar";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "bar")) {
            IsographResolutionNode::NonBracketToken(token) => match token.parent {
                ChunkContentItemParent::Unparsed(unparsed) => match unparsed.parent {
                    UnparsedChunkItemsParent::Literal(_) => {}
                },
                parent => panic!("expected an unparsed parent, got {parent:?}"),
            },
            node => panic!("expected the leftover token, got {node:?}"),
        }
    }
```

`leftover_after_an_entrypoint_resolves_to_the_leftover_token` stays.

`cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.

## Landing checklist

1. Change 1: rename `parent_from` to `from_container_parent`. Struct field still errors. `cargo test -p resolve_position_macros` passes.
2. Change 2: extra predicates on `type ResolvedNode<'a>`. `cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.
3. Change 3: `from_container_parent` on struct fields, `FromContainerParent`, the `From` predicate, `from_container_parent_struct.rs`. `cargo test -p resolve_position_macros` passes.
4. Change 4: generic `Slot`, `From<IsoLiteralSlotPath> for IsographResolutionNode`, `IsoLiteralSlot` replaces `Slot`, `UnparsedChunkItemsParent`, entrypoint parent paths, the parser tests above. Leftover span is unchanged. `cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.
5. Move this doc to refactors/past.
