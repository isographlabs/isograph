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
        Err(e) => return e.into_compile_error().into(),
    };

    match input.data {
        syn::Data::Struct(data_struct) => handle_data_struct(
            struct_name,
            resolve_position_args,
            data_struct,
            input.generics,
        ),
        syn::Data::Enum(data_enum) => {
            handle_data_enum(struct_name, resolve_position_args, data_enum)
        }
        syn::Data::Union(_) => {
            Error::new(input.span(), "This derive only works on structs and enums")
                .to_compile_error()
                .into()
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
    } = resolve_position_args;

    let generics_map = match validate_and_map_generics(input_generics, self_type_generics.clone()) {
        Ok(map) => map,
        Err(e) => {
            return e.into();
        }
    };

    let attributes_to_resolve = match data_struct
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| get_resolve_field_info(field, index, &generics_map))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(field_infos) => field_infos
            .into_iter()
            .flatten()
            .map(
                |ResolveFieldInfo {
                     field_accessor,
                     field_type,
                     parent_construction,
                 }| {
                    generate_resolve_code(&field_accessor, &field_type, &parent_construction)
                },
            )
            .collect::<Vec<_>>(),
        Err(e) => {
            return e.into();
        }
    };

    let output = quote! {
        impl ::resolve_position::ResolvePosition for #struct_name #self_type_generics {
            type Parent<'a> = #parent_type;
            type ResolvedNode<'a> = #resolved_node;

            fn resolve<'a>(
                &'a self,
                parent: Self::Parent<'a>,
                position: ::span::Span
            ) -> Self::ResolvedNode<'a> {
                #(#attributes_to_resolve)*

                return Self::ResolvedNode::#struct_name(self.path(parent).into());
            }
        }
    };

    output.into()
}

