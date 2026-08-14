# resolve-position: derive `LevelSlot<T>`

Prefactor on `resolve_position_macros`. The parsing series stores a concrete slot enum per list and does not wait on this. After this lands, `LevelSlot<T>` and `ParsedSlot<T>` derive, and a list field can be `Vec<WithSpan<LevelSlot<Selection>>>` the way it is `Vec<WithSpan<SelectionSlot>>` in parse-fields.md.

## The derive sites

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = <T as ResolvePosition>::Parent<'a>,
    resolved_node = <T as ResolvePosition>::ResolvedNode<'a>
)]
pub enum LevelSlot<T> {
    Parsed(ParsedSlot<T>),
    Unparsed(#[resolve_field(parent_from)] UnparsedItem),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = <T as ResolvePosition>::Parent<'a>,
    resolved_node = <T as ResolvePosition>::ResolvedNode<'a>
)]
pub struct ParsedSlot<T> {
    #[resolve_field(transparent)]
    pub item: T,
    pub trailing: Option<WithSpan<ParseError>>,
}
```

`UnparsedItem` is unchanged from parsing-standards.md. `parent_from` on the `Unparsed` payload builds `UnparsedItemParent` with `From`. `transparent` on `ParsedSlot::item` calls `T::resolve` with the same parent; `ParsedSlot` is not a path segment and is not a `ResolvedNode` variant.

A list field after this prefactor:

```rust
// from crates/isograph_parser/src/selections.rs
pub struct SelectionSet(#[resolve_field] pub Vec<WithSpan<LevelSlot<Selection>>>);
```

`Selection::Parent` is `SelectionSetPath`. `From<SelectionSetPath<'a>> for UnparsedItemParent<'a>` is the conversion `parent_from` uses. The same `From` is written per list when that list's field becomes `LevelSlot<T>`.

## Macro: type generics on enum derives

Before, `handle_data_enum` ignores the type's generics and emits a bare impl:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        syn::Data::Enum(data_enum) => {
            handle_data_enum(struct_name, resolve_position_args, data_enum)
        }
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
    let output = quote! {
        impl ::resolve_position::ResolvePosition for #enum_name #self_type_generics {
            type Parent<'a> = #parent_type;
            type ResolvedNode<'a> = #resolved_node;
            // ...
        }
    };
```

After: the enum path takes `input.generics` the way the struct path already does, and the impl uses `split_for_impl`.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        syn::Data::Enum(data_enum) => {
            handle_data_enum(struct_name, resolve_position_args, data_enum, input.generics)
        }
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
fn handle_data_enum(
    enum_name: syn::Ident,
    resolve_position_args: ResolvePositionArgs,
    data_enum: syn::DataEnum,
    input_generics: syn::Generics,
) -> TokenStream {
    let ResolvePositionArgs {
        parent_type,
        resolved_node,
        self_type_generics,
    } = resolve_position_args;

    let generics_map = match validate_and_map_generics(input_generics.clone(), self_type_generics.clone()) {
        Ok(map) => map,
        Err(e) => return e.into(),
    };

    let match_arms = data_enum.variants.iter().map(|variant| {
        // same per-variant match as today, using generate_enum_arm
    });

    let (impl_generics, ty_generics, where_clause) = input_generics.split_for_impl();
    let ty_generics = match &self_type_generics {
        Some(explicit) => quote!(#explicit),
        None => quote!(#ty_generics),
    };

    let output = quote! {
        impl #impl_generics ::resolve_position::ResolvePosition for #enum_name #ty_generics #where_clause {
            type Parent<'a>
                = #parent_type
            where
                Self: 'a;
            type ResolvedNode<'a>
                = #resolved_node
            where
                Self: 'a;

            fn resolve<'a>(
                &'a self,
                parent: Self::Parent<'a>,
                position: ::span::Span
            ) -> Self::ResolvedNode<'a> {
                match self {
                    #(#match_arms),*
                }
            }
        }
    };

    output.into()
}
```

The struct emit gains the same `where Self: 'a` on the associated types, so `parent_type = <T as ResolvePosition>::Parent<'a>` is legal. `split_for_impl` already exists for structs via `self_type_generics`; this change is that the type's own parameters and where-clause are kept when `self_type_generics` is absent.

`LevelSlot<T: ResolvePosition>` writes the `T: ResolvePosition` bound on the type. `for<'a> UnparsedItemParent<'a>: From<<T as ResolvePosition>::Parent<'a>>` is a where-clause on `LevelSlot`.

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = <T as ResolvePosition>::Parent<'a>,
    resolved_node = <T as ResolvePosition>::ResolvedNode<'a>
)]
pub enum LevelSlot<T: ResolvePosition>
where
    for<'a> UnparsedItemParent<'a>: From<<T as ResolvePosition>::Parent<'a>>,
{
    Parsed(ParsedSlot<T>),
    Unparsed(#[resolve_field(parent_from)] UnparsedItem),
}
```

## Macro: `parent_from` on an enum payload

`ParentConstruction` gains a third arm.

Before:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
enum ParentConstruction {
    ContainerPath,
    EnumVariant(syn::Ident),
}
```

After:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
enum ParentConstruction {
    ContainerPath,
    EnumVariant(syn::Ident),
    /// `#[resolve_field(parent_from)]`: the child's `Parent` is `From` the
    /// container's `Parent`.
    FromParent,
    /// `#[resolve_field(transparent)]`: a bare `ResolvePosition` field, no
    /// path segment, no span check.
    Transparent,
}
```

`parse_parent_construction` on `Meta::List` accepts a path or a name-value:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        syn::Meta::List(_) => {
            if let Ok(path) = attr.parse_args::<syn::Path>() {
                if path.is_ident("parent_from") {
                    return ParentConstruction::FromParent.wrap_ok();
                }
                if path.is_ident("transparent") {
                    return ParentConstruction::Transparent.wrap_ok();
                }
            }
            let name_value = attr
                .parse_args::<syn::MetaNameValue>()
                .map_err(|e| e.to_compile_error())?;
            if name_value.path.is_ident("parent_variant")
                && let syn::Expr::Path(value) = &name_value.value
                && let Some(variant) = value.path.get_ident()
            {
                return ParentConstruction::EnumVariant(variant.clone()).wrap_ok();
            }
            Error::new_spanned(
                &attr.meta,
                "expected `#[resolve_field]`, `#[resolve_field(parent_variant = SomeVariant)]`, \
                 `#[resolve_field(parent_from)]`, or `#[resolve_field(transparent)]`",
            )
            .to_compile_error()
            .wrap_err()
        }
