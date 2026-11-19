use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, parse_macro_input};

pub(crate) fn source_macro(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = &input.ident;

    let fields = match input.data {
        Data::Struct(ref data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
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

    let Some(key_field) = fields.iter().find(|field| {
        field.attrs.iter().any(|attr| {
            attr.path()
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "key")
        })
    }) else {
        return Error::new_spanned(
            struct_name,
            "#[key] attribute must be set on a struct field",
        )
        .to_compile_error()
        .into();
    };
    let field_name = &key_field.ident;
    let key_type = &key_field.ty;

    let output = quote! {
        impl #struct_name {
            /// Returns the source ID for this key, even before the source is set.
            pub fn id_for_key(key: &#key_type) -> ::pico::SourceId<Self> {
                use ::std::hash::{Hash, Hasher, DefaultHasher};
                let mut s = DefaultHasher::new();
                ::core::any::TypeId::of::<Self>().hash(&mut s);
                key.hash(&mut s);
                ::pico::Key::from(s.finish()).into()
            }
        }

        impl ::pico::Source for #struct_name {
            fn get_key(&self) -> ::pico::Key {
                Self::id_for_key(&self.#field_name).key
            }
        }
    };

    output.into()
}
