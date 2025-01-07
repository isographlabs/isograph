use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn derive_db(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;

    let output = quote! {
        impl ::pico_core::database::Database for #struct_name {
            fn storage(&self) -> &impl ::pico_core::storage::Storage<Self> {
                &self.storage
            }

            fn storage_mut(&mut self) -> &mut impl ::pico_core::storage::StorageMut<Self> {
                &mut self.storage
            }

            fn current_epoch(&self) -> ::pico_core::epoch::Epoch {
                use ::pico_core::storage::Storage;
                self.storage().current_epoch()
            }

            fn increment_epoch(&mut self) -> ::pico_core::epoch::Epoch {
                use ::pico_core::storage::StorageMut;
                self.storage_mut().increment_epoch()
            }

            fn get<T: Clone + 'static>(&mut self, id: ::pico_core::source::SourceId<T>) -> T {
                use ::pico_core::{storage::Storage, container::Container};
                let time_calculated = self.storage()
                    .sources()
                    .get(&id.key)
                    .expect("node should exist. This is indicative of a bug in Isograph.")
                    .time_calculated;
                ::pico::memo::register_dependency(self, ::pico_core::node::NodeKind::Source(id.key), time_calculated);
                self.storage()
                    .source_values()
                    .get(&id.key)
                    .expect("value should exist. This is indicative of a bug in Isograph.")
                    .as_any()
                    .downcast_ref::<T>()
                    .expect("unexpected struct type. This is indicative of a bug in Isograph.")
                    .clone()
            }

            fn set<T>(&mut self, source: T) -> ::pico_core::source::SourceId<T>
            where T: ::pico_core::source::Source + ::pico_core::dyn_eq::DynEq
            {
                use ::pico_core::{storage::StorageMut, container::Container};
                let current_epoch = self.increment_epoch();
                let id = ::pico_core::source::SourceId::new(&source);
                self.storage_mut().sources().insert(id.key, ::pico_core::node::SourceNode {
                    time_calculated: current_epoch,
                });
                self.storage_mut().source_values().insert(id.key, Box::new(source));
                id
            }

            fn remove<T>(&mut self, id: ::pico_core::source::SourceId<T>) {
                use ::pico_core::{storage::StorageMut, container::Container};
                self.increment_epoch();
                self.storage_mut().sources().remove(&id.key);
                self.storage_mut().source_values().remove(&id.key);
            }
        }
    };

    output.into()
}
