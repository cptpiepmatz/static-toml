# Changelog

## v1.3.0 - 2024-12-15

### Added

- **`cow` Attribute**:
  - Added `cow` configuration to the `static_toml!` macro.
  - When enabled, it replaces static slices (e.g., `&'static str`) and arrays
    with `std::borrow::Cow`.
  - This allows for dynamic modifications or ownership flexibility of the data.
  - Example:
    ```rust
    static_toml::static_toml! {
        #[static_toml(cow)]
        const CONFIG = include_toml!("config.toml");
    }
    ```
    This generates `Cow<'static, str>` instead of `&'static str`.
- **GitHub CI Improvements**:
  - Added a new **Build Examples** job to validate example builds.
  - Updated CI to use modern GitHub Actions features and simplify workflows.
  - Improved verbosity of cargo commands for better debugging.
- **New Example - Config Handling**:
  - Added `examples/config.rs` to demonstrate cow usage and deserialization of
    TOML data.
  - Supports mixing static defaults and user-provided dynamic configurations at
    runtime.

### Changed

- **Improved Documentation**:
  - Expanded the README and crate documentation to explain the new `cow`
    attribute.
  - Updated macro invocation examples for clarity.
- **Refactored Code**:
  - Optimized internal handling of `static` and `const` attributes.
  - Improved parsing logic to validate standalone attributes like `cow`.

## v1.2.0 - 2023-11-25

### Added

- **`const` Support**:
  - The `static_toml!` macro now supports embedding TOML data as `const` values
    in addition to `static`.
  - This is useful for const functions, const generics, or cases where a
    constant value is required.
  - Example:
    ```rust
    static_toml::static_toml! {
        const MESSAGES = include_toml!("messages.toml");
    }
    ```
- **Mixed Storage Classes**:
  - You can now mix `static` and `const` definitions in a single `static_toml!`
    macro call, provided the identifiers are unique:
    ```rust
    static_toml::static_toml! {
        static SETTINGS = include_toml!("settings.toml");
        const MESSAGES = include_toml!("messages.toml");
    }
    ```
- **`auto_doc` Enhancements**:
  - Documentation generation now reflects whether the TOML inclusion is `static`
    or `const`.
  - The generated comments will display "Static inclusion" or "Constant
    inclusion" accordingly.

### Changed

- Improved internal parsing and handling to support both `static` and `const`
  storage classes.

## v1.1.0 - 2023-11-19

### Added

- **`auto_doc` Attribute**:
  - A new `auto_doc` attribute can be added to `static_toml!` to generate
    automatic documentation comments.
  - When enabled (`auto_doc = true`), the macro appends the TOML file's path and
    contents as documentation, enhancing visibility in **rustdoc**.
  - Example:
    ```rust
    static_toml::static_toml! {
        #[static_toml(auto_doc = true)]
        static EXAMPLE = include_toml!("example.toml");
    }
    ```
- **Documentation Generation**:
  - `auto_doc` generates inline TOML content documentation wrapped in code
    blocks for better display.

### Changed

- Updated `toml` dependency from 0.7 to 0.8.

### Fixed

- Improved parsing of attributes to support `auto_doc` without breaking existing
  attribute handling.

## v1.0.1 - 2023-07-09

### Added

- **License**: Added an MIT License to the repository.
- **Documentation Improvements**:
  - Enhanced README with a clearer usage example for including TOML files.
  - Added structured examples for handling nested TOML data with the
    `static_toml!` macro.
- **Example Data**: Introduced a sample messages.toml file for demonstration
  purposes.

### Changed

- Updated `.gitignore` to exclude `Cargo.lock` for libraries as per
  [Cargo FAQ](https://doc.rust-lang.org/cargo/faq.html#why-have-cargolock-in-version-control).

### Removed

- Deleted `Cargo.lock` from version control since it is unnecessary for library
  crates.
