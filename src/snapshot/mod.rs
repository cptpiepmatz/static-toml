use std::collections::HashSet;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Ident as Ident2, LitBool, LitStr};
use toml::value::Array;
use toml::{Table, Value};

use crate::args::NamedArgs;

mod static_tokens;
mod type_tokens;

#[cfg(test)]
mod tests;

pub trait Snapshot {
    fn type_eq(&self, other: &Self) -> bool;

    fn type_tokens(&self, key: &str, config: &NamedArgs) -> TokenStream2;

    fn static_tokens(
        &self,
        key: &str,
        config: &NamedArgs,
        namespace: &mut Vec<Ident2>
    ) -> TokenStream2;
}

impl Snapshot for Value {
    fn type_eq(&self, other: &Self) -> bool {
        use Value::*;

        match (self, other) {
            (String(_), String(_)) => true,
            (Integer(_), Integer(_)) => true,
            (Float(_), Float(_)) => true,
            (Boolean(_), Boolean(_)) => true,
            (Datetime(_), Datetime(_)) => true,

            (Array(a), Array(b)) => {
                if a.len() != b.len() {
                    return false;
                }

                a.iter()
                    .zip(b.iter())
                    .map(|(a, b)| a.type_eq(b))
                    .reduce(|acc, b| acc && b)
                    .unwrap_or(true)
            }

            (Table(a), Table(b)) => HashSet::<std::string::String>::from_iter(
                a.keys().cloned().chain(b.keys().cloned())
            )
            .iter()
            .map(|k| (a.get(k), b.get(k)))
            .map(|(a, b)| match (a, b) {
                (Some(a), Some(b)) => a.type_eq(b),
                _ => false
            })
            .reduce(|acc, b| acc && b)
            .unwrap_or(true),

            _ => false
        }
    }

    fn type_tokens(&self, key: &str, config: &NamedArgs) -> TokenStream2 {
        use Value::*;

        let mod_ident = format_ident!("{}", key.to_case(Case::Snake));
        let type_ident = fixed_ident(key, &config.prefix, &config.suffix);

        let inner = match self {
            String(_) => quote!(pub type #type_ident = &'static str;),
            Integer(_) => quote!(pub type #type_ident = i64;),
            Float(_) => quote!(pub type #type_ident = f64;),
            Boolean(_) => quote!(pub type #type_ident = bool;),
            Datetime(_) => quote!(pub type #type_ident = &'static str;),
            Array(values) => type_tokens::array(values, &type_ident, config),
            Table(values) => type_tokens::table(values, &type_ident, config)
        };

        quote! {
            pub mod #mod_ident {
                #inner
            }
        }
    }

    fn static_tokens(
        &self,
        key: &str,
        config: &NamedArgs,
        namespace: &mut Vec<Ident2>
    ) -> TokenStream2 {
        let namespace_ts = quote!(#(#namespace)::*);

        match self {
            Value::String(s) => quote!(#s),
            Value::Integer(i) => quote!(#i),
            Value::Float(f) => quote!(#f),
            Value::Boolean(b) => quote!(#b),

            Value::Datetime(d) => {
                let d = d.to_string();
                quote!(#d)
            }

            Value::Array(values) => {
                static_tokens::array(values, key, config, namespace, namespace_ts)
            }

            Value::Table(values) => {
                static_tokens::table(values, key, config, namespace, namespace_ts)
            }
        }
    }
}

fn fixed_ident(ident: &str, prefix: &Option<Ident2>, suffix: &Option<Ident2>) -> Ident2 {
    let ident = ident.to_case(Case::Pascal);
    match (prefix, suffix) {
        (None, None) => format_ident!("{ident}"),
        (Some(prefix), None) => format_ident!("{prefix}{ident}"),
        (None, Some(suffix)) => format_ident!("{ident}{suffix}"),
        (Some(prefix), Some(suffix)) => format_ident!("{prefix}{ident}{suffix}")
    }
}

fn use_slices(array: &Array, config: &NamedArgs) -> bool {
    if !config
        .prefer_slices
        .as_ref()
        .map(|b| b.value())
        .unwrap_or(true)
    {
        return false;
    }

    array
        .iter()
        .zip(array.iter().skip(1))
        .map(|(a, b)| a.type_eq(b))
        .reduce(|acc, b| acc && b)
        .unwrap_or(true)
}
