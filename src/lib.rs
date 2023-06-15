extern crate proc_macro;

use proc_macro::TokenStream;
use std::fs;
use quote::quote;
use syn::{parse_macro_input, LitStr};

#[proc_macro]
pub fn toml(file_path: TokenStream) -> TokenStream {
    let file_path = parse_macro_input!(file_path as LitStr);

    let content = fs::read_to_string(file_path.value()).unwrap();

    let expanded = quote! {
        let file_conten = #content;
    };

    TokenStream::from(expanded)
}