```

`parent_from` and `transparent` take no value. `#[resolve_field(parent_from = ...)]` is a compile error.

`generate_enum_arm` for `FromParent`:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        Ok(ParentConstruction::FromParent) => {
            quote! {
                #enum_name::#variant_name(inner) => inner.resolve(
                    ::std::convert::From::from(parent),
                    position,
                )
            }
        }
```

Generated `LevelSlot` `Unparsed` arm:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
            LevelSlot::Unparsed(inner) => inner.resolve(
                ::std::convert::From::from(parent),
                position,
            ),
```

`parent_variant` is unchanged. Concrete slot enums in the parsing series keep using it.

## Macro: `transparent` on a struct field

A `#[resolve_field(transparent)]` field is a `ResolvePosition` value that is not `WithSpan`. The parent already checked the container span. The field adds no path segment.

`parse_parent_construction` accepts `transparent` the same way as `parent_from` (a name with no value). `ParentConstruction` gains `Transparent`.

`parse_resolve_field_type` already errors on a bare path that is not `WithSpan` / `Vec` / `Option` / `NonEmpty`. A `Transparent` field skips that parser and records the field type as-is:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
    if matches!(parent_construction, ParentConstruction::Transparent) {
        return ResolveFieldInfo {
            field_accessor,
            field_type: ResolveFieldInfoTypeWrapper::Transparent(field.ty.clone()),
            parent_construction,
        }
        .wrap_some()
        .wrap_ok();
    }
