use proc_macro2::Span as Span2;
use quote::{format_ident, quote};
use syn::{parse_quote, Attribute, LitBool};
use toml::value::Value;

use crate::parse::StaticTomlAttributes;
use crate::toml_tokens::TomlTokens;

#[test]
fn default_type_tokens_works() {
    let config = StaticTomlAttributes::default();
    let empty_derive = vec![];

    let toml: Value = toml::from_str(include_str!("../../../example.toml")).unwrap();
    let title = toml.get("title").unwrap();
    let database = toml.get("database").unwrap();
    let enabled = database.get("enabled").unwrap();
    let ports = database.get("ports").unwrap();
    let data = database.get("data").unwrap();
    let temp_targets = database.get("temp_targets").unwrap();

    let title_ts = title
        .type_tokens("title", &config, quote!(pub), &empty_derive)
        .unwrap();
    let title_ts_expected = quote! {
        pub mod title {
            pub type Title = &'static str;
        }
    };
    assert_eq!(title_ts.to_string(), title_ts_expected.to_string());

    let enabled_ts = enabled
        .type_tokens("enabled", &config, quote!(pub), &empty_derive)
        .unwrap();
    let enabled_ts_expected = quote! {
        pub mod enabled {
            pub type Enabled = bool;
        }
    };
    assert_eq!(enabled_ts.to_string(), enabled_ts_expected.to_string());

    let ports_ts = ports
        .type_tokens("ports", &config, quote!(pub), &empty_derive)
        .unwrap();
    let ports_ts_expected = quote! {
        pub mod ports {
            pub type Ports = [values::Values; 3usize];

            pub mod values {
                pub type Values = i64;
            }
        }
    };
    assert_eq!(ports_ts.to_string(), ports_ts_expected.to_string());

    let data_ts = data
        .type_tokens("data", &config, quote!(pub), &empty_derive)
        .unwrap();
    let data_ts_expected = quote! {
        pub mod data {
            pub struct Data(pub values_0::Values0, pub values_1::Values1);

            pub mod values_0 {
                pub type Values0 = [values::Values; 2usize];

                pub mod values {
                    pub type Values = &'static str;
                }
            }

            pub mod values_1 {
                pub type Values1 = [values::Values; 1usize];

                pub mod values {
                    pub type Values = f64;
                }
            }
        }
    };
    assert_eq!(data_ts.to_string(), data_ts_expected.to_string());

    let temp_targets_ts = temp_targets
        .type_tokens("temp_targets", &config, quote!(pub), &empty_derive)
        .unwrap();
    let temp_targets_ts_expected = quote! {
        pub mod temp_targets {
            pub struct TempTargets {
                pub case: case::Case,
                pub cpu: cpu::Cpu
            }

            pub mod case {
                pub type Case = f64;
            }

            pub mod cpu {
                pub type Cpu = f64;
            }
        }
    };
    assert_eq!(
        temp_targets_ts.to_string(),
        temp_targets_ts_expected.to_string()
    );

    let toml_ts = toml
        .type_tokens("toml", &config, quote!(pub), &empty_derive)
        .unwrap();
    let toml_ts_expected = quote! {
        pub mod toml {
            pub struct Toml {
                pub database: database::Database,
                pub owner: owner::Owner,
                pub servers: servers::Servers,
                pub title: title::Title
            }

            pub mod database {
                pub struct Database {
                    pub data: data::Data,
                    pub enabled: enabled::Enabled,
                    pub ports: ports::Ports,
                    pub temp_targets: temp_targets::TempTargets
                }

                pub mod data {
                    pub struct Data(pub values_0::Values0, pub values_1::Values1);

                    pub mod values_0 {
                        pub type Values0 = [values::Values; 2usize];

                        pub mod values {
                            pub type Values = &'static str;
                        }
                    }

                    pub mod values_1 {
                        pub type Values1 = [values::Values; 1usize];

                        pub mod values {
                            pub type Values = f64;
                        }
                    }
                }

                pub mod enabled {
                    pub type Enabled = bool;
                }

                pub mod ports {
                    pub type Ports = [values::Values; 3usize];

                    pub mod values {
                        pub type Values = i64;
                    }
                }

                pub mod temp_targets {
                    pub struct TempTargets {
                        pub case: case::Case,
                        pub cpu: cpu::Cpu
                    }

                    pub mod case {
                        pub type Case = f64;
                    }

                    pub mod cpu {
                        pub type Cpu = f64;
                    }
                }
            }

            pub mod owner {
                pub struct Owner {
                    pub dob: dob::Dob,
                    pub name: name::Name
                }

                pub mod dob {
                    pub type Dob = &'static str;
                }

                pub mod name {
                    pub type Name = &'static str;
                }
            }

            pub mod servers {
                pub struct Servers {
                    pub alpha: alpha::Alpha,
                    pub beta: beta::Beta
                }

                pub mod alpha {
                    pub struct Alpha {
                        pub ip: ip::Ip,
                        pub role: role::Role
                    }

                    pub mod ip {
                        pub type Ip = &'static str;
                    }

                    pub mod role {
                        pub type Role = &'static str;
                    }
                }

                pub mod beta {
                    pub struct Beta {
                        pub ip: ip::Ip,
                        pub role: role::Role
                    }

                    pub mod ip {
                        pub type Ip = &'static str;
                    }

                    pub mod role {
                        pub type Role = &'static str;
                    }
                }
            }

            pub mod title {
                pub type Title = &'static str;
            }
        }
    };
    assert_eq!(toml_ts.to_string(), toml_ts_expected.to_string());
}

