# Parent-variant attribute

`parent_variant` and `parent_from` are attributes of their own. `#[resolve_field]` is the struct-field opt-in. `#[resolve_field(transparent)]` is its one list form: a mode of that opt-in, never valid alone. Parent construction on an enum payload is `#[parent_variant(V)]` or `#[parent_from]`.

The full grammar, with every invalid combination a compile error:

- struct field, unmarked: skipped.
- struct field, `#[resolve_field]`: descend, container's path as the parent.
- struct field, `#[resolve_field]` + `#[parent_variant(V)]`: descend, container's path wrapped in `V`.
- struct field, `#[resolve_field(transparent)]`: descend without a span check, `parent` passed through, no container fallback.
- struct field, `#[parent_variant(V)]` alone: error, "`#[parent_variant]` requires `#[resolve_field]`".
- struct field, `#[parent_from]` alone: error, "`#[parent_from]` requires `#[resolve_field]`".
- struct field, `#[resolve_field]` + `#[parent_from]`: error, "`#[parent_from]` is an enum-payload attribute".
- struct field, `#[resolve_field(transparent)]` + `#[parent_variant(V)]`: error, "`#[resolve_field(transparent)]` cannot combine with `#[parent_variant]`".
- struct field, `#[resolve_field(transparent)]` + `#[parent_from]`: error, "`#[resolve_field(transparent)]` cannot combine with `#[parent_from]`".
- struct field, `#[resolve_field]` + `#[parent_variant(V)]` + `#[parent_from]`: error, "cannot combine `#[parent_variant]` and `#[parent_from]`".
- enum payload, unmarked: delegate, parent passed through unchanged.
- enum payload, `#[parent_variant(V)]`: delegate, parent wrapped in `V`.
- enum payload, `#[parent_from]`: delegate, parent converted with `From::from`.
- enum payload, `#[resolve_field]`: error, "an enum payload always resolves and passes the parent through; annotate only to construct the parent: `#[parent_variant(SomeVariant)]` or `#[parent_from]`".
- enum payload, `#[resolve_field(transparent)]`: error, "`#[resolve_field(transparent)]` is a struct-field attribute; an unmarked payload already forwards `parent`".
- enum payload, `#[resolve_field]` (either form) plus `parent_variant` or `parent_from`: the `resolve_field` error above, checked first.
- enum payload, `#[parent_variant(V)]` + `#[parent_from]`: error, "cannot combine `#[parent_variant]` and `#[parent_from]`".
- `#[resolve_field(parent_variant = V)]` and `#[resolve_field(parent_from)]` no longer parse; the error points at the standalone spelling.

Behavior is unchanged at every existing site: the generated code is identical, and the test suite passes with no assertion edits.

## Derive sites

