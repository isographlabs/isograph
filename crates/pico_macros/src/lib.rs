mod memo_macro;
mod source;

extern crate proc_macro2;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn memo(args: TokenStream, input: TokenStream) -> TokenStream {
    memo_macro::memo(args, input)
}

#[proc_macro_derive(Source, attributes(key))]
pub fn source(input: TokenStream) -> TokenStream {
    source::source(input)
}
