use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

pub(crate) fn singleton_macro(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = input.ident.clone();

    let output = quote! {
        impl ::pico::Singleton for #struct_name {
            fn get_singleton_key() -> ::pico::Key {
                use ::std::hash::{Hash, Hasher, DefaultHasher};
                let mut s = DefaultHasher::new();
                ::core::any::TypeId::of::<#struct_name>().hash(&mut s);
                s.finish().into()
            }
        }

        impl ::pico::Source for #struct_name {
            fn get_key(&self) -> ::pico::Key {
                <#struct_name as ::pico::Singleton>::get_singleton_key()
            }
        }
    };

    output.into()
}
