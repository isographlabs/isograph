# resolve-field-parents: per-pin `parent_variant`

A pinned struct generates one `ResolvePosition` impl per pin. `#[parent_variant(V)]` is one ident, the same in every impl. Leftover's parent is an enum of those impls' slot paths, a different variant per pin. Today that field uses `#[parent_from]` and `From<SlotPath>`.

`From<SelectionSlotPath> for UnparsedChunkItemsParent` can produce one variant. Two fields of the same leftover type that need two variants have no spelling.

`#[resolve_field(parents = [A, B, ...])]` is `#[parent_variant]` indexed by pin. The list is the same length as `pins`. Pin `i` emits variant `parents[i]`. Two leftover fields each carry their own list.

One pin still produces one `impl ResolvePosition for Slot<T, E>`. `Slot<TypeAnnotation, UnparsedChunkItems>` cannot take two container `parent_type`s. `parents = [...]` names leftover variants; it does not add pins.

A struct with `parent_type = Ty` (no `pins`) keeps `#[parent_variant(V)]` / `#[parent_from]`. `ListTypeAnnotation.extra_tokens` stays `#[parent_from]`.

## `ResolveFieldForm`

Before:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
enum ResolveFieldForm {
    Bare,
    Transparent,
}
```

After:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
enum ResolveFieldForm {
    Bare,
    Transparent,
    Parents(Vec<syn::Ident>),
}
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
fn parse_resolve_field_form(
    attr: &syn::Attribute,
) -> Result<ResolveFieldForm, proc_macro2::TokenStream> {
    match attr.meta.reference() {
        syn::Meta::Path(_) => ResolveFieldForm::Bare.wrap_ok(),
        syn::Meta::List(_) => {
            if let Ok(path) = attr.parse_args::<syn::Path>()
                && path.is_ident("transparent")
            {
                return ResolveFieldForm::Transparent.wrap_ok();
            }
            if let Ok(parents) = attr.parse_args_with(parse_resolve_field_parents) {
                if parents.is_empty() {
                    return Error::new_spanned(attr, "`parents` must contain at least one ident")
                        .to_compile_error()
                        .wrap_err();
                }
                return ResolveFieldForm::Parents(parents).wrap_ok();
            }
            Error::new_spanned(
                attr.meta.reference(),
                "expected `#[resolve_field]`, `#[resolve_field(transparent)]`, or \
                 `#[resolve_field(parents = [A, B, ...])]`",
            )
            .to_compile_error()
            .wrap_err()
        }
        syn::Meta::NameValue(name_value) => Error::new_spanned(
            name_value,
            "expected `#[resolve_field]`, `#[resolve_field(transparent)]`, or \
             `#[resolve_field(parents = [A, B, ...])]`",
        )
        .to_compile_error()
        .wrap_err(),
    }
}

fn parse_resolve_field_parents(input: ParseStream) -> syn::Result<Vec<syn::Ident>> {
    let ident: syn::Ident = input.parse()?;
    if ident != "parents" {
        return Error::new(ident.span(), "expected `parents`").wrap_err();
    }
    input.parse::<syn::Token![=]>()?;
    let content;
    syn::bracketed!(content in input);
    let idents = syn::punctuated::Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated(
        content.reference(),
    )?;
    idents.into_iter().collect::<Vec<_>>().wrap_ok()
}
```

`Parents` cannot combine with `#[parent_variant]` or `#[parent_from]`. Same errors as `transparent` combining with those.

## Pin index when collecting fields

