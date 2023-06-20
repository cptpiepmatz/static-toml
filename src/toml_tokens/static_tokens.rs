use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::Ident as Ident2;
use toml::value::Array;
use toml::Table;

use crate::parse::StaticTomlAttributes;
use crate::toml_tokens::TomlTokens;

pub fn array(
    array: &Array,
    key: &str,
    config: &StaticTomlAttributes,
    namespace: &mut Vec<Ident2>,
    namespace_ts: TokenStream2
) -> TokenStream2 {
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
    let inner: Vec<TokenStream2> = array
        .iter()
        .zip(key_iter)
        .map(|(v, k)| {
            namespace.push(format_ident!("{}", k.to_case(Case::Snake)));
            let value = v.static_tokens(&k, config, namespace);
            namespace.pop();
            value
        })
        .collect();
    let type_ident = super::fixed_ident(key, &config.prefix, &config.suffix);
    match use_slices {
        false => quote!(#namespace_ts::#type_ident(#(#inner),*)),
        true => quote!([#(#inner),*])
    }
}

pub fn table(
    table: &Table,
    key: &str,
    config: &StaticTomlAttributes,
    namespace: &mut Vec<Ident2>,
    namespace_ts: TokenStream2
) -> TokenStream2 {
    let inner: Vec<(Ident2, TokenStream2)> = table
        .iter()
        .map(|(k, v)| {
            let field_key = format_ident!("{}", k.to_case(Case::Snake));
            namespace.push(field_key.clone());
            let value = (field_key, v.static_tokens(k, config, namespace));
            namespace.pop();
            value
        })
        .collect();
    let field_keys: Vec<&Ident2> = inner.iter().map(|(k, _)| k).collect();
    let field_values: Vec<&TokenStream2> = inner.iter().map(|(_, v)| v).collect();
    let type_ident = super::fixed_ident(key, &config.prefix, &config.suffix);
    quote! {
        #namespace_ts::#type_ident {
            #(#field_keys: #field_values),*
        }
    }
}
