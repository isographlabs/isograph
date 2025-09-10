use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemStruct, Type, parse_macro_input};

pub(crate) fn db_macro(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_ident = input.ident.clone();
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.fields {
        syn::Fields::Named(named) => &named.named,
        _ => {
            return syn::Error::new_spanned(&input.fields, "#[db] requires named fields")
                .to_compile_error()
                .into();
        }
    };

    let mut getter_impls = Vec::new();
    let mut aux_struct_defs = Vec::new();

    for f in fields.iter() {
        let field_ident = f.ident.as_ref().unwrap();

        if field_ident == "storage" {
            continue;
        }

        if is_phantom_data(&f.ty) {
            continue;
        }

        let field_ty = &f.ty;
        let counter_ident =
            format_ident!("__{}Counter", field_ident.to_string().to_case(Case::Pascal));
        let get_ident = format_ident!("get_{field_ident}_tracked");
        let get_ident_mut = format_ident!("get_{field_ident}_tracked_mut");

        aux_struct_defs.push(quote! {
            #[derive(Clone, Copy, Default, PartialEq, Eq, Hash, ::pico_macros::Singleton)]
            struct #counter_ident(u64);
            impl #counter_ident {
                #[inline]
                fn increment(self) -> Self { Self(self.0.wrapping_add(1)) }
            }
        });

        getter_impls.push(quote! {
            #[inline]
            pub fn #get_ident(&self) -> &#field_ty {
                let _ = self.storage.get_singleton::<#counter_ident>();
                &self.#field_ident
            }

            #[inline]
            pub fn #get_ident_mut(&mut self) -> &mut #field_ty {
                let next = self.storage.get_singleton::<#counter_ident>().copied().unwrap_or_default().increment();
                self.storage.set(next);
                &mut self.#field_ident
            }
        });
    }

    let output = quote! {
        #(#aux_struct_defs)*

        impl #impl_generics #struct_ident #type_generics #where_clause {
            #(#getter_impls)*
        }

        impl #impl_generics ::pico::Database for #struct_ident #type_generics #where_clause {
            #[inline]
            fn get_storage(&self) -> &::pico::Storage<Self> {
                &self.storage
            }

            #[inline]
            fn get<T: 'static>(&self, id: ::pico::SourceId<T>) -> &T {
                self.storage.get(id)
            }

            #[inline]
            fn get_singleton<T: ::pico::Singleton + 'static>(&self) -> Option<&T> {
                self.storage.get_singleton::<T>()
            }

            #[inline]
            fn intern<T: Clone + std::hash::Hash + ::pico::DynEq + 'static>(&self, value: T) -> ::pico::MemoRef<T> {
                ::pico::intern(self, value)
            }

            #[inline]
            fn intern_ref<T: Clone + std::hash::Hash + ::pico::DynEq + 'static>(&self, value: &T) -> ::pico::MemoRef<T> {
                ::pico::intern_ref(self, value)
            }

            #[inline]
            fn set<T: ::pico::Source + ::pico::DynEq>(&mut self, source: T) -> ::pico::SourceId<T> {
                self.storage.set(source)
            }

            #[inline]
            fn remove<T>(&mut self, id: ::pico::SourceId<T>) {
                self.storage.remove(id)
            }

            #[inline]
            fn remove_singleton<T: ::pico::Singleton + 'static>(&mut self) {
                self.storage.remove_singleton::<T>()
            }

            #[inline]
            fn run_garbage_collection(&mut self) {
                self.storage.run_garbage_collection()
            }
        }

        impl #impl_generics ::pico::DatabaseDyn for #struct_ident #type_generics #where_clause {
            #[inline]
            fn get_storage_dyn(&self) -> &dyn ::pico::StorageDyn {
                use ::pico::Database;
                self.get_storage()
            }
        }
    };

    output.into()
}

fn is_phantom_data(ty: &Type) -> bool {
    matches!(ty,
        Type::Path(tp)
            if tp.path.segments.last().map(|seg| seg.ident == "PhantomData").unwrap_or(false)
    )
}
