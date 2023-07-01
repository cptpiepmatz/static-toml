use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Attribute, Ident as Ident2};
use toml::value::Array;
use toml::Table;

use crate::parse::StaticTomlAttributes;
use crate::toml_tokens::{fixed_ident, TomlTokens};

#[inline]
pub fn array(
    array: &Array,
    type_ident: &Ident2,
    config: &StaticTomlAttributes,
    derive: &[Attribute]
) -> TokenStream2 {
    let use_slices = super::use_slices(array, config);
    let values_ident = config
        .values_ident
        .as_ref()
        .map(|i| i.to_string())
        .unwrap_or_else(|| "values".to_string());
    let values_mod_ident = format_ident!("{}", values_ident.to_case(Case::Snake));

    // for the types, use prefix and suffix
    let values_type_ident = format_ident!(
        "{}",
        fixed_ident(&values_ident, &config.prefix, &config.suffix)
            .to_string()
            .to_case(Case::Pascal)
    );

    if use_slices {
        let len = array.len();
        let Some(value) = array.get(0) else {
            return quote! {
                    pub type #type_ident = [(); 0];
                }
        };

        let value_type_tokens = value.type_tokens(&values_ident, config, quote!(pub), derive);

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
                    &format!("{}{i}", &values_ident),
                    config,
                    quote!(pub),
                    derive
                )
            })
            .collect();
        let value_types: Vec<TokenStream2> = array
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let mod_ident = format_ident!("{}_{i}", values_ident.to_case(Case::Snake));
                let type_ident = format!("{}{i}", values_ident.to_case(Case::Pascal));
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

#[inline]
pub fn table(
    table: &Table,
    type_ident: &Ident2,
    config: &StaticTomlAttributes,
    derive: &[Attribute]
) -> TokenStream2 {
    let mods_tokens: Vec<TokenStream2> = table
        .iter()
        .map(|(k, v)| v.type_tokens(k, config, quote!(pub), derive))
        .collect();

    let fields_tokens: Vec<TokenStream2> = table
        .iter()
        .map(|(k, _)| {
            let field_key = format_ident!("{}", k.to_case(Case::Snake));
            let type_ident = super::fixed_ident(k, &config.prefix, &config.suffix);
            quote!(pub #field_key: #field_key::#type_ident)
        })
        .collect();

    quote! {
        #(#derive)*
        pub struct #type_ident {
            #(#fields_tokens),*
        }

        #(#mods_tokens)*
    }
}
