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

use crate::args::Args;

mod args;
mod snapshot;

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

    let mut namespace = vec![format_ident!("_example")];
    let const_value: Vec<TokenStream2> = table
        .iter()
        .map(|(k, v)| static_values(k, v, &mut namespace))
        .collect();
    let data_types: TokenStream2 = data_types((&String::from("Example"), &Value::Table(table)));

    quote! {
        const _: &str = include_str!(#include_file_path);

        #data_types

        pub static EXAMPLE: _example::_Example = _example::_Example {
            #(#const_value)*
        };
    }
}

fn data_types((key, value): (&'_ String, &'_ Value)) -> TokenStream2 {
    let key = match key.as_str() {
        "type" => "kind",
        key => key
    };

    let mod_key = format_ident!("_{}", key.to_case(Case::Snake));
    let type_key = format_ident!("_{}", key.to_case(Case::Pascal));

    let inner = match value {
        // for now Datetime is only represented as a string
        Value::String(_) | Value::Datetime(_) => quote! {
            pub type #type_key = &'static str;
        },

        Value::Integer(_) => quote! {
            pub type #type_key = i64;
        },

        Value::Float(_) => quote! {
            pub type #type_key = f64;
        },

        Value::Boolean(_) => quote! {
            pub type #type_key = bool;
        },

        Value::Array(values) => {
            let mods: Vec<TokenStream2> = values
                .iter()
                .enumerate()
                .map(|(i, v)| data_types((&i.to_string(), v)))
                .collect();

            let fields: Vec<TokenStream2> = values
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let mod_key = format_ident!("_{i}");
                    let type_key = format_ident!("_{i}");
                    let field_key = format_ident!("_{i}");
                    quote! { #field_key: __values::#mod_key::#type_key, }
                })
                .collect();

            quote! {
                pub struct #type_key {
                    #(#fields)*
                }

                pub mod __values {
                    #(#mods)*
                }
            }
        }

        Value::Table(table) => {
            let mods: Vec<TokenStream2> = table.iter().map(data_types).collect();
            let fields: Vec<TokenStream2> = table
                .keys()
                .map(|k| {
                    let k = match k.as_str() {
                        "type" => "kind",
                        k => k
                    };
                    let field_key = format_ident!("{k}");
                    let mod_key = format_ident!("_{}", k.to_case(Case::Snake));
                    let type_key = format_ident!("_{}", k.to_case(Case::Pascal));
                    quote! {
                        pub #field_key: #mod_key::#type_key,
                    }
                })
                .collect();

            quote! {
                #[derive(Debug)]
                pub struct #type_key {
                    #(#fields)*
                }

                #(#mods)*
            }
        }
    };

    quote! {
        pub mod #mod_key {
            #inner
        }
    }
}

fn static_values(key: &'_ String, value: &'_ Value, namespace: &mut Vec<Ident2>) -> TokenStream2 {
    let value = match value {
        Value::String(value) => quote!(#value),
        Value::Integer(value) => quote!(#value),
        Value::Float(value) => quote!(#value),
        Value::Boolean(value) => quote!(#value),
        Value::Datetime(value) => {
            let value = value.to_string();
            quote!(#value)
        }

        Value::Array(values) => {
            let this_mod_key = format_ident!("_{}", key.to_case(Case::Snake));
            let inner: Vec<TokenStream2> = values
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    namespace.push(format_ident!("__values"));
                    namespace.push(this_mod_key.clone());
                    let token_stream = static_values(&format!("_{i}"), v, namespace);
                    namespace.pop();
                    namespace.pop();
                    token_stream
                })
                .collect();
            let mod_key = quote!(#(#namespace)::*);
            let type_key = format_ident!("_{}", key.to_case(Case::Pascal));
            quote! {
                #mod_key::#this_mod_key::#type_key {
                    #(#inner)*
                }
            }
        }

        Value::Table(values) => {
            let this_mod_key = format_ident!("_{}", key.to_case(Case::Snake));
            let inner: Vec<TokenStream2> = values
                .iter()
                .map(|(k, v)| {
                    namespace.push(this_mod_key.clone());
                    let token_stream = static_values(k, v, namespace);
                    namespace.pop();
                    token_stream
                })
                .collect();
            let mod_key = quote!(#(#namespace)::*);
            let type_key = format_ident!("_{}", key.to_case(Case::Pascal));
            quote! {
                #mod_key::#this_mod_key::#type_key {
                    #(#inner)*
                }
            }
        }
    };

    let key = format_ident!("{}", key.to_case(Case::Snake));
    quote! {
        #key: #value,
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;

    use super::*;

    #[test]
    fn example_works() {
        toml2(quote!("example.toml"));
    }
}
