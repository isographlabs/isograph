use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, Error};

pub(crate) fn resolve_position_macro(item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as syn::DeriveInput);
    let struct_name = input.ident.clone();

    let ResolvePositionArgs {
        parent_type,
        resolved_node,
    } = match deluxe::extract_attributes(&mut input) {
        Ok(resolve_position_args) => resolve_position_args,
        Err(e) => return e.into_compile_error().into(),
    };

    match input.data {
        syn::Data::Struct(data_struct) => {
            handle_data_struct(struct_name, parent_type, resolved_node, data_struct)
        }
        syn::Data::Enum(data_enum) => {
            handle_data_enum(struct_name, parent_type, resolved_node, data_enum)
        }
        _ => Error::new(input.span(), "This derive only works on structs")
            .to_compile_error()
            .into(),
    }
}

fn handle_data_struct(
    struct_name: syn::Ident,
    parent_type: syn::Type,
    resolved_node: syn::Type,
    data_struct: syn::DataStruct,
) -> TokenStream {
    let attributes_to_resolve = data_struct
        .fields
        .iter()
        .flat_map(|field| get_resolve_field_info(field).ok())
        .flatten()
        .map(|ResolveFieldInfo { inner_type, field_name, is_iter}| {
            if is_iter {
                quote! {
                    for with_span in self.#field_name.iter() {
                        if with_span.span.contains(position) {
                            let new_parent = <#inner_type as ::resolve_position::ResolvePosition>::Parent::#struct_name(self.path(parent));
                            return with_span.item.resolve(new_parent, position);
                        }
                    }
                }
            } else {
                quote! {
                    if self.#field_name.span.contains(position) {
                        let new_parent = <#inner_type as ::resolve_position::ResolvePosition>::Parent::#struct_name(self.path(parent));
                        return self.#field_name.item.resolve(new_parent, position);
                    }
                }
            }
        });

    let output = quote! {
        impl ::resolve_position::ResolvePosition for #struct_name {
            type Parent<'a> = #parent_type;
            type ResolvedNode<'a> = #resolved_node;

            fn resolve<'a>(
                &'a self,
                parent: Self::Parent<'a>,
                position: ::common_lang_types::Span
            ) -> Self::ResolvedNode<'a> {
                #(#attributes_to_resolve)*

                return Self::ResolvedNode::#struct_name(self.path(parent));
            }
        }
    };

    output.into()
}

fn handle_data_enum(
    enum_name: syn::Ident,
    parent_type: syn::Type,
    resolved_node: syn::Type,
    data_enum: syn::DataEnum,
) -> TokenStream {
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
        impl ::resolve_position::ResolvePosition for #enum_name {
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
}

struct ResolveFieldInfo<'a> {
    inner_type: &'a syn::Type,
    field_name: syn::Ident,
    is_iter: bool,
}

fn get_resolve_field_info(
    field: &'_ syn::Field,
) -> Result<Option<ResolveFieldInfo<'_>>, proc_macro2::TokenStream> {
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
        // Check for direct WithSpan<T>
        if let Some(last_segment) = path.segments.last() {
            if last_segment.ident == "WithSpan" {
                if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Ok(Some(ResolveFieldInfo {
                            inner_type: inner_ty,
                            field_name,
                            is_iter: false,
                        }));
                    }
                }
                return Err(Error::new_spanned(
                    &field.ty,
                    "#[resolve_field] field must be WithSpan<T> with a type parameter",
                )
                .to_compile_error());
            }

            // Check for Vec<WithSpan<T>>
            if last_segment.ident == "Vec" || last_segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments {
                    if let Some(syn::GenericArgument::Type(syn::Type::Path(inner_path))) =
                        args.args.first()
                    {
                        if let Some(inner_segment) = inner_path.path.segments.last() {
                            if inner_segment.ident == "WithSpan" {
                                if let syn::PathArguments::AngleBracketed(inner_args) =
                                    &inner_segment.arguments
                                {
                                    if let Some(syn::GenericArgument::Type(inner_ty)) =
                                        inner_args.args.first()
                                    {
                                        return Ok(Some(ResolveFieldInfo {
                                            inner_type: inner_ty,
                                            field_name,
                                            is_iter: true,
                                        }));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Err(Error::new_spanned(
        &field.ty,
        "#[resolve_field] fields must be of type WithSpan<T> or Vec<WithSpan<T>>",
    )
    .to_compile_error())
}