#[test]
fn configured_type_tokens_work() {
    let values_ident_config = StaticTomlAttributes {
        values_ident: Some(format_ident!("items")),
        ..StaticTomlAttributes::default()
    };

    let prefer_slices_config = StaticTomlAttributes {
        prefer_slices: Some(LitBool::new(false, Span2::call_site())),
        ..StaticTomlAttributes::default()
    };

    let prefix_config = StaticTomlAttributes {
        prefix: Some(format_ident!("Prefix")),
        ..StaticTomlAttributes::default()
    };

    let suffix_config = StaticTomlAttributes {
        suffix: Some(format_ident!("Suffix")),
        ..StaticTomlAttributes::default()
    };

    let prefix_suffix_config = StaticTomlAttributes {
        prefix: Some(format_ident!("Prefix")),
        suffix: Some(format_ident!("Suffix")),
        ..StaticTomlAttributes::default()
    };

    let empty_derive = vec![];

    let toml: Value = toml::from_str(include_str!("../../../example.toml")).unwrap();
    let title = toml.get("title").unwrap();
    let database = toml.get("database").unwrap();
    let ports = database.get("ports").unwrap();

    let values_ident_ts = ports
        .type_tokens("ports", &values_ident_config, quote!(pub), &empty_derive)
        .unwrap();
    let values_ident_ts_expected = quote! {
        pub mod ports {
            pub type Ports = [items::Items; 3usize];

            pub mod items {
                pub type Items = i64;
            }
        }
    };
    assert_eq!(
        values_ident_ts.to_string(),
        values_ident_ts_expected.to_string()
    );

    let prefer_slices_ts = ports
        .type_tokens("ports", &prefer_slices_config, quote!(pub), &empty_derive)
        .unwrap();
    let prefer_slices_ts_expected = quote! {
        pub mod ports {
            pub struct Ports(pub values_0::Values0, pub values_1::Values1, pub values_2::Values2);

            pub mod values_0 {
                pub type Values0 = i64;
            }

            pub mod values_1 {
                pub type Values1 = i64;
            }

            pub mod values_2 {
                pub type Values2 = i64;
            }
        }
    };
    assert_eq!(
        prefer_slices_ts.to_string(),
        prefer_slices_ts_expected.to_string()
    );

    let prefix_ts = title
        .type_tokens("title", &prefix_config, quote!(pub), &empty_derive)
        .unwrap();
    let prefix_ts_expected = quote! {
        pub mod title {
            pub type PrefixTitle = &'static str;
        }
    };
    assert_eq!(prefix_ts.to_string(), prefix_ts_expected.to_string());

    let suffix_ts = title
        .type_tokens("title", &suffix_config, quote!(pub), &empty_derive)
        .unwrap();
    let suffix_ts_expected = quote! {
        pub mod title {
            pub type TitleSuffix = &'static str;
        }
    };
    assert_eq!(suffix_ts.to_string(), suffix_ts_expected.to_string());

    let prefix_suffix_ts = title
        .type_tokens("title", &prefix_suffix_config, quote!(pub), &empty_derive)
        .unwrap();
    let prefix_suffix_ts_expected = quote! {
        pub mod title {
            pub type PrefixTitleSuffix = &'static str;
        }
    };
    assert_eq!(
        prefix_suffix_ts.to_string(),
        prefix_suffix_ts_expected.to_string()
    );

    let prefix_suffix_ts2 = ports
        .type_tokens("ports", &prefix_suffix_config, quote!(pub), &empty_derive)
        .unwrap();
    let prefix_suffix_ts2_expected = quote! {
        pub mod ports {
            pub type PrefixPortsSuffix = [values::PrefixValuesSuffix; 3usize];

            pub mod values {
                pub type PrefixValuesSuffix = i64;
            }
        }
    };
    assert_eq!(
        prefix_suffix_ts2.to_string(),
        prefix_suffix_ts2_expected.to_string()
    );
}

#[test]
fn derive_propagation_works() {
    let config = StaticTomlAttributes::default();
    let derive: Vec<Attribute> = vec![
        parse_quote!(#[derive(PartialEq, Eq)]),
        parse_quote!(#[derive(Default)]),
    ];

    let toml: Value = toml::from_str(include_str!("../../../example.toml")).unwrap();
    let servers = toml.get("servers").unwrap();

    let servers_derived_ts = servers
        .type_tokens("servers", &config, quote!(pub), &derive)
        .unwrap();
    let servers_derived_ts_expected = quote! {
        pub mod servers {
            #[derive(PartialEq, Eq)]
            #[derive(Default)]
            pub struct Servers {
                pub alpha: alpha::Alpha,
                pub beta: beta::Beta
            }

            pub mod alpha {
                #[derive(PartialEq, Eq)]
                #[derive(Default)]
                pub struct Alpha {
                    pub ip: ip::Ip,
                    pub role: role::Role
                }

                pub mod ip {
                    pub type Ip = &'static str;
                }

                pub mod role {
                    pub type Role = &'static str;
                }
            }

            pub mod beta {
                #[derive(PartialEq, Eq)]
                #[derive(Default)]
                pub struct Beta {
                    pub ip: ip::Ip,
                    pub role: role::Role
                }

                pub mod ip {
                    pub type Ip = &'static str;
                }

                pub mod role {
                    pub type Role = &'static str;
                }
            }
        }
    };
    assert_eq!(
        servers_derived_ts.to_string(),
        servers_derived_ts_expected.to_string()
    );
}
