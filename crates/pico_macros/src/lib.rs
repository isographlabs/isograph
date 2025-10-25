mod db_macro;
mod legacy_memo_macro;
mod singleton_macro;
mod source_macro;

extern crate proc_macro2;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn legacy_memo(args: TokenStream, input: TokenStream) -> TokenStream {
    legacy_memo_macro::legacy_memo_macro(args, input)
}

#[proc_macro_derive(Source, attributes(key))]
pub fn source(input: TokenStream) -> TokenStream {
    source_macro::source_macro(input)
}

#[proc_macro_derive(Singleton)]
pub fn singleton(input: TokenStream) -> TokenStream {
    singleton_macro::singleton_macro(input)
}

#[proc_macro_derive(Db, attributes(tracked))]
pub fn db(input: TokenStream) -> TokenStream {
    db_macro::db_macro(input)
}