Origin: `crates/isograph_parser/src/chunk.rs` and `crates/resolve_position_macros/tests/generic_slot.rs` as they stand. Delta: every `parent_variant` and `parent_from` argument of `#[resolve_field]` becomes its own attribute. Bare `#[resolve_field]` and `#[resolve_field(transparent)]` are untouched. Unmarked enum arms are untouched.

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct ChunkedLevel(#[resolve_field(parent_variant = Level)] pub Vec<WithSpan<Chunk>>);

pub struct Chunk {
    #[resolve_field(parent_variant = Chunk)]
    contents: NonEmpty<WithSpan<ChunkContentItem>>,
    #[resolve_field]
    trailing_separator: Option<WithSpan<ChunkSeparator>>,
}

pub struct ChunkedGroup {
    #[resolve_field]
    pub opening: WithSpan<OpenBracket>,
    /// The wrapping `WithSpan`'s span runs from the opening's end to the closing's start.
    #[resolve_field(parent_variant = Interior)]
    pub children: WithSpan<ChunkedLevel>,
    #[resolve_field]
    pub closing: WithSpan<CloseBracket>,
}

pub struct UnparsedChunkItems(
    #[resolve_field(parent_variant = Unparsed)] pub NonEmpty<WithSpan<ChunkContentItem>>,
);

pub struct ExtraChunks(#[resolve_field(parent_variant = Extra)] pub NonEmpty<WithSpan<Chunk>>);
```

```rust
// from crates/resolve_position_macros/tests/generic_slot.rs
enum Slot<T: ResolvePosition>
where
    for<'a> SlotUnparsedParent<'a>: From<<T as ResolvePosition>::Parent<'a>>,
    for<'a> Unparsed: ResolvePosition<ResolvedNode<'a> = <T as ResolvePosition>::ResolvedNode<'a>>,
{
    Parsed(Parsed<T>),
    Unparsed(#[resolve_field(parent_from)] Unparsed),
}

struct Parsed<T: ResolvePosition>(#[resolve_field(transparent)] T);
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct ChunkedLevel(
    #[resolve_field]
    #[parent_variant(Level)]
    pub Vec<WithSpan<Chunk>>,
);

pub struct Chunk {
    #[resolve_field]
    #[parent_variant(Chunk)]
    contents: NonEmpty<WithSpan<ChunkContentItem>>,
    #[resolve_field]
    trailing_separator: Option<WithSpan<ChunkSeparator>>,
}

pub struct ChunkedGroup {
    #[resolve_field]
    pub opening: WithSpan<OpenBracket>,
    /// The wrapping `WithSpan`'s span runs from the opening's end to the closing's start.
    #[resolve_field]
    #[parent_variant(Interior)]
    pub children: WithSpan<ChunkedLevel>,
    #[resolve_field]
    pub closing: WithSpan<CloseBracket>,
}

pub struct UnparsedChunkItems(
    #[resolve_field]
    #[parent_variant(Unparsed)]
    pub NonEmpty<WithSpan<ChunkContentItem>>,
);

pub struct ExtraChunks(
    #[resolve_field]
    #[parent_variant(Extra)]
    pub NonEmpty<WithSpan<Chunk>>,
);
```

```rust
// from crates/resolve_position_macros/tests/generic_slot.rs
enum Slot<T: ResolvePosition>
where
    for<'a> SlotUnparsedParent<'a>: From<<T as ResolvePosition>::Parent<'a>>,
    for<'a> Unparsed: ResolvePosition<ResolvedNode<'a> = <T as ResolvePosition>::ResolvedNode<'a>>,
{
    Parsed(Parsed<T>),
    Unparsed(#[parent_from] Unparsed),
}

struct Parsed<T: ResolvePosition>(#[resolve_field(transparent)] T);
```

## The derive

Before:

```rust
// from crates/resolve_position_macros/src/lib.rs
#[proc_macro_derive(
    ResolvePosition,
    attributes(resolve_field, resolve_position, self_type_generics)
)]
```

After:

```rust
// from crates/resolve_position_macros/src/lib.rs
#[proc_macro_derive(
    ResolvePosition,
    attributes(
        parent_from,
        parent_variant,
        resolve_field,
        resolve_position,
        self_type_generics
    )
)]
```

## Types

`ParentConstruction` comments name the new spellings. `FromParent` is deleted: enum payloads emit `From::from` from `#[parent_from]` directly and never build this enum, and a struct field with `#[parent_from]` is an error. `new_parent_expr` loses that arm.

Before:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
/// How an emission builds the value it passes as the child's parent.
enum ParentConstruction {
    /// Bare `#[resolve_field]`: the child's `Parent` type is the container's own
    /// path, and `self.path(parent)` is passed unwrapped.
    ContainerPath,
    /// `#[resolve_field(parent_variant = V)]`: the child's `Parent` type is an
    /// enum, and the parent value is wrapped in its variant `V`.
    EnumVariant(syn::Ident),
    /// `#[resolve_field(parent_from)]`: the child's `Parent` is `From` the
    /// container's `Parent`.
    FromParent,
    /// `#[resolve_field(transparent)]`: a bare `ResolvePosition` field, no
    /// path segment, no span check.
    Transparent,
}
```

After:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
/// How an emission builds the value it passes as the child's parent.
enum ParentConstruction {
    /// Bare `#[resolve_field]`: the child's `Parent` type is the container's own
    /// path, and `self.path(parent)` is passed unwrapped.
    ContainerPath,
    /// `#[parent_variant(V)]`: the child's `Parent` type is an enum, and the
    /// parent value is wrapped in its variant `V`.
    EnumVariant(syn::Ident),
    /// `#[resolve_field(transparent)]`: a bare `ResolvePosition` field, no
    /// path segment, no span check.
    Transparent,
}
```

New types. `FieldAttributes` is the collected helper attributes on one field or payload. `ResolveFieldForm` is the parsed `#[resolve_field]` after the list forms other than `transparent` have been rejected.

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
struct FieldAttributes<'a> {
    resolve_field: Option<&'a syn::Attribute>,
    parent_variant: Option<&'a syn::Attribute>,
    parent_from: Option<&'a syn::Attribute>,
}

