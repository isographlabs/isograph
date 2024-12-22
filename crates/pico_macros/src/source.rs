use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub(crate) fn source(_args: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    let struct_name = input.clone().ident;
    let key = struct_name.to_string();

    let output = quote! {
        #[derive(Debug, Clone, PartialEq, Eq)]
        #input

        impl #struct_name {
            pub fn set(
                self,
                db: &mut pico::database::Database,
                static_key: &'static str,
            ) {
                db.current_epoch += 1;
                let param_id = pico::params::param_id(db, static_key);
                let node_id = pico::node::NodeId::source(#key, param_id);
                db.sources.put(node_id, pico::node::SourceNode {
                    time_calculated: db.current_epoch,
                });
                db.values.put(node_id, Box::new(self));
            }

            pub fn get(db: &mut pico::database::Database, static_key: &'static str) -> Self {
                let param_id = pico::params::param_id(db, static_key);
                let node_id = pico::node::NodeId::source(#key, param_id);
                let time_calculated = db.sources
                    .get(&node_id)
                    .expect("node should exist. This is indicative of a bug in Isograph.")
                    .time_calculated;
                db.register_dependency(node_id, time_calculated);
                db.values
                    .get(&node_id)
                    .expect("value should exist. This is indicative of a bug in Isograph.")
                    .as_any()
                    .downcast_ref::<Self>()
                    .expect("unexpected struct type. This is indicative of a bug in Isograph.")
                    .clone()
            }

            pub fn remove(db: &mut pico::database::Database, static_key: &'static str) {
                db.current_epoch += 1;
                let param_id = pico::params::param_id(db, static_key);
                let node_id = pico::node::NodeId::source(#key, param_id);
                db.sources.pop(&node_id);
                db.values.pop(&node_id);
            }
        }
    };

    output.into()
}
