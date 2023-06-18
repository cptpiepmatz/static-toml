extern crate proc_macro;

use std::path::{Path, PathBuf};
use std::{env, fs};

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::{Group, Ident as Ident2, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, LitBool, LitStr, Token};
use toml::value::{Table, Value};
use crate::toml_tokens::TomlTokens;

use crate::args::Args;

mod args;
mod toml_tokens;

#[proc_macro]
pub fn toml(input: TokenStream) -> TokenStream {
    let token_stream2 = TokenStream2::from(input);
    toml2(token_stream2).into()
}

fn toml2(input: TokenStream2) -> TokenStream2 {
    let args: Args = syn::parse2(input).unwrap();

    let mut file_path = PathBuf::new();
    file_path.push(env::var("CARGO_MANIFEST_DIR").unwrap());
    file_path.push(args.file_path.value());
    let include_file_path = file_path.to_str().unwrap();

    let content = fs::read_to_string(&file_path).unwrap();
    let table: Table = toml::from_str(&content).unwrap();
    let value_table = Value::Table(table);

    let mut namespace = vec![format_ident!("example")];
    let static_tokens = value_table.static_tokens("example", &args.named_args, &mut namespace);
    let type_tokens = value_table.type_tokens("example", &args.named_args);

    quote! {
        pub static EXAMPLE: example::Example = #static_tokens;

        #type_tokens

        const _: &str = include_str!(#include_file_path);
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use crate::toml2;

    #[test]
    fn toml2_works() {
        let quoted = toml2(quote!("example.toml"));
        println!("{quoted}");
        assert!(false)
    }
}
