mod resolve_position_macro;

use proc_macro::TokenStream;

use crate::resolve_position_macro::resolve_position_macro;

#[proc_macro_derive(
    ResolvePosition,
    attributes(resolve_field, resolve_position, self_type_generics)
)]
pub fn resolve_position(input: TokenStream) -> TokenStream {
    resolve_position_macro(input)
}
