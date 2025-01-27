use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn derive_storage(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;

    let output = quote! {
        impl<Db: ::pico_core::database::Database> ::pico_core::storage::Storage<Db> for #struct_name<Db> {
            fn derived_nodes(&self) -> &impl ::pico_core::container::Container<DerivedNodeId, DerivedNode<Db>> {
                &self.derived_nodes
            }

            fn source_nodes(&self) -> &impl ::pico_core::container::Container<Key, SourceNode> {
                &self.source_nodes
            }

            fn params(&self) -> &impl ::pico_core::container::Container<ParamId, Box<dyn Any>> {
                &self.params
            }

            fn current_epoch(&self) -> ::pico_core::epoch::Epoch {
                self.current_epoch
            }
        }

        impl<Db: ::pico_core::database::Database> ::pico_core::storage::StorageMut<Db> for #struct_name<Db> {
            fn derived_nodes(&mut self) -> &mut impl ::pico_core::container::Container<DerivedNodeId, DerivedNode<Db>> {
                &mut self.derived_nodes
            }

            fn source_nodes(&mut self) -> &mut impl ::pico_core::container::Container<Key, SourceNode> {
                &mut self.source_nodes
            }

            fn params(&mut self) -> &mut impl ::pico_core::container::Container<ParamId, Box<dyn Any>> {
                &mut self.params
            }

            fn increment_epoch(&mut self) -> ::pico_core::epoch::Epoch {
                self.current_epoch.increment()
            }

            fn dependency_stack(&mut self) ->&mut Vec<Vec<(Epoch, Dependency)>> {
                &mut self.dependency_stack
            }
        }
    };

    output.into()
}
