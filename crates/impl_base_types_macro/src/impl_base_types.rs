use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    Error, GenericParam, Ident, ItemTrait, TraitItem, parse_macro_input, punctuated::Punctuated,
};

#[derive(Clone, Copy)]
pub(crate) struct BaseType {
    pub(crate) crate_name: &'static str,
    pub(crate) name: &'static str,
    pub(crate) variant_names: &'static [&'static str],
}

pub(crate) fn impl_base_types(
    _args: TokenStream,
    item: TokenStream,
    base_types: &[BaseType],
    invocation_name: &'static str,
) -> TokenStream {
    let item_trait = parse_macro_input!(item as ItemTrait);

    let trait_name = &item_trait.ident;
    let items = &item_trait.items;

    let trait_impls_for_base_types = base_types.iter().map(|base_type| {
        let base_type_name = Ident::new(base_type.name, Span::call_site());
        let crate_name = Ident::new(base_type.crate_name, Span::call_site());

        let method_impls = items.iter().map(|item| match item {
            TraitItem::Const(trait_item_const) => Error::new_spanned(
                item,
                format!(
                    "{}: const items in traits are not supported for now ({})",
                    invocation_name, trait_item_const.ident
                ),
            )
            .to_compile_error(),
            TraitItem::Fn(trait_item_fn) => {
                let sig = &trait_item_fn.sig;
                let fn_name = &sig.ident;
                let variants = base_type.variant_names.iter().map(|variant_name| {
                    let variant_name = Ident::new(variant_name, Span::call_site());
                    quote!(
                        ::#crate_name::#base_type_name::#variant_name(x) => x.#fn_name(),
                    )
                });

                quote!(
                    #sig {
                        match self {
                            #(
                                #variants
                            )*
                        }
                    }
                )
            }
            TraitItem::Type(trait_item_type) => Error::new_spanned(
                item,
                format!(
                    "{}: associated types in traits are not supported for now ({})",
                    invocation_name, &trait_item_type.ident
                ),
            )
            .to_compile_error(),
            TraitItem::Macro(_) => Error::new_spanned(
                item,
                format!("{invocation_name}: macros in traits are not supported for now"),
            )
            .to_compile_error(),
            TraitItem::Verbatim(_) => {
                Error::new_spanned(item, format!("{invocation_name}: unknown trait item"))
                    .to_compile_error()
            }
            _ => Error::new_spanned(item, format!("{invocation_name}: Unknown trait item"))
                .to_compile_error(),
        });

        let struct_generics = base_type
            .variant_names
            .iter()
            .enumerate()
            .map(|(count, _)| Ident::new(&format!("T{count}"), Span::call_site()));
        let struct_generics_2 = struct_generics.clone();
        let trait_generics_without_angle_brackets = &item_trait.generics.params;
        let mut trait_generics = item_trait.generics.clone();
        for mut param in trait_generics.params.iter_mut() {
            if let GenericParam::Type(t) = &mut param {
                t.bounds = Punctuated::new();
            }
        }

        quote! {
            impl<
                #trait_generics_without_angle_brackets
                #(#struct_generics: #trait_name #trait_generics,)*
            > #trait_name #trait_generics for ::#crate_name::#base_type_name<
                #(#struct_generics_2,)*
            > {
                #(
                    #method_impls
                )*
            }
        }
    });

    quote! {
        #(
            #trait_impls_for_base_types
        )*
        #item_trait
    }
    .into()
}
