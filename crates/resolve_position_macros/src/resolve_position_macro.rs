use std::collections::HashMap;

use prelude::Postfix;
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Error,
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
};

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
    args: ResolvePositionArgs,
    data_struct: syn::DataStruct,
    input_generics: syn::Generics,
) -> TokenStream {
    match args.self_type_generics.reference() {
        None => {
            let parent_type = match require_parent_type(&args) {
                Ok(parent_type) => parent_type,
                Err(e) => return e.to(),
            };
            let field_infos = match collect_field_infos(&data_struct, &HashMap::new()) {
                Ok(field_infos) => field_infos,
                Err(e) => return e.to(),
            };
            let (impl_generics, ty_generics, where_clause) = input_generics.split_for_impl();
            emit_one_impl(EmitImpl {
                struct_name: struct_name.reference(),
                resolved_node: args.resolved_node.reference(),
                parent_type: parent_type.reference(),
                on_unmatched_span: args.on_unmatched_span.as_ref(),
                impl_generics: quote!(#impl_generics),
                ty_generics: quote!(#ty_generics),
                where_clause: quote!(#where_clause),
                field_infos: field_infos.reference(),
            })
            .to()
        }
        Some(pins) => {
            if let Some(parent_type) = args.parent_type.as_ref() {
                return Error::new_spanned(
                    parent_type,
                    "`parent_type` is on each pin when `self_type_generics` is present",
                )
                .to_compile_error()
                .to();
            }
            if let Err(e) =
                require_from_path_with_pins(pins.0.len(), args.on_unmatched_span.as_ref())
            {
                return e.to();
            }
            let mut impls = Vec::new();
            for pin in pins.0.iter() {
                let generics_map = match validate_and_map_generics(
                    input_generics.clone(),
                    pin.args.clone().wrap_some(),
                ) {
                    Ok(generics_map) => generics_map,
                    Err(e) => return e.to(),
                };
                let field_infos = match collect_field_infos(&data_struct, generics_map.reference())
                {
                    Ok(field_infos) => field_infos,
                    Err(e) => return e.to(),
                };
                let pin_args = pin.args.reference();
                impls.push(emit_one_impl(EmitImpl {
                    struct_name: struct_name.reference(),
                    resolved_node: args.resolved_node.reference(),
                    parent_type: pin.parent_type.reference(),
                    on_unmatched_span: args.on_unmatched_span.as_ref(),
                    impl_generics: quote!(),
                    ty_generics: quote!(#pin_args),
                    where_clause: quote!(),
                    field_infos: field_infos.reference(),
                }));
            }
            quote!(#(#impls)*).to()
        }
    }
}

fn collect_field_infos(
    data_struct: &syn::DataStruct,
    generics_map: &HashMap<syn::Ident, syn::GenericArgument>,
) -> Result<Vec<ResolveFieldInfo>, proc_macro2::TokenStream> {
    data_struct
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| get_resolve_field_info(field, index, generics_map))
        .collect::<Result<Vec<_>, _>>()
        .map(|field_infos| field_infos.into_iter().flatten().collect())
}

struct EmitImpl<'a> {
    struct_name: &'a syn::Ident,
    resolved_node: &'a syn::Type,
    parent_type: &'a syn::Type,
    on_unmatched_span: Option<&'a syn::Ident>,
    impl_generics: proc_macro2::TokenStream,
    ty_generics: proc_macro2::TokenStream,
    where_clause: proc_macro2::TokenStream,
    field_infos: &'a [ResolveFieldInfo],
}

fn emit_one_impl(
    EmitImpl {
        struct_name,
        resolved_node,
        parent_type,
        on_unmatched_span,
        impl_generics,
        ty_generics,
        where_clause,
        field_infos,
    }: EmitImpl<'_>,
) -> proc_macro2::TokenStream {
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

    let has_transparent = field_infos
        .iter()
        .any(|info| matches!(info.field_type, ResolveFieldInfoTypeWrapper::Transparent(_)));

    let mut parent_predicates = quote!(Self: 'a).wrap_vec();
    if let Some(bound) = qself_trait_bound(parent_type) {
        parent_predicates.push(bound);
    }

    let mut resolved_node_predicates = field_resolved_node_predicates(
        field_infos,
        resolved_node,
        parent_type,
        struct_name,
        ty_generics.reference(),
    );

    // A transparent field always answers; the container is not a path segment.
    let unmatched = if has_transparent {
        quote!()
    } else {
        match on_unmatched_span {
            None => quote! {
                return Self::ResolvedNode::#struct_name(self.path(parent).to());
            },
            Some(ident) if ident == "struct_name" => quote! {
                return Self::ResolvedNode::#struct_name(self.path(parent).to());
            },
            Some(ident) if ident == "from_path" => {
                resolved_node_predicates.push(from_path_predicate(
                    resolved_node,
                    struct_name,
                    ty_generics.reference(),
                    parent_type,
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

    quote! {
        impl #impl_generics ::resolve_position::ResolvePosition for #struct_name #ty_generics #where_clause {
            type Parent<'a>
                = #parent_type
            where
                #(#parent_predicates),*;
            type ResolvedNode<'a>
                = #resolved_node
            where
                #(#resolved_node_predicates),*;

            fn resolve<'a>(
                &'a self,
                parent: Self::Parent<'a>,
                position: ::span::Span
            ) -> Self::ResolvedNode<'a> {
                #(#attributes_to_resolve)*

                #unmatched
            }
        }
    }
}

fn require_parent_type(args: &ResolvePositionArgs) -> Result<syn::Type, proc_macro2::TokenStream> {
    args.parent_type.clone().ok_or_else(|| {
        Error::new_spanned(
            args.resolved_node.reference(),
            "`parent_type` is required when `self_type_generics` is omitted",
        )
        .to_compile_error()
    })
}

fn require_from_path_with_pins(
    pin_count: usize,
    on_unmatched_span: Option<&syn::Ident>,
) -> Result<(), proc_macro2::TokenStream> {
    if pin_count < 2 {
        return ().wrap_ok();
    }
    match on_unmatched_span {
        Some(ident) if ident == "from_path" => ().wrap_ok(),
        Some(ident) => Error::new_spanned(
            ident,
            "`on_unmatched_span = from_path` is required when `self_type_generics` has more than one pin",
        )
        .to_compile_error()
        .wrap_err(),
        None => Error::new(
            proc_macro2::Span::call_site(),
            "`on_unmatched_span = from_path` is required when `self_type_generics` has more than one pin",
        )
        .to_compile_error()
        .wrap_err(),
    }
}

fn qself_trait_bound(parent_type: &syn::Type) -> Option<proc_macro2::TokenStream> {
    let syn::Type::Path(type_path) = parent_type else {
        return None;
    };
    let qself = type_path.qself.as_ref()?;
    qself.as_token.as_ref()?;
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
        // `return field.resolve(...)` must be `#resolved_node`.
        predicates.push(quote! {
            #inner_type: ::resolve_position::ResolvePosition<
                ResolvedNode<'a> = #resolved_node
            >
        });
        if matches!(info.parent_construction, ParentConstruction::ContainerPath) {
            // Bare `#[resolve_field]` passes `self.path(parent)`.
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

// `on_unmatched_span = from_path`: `self.path(parent).to()`.
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

fn handle_data_enum(
    enum_name: syn::Ident,
    args: ResolvePositionArgs,
    data_enum: syn::DataEnum,
    input_generics: syn::Generics,
) -> TokenStream {
    if let Some(ident) = args.on_unmatched_span.as_ref() {
        return Error::new_spanned(
            ident,
            "`on_unmatched_span` is a struct attribute; enums have no unmatched-span arm",
        )
        .to_compile_error()
        .to();
    }

    let match_arms = data_enum
        .variants
        .iter()
        .map(|variant| {
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
        })
        .collect::<Vec<_>>();

    match args.self_type_generics.reference() {
        None => {
            let parent_type = match require_parent_type(&args) {
                Ok(parent_type) => parent_type,
                Err(e) => return e.to(),
            };
            let (impl_generics, ty_generics, where_clause) = input_generics.split_for_impl();
            emit_one_enum_impl(
                enum_name.reference(),
                parent_type.reference(),
                args.resolved_node.reference(),
                quote!(#impl_generics),
                quote!(#ty_generics),
                quote!(#where_clause),
                match_arms.reference(),
            )
            .to()
        }
        Some(pins) => {
            if let Some(parent_type) = args.parent_type.as_ref() {
                return Error::new_spanned(
                    parent_type,
                    "`parent_type` is on each pin when `self_type_generics` is present",
                )
                .to_compile_error()
                .to();
            }
            let mut impls = Vec::new();
            for pin in pins.0.iter() {
                if let Err(e) =
                    validate_and_map_generics(input_generics.clone(), pin.args.clone().wrap_some())
                {
                    return e.to();
                }
                let pin_args = pin.args.reference();
                impls.push(emit_one_enum_impl(
                    enum_name.reference(),
                    pin.parent_type.reference(),
                    args.resolved_node.reference(),
                    quote!(),
                    quote!(#pin_args),
                    quote!(),
                    match_arms.reference(),
                ));
            }
            quote!(#(#impls)*).to()
        }
    }
}

fn emit_one_enum_impl(
    enum_name: &syn::Ident,
    parent_type: &syn::Type,
    resolved_node: &syn::Type,
    impl_generics: proc_macro2::TokenStream,
    ty_generics: proc_macro2::TokenStream,
    where_clause: proc_macro2::TokenStream,
    match_arms: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    quote! {
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
    }
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
    parent_type: Option<syn::Type>,
    resolved_node: syn::Type,
    self_type_generics: Option<SelfTypeGenerics>,
    on_unmatched_span: Option<syn::Ident>,
}

struct SelfTypeGenerics(Vec<SelfTypePin>);

struct SelfTypePin {
    /// One argument per generic parameter of the struct, in declaration order.
    /// `validate_and_map_generics` errors if the counts differ.
    args: syn::AngleBracketedGenericArguments,
    parent_type: syn::Type,
}

impl Parse for SelfTypeGenerics {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::bracketed!(content in input);
        let mut pins = Vec::new();
        while !content.is_empty() {
            let inner;
            syn::parenthesized!(inner in content);
            let args = inner.parse::<syn::AngleBracketedGenericArguments>()?;
            inner.parse::<syn::Token![,]>()?;
            let parent_type = inner.parse::<syn::Type>()?;
            pins.push(SelfTypePin { args, parent_type });
            if content.peek(syn::Token![,]) {
                content.parse::<syn::Token![,]>()?;
            }
        }
        if pins.is_empty() {
            return Error::new(
                input.span(),
                "`self_type_generics` must contain at least one pin",
            )
            .wrap_err();
        }
        SelfTypeGenerics(pins).wrap_ok()
    }
}

impl deluxe::ParseMetaItem for SelfTypeGenerics {
    fn parse_meta_item(input: ParseStream, _mode: deluxe::ParseMode) -> deluxe::Result<Self> {
        input.parse()
    }
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
