use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub(crate) fn db_macro(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = input.ident.clone();

    let output = quote! {
        impl ::pico::Database for #struct_name {
            fn get_storage(&self) -> &::pico::Storage<Self> {
                &self.storage
            }

            fn get<T: 'static>(&self, id: ::pico::SourceId<T>) -> &T {
                self.storage.get(id)
            }

            fn get_singleton<T: ::pico::Singleton + 'static>(&self) -> Option<&T> {
                self.storage.get_singleton::<T>()
            }

            fn intern<T: Clone + std::hash::Hash + ::pico::DynEq + 'static>(&self, value: T) -> ::pico::MemoRef<T> {
                ::pico::intern(self, value)
            }

            fn intern_ref<T: Clone + std::hash::Hash + ::pico::DynEq + 'static>(&self, value: &T) -> ::pico::MemoRef<T> {
                ::pico::intern_ref(self, value)
            }

            fn set<T: ::pico::Source + ::pico::DynEq>(&mut self, source: T) -> ::pico::SourceId<T> {
                self.storage.set(source)
            }

            fn remove<T>(&mut self, id: ::pico::SourceId<T>) {
                self.storage.remove(id)
            }

            fn remove_singleton<T: ::pico::Singleton + 'static>(&mut self) {
                self.storage.remove_singleton::<T>()
            }

            fn run_garbage_collection(&mut self) {
                self.storage.run_garbage_collection()
            }
        }

        impl ::pico::DatabaseDyn for #struct_name {
            fn get_storage_dyn(&self) -> &dyn ::pico::StorageDyn {
                use ::pico::Database;
                self.get_storage()
            }
        }
    };

    output.into()
}
