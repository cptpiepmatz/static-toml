use quote::quote;
use toml::value::Value;

use crate::parse::StaticTomlAttributes;
use crate::toml_tokens::TomlTokens;
use crate::Error;

mod static_tokens;
mod type_tokens;

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

#[test]
fn ident_validator_works() {
    let toml: Value = toml::from_str("123_key = 123").unwrap();
    let config = StaticTomlAttributes::default();
    let expected = "123_key".to_string();

    let type_tokens_res = toml.type_tokens("key", &config, quote!(), &[]);
    let Err(Error::KeyInvalid(key)) = type_tokens_res
    else {
        panic!("unexpected type");
    };
    assert_eq!(key, expected);

    let static_tokens_res = toml.static_tokens("key", &config, &mut Vec::new());
    let Err(Error::KeyInvalid(key)) = static_tokens_res
    else {
        panic!("unexpected type");
    };
    assert_eq!(key, expected);
}
