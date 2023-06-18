use crate::args::NamedArgs;
use convert_case::Case;
use convert_case::Casing;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::Ident as Ident2;
use syn::LitBool;
use toml::value::Array;
use toml::{Table, Value};

pub trait Snapshot {
    fn type_eq(&self, other: &Self) -> bool;

    fn type_tokens(&self, key: &str, config: &NamedArgs) -> TokenStream2;

    fn static_tokens(&self, config: &NamedArgs) -> TokenStream2;
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
                a.keys().cloned().chain(b.keys().cloned()),
            )
            .iter()
            .map(|k| (a.get(k), b.get(k)))
            .map(|(a, b)| match (a, b) {
                (Some(a), Some(b)) => a.type_eq(b),
                _ => false,
            })
            .reduce(|acc, b| acc && b)
            .unwrap_or(true),

            _ => false,
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
            Array(values) => type_tokens_array(values, &type_ident, config),
            Table(values) => type_tokens_table(values, &type_ident, config),
        };

        quote! {
            pub mod #mod_ident {
                #inner
            }
        }
    }

    fn static_tokens(&self, config: &NamedArgs) -> TokenStream2 {
        todo!()
    }
}

fn fixed_ident(ident: &str, prefix: &Option<Ident2>, suffix: &Option<Ident2>) -> Ident2 {
    let ident = ident.to_case(Case::Pascal);
    match (prefix, suffix) {
        (None, None) => format_ident!("{ident}"),
        (Some(prefix), None) => format_ident!("{prefix}{ident}"),
        (None, Some(suffix)) => format_ident!("{ident}{suffix}"),
        (Some(prefix), Some(suffix)) => format_ident!("{prefix}{ident}{suffix}"),
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

#[inline]
fn type_tokens_array(array: &Array, type_ident: &Ident2, config: &NamedArgs) -> TokenStream2 {
    let use_slices = use_slices(array, config);
    let values_ident = config
        .values_ident
        .as_ref()
        .map(|i| i.to_string())
        .unwrap_or_else(|| "values".to_string());
    let values_type_ident = format_ident!("{}", values_ident.to_case(Case::Pascal));
    let values_mod_ident = format_ident!("{}", values_ident.to_case(Case::Snake));
    if use_slices {
        let len = array.len();
        let Some(value) = array.get(0) else {
            return quote! {
                pub type #type_ident = [(); 0];
            }
        };

        let value_type_tokens = value.type_tokens(&values_ident, config);

        quote! {
            pub type #type_ident = [#values_mod_ident::#values_type_ident; #len];

            #value_type_tokens
        }
    } else {
        let value_tokens: Vec<TokenStream2> = array
            .iter()
            .enumerate()
            .map(|(i, v)| v.type_tokens(&format!("{}{i}", &values_ident), config))
            .collect();
        let value_types: Vec<TokenStream2> = array
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let mod_ident = format_ident!("{}_{i}", values_ident.to_case(Case::Snake));
                let type_ident = format_ident!("{}{i}", values_ident.to_case(Case::Pascal));
                quote!(#mod_ident::#type_ident)
            })
            .collect();

        quote! {
            pub struct #type_ident(#(#value_types),*);

            #(#value_tokens)*
        }
    }
}

fn type_tokens_table(table: &Table, type_ident: &Ident2, config: &NamedArgs) -> TokenStream2 {
    let mods_tokens: Vec<TokenStream2> = table
        .iter()
        .map(|(k, v)| v.type_tokens(k, config))
        .collect();

    let fields_tokens: Vec<TokenStream2> = table
        .iter()
        .map(|(k, v)| {
            let field_key = format_ident!("{}", k.to_case(Case::Snake));
            let type_ident = fixed_ident(k, &config.prefix, &config.suffix);
            quote!(#field_key: #field_key::#type_ident)
        })
        .collect();

    quote! {
        pub struct #type_ident {
            #(#fields_tokens),*
        }

        #(#mods_tokens)*
    }
}

#[cfg(test)]
mod tests {
    use crate::args::NamedArgs;
    use crate::snapshot::Snapshot;
    use quote::quote;
    use toml::value::Value;

    #[test]
    fn type_eq_works() {
        let toml: Value = toml::from_str(include_str!("../example.toml")).unwrap();

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

        let toml: Value = toml::from_str(include_str!("../example.toml")).unwrap();
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
}
