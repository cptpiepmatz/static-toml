//! Generates Rust tokens for representing static data derived from TOML.
//!
//! The `static_tokens` submodule specializes in generating Rust tokens that
//! represent TOML data as static data structures. This allows for creating
//! compile-time representations of the data in TOML files.

use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::Ident as Ident2;
use toml::value::Array;
use toml::Table;

use crate::parse::StaticTomlAttributes;
use crate::toml_tokens::TomlTokens;

/// Generates the Rust tokens for a TOML array.
///
/// Returns a TokenStream2 representing the Rust code generated for the array.
#[inline]
pub(crate) fn array(
    array: &Array,
    key: &str,
    config: &StaticTomlAttributes,
    namespace: &mut Vec<Ident2>,
    namespace_ts: TokenStream2
) -> Result<TokenStream2, super::super::Error> {
    // Check if slices should be used
    let use_slices = super::use_slices(array, config);
    let values_ident = [config
        .values_ident
        .as_ref()
        .map(Ident2::to_string)
        .unwrap_or_else(|| String::from("values"))];
    let key_iter: Box<dyn Iterator<Item = String>> = match use_slices {
        true => Box::new(values_ident.iter().cycle().cloned()),
        false => Box::new(
            values_ident
                .iter()
                .cycle()
                .enumerate()
                .map(|(i, v)| format!("{v}{i}"))
        )
    };

    // Generate the inner token streams for the array elements
    let inner: Vec<TokenStream2> = array
        .iter()
        .zip(key_iter)
        .map(|(v, k)| {
            namespace.push(format_ident!("{}", k.to_case(Case::Snake)));
            let value = v.static_tokens(&k, config, namespace).unwrap();
            namespace.pop();
            value
        })
        .collect();

    // Generate the final token stream based on whether slices are used or not
    let type_ident = super::fixed_ident(key, &config.prefix, &config.suffix);
    Ok(match use_slices {
        true => quote!([#(#inner),*]),
        false => quote!(#namespace_ts::#type_ident(#(#inner),*))
    })
}

/// Generates the Rust tokens for a TOML table.
///
/// Returns a TokenStream2 representing the Rust code generated for the table.
#[inline]
pub(crate) fn table(
    table: &Table,
    key: &str,
    config: &StaticTomlAttributes,
    namespace: &mut Vec<Ident2>,
    namespace_ts: TokenStream2
) -> Result<TokenStream2, super::super::Error> {
    // Generate the inner token streams for the table fields
    let inner: Vec<(Ident2, TokenStream2)> = table
        .iter()
        .map(|(k, v)| {
            let field_key = format_ident!("{}", k.to_case(Case::Snake));
            namespace.push(field_key.clone());
            let value = (field_key, v.static_tokens(k, config, namespace).unwrap());
            namespace.pop();
            value
        })
        .collect();

    // Collect the field keys and values
    let field_keys: Vec<&Ident2> = inner.iter().map(|(k, _)| k).collect();
    let field_values: Vec<&TokenStream2> = inner.iter().map(|(_, v)| v).collect();

    // Generate the final token stream for the table
    let type_ident = super::fixed_ident(key, &config.prefix, &config.suffix);
    Ok(quote! {
        #namespace_ts::#type_ident {
            #(#field_keys: #field_values),*
        }
    })
}
