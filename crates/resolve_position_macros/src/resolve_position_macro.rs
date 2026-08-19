use std::collections::HashMap;

use prelude::Postfix;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Error, parse_macro_input, spanned::Spanned};

use crate::map_generics::{replace_generics_in_type, validate_and_map_generics};

pub(crate) fn resolve_position_macro(item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as syn::DeriveInput);
    let struct_name = input.ident.clone();

    let resolve_position_args = match deluxe::extract_attributes(&mut input) {
        Ok(resolve_position_args) => resolve_position_args,
        Err(e) => return e.into_compile_error().to(),
    };

    match input.data {
        syn::Data::Struct(data_struct) => handle_data_struct(
            struct_name,
            resolve_position_args,
            data_struct,
            input.generics,
        ),
        syn::Data::Enum(data_enum) => handle_data_enum(
            struct_name,
            resolve_position_args,
            data_enum,
            input.generics,
        ),
        syn::Data::Union(_) => {
            Error::new(input.span(), "This derive only works on structs and enums")
                .to_compile_error()
                .to()
        }
    }
}

fn handle_data_struct(
    struct_name: syn::Ident,
    resolve_position_args: ResolvePositionArgs,
    data_struct: syn::DataStruct,
    input_generics: syn::Generics,
) -> TokenStream {
    let ResolvePositionArgs {
        parent_type,
        resolved_node,
        self_type_generics,
        on_unmatched_span,
    } = resolve_position_args;

    let generics_map =
        match validate_and_map_generics(input_generics.clone(), self_type_generics.clone()) {
            Ok(map) => map,
            Err(e) => {
                return e.to();
            }
        };

    let field_infos = match data_struct
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| get_resolve_field_info(field, index, generics_map.reference()))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(field_infos) => field_infos.into_iter().flatten().collect::<Vec<_>>(),
        Err(e) => {
            return e.to();
        }
    };

    let attributes_to_resolve = field_infos
        .iter()
        .map(
            |ResolveFieldInfo {
                 field_accessor,
                 field_type,
                 parent_construction,
             }| {
                generate_resolve_code(
                    field_accessor.reference(),
                    field_type.reference(),
                    parent_construction.reference(),
                )
            },
        )
        .collect::<Vec<_>>();

    // A transparent field always answers; the container is not a path segment.
    let unmatched = if field_infos
        .iter()
        .any(|info| matches!(info.field_type, ResolveFieldInfoTypeWrapper::Transparent(_)))
    {
        quote!()
    } else {
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
    };

    let (impl_generics, ty_generics, where_clause) = input_generics.split_for_impl();
    let (impl_generics, ty_generics, where_clause) = match self_type_generics.reference() {
        Some(explicit) => (quote!(), quote!(#explicit), None),
        None => (quote!(#impl_generics), quote!(#ty_generics), where_clause),
    };

    let output = quote! {
        impl #impl_generics ::resolve_position::ResolvePosition for #struct_name #ty_generics #where_clause {
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
                #(#attributes_to_resolve)*

                #unmatched
            }
        }
    };

    output.to()
}

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
        on_unmatched_span,
    } = resolve_position_args;

    if let Some(ident) = on_unmatched_span {
        return Error::new_spanned(
            ident,
            "`on_unmatched_span` is a struct attribute; enums have no unmatched-span arm",
        )
        .to_compile_error()
        .to();
    }

    let _generics_map =
        match validate_and_map_generics(input_generics.clone(), self_type_generics.clone()) {
            Ok(map) => map,
            Err(e) => return e.to(),
        };

    let match_arms = data_enum.variants.iter().map(|variant| {
        let variant_name = variant.ident.reference();

        match variant.fields.reference() {
            syn::Fields::Unnamed(fields) => {
                let mut payloads = fields.unnamed.iter();
                match (payloads.next(), payloads.next()) {
                    (Some(payload), None) => {
                        generate_enum_arm(enum_name.reference(), variant_name, payload)
                    }
                    _ => single_payload_error(variant),
                }
            }
            _ => single_payload_error(variant),
        }
    });

    let (impl_generics, ty_generics, where_clause) = input_generics.split_for_impl();
    let ty_generics = match self_type_generics.reference() {
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

    output.to()
}

