use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub(crate) fn singleton(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = input.ident.clone();

    let output = quote! {
        impl ::pico::Source for #struct_name {
            fn get_key(&self) -> ::pico::Key {
                use ::std::hash::{Hash, Hasher, DefaultHasher};
                let mut s = DefaultHasher::new();
                ::core::any::TypeId::of::<#struct_name>().hash(&mut s);
                s.finish().into()
            }
        }
    };

    output.into()
}
