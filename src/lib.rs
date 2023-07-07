#![doc = include_str!("../doc/crate.md")]

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

#[doc = include_str!("../doc/macro.md")]
#[proc_macro]
pub fn static_toml(input: TokenStream) -> TokenStream {
    let token_stream2 = TokenStream2::from(input);
    static_toml2(token_stream2).unwrap().into()
}

/// Process the input token stream and generate the corresponding Rust code
/// using `proc_macro2`.
///
/// This function serves as the `proc_macro2` variant of the `static_toml`
/// procedural macro.
/// It is necessary for making the library testable.
/// By using `proc_macro2` data structures, this function can be tested in
/// environments where procedural macros are not natively supported.
fn static_toml2(input: TokenStream2) -> Result<TokenStream2, Error> {
    // Parse the input into StaticToml data structure.
    let static_toml_data: StaticToml = syn::parse2(input).map_err(|e| Error::Syn(e))?;

    // Iterate through each static_toml item, process it, and generate the
    // corresponding Rust code.
    let mut tokens = Vec::with_capacity(static_toml_data.0.len());
    for static_toml in static_toml_data.0.iter() {
        // Construct the full path to the TOML file that needs to be embedded.
        let mut file_path = PathBuf::new();
        file_path.push(env::var("CARGO_MANIFEST_DIR").or(Err(Error::MissingCargoManifestDirEnv))?);
        file_path.push(static_toml.path.value());
        let include_file_path = file_path.to_str().ok_or(Error::FilePathInvalid)?;

        // Read the TOML file and parse it into a TOML table.
        let content = fs::read_to_string(&file_path).or(Err(Error::ReadToml))?;
        let table: Table = toml::from_str(&content).map_err(|e| Error::ParseToml(e))?;
        let value_table = Value::Table(table);

        // Determine the root module name, either specified by the user or the default
        // based on the static value's name.
        let root_mod = static_toml.attrs.root_mod.clone().unwrap_or(format_ident!(
            "{}",
            static_toml.name.to_string().to_case(Case::Snake)
        ));
        let mut namespace = vec![root_mod.clone()];

        // Determine the visibility of the generated code, either specified by the user
        // or default.
        let visibility = static_toml
            .visibility
            .as_ref()
            .map(|vis| vis.to_token_stream())
            .unwrap_or_default();

        // Generate the tokens for the static value based on the parsed TOML data.
        let static_tokens = value_table.static_tokens(
            root_mod.to_string().as_str(),
            &static_toml.attrs,
            &mut namespace
        )?;

        // Generate the tokens for the types based on the parsed TOML data.
        let type_tokens = value_table.type_tokens(
            root_mod.to_string().as_str(),
            &static_toml.attrs,
            visibility,
            &static_toml.derive
        )?;

        // Extract relevant fields from the StaticTomlItem.
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

        // Generate the final Rust code for the static value and types.
        tokens.push(quote! {
            #(#doc)*
            #visibility static #name: #root_mod::#root_type = #static_tokens;

            #(#other_attrs)*
            #type_tokens

            // This is a trick to make the compiler re-evaluate the macro call when the included file changes.
            const _: &str = include_str!(#include_file_path);
        });
    }

    Ok(TokenStream2::from_iter(tokens.into_iter()))
}

#[derive(Debug)]
pub(crate) enum Error {
    Syn(syn::Error),
    MissingCargoManifestDirEnv,
    FilePathInvalid,
    ReadToml,
    ParseToml(toml::de::Error)
}
