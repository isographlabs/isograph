mod map_generics;
mod resolve_position_macro;

use proc_macro::TokenStream;

use crate::resolve_position_macro::resolve_position_macro;

#[proc_macro_derive(
    ResolvePosition,
    attributes(from_container_parent, parent_variant, resolve_field, resolve_position)
)]
pub fn resolve_position(input: TokenStream) -> TokenStream {
    resolve_position_macro(input)
}
