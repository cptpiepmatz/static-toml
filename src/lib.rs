extern crate proc_macro;

use proc_macro::TokenStream;
use std::{env, fs};
use std::path::{Path, PathBuf};
use convert_case::{Case, Casing};
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, LitStr, Token};
use proc_macro2::{Group, Ident as Ident2, TokenStream as TokenStream2};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::ext::IdentExt;
use toml::{Table, Value};

struct Args {
    file_path: LitStr,
    named_args: NamedArgs,
}

#[derive(Default)]
struct NamedArgs {
    prefix: Option<Ident2>,
    suffix: Option<Ident2>,
    entry: Option<Ident2>
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let file_path: LitStr = input.parse()?;
        if input.is_empty() {
            return Ok(Args {
                file_path,
                named_args: NamedArgs::default()
            });
        }

        let _: Token![,] = input.parse()?;
        let named_args = NamedArgs::parse(input)?;

        Ok(Args { file_path, named_args })
    }
}

impl Parse for NamedArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut prefix = None;
        let mut suffix = None;
        let mut entry = None;

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(Ident2::peek_any) {
                let name: Ident2 = input.parse()?;
                let _: Token![,] = input.parse()?;

                match name.to_string().as_str() {
                    "prefix" => prefix = Some(input.parse()?),
                    "suffix" => suffix = Some(input.parse()?),
                    "entry" => entry = Some(input.parse()?),
                    _ => return Err(input.error("Expected `prefix`, `suffix` or `entry`")),
                }
            } else {
                return Err(lookahead.error());
            }

            if !input.is_empty() {
                let _: Token![,] = input.parse()?;
            }
        }

        Ok(NamedArgs { prefix, suffix, entry })
    }
}

#[proc_macro]
pub fn toml(input: TokenStream) -> TokenStream {
    // TODO: make meaningful error messages

    let args = parse_macro_input!(input as Args);

    let mut file_path = PathBuf::new();
    file_path.push(env::var("CARGO_MANIFEST_DIR").unwrap());
    file_path.push(args.file_path.value());
    let include_file_path = file_path.to_str().unwrap();

    let content = fs::read_to_string(&file_path).unwrap();
    let table: Table = toml::from_str(&content).unwrap();

    let mut namespace = vec![format_ident!("_example")];
    let const_value: Vec<TokenStream2> = table.iter().map(|(k, v)| const_value(k, v, &mut namespace)).collect();
    let data_types: TokenStream2 = data_types((&String::from("Example"), &Value::Table(table)));

    TokenStream::from(quote! {
        const _: &str = include_str!(#include_file_path);

        #data_types

        pub const EXAMPLE: _example::_Example = _example::_Example {
            #(#const_value)*
        };
    })
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

            let fields: Vec<TokenStream2> = values.iter().enumerate().map(|(i, _)| {
                let mod_key = format_ident!("_{i}");
                let type_key = format_ident!("_{i}");
                let field_key = format_ident!("_{i}");
                quote! { #field_key: __values::#mod_key::#type_key, }
            }).collect();

            quote! {
                pub struct #type_key {
                    #(#fields)*
                }

                pub mod __values {
                    #(#mods)*
                }
            }
        },

        Value::Table(table) => {
            let mods: Vec<TokenStream2> = table.iter().map(data_types).collect();
            let fields: Vec<TokenStream2> = table.keys().map(|k| {
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
            }).collect();

            quote! {
                #[derive(Debug)]
                pub struct #type_key {
                    #(#fields)*
                }

                #(#mods)*
            }
        },
    };

    quote! {
        pub mod #mod_key {
            #inner
        }
    }
}

fn const_value(key: &'_ String, value: &'_ Value, namespace: &mut Vec<Ident2>) -> TokenStream2 {
    let value = match value {

        Value::String(value) => quote!(#value),
        Value::Integer(value) => quote!(#value),
        Value::Float(value) => quote!(#value),
        Value::Boolean(value) => quote!(#value),
        Value::Datetime(value) => {
            let value = value.to_string();
            quote!(#value)
        }

        Value::Array(values) => todo!(),

        Value::Table(values) => {
            let this_mod_key = format_ident!("_{}", key.to_case(Case::Snake));
            let inner: Vec<TokenStream2> = values.iter().map(|(k, v)| {
                namespace.push(this_mod_key.clone());
                let token_stream = const_value(k, v, namespace);
                namespace.pop();
                token_stream
            }).collect();
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
