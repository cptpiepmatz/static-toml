use quote::quote;
use toml::value::Value;

use crate::args::NamedArgs;
use crate::toml_tokens::TomlTokens;

#[test]
fn type_eq_works() {
    let toml: Value = toml::from_str(include_str!("../../example.toml")).unwrap();

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
fn type_tokens_works() {
    let config = NamedArgs::default();

    let toml: Value = toml::from_str(include_str!("../../example.toml")).unwrap();
    let title = toml.get("title").unwrap();
    let database = toml.get("database").unwrap();
    let enabled = database.get("enabled").unwrap();
    let ports = database.get("ports").unwrap();
    let data = database.get("data").unwrap();
    let temp_targets = database.get("temp_targets").unwrap();

    let title_ts = title.type_tokens("title", &config);
    let title_ts_expected = quote! {
        pub mod title {
            pub type Title = &'static str;
        }
    };
    assert_eq!(title_ts.to_string(), title_ts_expected.to_string());

    let enabled_ts = enabled.type_tokens("enabled", &config);
    let enabled_ts_expected = quote! {
        pub mod enabled {
            pub type Enabled = bool;
        }
    };
    assert_eq!(enabled_ts.to_string(), enabled_ts_expected.to_string());

    let ports_ts = ports.type_tokens("ports", &config);
    let ports_ts_expected = quote! {
        pub mod ports {
            pub type Ports = [values::Values; 3usize];

            pub mod values {
                pub type Values = i64;
            }
        }
    };
    assert_eq!(ports_ts.to_string(), ports_ts_expected.to_string());

    let data_ts = data.type_tokens("data", &config);
    let data_ts_expected = quote! {
        pub mod data {
            pub struct Data(values_0::Values0, values_1::Values1);

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

    let temp_targets_ts = temp_targets.type_tokens("temp_targets", &config);
    let temp_targets_ts_expected = quote! {
        pub mod temp_targets {
            pub struct TempTargets {
                case: case::Case,
                cpu: cpu::Cpu
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

    let toml_ts = toml.type_tokens("toml", &config);
    let toml_ts_expected = quote! {
        pub mod toml {
            pub struct Toml {
                database: database::Database,
                owner: owner::Owner,
                servers: servers::Servers,
                title: title::Title
            }

            pub mod database {
                pub struct Database {
                    data: data::Data,
                    enabled: enabled::Enabled,
                    ports: ports::Ports,
                    temp_targets: temp_targets::TempTargets
                }

                pub mod data {
                    pub struct Data(values_0::Values0, values_1::Values1);

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
                        case: case::Case,
                        cpu: cpu::Cpu
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
                    dob: dob::Dob,
                    name: name::Name
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
                    alpha: alpha::Alpha,
                    beta: beta::Beta
                }

                pub mod alpha {
                    pub struct Alpha {
                        ip: ip::Ip,
                        role: role::Role
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
                        ip: ip::Ip,
                        role: role::Role
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