Before, the pin loop collects field infos with no pin index. `Parents` reduces to `ParentConstruction::EnumVariant` at that point, so `emit_one_impl` and `new_parent_expr` stay as they are.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
struct PinFieldContext {
    index: usize,
    count: usize,
}
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
            for (pin_index, pin) in pins.0.iter().enumerate() {
                let generics_map = match validate_and_map_generics(
                    input_generics.clone(),
                    pin.args.clone().wrap_some(),
                ) {
                    Ok(generics_map) => generics_map,
                    Err(e) => return e.to(),
                };
                let field_infos = match collect_field_infos(
                    &data_struct,
                    generics_map.reference(),
                    PinFieldContext {
                        index: pin_index,
                        count: pins.0.len(),
                    }
                    .wrap_some(),
                ) {
                    Ok(field_infos) => field_infos,
                    Err(e) => return e.to(),
                };
```

The no-`pins` impl passes `None`. `get_resolve_field_info` takes `Option<PinFieldContext>`.

`Parents` plus `None`: `` `#[resolve_field(parents = [...])]` requires `pins` ``.

`Parents` plus `idents.len() != context.count`: `` `parents` length must equal `pins` length ``.

`Parents` plus a matching length: `ParentConstruction::EnumVariant(idents[context.index].clone())`.

Generated leftover walk is the live `parent_variant` emission:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
            <#inner_type as ::resolve_position::ResolvePosition>::Parent::#variant(self.path(parent).into())
```

The `FromContainer` predicate is not added. The `From<SlotPath> for UnparsedChunkItemsParent` impls go away.

## `Slot.extra_tokens`

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
#[resolve_position(
    resolved_node = IsographResolutionNode<'a>,
    on_unmatched_span = from_path,
    pins = [
        (<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>),
        (<SelectionFieldArgument, UnparsedChunkItems>, ArgumentListPath<'a>),
        (<ObjectEntry, UnparsedChunkItems>, ObjectLiteralPath<'a>),
        (<Selection, UnparsedChunkItems>, SelectionSetPath<'a>),
    ]
)]
pub struct Slot<T, E> {
    #[resolve_field]
    pub item: Option<WithSpan<T>>,
    #[resolve_field]
    #[parent_from]
    pub extra_tokens: Option<WithSpan<E>>,
}
```

```rust
// from crates/isograph_parser/src/chunk.rs
impl<'a> From<IsoLiteralSlotPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: IsoLiteralSlotPath<'a>) -> Self {
        UnparsedChunkItemsParent::IsoLiteralSlot(path)
    }
}
```

The same `From` for `SelectionFieldArgumentSlotPath`, `ObjectEntrySlotPath`, `SelectionSlotPath`.

After:

```rust
// from crates/isograph_parser/src/chunk.rs
#[resolve_position(
    resolved_node = IsographResolutionNode<'a>,
    on_unmatched_span = from_path,
    pins = [
        (<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>),
        (<SelectionFieldArgument, UnparsedChunkItems>, ArgumentListPath<'a>),
        (<ObjectEntry, UnparsedChunkItems>, ObjectLiteralPath<'a>),
        (<Selection, UnparsedChunkItems>, SelectionSetPath<'a>),
    ]
)]
pub struct Slot<T, E> {
    #[resolve_field]
    pub item: Option<WithSpan<T>>,
    #[resolve_field(parents = [
        IsoLiteralSlot,
        SelectionFieldArgumentSlot,
        ObjectEntrySlot,
        SelectionSlot,
    ])]
    pub extra_tokens: Option<WithSpan<E>>,
}
```

`From<IsoLiteralSlotPath> for IsographResolutionNode` and the other `on_unmatched_span = from_path` impls stay. `UnparsedChunkItemsParent` stays. Its `From<…SlotPath>` impls are deleted.

A second leftover field of the same `E` takes a second list:

```rust
// from crates/resolve_position_macros/tests/resolve_field_parents.rs
struct Slot<T, E> {
    #[resolve_field]
    item: Option<WithSpan<T>>,
    #[resolve_field(parents = [SlotA, SlotB])]
    extra_tokens: Option<WithSpan<E>>,
    #[resolve_field(parents = [MoreA, MoreB])]
    more: Option<WithSpan<E>>,
}

enum ExtraParent<'a> {
    SlotA(SlotAPath<'a>),
    SlotB(SlotBPath<'a>),
    MoreA(SlotAPath<'a>),
    MoreB(SlotBPath<'a>),
}
```

## Tests

```rust
// from crates/resolve_position_macros/tests/resolve_field_parents.rs
#[test]
fn leftover_parent_is_the_named_pin_variant() {
    let list = ListA(
        Slot {
            item: ChildA.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: Extra.with_span(Span::new(6, 8)).wrap_some(),
            more: None,
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
}

#[test]
fn two_leftover_fields_of_the_same_type_take_different_variants() {
    let list = ListA(
        Slot {
            item: ChildA.with_span(Span::new(0, 4)).wrap_some(),
            extra_tokens: None,
            more: Extra.with_span(Span::new(6, 8)).wrap_some(),
        }
        .with_span(Span::new(0, 8))
        .wrap_vec(),
    );
    match list.resolve((), Span::new(6, 7)) {
        TestResolvedNode::Extra(path) => match path.parent {
            ExtraParent::MoreA(_) => {}
            parent => panic!("expected ExtraParent::MoreA, got {parent:?}"),
        },
        node => panic!("expected Extra, got {node:?}"),
    }
}
```

`from_container_parent_field.rs` keeps testing `#[parent_from]`. This file tests `parents = [...]`.

ui tests: `parents` without `pins`; `parents` length ≠ `pins` length; `parents` combined with `parent_from` / `parent_variant` / `transparent`.

## Landing checklist

1. `ResolveFieldForm::Parents`, `PinFieldContext`, `Slot.extra_tokens`, the deleted `From<…SlotPath> for UnparsedChunkItemsParent` impls, the tests. `cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.
2. Move this doc to refactors/past.