fn generate_enum_arm(
    enum_name: &syn::Ident,
    variant_name: &syn::Ident,
    payload: &syn::Field,
) -> proc_macro2::TokenStream {
    let FieldAttributes {
        resolve_field,
        parent_variant,
        from_container_parent,
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
                 only to construct the parent: `#[parent_variant(SomeVariant)]` or `#[from_container_parent]`",
            )
            .to_compile_error(),
            Err(e) => e,
        };
    }

    // An unannotated payload delegates with the parent unchanged, which requires the
    // payload's Parent type to equal the enum's. The payload implements ResolvePosition
    // itself; a located wrapper delegates through the blanket impl in resolve_position.
    match (parent_variant, from_container_parent) {
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
        (None, Some(attr)) => match parse_from_container_parent(attr) {
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
            "cannot combine `#[parent_variant]` and `#[from_container_parent]`",
        )
        .to_compile_error(),
    }
}

fn single_payload_error(variant: &syn::Variant) -> proc_macro2::TokenStream {
    Error::new_spanned(
        variant,
        "ResolvePosition only supports enum variants with a single unnamed field",
    )
    .to_compile_error()
}

#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(resolve_position))]
struct ResolvePositionArgs {
    parent_type: syn::Type,
    resolved_node: syn::Type,
    self_type_generics: Option<syn::AngleBracketedGenericArguments>,
    on_unmatched_span: Option<syn::Ident>,
}

enum ResolveFieldInfoType {
    WithSpan(syn::Type),
    WithLocation(syn::Type),
    WithEmbeddedLocation(syn::Type),
    GraphQLTypeAnnotation(syn::Type),
}

enum ResolveFieldInfoTypeWrapper {
    None(Box<ResolveFieldInfoType>),
    IteratorWrapper(Box<ResolveFieldInfoTypeWrapper>),
    #[expect(dead_code)]
    Transparent(Box<syn::Type>),
}

/// How an emission builds the value it passes as the child's parent.
enum ParentConstruction {
    /// Bare `#[resolve_field]`: the child's `Parent` type is the container's own
    /// path, and `self.path(parent)` is passed unwrapped.
    ContainerPath,
    /// `#[parent_variant(V)]`: the child's `Parent` type is an
    /// enum, and the parent value is wrapped in its variant `V`.
    EnumVariant(syn::Ident),
    /// `#[resolve_field(transparent)]`: a bare `ResolvePosition` field, no
    /// path segment, no span check.
    Transparent,
}

struct FieldAttributes<'a> {
    resolve_field: Option<&'a syn::Attribute>,
    parent_variant: Option<&'a syn::Attribute>,
    from_container_parent: Option<&'a syn::Attribute>,
}

enum ResolveFieldForm {
    Bare,
    Transparent,
}

