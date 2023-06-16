pub mod expected {
    pub struct Toml {
        pub package: package::Toml,
        pub dependencies: dependencies::Toml
    }

    pub mod package {
        pub struct Toml {
            pub name: name::Toml,
            pub version: version::Toml,
            pub edition: edition::Toml,
        }

        pub mod name {
            pub type Toml = &'static str;
        }

        pub mod version {
            pub type Toml = &'static str;
        }

        pub mod edition {
            pub type Toml = &'static str;
        }
    }

    pub mod dependencies {
        pub struct Toml {
            pub toml_struct_gen: toml_struct_gen::Toml,
        }

        pub mod toml_struct_gen {
            pub struct Toml {
                pub path: path::Toml,
            }

            pub mod path {
                pub type Toml = &'static str;
            }
        }
    }
}

toml_struct_gen::toml!("Cargo.toml");