fn handle_data_enum(
    enum_name: syn::Ident,
    resolve_position_args: ResolvePositionArgs,
    data_enum: syn::DataEnum,
) -> TokenStream {
    let ResolvePositionArgs {
        parent_type,
        resolved_node,
        self_type_generics,
    } = resolve_position_args;

    let match_arms = data_enum.variants.iter().map(|variant| {
        let variant_name = &variant.ident;

        match &variant.fields {
            syn::Fields::Unnamed(fields) => {
                let mut payloads = fields.unnamed.iter();
                match (payloads.next(), payloads.next()) {
                    (Some(payload), None) => generate_enum_arm(&enum_name, variant_name, payload),
                    _ => single_payload_error(variant),
                }
            }
            _ => single_payload_error(variant),
        }
    });

    let output = quote! {
        impl ::resolve_position::ResolvePosition for #enum_name #self_type_generics {
            type Parent<'a> = #parent_type;
            type ResolvedNode<'a> = #resolved_node;

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

fn generate_enum_arm(
    enum_name: &syn::Ident,
    variant_name: &syn::Ident,
    payload: &syn::Field,
) -> proc_macro2::TokenStream {
    let attr = match find_resolve_field_attr(&payload.attrs) {
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
            let payload_type = &payload.ty;
            quote! {
                #enum_name::#variant_name(inner) => inner.resolve(
                    <#payload_type as ::resolve_position::ResolvePosition>::Parent::#parent_variant(parent.into()),
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
        Err(e) => e,
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
}

/// How an emission builds the value it passes as the child's parent.
enum ParentConstruction {
    /// Bare `#[resolve_field]`: the child's `Parent` type is the container's own
    /// path, and `self.path(parent)` is passed unwrapped.
    ContainerPath,
    /// `#[resolve_field(parent_variant = V)]`: the child's `Parent` type is an
    /// enum, and the parent value is wrapped in its variant `V`.
    EnumVariant(syn::Ident),
}

struct ResolveFieldInfo {
    field_accessor: proc_macro2::TokenStream,
    field_type: ResolveFieldInfoTypeWrapper,
    parent_construction: ParentConstruction,
}

fn find_resolve_field_attr(
    attrs: &[syn::Attribute],
) -> Result<Option<&syn::Attribute>, proc_macro2::TokenStream> {
    let mut matching = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("resolve_field"));
    match (matching.next(), matching.next()) {
        (first, None) => Ok(first),
        (_, Some(duplicate)) => Error::new_spanned(duplicate, "duplicate #[resolve_field]")
            .to_compile_error()
            .wrap_err(),
    }
}

fn parse_parent_construction(
    attr: &syn::Attribute,
) -> Result<ParentConstruction, proc_macro2::TokenStream> {
    match &attr.meta {
        syn::Meta::Path(_) => ParentConstruction::ContainerPath.wrap_ok(),
        syn::Meta::List(_) => {
            let name_value = attr
                .parse_args::<syn::MetaNameValue>()
                .map_err(|e| e.to_compile_error())?;
            if let syn::Expr::Path(value) = &name_value.value
                && name_value.path.is_ident("parent_variant")
                && let Some(variant) = value.path.get_ident()
            {
                return ParentConstruction::EnumVariant(variant.clone()).wrap_ok();
            }
            Error::new_spanned(
                &attr.meta,
                "expected `#[resolve_field(parent_variant = SomeVariant)]`",
            )
            .to_compile_error()
            .wrap_err()
        }
        syn::Meta::NameValue(name_value) => Error::new_spanned(
            name_value,
            "expected `#[resolve_field]` or `#[resolve_field(parent_variant = SomeVariant)]`",
        )
        .to_compile_error()
        .wrap_err(),
    }
}

// Attempts to extract the single generic type from angle bracketed path arguments, e.g. X<Inner>
fn extract_single_generic_type(segment: &syn::PathSegment) -> Option<&syn::Type> {
    match &segment.arguments {
        syn::PathArguments::AngleBracketed(args) => args.args.first().and_then(|arg| {
            if let syn::GenericArgument::Type(ty) = arg {
                Some(ty)
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
        ResolveFieldInfoTypeWrapper::None(Box::new(ctor(replace_generics_in_type(
            inner_type.clone(),
            generics_map,
        ))))
        .wrap_ok()
    } else {
        Err(Error::new_spanned(
            last_segment,
            format!("{} must have a type parameter", last_segment.ident),
        )
        .to_compile_error())
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

            return Ok(ResolveFieldInfoTypeWrapper::IteratorWrapper(Box::new(
                inner_wrapper,
            )));
        }
    }

    Err(Error::new_spanned(
        path,
        "Expected WithSpan<T>, WithLocation<T>, WithGenericLocation<T>, GraphQLTypeAnnotation, \
        Vec<T>, Option<T>, or NonEmpty<T> where T is a valid resolve field type",
    )
    .to_compile_error())
}

fn get_resolve_field_info(
    field: &'_ syn::Field,
    index: usize,
    generics_map: &HashMap<syn::Ident, syn::GenericArgument>,
) -> Result<Option<ResolveFieldInfo>, proc_macro2::TokenStream> {
    let Some(attr) = find_resolve_field_attr(&field.attrs)? else {
        return Ok(None);
    };

    let parent_construction = parse_parent_construction(attr)?;

    // A named field is accessed by name, a tuple field by index.
    let field_accessor = match &field.ident {
        Some(ident) => quote!(#ident),
        None => {
            let index = syn::Index::from(index);
            quote!(#index)
        }
    };

    if let syn::Type::Path(syn::TypePath { path, .. }) = &field.ty {
        match parse_resolve_field_type(path, generics_map) {
            Ok(field_type) => ResolveFieldInfo {
                field_accessor,
                field_type,
                parent_construction,
            }
            .wrap_some()
            .wrap_ok(),
            Err(e) => Err(e),
        }
    } else {
        Error::new_spanned(&field.ty, "#[resolve_field] fields must be path types")
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
    }
}

fn generate_resolve_code_recursive(
    wrapper: &ResolveFieldInfoTypeWrapper,
    parent_construction: &ParentConstruction,
    field_expr: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    match wrapper {
        ResolveFieldInfoTypeWrapper::None(inner) => match &**inner {
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
    }
}
