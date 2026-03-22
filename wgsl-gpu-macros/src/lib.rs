mod arguments;
mod entry;

extern crate proc_macro;

use proc_macro::TokenStream;
use syn::DeriveInput;

#[proc_macro_derive(Arguments, attributes(wgsl_gpu))]
pub fn arguments(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    arguments::arguments(input).into()
}

#[proc_macro_attribute]
pub fn entry(_arguments: TokenStream, input: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(input as syn::Item);
    match entry::entry(item) {
        Ok(tokens) => tokens.into(),
        Err(tokens) => tokens.into_compile_error().into(),
    }
}
