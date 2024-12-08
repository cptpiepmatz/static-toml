<h1 align="center">static-toml</h1>
<p align="center">
  <b>
    Effortlessly embed TOML files into your Rust code as static data with 
    custom data structures.
  </b>
</p>

<br>

<div align="center">

  [![Version](https://img.shields.io/crates/v/static-toml?style=for-the-badge)](https://crates.io/crates/static-toml)
  [![License](https://img.shields.io/crates/l/static-toml?style=for-the-badge)](https://github.com/cptpiepmatz/static-toml/blob/main/LICENSE)
  [![Docs](https://img.shields.io/docsrs/static-toml?style=for-the-badge&logo=docs.rs)](https://docs.rs/static-toml)
  [![CI](https://img.shields.io/github/actions/workflow/status/cptpiepmatz/static-toml/cargo.yml?style=for-the-badge&logo=github&label=CI)](https://docs.rs/static-toml)

</div>



## About

Embed TOML files into your Rust binaries via a procedural macro.
This library enables the inclusion of TOML files at compile-time and generates 
static data structures that can be directly accessed by your Rust code without 
the need for runtime parsing.

## Key Features
- 📝 Embed TOML configuration files effortlessly.
- 🔨 Generate reliable Rust data structures to represent your TOML contents.
- 🔧 Customize your generated types with prefixes and suffixes for flexibility.
- 🚦 Enjoy clear and concise compile-time error messages for easier debugging.

## Usage
First, make sure to add `static-toml` to your `Cargo.toml` dependencies:
Either by command line:
```shell 
cargo add static-toml
```
Or by adding it to your `Cargo.toml` directly:
```toml
[dependencies]
static-toml = "1"
```

Then, make use of the `static_toml!` macro to include your TOML file:
```toml
# message.toml

[info]
welcome = "Welcome to our application!"
update = "Your data has been updated successfully."

[errors]
file_not_found = "The requested file could not be found."
permission_denied = "You do not have permission to perform this action."
```

```rust
static_toml::static_toml! {
    static MESSAGES = include_toml!("messages.toml");
}

const WELCOME_MESSAGE: &str = MESSAGES.info.welcome;
```
This will read your TOML file and generate Rust data structures accordingly.
Now you can access the values from the TOML file with ease.

In addition to using `static`, the `static_toml!` macro also allows the use of 
`const` for embedding TOML data. 
This can be particularly useful in scenarios where a constant value is required, 
such as within const functions or for const generics. 
To use this feature, simply replace `static` with `const` in the macro call when 
necessary.

## Customization Options
You can configure how the macro should generate data types:
```rust
static_toml! {
    #[static_toml(
        prefix = Prefix, 
        suffix = Suffix, 
        root_mod = cfg, 
        values_ident = items, 
        prefer_slices = false
    )]
    static CONFIG = include_toml!("config.toml");
}
```

- `prefix`: 
  Adds a prefix to the generated data types. 
  It's recommended to use `PascalCase` for the prefix.

- `suffix`: 
  Similar to prefix, but it’s a suffix.

- `root_mod`: 
  Sets the identifier for the root module that will contain the data types for 
  the TOML file.

- `values_ident`: 
  When generating data types for arrays, this specifies a different name for 
  the modules and data types (default is `values`).

- `prefer_slices`: 
  Determines whether the macro should generate fixed-size slices for arrays. 
  If set to `false`, tuples will be generated (default is `true`).

## Enhancing Your Types
You can use doc comments, derive attributes, and other attributes.
Additionally, you can set visibility. 
Your code can be clean and descriptive.

```rust
static_toml! {
    /// The configuration.
    #[derive(Debug)]
    #[allow(missing_docs)]
    pub static CONFIG = include_toml!("config.toml");
}
```

## Error Handling
Encounter compile errors? 
No worries. 
`static-toml` provides informative error messages, whether it's TOML parsing or 
file access issues.

## Example
Suppose you have a simple TOML file like this:
```toml
# config.toml
[database]
url = "localhost"
port = 5432
```

Use the `static_toml!` macro:
```rust
static_toml! {
    static CONFIG = include_toml!("config.toml");
}
```

And just like that, you have Rust data structures ready to use.
```rust
assert_eq!(CONFIG.database.url, "localhost");
assert_eq!(CONFIG.database.port, 5432);
```
