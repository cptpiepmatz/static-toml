Macro to statically embed TOML files.

For basic usage and configuration, see the
[crate-level documentation](crate).

# Macro Invocation Overview
Consider the following example where the macro is invoked:
```
# mod _macro_invocation_overview {
static_toml::static_toml! {
    /// Example TOML file from [toml.io](https://toml.io/en).
    #[allow(missing_docs)]
    #[derive(Debug)]
    #[static_toml(suffix = Toml)]
    pub(crate) static EXAMPLE = include_toml!("example.toml");
}
# }
```

Note: In the subsequent sections showing generated values, it is assumed
that the macro is invoked with default configurations.
This assumption is made to keep the examples concise and focused on
illustrating the structure of the generated code, rather than the effects
of various configuration options.

Alongside an [official example TOML file](https://toml.io/en/) structured as follows:
```toml
# This is a TOML document

title = "TOML Example"

[owner]
name = "Tom Preston-Werner"
dob = 1979-05-27T07:32:00-08:00

[database]
enabled = true
ports = [ 8000, 8001, 8002 ]
data = [ ["delta", "phi"], [3.14] ]
temp_targets = { cpu = 79.5, case = 72.0 }

[servers]

[servers.alpha]
ip = "10.0.0.1"
role = "frontend"

[servers.beta]
ip = "10.0.0.2"
role = "backend"
```

The macro begins by parsing the entire input.
During this process, it identifies and extracts documentation comments,
`derive` attributes, `static_toml` attributes, other attributes,
visibility modifiers (if any), the name of the  static value, and the
file path of the TOML file.

The elements like `static` keyword, `=` sign, and the pseudo `include_toml!`
macro are utilized to create a Rust-like syntax, ensuring that IDEs provide
appropriate syntax highlighting.

The macro then applies the extracted documentation comments to the static
value, and `derive` attributes are applied to every data type that is
generated. This is particularly useful when deriving traits that require all
fields to implement the same trait.
Other attributes are applied to the root module of the generated data types.

The `static_toml` attribute is used exclusively for configuring the macro
invocation and doesn't appear in any generated code.

Visibility modifiers are applied to both the static value and the outermost
module that contains the data types.

The file path specified in the `include_toml!` macro is utilized to locate
the TOML file.
Since integrating external files during macro expansion can be challenging,
the provided path is concatenated with the
[`CARGO_MANIFEST_DIR`](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates)
environment variable during the build. This causes the path resolution to
always start from the root of the top-level crate, which currently hampers
the macro's compatibility with libraries due to path resolution challenges.

If you need support for library usage and have any suggestions or
workarounds, please open an issue in the
[GitHub repository](https://github.com/cptpiepmatz/static-toml/issues).

As for the output, the macro generates a static value, a module containing
the data types that represent the TOML content, and a constant named `_`
that leverages the `include_str!` macro to include the TOML file.
This usage of the `include_str!` macro ensures that the compiler is aware
of the file dependency, and as such, it will trigger a recompilation if the
file changes.
Furthermore, since this constant is named `_`, the compiler should optimize
it out, leaving no trace in the final binary.

# Generated Data Types
To statically embed a TOML file and facilitate its use in your Rust
application, the macro generates data types capable of representing the
structure of the included TOML file.
Initially, the macro reads the TOML file as a string and parses it using
the [toml crate](https://crates.io/crates/toml).
Subsequently, the macro traverses the resulting data structure to create
the necessary Rust types.

During this traversal, the macro generates structs and type aliases.
For primitive values, a type alias is created with its name being the key
used in the TOML file, converted to `PascalCase`.
The underlying data type for these aliases will be primitive types like
`i64` for integers, `f64` for floating-point numbers, `bool` for booleans,
and `&'static str` for strings.

For structured data, the macro generates structs, where the struct's name is
derived from the key in the TOML file, converted to `PascalCase`.
The fields within these structs are named after the keys in the structured
data, converted to `snake_case`.

For arrays in the TOML file, the macro generates slices or tuples, depending
on the complexity and uniformity of the elements.
The [crate-level documentation](crate) provides more information on the
conditions under which fixed-size slices or tuples are generated.
To facilitate this, the macro employs a custom type equality function
(attached via a trait to [`toml::Value`]) that recursively checks whether
two TOML values necessitate the *exact* same data type.
If all elements within an array can be represented using the same data type,
slices are used.

Because the macro could generate multiple data types with the same name, it
organizes them into separate modules to avoid name collisions.

Below is an example representation of the data types generated for the
example TOML file provided earlier:

```
# mod _generated_data_types {
mod example {
    pub struct Example {
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
# } // mod _generated_data_types
```

Note that the order of the generated token stream may differ from the TOML
file.
This discrepancy arises from how the [toml::Value] represents the
structure via binary trees where the keys are sorted.
However, this does not affect the integrity of the data representation,
and ordered data such as arrays are preserved correctly.

Furthermore, all internal fields within the generated structs are public,
allowing for easy access to the values.
However, it's essential to realize that making fields public is a
[strong commitment](https://rust-lang.github.io/api-guidelines/future-proofing.html#structs-have-private-fields-c-struct-private)
and can impact future compatibility.
This is yet another reason why this crate is recommended for use in binaries
rather than libraries.

# Generated Static Value
The primary purpose of this macro is to generate a static representation of
the TOML file you want to embed.
This static representation encompasses all the values contained within the
TOML file.

Here's what the generated static value would look like for the example TOML
file discussed earlier:
```
# mod _generated_static_value {
# static_toml::static_toml! {
#     static _EXAMPLE = include_toml!("example.toml");
# }
#
static EXAMPLE: example::Example = example::Example {
    database: example::database::Database {
        data: example::database::data::Data(["delta", "phi"], [3.14f64]),
        enabled: true,
        ports: [8000i64, 8001i64, 8002i64],
        temp_targets: example::database::temp_targets::TempTargets {
            case: 72f64,
            cpu: 79.5f64
        }
    },
    owner: example::owner::Owner {
        dob: "1979-05-27T07:32:00-08:00",
        name: "Tom Preston-Werner"
    },
    servers: example::servers::Servers {
        alpha: example::servers::alpha::Alpha {
            ip: "10.0.0.1",
            role: "frontend"
        },
        beta: example::servers::beta::Beta {
            ip: "10.0.0.2",
            role: "backend"
        }
    },
    title: "TOML Example"
};
# }
```

The macro constructs a static value based on the generated data types, and
initializes it with the contents of the TOML file.
This allows your program to access the configuration data at compile time
without needing to parse it at runtime.

# Configuration Details
The usage of the configuration options is explained in the
[crate level documentation](crate).
Here, we will outline the various options available for configuration
with some implementation details.

- `#[static_toml(prefix = Prefix)]`

  This attribute allows you to specify a prefix that will be added to the
  data types generated by the macro.
  The prefix should be a valid identifier.
  It's recommended to use `PascalCase` for the prefix, as this aligns with
  Rust's naming conventions.
  If it isn't in `PascalCase`, the compiler will issue a warning.
  You can suppress this warning by applying the
  `#[allow(non_camel_case_types)]` attribute, but it is not recommended.

  <br>

- `#[static_toml(suffix = Suffix)]`

  Similar to the prefix attribute, this adds a suffix to the generated data
  types.
  Note that if the suffix isn't capitalized, the compiler won't issue a
  warning, but the resulting names may not adhere to Rust's naming
  conventions.

  <br>

- `#[static_toml(root_mod = toml)]`

  This attribute sets the identifier for the root module that will contain
  the data types for the TOML file.
  This identifier also becomes the name of the root data type among the
  generated types.
  If this is not set, the name of the static value is converted to
  `snake_case` and used as the root module name.

  <br>

- `#[static_toml(values_ident = values)]`

  When generating data types for arrays, a separate namespace is needed.
  By default, this macro uses `values` for naming the modules and data
  types.
  This attribute allows you to specify a different name.
  For example, changing the value to "items" would alter the names of the
  generated modules and data types accordingly.
  Note that this configuration does not influence the key `value` in the
  TOML file.

  <br>

- `#[static_toml(prefer_slices = true)]`

  Determines whether the macro should attempt to generate fixed-size slices
  when handling arrays.
  If set to `false`, tuples will be generated for all arrays.
  The default value is `true`.
  Disabling this option will not only prevent the generation of fixed-size
  slices but also skip the equality check between array items, which might
  marginally speed up the compilation process.

Below is an example that illustrates how changing the `values_ident` to
"items" affects the generated structure:
```toml
# TOML file
[[list]]
value = 123

[[list]]
value = 456

[[tuple]]
a = 1

[[tuple]]
b = 2
```

```
// With `values_ident` set to "items"
mod lists {
    pub struct Lists {
        pub list: list::List,
        pub tuple: tuple::Tuple
    }

    pub mod list {
        pub type List = [items::Items; 2];

        pub mod items {
            pub struct Items {
                pub value: value::Value
            }

            mod value {
                pub type Value = i64;
            }
        }
    }

    pub mod tuple {
        pub struct Tuple(pub items_0::Items0, pub items_1::Items1);

        pub mod items_0 {
            pub struct Items0 {
                pub a: a::A
            }

            pub mod a {
                pub type A = i64;
            }
        }

        pub mod items_1 {
            pub struct Items1 {
                pub b: b::B
            }

            pub mod b {
                pub type B = i64;
            }
        }
    }
}
```

Note that the value identifiers within the TOML file are unaffected by this
configuration setting.

```
// With `values_ident` not being set
mod lists {
    pub struct Lists {
        pub list: list::List,
        pub tuple: tuple::Tuple
    }

    pub mod list {
        pub type List = [values::Values; 2];

        pub mod values {
            pub struct Values {
                pub value: value::Value
            }

            mod value {
                pub type Value = i64;
            }
        }
    }

    pub mod tuple {
        pub struct Tuple(pub values_0::Values0, pub values_1::Values1);

        pub mod values_0 {
            pub struct Values0 {
                pub a: a::A
            }

            pub mod a {
                pub type A = i64;
            }
        }

        pub mod values_1 {
            pub struct Values1 {
                pub b: b::B
            }

            pub mod b {
                pub type B = i64;
            }
        }
    }
}
```

# Error Handling

When using this macro, you might encounter errors in various scenarios.
The  macro is designed to provide descriptive error messages that will
help you to identify and fix issues quickly.
Here is an overview of the types of errors you might encounter and how
they are handled:

**TOML Parsing Errors**

If the TOML file included is malformed or contains syntax errors, the macro
will return a compile-time error with details about the parsing issue.
Make sure that the TOML file is valid and follows the TOML specification.

**Configuration Errors**

The macro accepts several configuration options through attributes (like
`prefix`, `suffix`, etc.).
If there is an issue with these configuration options
(e.g., invalid values), you will get a descriptive error message.

**File Not Found Errors**

If the TOML file specified to be embedded is not found, a compile-time error
will be triggered.

**Handling Errors**

When you encounter an error, read the error message carefully as it usually
contains information on what went wrong.
Correct the TOML file or configuration options as suggested by the error
message and try again.

In case the error messages are not descriptive enough or you encounter an
unexpected error, consider creating an issue in the project repository.
