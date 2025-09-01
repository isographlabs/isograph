use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, parse_macro_input};

pub(crate) fn source_macro(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = input.ident.clone();

    let fields = match input.data {
        Data::Struct(ref data) => match &data.fields {
            Fields::Named(fields) => fields.named.clone(),
            _ => {
                return Error::new_spanned(&data.fields, "expected named fields")
                    .to_compile_error()
                    .into();
            }
        },
        _ => {
            return Error::new_spanned(&input, "expected a struct")
                .to_compile_error()
                .into();
        }
    };

    let key_field_name = fields
        .iter()
        .find(|field| {
            field.attrs.iter().any(|attr| {
                attr.path()
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "key")
            })
        })
        .and_then(|field| field.ident.clone());

    let field_name = match key_field_name {
        Some(field_name) => field_name,
        None => {
            return Error::new_spanned(
                &struct_name,
                "#[key] attribute must be set on a struct field",
            )
            .to_compile_error()
            .into();
        }
    };

    let output = quote! {
        impl ::pico::Source for #struct_name {
            fn get_key(&self) -> ::pico::Key {
                use ::std::hash::{Hash, Hasher, DefaultHasher};
                let mut s = DefaultHasher::new();
                ::core::any::TypeId::of::<#struct_name>().hash(&mut s);
                self.#field_name.hash(&mut s);
                s.finish().into()
            }
        }
    };

    output.into()
}
