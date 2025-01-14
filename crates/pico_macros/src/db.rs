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
                let current_epoch = self.current_epoch();
                let time_updated = self.storage()
                    .source_nodes()
                    .get(&id.key)
                    .expect("node should exist. This is indicative of a bug in Pico.")
                    .time_updated;
                ::pico::memo::register_dependency_in_parent_memoized_fn(
                    self,
                    ::pico_core::node::NodeKind::Source(id.key),
                    time_updated,
                    current_epoch,
                );
                self.storage()
                    .source_nodes()
                    .get(&id.key)
                    .expect("value should exist. This is indicative of a bug in Pico.")
                    .value
                    .as_any()
                    .downcast_ref::<T>()
                    .expect("unexpected struct type. This is indicative of a bug in Pico.")
                    .clone()
            }

            fn set<T>(&mut self, source: T) -> ::pico_core::source::SourceId<T>
            where T: ::pico_core::source::Source + ::pico_core::dyn_eq::DynEq
            {
                use ::pico_core::{storage::StorageMut, container::Container};
                let current_epoch = self.increment_epoch();
                let id = ::pico_core::source::SourceId::new(&source);
                self.storage_mut().source_nodes().insert(id.key, ::pico_core::node::SourceNode {
                    time_updated: current_epoch,
                    value: Box::new(source),
                });
                id
            }

            fn remove<T>(&mut self, id: ::pico_core::source::SourceId<T>) {
                use ::pico_core::{storage::StorageMut, container::Container};
                self.increment_epoch();
                self.storage_mut().source_nodes().remove(&id.key);
            }
        }
    };

    output.into()
}
