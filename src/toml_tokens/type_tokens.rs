//! Generates Rust tokens representing the types corresponding to TOML data.
//!
//! The `type_tokens` submodule focuses on generating Rust tokens that represent
//! types based on TOML data structures. This facilitates the creation of Rust
//! types that mirror the structure of the data in the TOML files.

use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Attribute, Ident as Ident2};
use toml::value::Array;
use toml::Table;

use crate::parse::StaticTomlAttributes;
use crate::toml_tokens::{fixed_ident, TomlTokens};

/// Generates the Rust tokens for a TOML array type.
///
/// Returns a TokenStream2 representing the Rust code generated for the array
/// type.
#[inline]
pub fn array(
    array: &Array,
    type_ident: &Ident2,
    config: &StaticTomlAttributes,
    derive: &[Attribute]
) -> TokenStream2 {
    // Check if slices should be used
    let use_slices = super::use_slices(array, config);

    // Define identifiers for the values
    let values_ident = config
        .values_ident
        .as_ref()
        .map(|i| i.to_string())
        .unwrap_or_else(|| "values".to_string());
    let values_mod_ident = format_ident!("{}", values_ident.to_case(Case::Snake));
    let values_type_ident = format_ident!(
        "{}",
        fixed_ident(&values_ident, &config.prefix, &config.suffix)
            .to_string()
            .to_case(Case::Pascal)
    );

    // Generate tokens based on whether slices are used or not
    if use_slices {
        let len = array.len();
        let Some(value) = array.get(0)
        else {
            return quote! {
                pub type #type_ident = [(); 0];
            };
        };
        let value_type_tokens = value
            .type_tokens(&values_ident, config, quote!(pub), derive)
            .unwrap();

        quote! {
            pub type #type_ident = [#values_mod_ident::#values_type_ident; #len];
            #value_type_tokens
        }
    }
    else {
        let value_tokens: Vec<TokenStream2> = array
            .iter()
            .enumerate()
            .map(|(i, v)| {
                v.type_tokens(
                    &format!("{}{}", values_ident, i),
                    config,
                    quote!(pub),
                    derive
                )
                .unwrap()
            })
            .collect();
        let value_types: Vec<TokenStream2> = (0..array.len())
            .map(|i| {
                let mod_ident = format_ident!("{}_{}", values_ident.to_case(Case::Snake), i);
                let type_ident = format!("{}{}", values_ident.to_case(Case::Pascal), i);
                let type_ident = fixed_ident(&type_ident, &config.prefix, &config.suffix);
                quote!(pub #mod_ident::#type_ident)
            })
            .collect();

        quote! {
            #(#derive)*
            pub struct #type_ident(#(#value_types),*);
            #(#value_tokens)*
        }
    }
}

/// Generates the Rust tokens for a TOML table type.
///
/// Returns a TokenStream2 representing the Rust code generated for the table
/// type.
#[inline]
pub fn table(
    table: &Table,
    type_ident: &Ident2,
    config: &StaticTomlAttributes,
    derive: &[Attribute]
) -> TokenStream2 {
    // Generate the inner modules tokens
    let mods_tokens: Vec<TokenStream2> = table
        .iter()
        .map(|(k, v)| v.type_tokens(k, config, quote!(pub), derive).unwrap())
        .collect();

    // Generate the field tokens
    let fields_tokens: Vec<TokenStream2> = table
        .iter()
        .map(|(k, _)| {
            let field_key = format_ident!("{}", k.to_case(Case::Snake));
            let type_ident = super::fixed_ident(k, &config.prefix, &config.suffix);
            quote!(pub #field_key: #field_key::#type_ident)
        })
        .collect();

    // Combine the tokens into the final structure
    quote! {
        #(#derive)*
        pub struct #type_ident {
            #(#fields_tokens),*
        }

        #(#mods_tokens)*
    }
}
