extern crate proc_macro;

use std::path::PathBuf;
use std::{env, fs};

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use toml::value::{Table, Value};

use crate::parse::{StaticToml, StaticTomlItem};
use crate::toml_tokens::{fixed_ident, TomlTokens};

mod parse;
mod toml_tokens;

#[proc_macro]
pub fn static_toml(input: TokenStream) -> TokenStream {
    let token_stream2 = TokenStream2::from(input);
    static_toml2(token_stream2).into()
}

fn static_toml2(input: TokenStream2) -> TokenStream2 {
    let static_toml_data: StaticToml = syn::parse2(input).unwrap();

    static_toml_data
        .0
        .iter()
        .map(|static_toml| {
            let mut file_path = PathBuf::new();
            file_path.push(env::var("CARGO_MANIFEST_DIR").unwrap());
            file_path.push(static_toml.path.value());
            let include_file_path = file_path.to_str().unwrap();

            let content = fs::read_to_string(&file_path).unwrap();
            let table: Table = toml::from_str(&content).unwrap();
            let value_table = Value::Table(table);

            let root_mod = static_toml.attrs.root_mod.clone().unwrap_or(format_ident!(
                "{}",
                static_toml.name.to_string().to_case(Case::Snake)
            ));
            let mut namespace = vec![root_mod.clone()];
            let visibility = static_toml
                .visibility
                .as_ref()
                .map(|vis| vis.to_token_stream())
                .unwrap_or_default();
            let static_tokens = value_table.static_tokens(
                root_mod.to_string().as_str(),
                &static_toml.attrs,
                &mut namespace
            );
            let type_tokens = value_table.type_tokens(
                root_mod.to_string().as_str(),
                &static_toml.attrs,
                visibility,
                &static_toml.derive
            );

            let name = &static_toml.name;
            let root_type = fixed_ident(
                root_mod.to_string().as_str(),
                &static_toml.attrs.prefix,
                &static_toml.attrs.suffix
            );

            let StaticTomlItem {
                doc,
                other_attrs,
                visibility,
                ..
            } = static_toml;

            quote! {
                #(#doc)*
                #visibility static #name: #root_mod::#root_type = #static_tokens;

                #(#other_attrs)*
                #type_tokens

                // trick to re-evaluate macro call when included file changed
                const _: &str = include_str!(#include_file_path);
            }
        })
        .collect()
}