enum ResolveFieldForm {
    Bare,
    Transparent,
}
```

## Collection and parse

Origin: `find_resolve_field_attr` and `parse_parent_construction` in `crates/resolve_position_macros/src/resolve_position_macro.rs`. Delta: those two functions are replaced by the functions below. Three attributes are collected, at most one of each. `#[resolve_field]` is `Meta::Path` or `Meta::List` whose only argument is `transparent`. `#[parent_variant(V)]` is `Meta::List` whose only argument is a path ident. `#[parent_from]` is `Meta::Path`.

Before:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
fn find_resolve_field_attr(
    attrs: &[syn::Attribute],
) -> Result<Option<&syn::Attribute>, proc_macro2::TokenStream> {
    let mut matching = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("resolve_field"));
    match (matching.next(), matching.next()) {
        (first, None) => first.wrap_ok(),
        (_, Some(duplicate)) => Error::new_spanned(duplicate, "duplicate #[resolve_field]")
            .to_compile_error()
            .wrap_err(),
    }
}

fn parse_parent_construction(
    attr: &syn::Attribute,
) -> Result<ParentConstruction, proc_macro2::TokenStream> {
    match attr.meta.reference() {
        syn::Meta::Path(_) => ParentConstruction::ContainerPath.wrap_ok(),
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
                && let syn::Expr::Path(value) = name_value.value.reference()
                && let Some(variant) = value.path.get_ident()
            {
                return ParentConstruction::EnumVariant(variant.clone()).wrap_ok();
            }
            Error::new_spanned(
                attr.meta.reference(),
                "expected `#[resolve_field]`, `#[resolve_field(parent_variant = SomeVariant)]`, \
                 `#[resolve_field(parent_from)]`, or `#[resolve_field(transparent)]`",
            )
            .to_compile_error()
            .wrap_err()
        }
        syn::Meta::NameValue(name_value) => Error::new_spanned(
            name_value,
            "expected `#[resolve_field]`, `#[resolve_field(parent_variant = SomeVariant)]`, \
             `#[resolve_field(parent_from)]`, or `#[resolve_field(transparent)]`",
        )
        .to_compile_error()
        .wrap_err(),
    }
}
```

After:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
fn find_unique_attr<'a>(
    attrs: &'a [syn::Attribute],
    name: &str,
) -> Result<Option<&'a syn::Attribute>, proc_macro2::TokenStream> {
    let mut matching = attrs.iter().filter(|attr| attr.path().is_ident(name));
    match (matching.next(), matching.next()) {
        (first, None) => first.wrap_ok(),
        (_, Some(duplicate)) => Error::new_spanned(duplicate, format!("duplicate #[{name}]"))
            .to_compile_error()
            .wrap_err(),
    }
}

fn collect_field_attributes(
    attrs: &[syn::Attribute],
) -> Result<FieldAttributes<'_>, proc_macro2::TokenStream> {
    FieldAttributes {
        resolve_field: find_unique_attr(attrs, "resolve_field")?,
        parent_variant: find_unique_attr(attrs, "parent_variant")?,
        parent_from: find_unique_attr(attrs, "parent_from")?,
    }
    .wrap_ok()
}

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
            Error::new_spanned(
                attr.meta.reference(),
                "expected bare `#[resolve_field]` or `#[resolve_field(transparent)]`; \
                 parent wrapping is `#[parent_variant(SomeVariant)]`, \
                 parent conversion is `#[parent_from]`",
            )
            .to_compile_error()
            .wrap_err()
        }
        syn::Meta::NameValue(name_value) => Error::new_spanned(
            name_value,
            "expected bare `#[resolve_field]` or `#[resolve_field(transparent)]`; \
             parent wrapping is `#[parent_variant(SomeVariant)]`, \
             parent conversion is `#[parent_from]`",
        )
        .to_compile_error()
        .wrap_err(),
    }
}

