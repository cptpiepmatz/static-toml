//! Converts TOML data into Rust tokens representing both type definitions and
//! static data.
//!
//! The `toml_tokens` module handles the conversion of parsed TOML data into
//! Rust tokens. These tokens can be used for generating Rust source code files
//! that include both the type definitions and the static data based on the TOML
//! structure.

use std::collections::HashSet;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_quote, Attribute, Ident as Ident2};
use toml::value::Array;
use toml::Value;

use crate::parse::StaticTomlAttributes;

mod static_tokens;
mod type_tokens;

#[cfg(test)]
mod tests;

/// Trait for generating Rust tokens based on TOML values.
pub(crate) trait TomlTokens {
    /// Compares the type of two TOML values.
    ///
    /// Returns `true` if both values have the same type.
    fn type_eq(&self, other: &Self) -> bool;

    /// Generates the Rust type definition tokens based on a TOML value.
    ///
    /// This method takes a TOML key, configuration, visibility, and derive
    /// attributes and generates Rust type definitions.
    fn type_tokens(
        &self,
        key: &str,
        config: &StaticTomlAttributes,
        visibility: TokenStream2,
        derive: &[Attribute]
    ) -> Result<TokenStream2, super::TomlError>;

    /// Generates the Rust static value tokens based on a TOML value.
    ///
    /// This method takes a TOML key, configuration, and namespace and generates
    /// Rust static values.
    fn static_tokens(
        &self,
        key: &str,
        config: &StaticTomlAttributes,
        namespace: &mut Vec<Ident2>
    ) -> Result<TokenStream2, super::TomlError>;
}

impl TomlTokens for Value {
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

    fn type_tokens(
        &self,
        key: &str,
        config: &StaticTomlAttributes,
        visibility: TokenStream2,
        derive: &[Attribute]
    ) -> Result<TokenStream2, super::TomlError> {
        use Value::*;

        if !is_valid_identifier(key.to_case(Case::Snake).as_str()) {
            return Err(super::TomlError::KeyInvalid(key.to_string()));
        }

        let mod_ident = format_ident!("{}", key.to_case(Case::Snake));
        let type_ident = fixed_ident(key, &config.prefix, &config.suffix);

        let inner = match self {
            String(_) => quote!(pub type #type_ident = &'static str;),
            Integer(_) => quote!(pub type #type_ident = i64;),
            Float(_) => quote!(pub type #type_ident = f64;),
            Boolean(_) => quote!(pub type #type_ident = bool;),
            Datetime(_) => quote!(pub type #type_ident = &'static str;),
            Array(values) => type_tokens::array(values, &type_ident, config, derive)?,
            Table(values) => type_tokens::table(values, &type_ident, config, derive)?
        };

        Ok(quote! {
            #visibility mod #mod_ident {
                #inner
            }
        })
    }

    fn static_tokens(
        &self,
        key: &str,
        config: &StaticTomlAttributes,
        namespace: &mut Vec<Ident2>
    ) -> Result<TokenStream2, super::TomlError> {
        if !is_valid_identifier(key.to_case(Case::Snake).as_str()) {
            return Err(super::TomlError::KeyInvalid(key.to_string()));
        }

        let namespace_ts = quote!(#(#namespace)::*);

        Ok(match self {
            Value::String(s) => quote!(#s),
            Value::Integer(i) => quote!(#i),
            Value::Float(f) => quote!(#f),
            Value::Boolean(b) => quote!(#b),

            Value::Datetime(d) => {
                let d = d.to_string();
                quote!(#d)
            }

            Value::Array(values) => {
                static_tokens::array(values, key, config, namespace, namespace_ts)?
            }

            Value::Table(values) => {
                static_tokens::table(values, key, config, namespace, namespace_ts)?
            }
        })
    }
}

/// Creates an identifier with optional prefix and suffix.
///
/// Given an identifier, a prefix and a suffix, it constructs a new identifier
/// concatenating the prefix, identifier, and suffix.
pub fn fixed_ident(ident: &str, prefix: &Option<Ident2>, suffix: &Option<Ident2>) -> Ident2 {
    let ident = ident.to_case(Case::Pascal);
    match (prefix, suffix) {
        (None, None) => format_ident!("{ident}"),
        (Some(prefix), None) => format_ident!("{prefix}{ident}"),
        (None, Some(suffix)) => format_ident!("{ident}{suffix}"),
        (Some(prefix), Some(suffix)) => format_ident!("{prefix}{ident}{suffix}")
    }
}

/// Determines if slices should be used for TOML arrays based on the
/// configuration.
///
/// Returns `true` if slices should be used instead of arrays based on the
/// configuration and the content of the array.
fn use_slices(array: &Array, config: &StaticTomlAttributes) -> bool {
    // If prefer_slices is explicitly set to false, return false.
    if !config
        .prefer_slices
        .as_ref()
        .map(|b| b.value())
        .unwrap_or(true)
    {
        return false;
    }

    // Check if all elements in the array are of the same type.
    array
        .iter()
        .zip(array.iter().skip(1))
        .map(|(a, b)| a.type_eq(b))
        .reduce(|acc, b| acc && b)
        .unwrap_or(true)
}

fn is_valid_identifier(input: &str) -> bool {
    let mut chars = input.chars();

    // First char must be a letter or underscore.
    let Some(first) = chars.next()
    else {
        return false;
    };
    if !(first.is_alphabetic() || first == '_') {
        return false;
    }

    // All others must be must be numbers, letters or underscore.
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Generate the auto doc comment for the statics.
pub fn gen_auto_doc(path: &str, content: &str) -> TokenStream2 {
    let summary = format!("Static inclusion of `{path}`.");
    quote! {
        #[doc = ""]
        #[doc = #summary]
        #[doc = ""]
        #[doc = "```toml"]
        #[doc = #content]
        #[doc = "```"]
    }
}