struct ResolveFieldInfo {
    field_accessor: proc_macro2::TokenStream,
    field_type: ResolveFieldInfoTypeWrapper,
    parent_construction: ParentConstruction,
}

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
        from_container_parent: find_unique_attr(attrs, "from_container_parent")?,
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
                 parent conversion is `#[from_container_parent]`",
            )
            .to_compile_error()
            .wrap_err()
        }
        syn::Meta::NameValue(name_value) => Error::new_spanned(
            name_value,
            "expected bare `#[resolve_field]` or `#[resolve_field(transparent)]`; \
             parent wrapping is `#[parent_variant(SomeVariant)]`, \
             parent conversion is `#[from_container_parent]`",
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

fn parse_from_container_parent(attr: &syn::Attribute) -> Result<(), proc_macro2::TokenStream> {
    match attr.meta.reference() {
        syn::Meta::Path(_) => ().wrap_ok(),
        _ => Error::new_spanned(attr, "expected `#[from_container_parent]`")
            .to_compile_error()
            .wrap_err(),
    }
}

// Attempts to extract the single generic type from angle bracketed path arguments, e.g. X<Inner>
fn extract_single_generic_type(segment: &syn::PathSegment) -> Option<&syn::Type> {
    match segment.arguments.reference() {
        syn::PathArguments::AngleBracketed(args) => args.args.first().and_then(|arg| {
            if let syn::GenericArgument::Type(ty) = arg {
                ty.wrap_some()
            } else {
                None
            }
        }),
        _ => None,
    }
}

fn handle_case(
    last_segment: &syn::PathSegment,
    generics_map: &HashMap<syn::Ident, syn::GenericArgument>,
    ctor: fn(syn::Type) -> ResolveFieldInfoType,
) -> Result<ResolveFieldInfoTypeWrapper, proc_macro2::TokenStream> {
    if let Some(inner_type) = extract_single_generic_type(last_segment) {
        ResolveFieldInfoTypeWrapper::None(
            ctor(replace_generics_in_type(inner_type.clone(), generics_map)).boxed(),
        )
        .wrap_ok()
    } else {
        Error::new_spanned(
            last_segment,
            format!("{} must have a type parameter", last_segment.ident),
        )
        .to_compile_error()
        .wrap_err()
    }
}

fn parse_resolve_field_type(
    path: &syn::Path,
    generics_map: &HashMap<syn::Ident, syn::GenericArgument>,
) -> Result<ResolveFieldInfoTypeWrapper, proc_macro2::TokenStream> {
    if let Some(last_segment) = path.segments.last() {
        // Base cases: WithSpan<T>, WithLocation<T>, WithEmbeddedLocation<T>,
        // GraphQLTypeAnnotation
        match last_segment.ident.to_string().as_str() {
            "WithSpan" => {
                return handle_case(last_segment, generics_map, ResolveFieldInfoType::WithSpan);
            }
            "WithLocation" => {
                return handle_case(
                    last_segment,
                    generics_map,
                    ResolveFieldInfoType::WithLocation,
                );
            }
            "WithGenericLocation" => {
                // NOTE: It has to be a WithGenericLocation<T, EmbeddedLocation> for
                // this to work.
                return handle_case(
                    last_segment,
                    generics_map,
                    ResolveFieldInfoType::WithEmbeddedLocation,
                );
            }
            "WithEmbeddedLocation" => {
                return handle_case(
                    last_segment,
                    generics_map,
                    ResolveFieldInfoType::WithEmbeddedLocation,
                );
            }
            "GraphQLTypeAnnotation" => {
                return handle_case(
                    last_segment,
                    generics_map,
                    ResolveFieldInfoType::GraphQLTypeAnnotation,
                );
            }
            _ => {}
        }

        // Container types: Vec<T>, Option<T>, or NonEmpty<T>
        if (last_segment.ident == "Vec"
            || last_segment.ident == "Option"
            || last_segment.ident == "NonEmpty")
            && let Some(syn::Type::Path(syn::TypePath {
                path: inner_path, ..
            })) = extract_single_generic_type(last_segment)
        {
            // Recursively parse the inner type
            let inner_wrapper = parse_resolve_field_type(inner_path, generics_map)?;

            return ResolveFieldInfoTypeWrapper::IteratorWrapper(inner_wrapper.boxed()).wrap_ok();
        }
    }

    Error::new_spanned(
        path,
        "Expected WithSpan<T>, WithLocation<T>, WithGenericLocation<T>, GraphQLTypeAnnotation, \
        Vec<T>, Option<T>, or NonEmpty<T> where T is a valid resolve field type",
    )
    .to_compile_error()
    .wrap_err()
}

fn get_resolve_field_info(
    field: &'_ syn::Field,
    index: usize,
    generics_map: &HashMap<syn::Ident, syn::GenericArgument>,
) -> Result<Option<ResolveFieldInfo>, proc_macro2::TokenStream> {
    let FieldAttributes {
        resolve_field,
        parent_variant,
        from_container_parent,
    } = collect_field_attributes(field.attrs.reference())?;

    let Some(resolve_field) = resolve_field else {
        if let Some(attr) = parent_variant {
            return Error::new_spanned(attr, "`#[parent_variant]` requires `#[resolve_field]`")
                .to_compile_error()
                .wrap_err();
        }
        if let Some(attr) = from_container_parent {
            return Error::new_spanned(
                attr,
                "`#[from_container_parent]` requires `#[resolve_field]`",
            )
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
            if let Some(attr) = from_container_parent {
                return Error::new_spanned(
                    attr,
                    "`#[resolve_field(transparent)]` cannot combine with `#[from_container_parent]`",
                )
                .to_compile_error()
                .wrap_err();
            }
            ParentConstruction::Transparent
        }
        ResolveFieldForm::Bare => {
            if let (Some(_), Some(attr)) = (parent_variant, from_container_parent) {
                return Error::new_spanned(
                    attr,
                    "cannot combine `#[parent_variant]` and `#[from_container_parent]`",
                )
                .to_compile_error()
                .wrap_err();
            }
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

fn generate_resolve_code(
    field_accessor: &proc_macro2::TokenStream,
    wrapper: &ResolveFieldInfoTypeWrapper,
    parent_construction: &ParentConstruction,
) -> proc_macro2::TokenStream {
    generate_resolve_code_recursive(wrapper, parent_construction, quote!(self.#field_accessor))
}

/// The expression passed as the child's parent. `self.path(parent)` is the
/// container's own path in both arms; the variant wrapping is the only difference.
fn new_parent_expr(
    parent_construction: &ParentConstruction,
    inner_type: &syn::Type,
) -> proc_macro2::TokenStream {
    match parent_construction {
        ParentConstruction::ContainerPath => quote!(self.path(parent)),
        ParentConstruction::EnumVariant(variant) => quote!(
            <#inner_type as ::resolve_position::ResolvePosition>::Parent::#variant(self.path(parent).into())
        ),
        ParentConstruction::Transparent => {
            Error::new_spanned(inner_type, "`transparent` does not build a field parent")
                .to_compile_error()
        }
    }
}

fn generate_resolve_code_recursive(
    wrapper: &ResolveFieldInfoTypeWrapper,
    parent_construction: &ParentConstruction,
    field_expr: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    match wrapper {
        ResolveFieldInfoTypeWrapper::None(inner) => match (**inner).reference() {
            ResolveFieldInfoType::WithSpan(inner_type) => {
                let new_parent = new_parent_expr(parent_construction, inner_type);
                quote! {
                    if #field_expr.location.contains(position) {
                        let new_parent = #new_parent;
                        return #field_expr.item.resolve(new_parent, position);
                    }
                }
            }
            ResolveFieldInfoType::WithLocation(inner_type) => {
                let new_parent = new_parent_expr(parent_construction, inner_type);
                quote! {
                    if let Some(span) = #field_expr.location.span() {
                        if span.contains(position) {
                            let new_parent = #new_parent;
                            return #field_expr.item.resolve(new_parent, position);
                        }
                    }
                }
            }
            ResolveFieldInfoType::WithEmbeddedLocation(inner_type) => {
                let new_parent = new_parent_expr(parent_construction, inner_type);
                quote! {
                    if #field_expr.location.span.contains(position) {
                        let new_parent = #new_parent;
                        return #field_expr.item.resolve(new_parent, position);
                    }
                }
            }
            ResolveFieldInfoType::GraphQLTypeAnnotation(inner_type) => {
                let new_parent = new_parent_expr(parent_construction, inner_type);
                quote! {
                    if #field_expr.span().contains(position) {
                        let new_parent = #new_parent;
                        return #field_expr.inner().resolve(new_parent, position);
                    }
                }
            }
        },

        ResolveFieldInfoTypeWrapper::IteratorWrapper(inner) => {
            let inner_code =
                generate_resolve_code_recursive(inner, parent_construction, quote!(item));

            quote! {
                for item in #field_expr.iter() {
                    #inner_code
                }
            }
        }

        ResolveFieldInfoTypeWrapper::Transparent(_) => {
            quote! {
                return #field_expr.resolve(parent, position);
            }
        }
    }
}

fn type_is_with_span(ty: &syn::Type) -> bool {
    let syn::Type::Path(type_path) = ty else {
        return false;
    };
    type_path
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "WithSpan")
}
