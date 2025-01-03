mod memo;
mod source;

extern crate proc_macro2;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn memo(args: TokenStream, input: TokenStream) -> TokenStream {
    memo::memo(args, input)
}

#[proc_macro_attribute]
pub fn source(args: TokenStream, input: TokenStream) -> TokenStream {
    source::source(args, input)
}
