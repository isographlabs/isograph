# from-container-parent-field: `#[from_container_parent]` on a struct field

A struct field may take `#[from_container_parent]`. The child's parent is `From::from(self.path(parent))`. Enum payloads keep the live emission. Lands before parse-arguments.md. parse-arguments.md is the first caller on `Slot.extra_tokens`.

`UnparsedChunkItems` is `E` on every `Slot` pin. One pin: leftover's parent is `IsoLiteralSlotPath`, and `extra_tokens` is bare `#[resolve_field]`. A second pin: leftover is still one type, so its parent is an enum of the slot paths. `extra_tokens` cannot stay bare (`Parent` is no longer the slot path) and cannot use `#[parent_variant]` (one field, a different variant per pin). It uses `#[from_container_parent]`.

The live attribute is enum-payload only. A struct field with that attribute is a compile error: `` `#[from_container_parent]` is an enum-payload attribute ``.

## Change 1: struct-field `#[from_container_parent]`

Origin: `get_resolve_field_info` and `new_parent_expr` in `crates/resolve_position_macros/src/resolve_position_macro.rs`. Delta: a struct field may take `#[from_container_parent]`. The child's parent is `From::from(self.path(parent))`. Enum payloads stay `From::from(parent)`.

Before:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
enum ParentConstruction {
    ContainerPath,
    EnumVariant(syn::Ident),
    Transparent,
}
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
            if let Some(attr) = from_container_parent {
                parse_from_container_parent(attr)?;
                return Error::new_spanned(
                    attr,
                    "`#[from_container_parent]` is an enum-payload attribute",
                )
                .to_compile_error()
                .wrap_err();
            }
            match parent_variant {
                Some(attr) => ParentConstruction::EnumVariant(parse_parent_variant(attr)?),
                None => ParentConstruction::ContainerPath,
            }
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        ParentConstruction::ContainerPath => quote!(self.path(parent)),
        ParentConstruction::EnumVariant(variant) => quote!(
            <#inner_type as ::resolve_position::ResolvePosition>::Parent::#variant(self.path(parent).into())
        ),
        ParentConstruction::Transparent => {
            Error::new_spanned(inner_type, "`transparent` does not build a field parent")
                .to_compile_error()
        }
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
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
```

After:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
enum ParentConstruction {
    ContainerPath,
    EnumVariant(syn::Ident),
    FromContainer,
    Transparent,
}
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
            if let Some(attr) = from_container_parent {
                parse_from_container_parent(attr)?;
                ParentConstruction::FromContainer
            } else {
                match parent_variant {
                    Some(attr) => ParentConstruction::EnumVariant(parse_parent_variant(attr)?),
                    None => ParentConstruction::ContainerPath,
                }
            }
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        ParentConstruction::ContainerPath => quote!(self.path(parent)),
        ParentConstruction::EnumVariant(variant) => quote!(
            <#inner_type as ::resolve_position::ResolvePosition>::Parent::#variant(self.path(parent).into())
        ),
        ParentConstruction::FromContainer => {
            quote!(::std::convert::From::from(self.path(parent)))
        }
        ParentConstruction::Transparent => {
            Error::new_spanned(inner_type, "`transparent` does not build a field parent")
                .to_compile_error()
        }
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
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
        if matches!(info.parent_construction, ParentConstruction::FromContainer) {
            predicates.push(quote! {
                <#inner_type as ::resolve_position::ResolvePosition>::Parent<'a>:
                    ::std::convert::From<
                        ::resolve_position::PositionResolutionPath<
                            &'a #struct_name #ty_generics,
                            #parent_type
                        >
                    >
            });
        }
```

Combining `#[parent_variant]` and `#[from_container_parent]` stays a compile error. `#[resolve_field(transparent)]` plus `#[from_container_parent]` stays a compile error.

The generated `extra_tokens` arm:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
        for item in self.extra_tokens.iter() {
            if item.location.contains(position) {
                let new_parent = ::std::convert::From::from(self.path(parent));
                return item.item.resolve(new_parent, position);
            }
        }
```

Origin for the test: `crates/resolve_position_macros/tests/self_type_generics_pins.rs`. Delta: `E` is one type whose parent is an enum of the two slot paths, and `extra_tokens` takes `#[from_container_parent]`.

```rust
// from crates/resolve_position_macros/tests/from_container_parent_field.rs
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
```

`cargo test -p resolve_position_macros` passes. `cargo test -p isograph_parser` passes. `Slot.extra_tokens` in the parser stays bare until parse-arguments.md.

## Landing checklist

1. Struct-field `#[from_container_parent]`, the macro test. `cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.
2. Move this doc to refactors/past.
