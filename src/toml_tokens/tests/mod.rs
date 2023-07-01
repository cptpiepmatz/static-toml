use proc_macro2::Span as Span2;
use quote::{format_ident, quote};
use syn::{Attribute, LitBool, parse_quote};
use toml::value::Value;

use crate::parse::StaticTomlAttributes;
use crate::toml_tokens::TomlTokens;

mod type_tokens;
mod static_tokens;

#[test]
fn type_eq_works() {
    let toml: Value = toml::from_str(include_str!("../../../example.toml")).unwrap();

    let servers = toml.get("servers").unwrap();
    let alpha = servers.get("alpha").unwrap();
    let beta = servers.get("beta").unwrap();
    assert!(alpha.type_eq(beta));

    let database = toml.get("database").unwrap();
    let ports = database.get("ports").unwrap();
    assert!(ports[0].type_eq(&ports[1]));
    assert!(ports[1].type_eq(&ports[2]));

    let data = database.get("data").unwrap();
    assert!(!data[0].type_eq(&data[1]));
}
