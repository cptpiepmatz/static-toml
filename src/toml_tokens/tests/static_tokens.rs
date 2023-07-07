use quote::{format_ident, quote};
use toml::Value;

use crate::parse::StaticTomlAttributes;
use crate::toml_tokens::TomlTokens;

#[test]
fn default_static_tokens_works() {
    let config = StaticTomlAttributes::default();
    let mut namespace = vec![format_ident!("toml")];

    let toml: Value = toml::from_str(include_str!("../../../example.toml")).unwrap();
    let toml_ts = toml
        .static_tokens(namespace[0].to_string().as_str(), &config, &mut namespace)
        .unwrap();
    let toml_ts_expected = quote! {
        toml::Toml {
            database: toml::database::Database {
                data: toml::database::data::Data(["delta", "phi"], [3.14f64]),
                enabled: true,
                ports: [8000i64, 8001i64, 8002i64],
                temp_targets: toml::database::temp_targets::TempTargets {
                    case: 72f64,
                    cpu: 79.5f64
                }
            },
            owner: toml::owner::Owner {
                dob: "1979-05-27T07:32:00-08:00",
                name: "Tom Preston-Werner"
            },
            servers: toml::servers::Servers {
                alpha: toml::servers::alpha::Alpha {
                    ip: "10.0.0.1",
                    role: "frontend"
                },
                beta: toml::servers::beta::Beta {
                    ip: "10.0.0.2",
                    role: "backend"
                }
            },
            title: "TOML Example"
        }
    };

    assert_eq!(toml_ts.to_string(), toml_ts_expected.to_string());
}

#[test]
fn values_ident_works() {
    let default_config = StaticTomlAttributes::default();
    let value_ident_config = StaticTomlAttributes {
        values_ident: Some(format_ident!("items")),
        ..StaticTomlAttributes::default()
    };
    let mut namespace = vec![format_ident!("toml")];

    let toml: Value = toml::from_str(
        "
    [[list]]
    value = 123

    [[list]]
    value = 456

    [[tuple]]
    a = 1

    [[tuple]]
    b = 2
    "
    )
    .unwrap();

    let default_toml_ts = toml
        .static_tokens(
            namespace[0].to_string().as_str(),
            &default_config,
            &mut namespace
        )
        .unwrap();
    let default_toml_ts_expected = quote! {
        toml::Toml {
            list: [
                toml::list::values::Values { value: 123i64 },
                toml::list::values::Values { value: 456i64 }
            ],
            tuple: toml::tuple::Tuple(
                toml::tuple::values_0::Values0 { a: 1i64 },
                toml::tuple::values_1::Values1 { b: 2i64 }
            )
        }
    };
    assert_eq!(
        default_toml_ts.to_string(),
        default_toml_ts_expected.to_string()
    );

    let items_toml_ts = toml
        .static_tokens(
            namespace[0].to_string().as_str(),
            &value_ident_config,
            &mut namespace
        )
        .unwrap();
    let items_toml_ts_expected = quote! {
        toml::Toml {
            list: [
                toml::list::items::Items { value: 123i64 },
                toml::list::items::Items { value: 456i64 }
            ],
            tuple: toml::tuple::Tuple(
                toml::tuple::items_0::Items0 { a: 1i64 },
                toml::tuple::items_1::Items1 { b: 2i64 }
            )
        }
    };
    assert_eq!(
        items_toml_ts.to_string(),
        items_toml_ts_expected.to_string()
    );
}
