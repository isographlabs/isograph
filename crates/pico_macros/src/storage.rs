use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn derive_storage(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;

    let output = quote! {
        impl<Db: ::pico_core::database::Database> ::pico_core::storage::Storage<Db> for #struct_name<Db> {
            fn nodes(&self) -> &impl ::pico_core::container::Container<NodeId, DerivedNode<Db>> {
                &self.nodes
            }

            fn values(&self) -> &impl ::pico_core::container::Container<NodeId, Box<dyn DynEq>> {
                &self.values
            }

            fn sources(&self) -> &impl ::pico_core::container::Container<SourceKey, SourceNode> {
                &self.sources
            }

            fn source_values(&self) -> &impl ::pico_core::container::Container<SourceKey, Box<dyn DynEq>> {
                &self.source_values
            }

            fn params(&self) -> &impl ::pico_core::container::Container<ParamId, Box<dyn Any>> {
                &self.params
            }

            fn current_epoch(&self) -> ::pico_core::epoch::Epoch {
                self.current_epoch
            }
        }

        impl<Db: ::pico_core::database::Database> ::pico_core::storage::StorageMut<Db> for #struct_name<Db> {
            fn nodes(&mut self) -> &mut impl ::pico_core::container::Container<NodeId, DerivedNode<Db>> {
                &mut self.nodes
            }

            fn values(&mut self) -> &mut impl ::pico_core::container::Container<NodeId, Box<dyn DynEq>> {
                &mut self.values
            }

            fn sources(&mut self) -> &mut impl ::pico_core::container::Container<SourceKey, SourceNode> {
                &mut self.sources
            }

            fn source_values(&mut self) -> &mut impl ::pico_core::container::Container<SourceKey, Box<dyn DynEq>> {
                &mut self.source_values
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
