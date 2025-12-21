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
        .map(|field| get_resolve_field_info(field, &generics_map))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(field_infos) => field_infos
            .into_iter()
            .flatten()
            .map(
                |ResolveFieldInfo {
                     field_name,
                     field_type,
                 }| {
                    generate_resolve_code(&field_name, &field_type, &struct_name)
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
                position: ::common_lang_types::Span
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
            syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                // Single unnamed field - delegate to it
                quote! {
                    #enum_name::#variant_name(inner) => inner.item.resolve(parent, position)
                }
            }
            _ => {
                // Named fields or multiple unnamed fields - error or handle differently
                Error::new_spanned(
                    variant,
                    "ResolvePosition only supports enum variants with a single unnamed field",
                )
                .to_compile_error()
            }
        }
    });

    let output = quote! {
        impl ::resolve_position::ResolvePosition for #enum_name #self_type_generics {
            type Parent<'a> = #parent_type;
            type ResolvedNode<'a> = #resolved_node;

            fn resolve<'a>(
                &'a self,
                parent: Self::Parent<'a>,
                position: ::common_lang_types::Span
            ) -> Self::ResolvedNode<'a> {
                match self {
                    #(#match_arms),*
                }
            }
        }
    };

    output.into()
}

#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(resolve_position))]
struct ResolvePositionArgs {
    parent_type: syn::Type,
    resolved_node: syn::Type,
    self_type_generics: Option<syn::AngleBracketedGenericArguments>,
}

enum ResolveFieldInfoType {
    WithLocation(syn::Type),
    WithEmbeddedLocation(syn::Type),
    WithSpan(syn::Type),
    GraphQLTypeAnnotation(syn::Type),
}

enum ResolveFieldInfoTypeWrapper {
    None(Box<ResolveFieldInfoType>),
    IteratorWrapper(Box<ResolveFieldInfoTypeWrapper>),
}

struct ResolveFieldInfo {
    field_name: syn::Ident,
    field_type: ResolveFieldInfoTypeWrapper,
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
        // Base cases: WithLocation<T>, WithEmbeddedLocation<T>, GraphQLTypeAnnotation or WithSpan<T>
        match last_segment.ident.to_string().as_str() {
            "WithLocation" => {
                return handle_case(
                    last_segment,
                    generics_map,
                    ResolveFieldInfoType::WithLocation,
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
            "WithSpan" => {
                return handle_case(last_segment, generics_map, ResolveFieldInfoType::WithSpan);
            }
            _ => {}
        }

        // Container types: Vec<T> or Option<T>
        if (last_segment.ident == "Vec" || last_segment.ident == "Option")
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
        "Expected WithLocation<T>, WithSpan<T>, GraphQLTypeAnnotation, Vec<T>, or Option<T> where T is a valid resolve field type",
    )
    .to_compile_error())
}

fn get_resolve_field_info(
    field: &'_ syn::Field,
    generics_map: &HashMap<syn::Ident, syn::GenericArgument>,
) -> Result<Option<ResolveFieldInfo>, proc_macro2::TokenStream> {
    let has_resolve = field
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("resolve_field"));

    if !has_resolve {
        return Ok(None);
    }

    let field_name = field.ident.clone().ok_or_else(|| {
        Error::new_spanned(field, "#[resolve_field] can only be used on named fields")
            .to_compile_error()
    })?;

    if let syn::Type::Path(syn::TypePath { path, .. }) = &field.ty {
        match parse_resolve_field_type(path, generics_map) {
            Ok(field_type) => ResolveFieldInfo {
                field_name,
                field_type,
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
    field_name: &syn::Ident,
    wrapper: &ResolveFieldInfoTypeWrapper,
    struct_name: &syn::Ident,
) -> proc_macro2::TokenStream {
    generate_resolve_code_recursive(wrapper, struct_name, quote!(self.#field_name))
}

fn generate_resolve_code_recursive(
    wrapper: &ResolveFieldInfoTypeWrapper,
    struct_name: &syn::Ident,
    field_expr: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    match wrapper {
        ResolveFieldInfoTypeWrapper::None(inner) => match &**inner {
            ResolveFieldInfoType::WithLocation(inner_type) => quote! {
                if let Some(span) = #field_expr.location.span() {
                    if span.contains(position) {
                        let new_parent = <#inner_type as ::resolve_position::ResolvePosition>::Parent::#struct_name(self.path(parent).into());
                        return #field_expr.item.resolve(new_parent, position);
                    }
                }
            },
            ResolveFieldInfoType::WithEmbeddedLocation(inner_type) => quote! {
                if #field_expr.embedded_location.span.contains(position) {
                    let new_parent = <#inner_type as ::resolve_position::ResolvePosition>::Parent::#struct_name(self.path(parent).into());
                    return #field_expr.item.resolve(new_parent, position);
                }
            },
            ResolveFieldInfoType::WithSpan(inner_type) => quote! {
                if #field_expr.span.contains(position) {
                    let new_parent = <#inner_type as ::resolve_position::ResolvePosition>::Parent::#struct_name(self.path(parent).into());
                    return #field_expr.item.resolve(new_parent, position);
                }
            },
            ResolveFieldInfoType::GraphQLTypeAnnotation(inner_type) => quote! {
                if #field_expr.span().contains(position) {
                    let new_parent = <#inner_type as ::resolve_position::ResolvePosition>::Parent::#struct_name(self.path(parent).into());
                    return #field_expr.inner().resolve(new_parent, position);
                }
            },
        },

        ResolveFieldInfoTypeWrapper::IteratorWrapper(inner) => {
            let inner_code = generate_resolve_code_recursive(inner, struct_name, quote!(item));

            quote! {
                for item in #field_expr.iter() {
                    #inner_code
                }
            }
        }
    }
}