fn parse_parent_variant(attr: &syn::Attribute) -> Result<syn::Ident, proc_macro2::TokenStream> {
    match attr.meta.reference() {
        syn::Meta::List(_) => {
            if let Ok(path) = attr.parse_args::<syn::Path>()
                && let Some(variant) = path.get_ident()
            {
                return variant.clone().wrap_ok();
            }
            Error::new_spanned(attr, "expected `#[parent_variant(SomeVariant)]`")
                .to_compile_error()
                .wrap_err()
        }
        _ => Error::new_spanned(attr, "expected `#[parent_variant(SomeVariant)]`")
            .to_compile_error()
            .wrap_err(),
    }
}

fn parse_parent_from(attr: &syn::Attribute) -> Result<(), proc_macro2::TokenStream> {
    match attr.meta.reference() {
        syn::Meta::Path(_) => ().wrap_ok(),
        _ => Error::new_spanned(attr, "expected `#[parent_from]`")
            .to_compile_error()
            .wrap_err(),
    }
}
```

`new_parent_expr` before:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        ParentConstruction::FromParent | ParentConstruction::Transparent => Error::new_spanned(
            inner_type,
            "`parent_from` and `transparent` do not build a field parent",
        )
        .to_compile_error(),
```

After:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
        ParentConstruction::Transparent => Error::new_spanned(
            inner_type,
            "`transparent` does not build a field parent",
        )
        .to_compile_error(),
```

## Struct fields

Origin: `get_resolve_field_info` in `crates/resolve_position_macros/src/resolve_position_macro.rs`. Delta: it collects the three attributes and maps the combinations above onto `ParentConstruction`. `#[parent_from]` on a struct field still errors; the message uses the new spelling. The `FromParent` check that followed `parse_parent_construction` is the Bare + `parent_from` arm. Field accessor, the `Transparent` `WithSpan` check, and the path-type parse are unchanged. Emissions are unchanged.