```

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
enum ResolveFieldInfoTypeWrapper {
    None(Box<ResolveFieldInfoType>),
    IteratorWrapper(Box<ResolveFieldInfoTypeWrapper>),
    Transparent(syn::Type),
}
```

`generate_resolve_code_recursive` for that arm:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        ResolveFieldInfoTypeWrapper::Transparent(_) => {
            quote! {
                return #field_expr.resolve(parent, position);
            }
        }
```

Generated `ParsedSlot`:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl<T: ResolvePosition> ::resolve_position::ResolvePosition for ParsedSlot<T> {
    type Parent<'a>
        = <T as ResolvePosition>::Parent<'a>
    where
        Self: 'a;
    type ResolvedNode<'a>
        = <T as ResolvePosition>::ResolvedNode<'a>
    where
        Self: 'a;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        return self.item.resolve(parent, position);
    }
}
```

`ParsedSlot` has no `ResolvedNode` variant. A position that reached the slot is answered by `T`.

`#[resolve_field(transparent)]` on a `WithSpan<T>` field is a compile error: span-checked descent is the unmarked `#[resolve_field]` form.

`#[resolve_field(transparent)]` on an enum payload is a compile error: an unmarked payload already forwards `parent`.

## Generated `LevelSlot`

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl<T: ResolvePosition> ::resolve_position::ResolvePosition for LevelSlot<T>
where
    for<'a> UnparsedItemParent<'a>: From<<T as ResolvePosition>::Parent<'a>>,
{
    type Parent<'a>
        = <T as ResolvePosition>::Parent<'a>
    where
        Self: 'a;
    type ResolvedNode<'a>
        = <T as ResolvePosition>::ResolvedNode<'a>
    where
        Self: 'a;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        match self {
            LevelSlot::Parsed(inner) => inner.resolve(parent, position),
            LevelSlot::Unparsed(inner) => inner.resolve(
                ::std::convert::From::from(parent),
                position,
            ),
        }
    }
}
```

## Invalid combinations

- struct field, `#[resolve_field(transparent)]` + `parent_variant`: error
- struct field, `#[resolve_field(parent_from)]`: error (`parent_from` is an enum-payload attribute)
- enum payload, `#[resolve_field(transparent)]`: error
- enum payload, `#[resolve_field(parent_from)]` + `parent_variant`: error
- `#[resolve_field(parent_from = Ident)]`: error
- `#[resolve_field(transparent = Ident)]`: error

## Tests

In `crates/resolve_position_macros` (or the existing derive test crate), a fixture type `Slot<T>` matching `LevelSlot`'s shape, with `T = Child` whose `Parent` is `ParentPath` and a `SlotUnparsedParent: From<ParentPath>`. Assert:

- a position on a parsed child's span resolves to that child; the path's parent is `ParentPath`, not a `ParsedSlot` segment
- a position on an unparsed child's span resolves through the unparsed payload; the unparsed parent is `From::from` the list path
- `Slot<T>` does not appear as a `ResolvedNode` variant

The parsing series is not updated by this doc. Collapsing `SelectionSlot` / `ArgumentSlot` / `ObjectEntrySlot` / `ConstantObjectEntrySlot` / `VariableDeclarationSlot` onto `LevelSlot<T>` is a later mechanical change against this emit.

## Landing checklist

1. The `ParentConstruction` arm, the enum `split_for_impl` path, `parent_from`, `transparent`, the associated-type `where Self: 'a` bounds, and the tests; `cargo test -p resolve_position_macros` and `cargo test -p isograph_parser` pass.
2. Move this doc to refactors/past.