Before:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
fn get_resolve_field_info(
    field: &'_ syn::Field,
    index: usize,
    generics_map: &HashMap<syn::Ident, syn::GenericArgument>,
) -> Result<Option<ResolveFieldInfo>, proc_macro2::TokenStream> {
    let Some(attr) = find_resolve_field_attr(field.attrs.reference())? else {
        return None.wrap_ok();
    };

    let parent_construction = parse_parent_construction(attr)?;

    // A named field is accessed by name, a tuple field by index.
    let field_accessor = match field.ident.reference() {
        Some(ident) => quote!(#ident),
        None => {
            let index = syn::Index::from(index);
            quote!(#index)
        }
    };

    if matches!(parent_construction, ParentConstruction::FromParent) {
        return Error::new_spanned(
            attr,
            "`#[resolve_field(parent_from)]` is an enum-payload attribute",
        )
        .to_compile_error()
        .wrap_err();
    }

    if matches!(parent_construction, ParentConstruction::Transparent) {
        if type_is_with_span(field.ty.reference()) {
            return Error::new_spanned(
                field.ty.reference(),
                "`#[resolve_field(transparent)]` on a `WithSpan<T>` field is a compile error: \
                 span-checked descent is the unmarked `#[resolve_field]` form",
            )
            .to_compile_error()
            .wrap_err();
        }
        return ResolveFieldInfo {
            field_accessor,
            field_type: ResolveFieldInfoTypeWrapper::Transparent(field.ty.clone().boxed()),
            parent_construction,
        }
        .wrap_some()
        .wrap_ok();
    }

    if let syn::Type::Path(syn::TypePath { path, .. }) = field.ty.reference() {
        match parse_resolve_field_type(path, generics_map) {
            Ok(field_type) => ResolveFieldInfo {
                field_accessor,
                field_type,
                parent_construction,
            }
            .wrap_some()
            .wrap_ok(),
            Err(e) => e.wrap_err(),
        }
    } else {
        Error::new_spanned(
            field.ty.reference(),
            "#[resolve_field] fields must be path types",
        )
        .to_compile_error()
        .wrap_err()
    }
}
```

After:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
fn get_resolve_field_info(
    field: &'_ syn::Field,
    index: usize,
    generics_map: &HashMap<syn::Ident, syn::GenericArgument>,
) -> Result<Option<ResolveFieldInfo>, proc_macro2::TokenStream> {
    let FieldAttributes {
        resolve_field,
        parent_variant,
        parent_from,
    } = collect_field_attributes(field.attrs.reference())?;

    let Some(resolve_field) = resolve_field else {
        if let Some(attr) = parent_variant {
            return Error::new_spanned(attr, "`#[parent_variant]` requires `#[resolve_field]`")
                .to_compile_error()
                .wrap_err();
        }
        if let Some(attr) = parent_from {
            return Error::new_spanned(attr, "`#[parent_from]` requires `#[resolve_field]`")
                .to_compile_error()
                .wrap_err();
        }
        return None.wrap_ok();
    };

    let form = parse_resolve_field_form(resolve_field)?;

    let parent_construction = match form {
        ResolveFieldForm::Transparent => {
            if let Some(attr) = parent_variant {
                return Error::new_spanned(
                    attr,
                    "`#[resolve_field(transparent)]` cannot combine with `#[parent_variant]`",
                )
                .to_compile_error()
                .wrap_err();
            }
            if let Some(attr) = parent_from {
                return Error::new_spanned(
                    attr,
                    "`#[resolve_field(transparent)]` cannot combine with `#[parent_from]`",
                )
                .to_compile_error()
                .wrap_err();
            }
            ParentConstruction::Transparent
        }
        ResolveFieldForm::Bare => {
            if let (Some(_), Some(attr)) = (parent_variant, parent_from) {
                return Error::new_spanned(
                    attr,
                    "cannot combine `#[parent_variant]` and `#[parent_from]`",
                )
                .to_compile_error()
                .wrap_err();
            }
            if let Some(attr) = parent_from {
                parse_parent_from(attr)?;
                return Error::new_spanned(attr, "`#[parent_from]` is an enum-payload attribute")
                    .to_compile_error()
                    .wrap_err();
            }
            match parent_variant {
                Some(attr) => ParentConstruction::EnumVariant(parse_parent_variant(attr)?),
                None => ParentConstruction::ContainerPath,
            }
        }
    };

    // A named field is accessed by name, a tuple field by index.
    let field_accessor = match field.ident.reference() {
        Some(ident) => quote!(#ident),
        None => {
            let index = syn::Index::from(index);
            quote!(#index)
        }
    };

    if matches!(parent_construction, ParentConstruction::Transparent) {
        if type_is_with_span(field.ty.reference()) {
            return Error::new_spanned(
                field.ty.reference(),
                "`#[resolve_field(transparent)]` on a `WithSpan<T>` field is a compile error: \
                 span-checked descent is the unmarked `#[resolve_field]` form",
            )
            .to_compile_error()
            .wrap_err();
        }
        return ResolveFieldInfo {
            field_accessor,
            field_type: ResolveFieldInfoTypeWrapper::Transparent(field.ty.clone().boxed()),
            parent_construction,
        }
        .wrap_some()
        .wrap_ok();
    }

    if let syn::Type::Path(syn::TypePath { path, .. }) = field.ty.reference() {
        match parse_resolve_field_type(path, generics_map) {
            Ok(field_type) => ResolveFieldInfo {
                field_accessor,
                field_type,
                parent_construction,
            }
            .wrap_some()
            .wrap_ok(),
            Err(e) => e.wrap_err(),
        }
    } else {
        Error::new_spanned(
            field.ty.reference(),
            "#[resolve_field] fields must be path types",
        )
        .to_compile_error()
        .wrap_err()
    }
}
```

## Enum payloads

Origin: `generate_enum_arm` in `crates/resolve_position_macros/src/resolve_position_macro.rs`. Delta: it collects the three attributes. `#[resolve_field]` of either form errors (transparent keeps its current message; bare uses the new spelling and names `#[parent_from]`). `#[parent_from]` emits `From::from` after `parse_parent_from`. Emissions are unchanged.

Before:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
fn generate_enum_arm(
    enum_name: &syn::Ident,
    variant_name: &syn::Ident,
    payload: &syn::Field,
) -> proc_macro2::TokenStream {
    let attr = match find_resolve_field_attr(payload.attrs.reference()) {
        Ok(attr) => attr,
        Err(e) => return e,
    };

    // An unannotated payload delegates with the parent unchanged, which requires the
    // payload's Parent type to equal the enum's. The payload implements ResolvePosition
    // itself; a located wrapper delegates through the blanket impl in resolve_position.
    let Some(attr) = attr else {
        return quote! {
            #enum_name::#variant_name(inner) => inner.resolve(parent, position)
        };
    };

    match parse_parent_construction(attr) {
        Ok(ParentConstruction::EnumVariant(parent_variant)) => {
            let payload_type = payload.ty.reference();
            quote! {
                #enum_name::#variant_name(inner) => inner.resolve(
                    <#payload_type as ::resolve_position::ResolvePosition>::Parent::#parent_variant(parent.into()),
                    position,
                )
            }
        }
        Ok(ParentConstruction::FromParent) => {
            quote! {
                #enum_name::#variant_name(inner) => inner.resolve(
                    ::std::convert::From::from(parent),
                    position,
                )
            }
        }
        Ok(ParentConstruction::ContainerPath) => Error::new_spanned(
            attr,
            "an enum payload always resolves and passes the parent through; annotate \
            only to wrap it: #[resolve_field(parent_variant = SomeVariant)]",
        )
        .to_compile_error(),
        Ok(ParentConstruction::Transparent) => Error::new_spanned(
            attr,
            "`#[resolve_field(transparent)]` is a struct-field attribute; \
            an unmarked payload already forwards `parent`",
        )
        .to_compile_error(),
        Err(e) => e,
    }
}
```

After:

```rust
// from crates/resolve_position_macros/src/resolve_position_macro.rs
fn generate_enum_arm(
    enum_name: &syn::Ident,
    variant_name: &syn::Ident,
    payload: &syn::Field,
) -> proc_macro2::TokenStream {
    let FieldAttributes {
        resolve_field,
        parent_variant,
        parent_from,
    } = match collect_field_attributes(payload.attrs.reference()) {
        Ok(attrs) => attrs,
        Err(e) => return e,
    };

    if let Some(resolve_field) = resolve_field {
        return match parse_resolve_field_form(resolve_field) {
            Ok(ResolveFieldForm::Transparent) => Error::new_spanned(
                resolve_field,
                "`#[resolve_field(transparent)]` is a struct-field attribute; \
                 an unmarked payload already forwards `parent`",
            )
            .to_compile_error(),
            Ok(ResolveFieldForm::Bare) => Error::new_spanned(
                resolve_field,
                "an enum payload always resolves and passes the parent through; annotate \
                 only to construct the parent: `#[parent_variant(SomeVariant)]` or `#[parent_from]`",
            )
            .to_compile_error(),
            Err(e) => e,
        };
    }

    // An unannotated payload delegates with the parent unchanged, which requires the
    // payload's Parent type to equal the enum's. The payload implements ResolvePosition
    // itself; a located wrapper delegates through the blanket impl in resolve_position.
    match (parent_variant, parent_from) {
        (None, None) => quote! {
            #enum_name::#variant_name(inner) => inner.resolve(parent, position)
        },
        (Some(attr), None) => match parse_parent_variant(attr) {
            Ok(parent_variant) => {
                let payload_type = payload.ty.reference();
                quote! {
                    #enum_name::#variant_name(inner) => inner.resolve(
                        <#payload_type as ::resolve_position::ResolvePosition>::Parent::#parent_variant(parent.into()),
                        position,
                    )
                }
            }
            Err(e) => e,
        },
        (None, Some(attr)) => match parse_parent_from(attr) {
            Ok(()) => quote! {
                #enum_name::#variant_name(inner) => inner.resolve(
                    ::std::convert::From::from(parent),
                    position,
                )
            },
            Err(e) => e,
        },
        (Some(_), Some(attr)) => Error::new_spanned(
            attr,
            "cannot combine `#[parent_variant]` and `#[parent_from]`",
        )
        .to_compile_error(),
    }
}
```

## Tests

No new behavior. `generic_slot.rs` respells `#[resolve_field(parent_from)]` to `#[parent_from]`. The suite passing with zero assertion edits is the check.

## Landing checklist

1. The macro changes and the respelling of every existing derive site; `cargo test -p resolve_position_macros`, `cargo test -p isograph_parser`, and the clippy pre-commit hook pass with no assertion edits.
2. Move this doc to refactors/past.
