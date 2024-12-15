//! Provides functionality for parsing TOML configuration files into Rust data
//! structures.
//!
//! The `parse` module is primarily concerned with reading TOML files and
//! converting their contents into a structured format suitable for further
//! processing. This acts as a foundation for generating Rust source code that
//! represents the configuration specified in the TOML files.

use syn::meta::ParseNestedMeta;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Error, Ident as Ident2, LitBool, LitStr, Token, Visibility};

/// Represents the input to the static_toml macro.
///
/// Contains a collection of `StaticTomlItem` structs which represent individual
/// TOML files and the associated configurations and attributes.
pub struct StaticToml(pub Vec<StaticTomlItem>);

/// Represents a single TOML file and its associated configurations and
/// attributes.
pub struct StaticTomlItem {
    /// Configuration attributes specific to static_toml macro.
    pub attrs: StaticTomlAttributes,
    /// Attributes other than doc and derive.
    pub other_attrs: Vec<Attribute>,
    /// Documentation attributes.
    pub doc: Vec<Attribute>,
    /// Derive attributes.
    pub derive: Vec<Attribute>,
    /// Visibility of the static value (e.g., `pub`, `pub(crate)`).
    pub visibility: Option<Visibility>,
    /// Storage class of the variable (`static` or `const`).
    pub storage_class: StorageClass,
    /// The name of the static value.
    pub name: Ident2,
    /// The path to the TOML file.
    pub path: LitStr
}

/// Contains configuration attributes for the static_toml macro.
#[derive(Default)]
pub struct StaticTomlAttributes {
    pub prefix: Option<Ident2>,
    pub suffix: Option<Ident2>,
    pub root_mod: Option<Ident2>,
    pub values_ident: Option<Ident2>,
    pub prefer_slices: Option<LitBool>,
    pub auto_doc: Option<LitBool>,
    pub cow: Option<()>
}

/// A token representing the 'include_toml' keyword.
struct IncludeTomlToken;

/// Storage class for the literal value.
pub enum StorageClass {
    Static(Token![static]),
    Const(Token![const])
}

/// Parse implementation for `StaticToml`.
///
/// Parses the input into a `StaticToml` struct which contains a vector of
/// `StaticTomlItem` structs.
impl Parse for StaticToml {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = vec![];

        // Parse StaticTomlItems until the input stream is empty.
        while !input.is_empty() {
            items.push(input.parse()?);
        }

        Ok(Self(items))
    }
}

/// Parse implementation for `StaticTomlItem`.
///
/// Parses the input into a `StaticTomlItem` struct which represents a single
/// TOML file and its associated configurations and attributes.
impl Parse for StaticTomlItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse attributes.
        let all_attrs = match input.peek(Token![#]) {
            false => None,
            true => Some(input.call(Attribute::parse_outer)?)
        };

        let mut attrs = StaticTomlAttributes::default();
        let mut other_attrs = Vec::new();
        let mut doc = Vec::new();
        let mut derive = Vec::new();

        // Separate attributes into doc, derive, other attributes, and static_toml
        // specific attributes.
        if let Some(all_attrs) = all_attrs {
            for attr in all_attrs {
                if attr.path().is_ident("doc") {
                    doc.push(attr);
                    continue;
                }

                if attr.path().is_ident("derive") {
                    derive.push(attr);
                    continue;
                }

                if !attr.path().is_ident("static_toml") {
                    other_attrs.push(attr);
                    continue;
                }

                // Parse static_toml specific attributes.
                attr.parse_nested_meta(|meta| {
                    let Some(key) = meta.path.get_ident()
                    else {
                        return Ok(());
                    };

                    match key.to_string().as_str() {
                        "prefix" => attrs.prefix = Some(meta.value()?.parse()?),
                        "suffix" => attrs.suffix = Some(meta.value()?.parse()?),
                        "root_mod" => attrs.root_mod = Some(meta.value()?.parse()?),
                        "values_ident" => attrs.values_ident = Some(meta.value()?.parse()?),
                        "prefer_slices" => attrs.prefer_slices = Some(meta.value()?.parse()?),
                        "auto_doc" => attrs.auto_doc = Some(meta.value()?.parse()?),
                        "cow" => attrs.cow = Some(Self::validate_no_value(&meta, "cow")?),
                        _ => {
                            return Err(meta.error(
                                "unexpected attribute, expected one of `prefix`, `suffix`, \
                                 `root_mod`, `values_ident`, `prefer_slices` or `auto_doc`"
                            ))
                        }
                    }

                    Ok(())
                })?;
            }
        }

        // Parse visibility.
        let visibility = match input.peek(Token![pub]) {
            false => None,
            true => Some(input.parse()?)
        };

        // Parse the remainder of the StaticTomlItem.
        let storage_class = input.parse()?;
        let name = input.parse()?;
        input.parse::<Token![=]>()?;
        input.parse::<IncludeTomlToken>()?;
        input.parse::<Token![!]>()?;
        let content;
        syn::parenthesized!(content in input);
        let path = content.parse()?;
        input.parse::<Token![;]>()?;

        Ok(Self {
            attrs,
            other_attrs,
            doc,
            derive,
            visibility,
            storage_class,
            name,
            path
        })
    }
}

impl StaticTomlItem {
    /// Validate that the `value()` part of [`ParseNestedMeta`] is empty.
    ///
    /// This is used to validate that static toml items which do not accept
    /// values got no value.
    fn validate_no_value(meta: &ParseNestedMeta, item: &str) -> syn::Result<()> {
        let value = meta.value();
        if let Ok(value) = value {
            return Err(value.error(format!(
                "`{item}` is a standalone attribute and does not accept a value"
            )));
        }

        Ok(())
    }
}

const EXPECTED_INCLUDE_TOML: &str = "expected `include_toml`";

/// Parse implementation for `IncludeTomlToken`.
///
/// Ensures that the token is the 'include_toml' keyword.
impl Parse for IncludeTomlToken {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse the token and ensure it matches 'include_toml'.
        let include_toml: Ident2 = input
            .parse()
            .map_err(|e| syn::Error::new(e.span(), EXPECTED_INCLUDE_TOML))?;
        if include_toml != "include_toml" {
            return Err(Error::new_spanned(include_toml, EXPECTED_INCLUDE_TOML));
        }

        Ok(IncludeTomlToken)
    }
}

/// Parse implementation for `StorageClass`.
///
/// Parses the storage classes `static` or `const`.
impl Parse for StorageClass {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![static]) {
            return Ok(StorageClass::Static(input.parse::<Token![static]>()?));
        }

        if input.peek(Token![const]) {
            return Ok(StorageClass::Const(input.parse::<Token![const]>()?));
        }

        Err(input.error("expected `static` or `const`"))
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span as Span2;
    use quote::{format_ident, quote, ToTokens};
    use syn::{parse_quote, LitBool, Token, Visibility};

    use crate::parse::{IncludeTomlToken, StaticToml, StorageClass, EXPECTED_INCLUDE_TOML};

    impl StorageClass {
        fn is_static(&self) -> bool {
            match self {
                StorageClass::Static(_) => true,
                StorageClass::Const(_) => false
            }
        }

        fn is_const(&self) -> bool {
            match self {
                StorageClass::Static(_) => false,
                StorageClass::Const(_) => true
            }
        }
    }

    #[test]
    fn parse_include_toml_token() {
        let input = quote!(include_toml);
        assert!(syn::parse2::<IncludeTomlToken>(input).is_ok());

        let input = quote!(include_json);
        match syn::parse2::<IncludeTomlToken>(input) {
            Err(e) => assert_eq!(e.to_string(), EXPECTED_INCLUDE_TOML),
            Ok(_) => panic!("should be error variant")
        }
    }

    #[test]
    fn parse_static_toml() {
        let items: StaticToml = parse_quote! {
            #[static_toml(prefix = Cool, root_mod = img)]
            static IMAGES = include_toml!("images.toml");

            #[derive(PartialEq, Eq)]
            #[derive(Default)]
            #[static_toml(values_ident = items, suffix = Config, prefer_slices = false)]
            pub const CONFIG = include_toml!("config.toml");

            /// Documentation comment
            #[must_use]
            pub(crate) static EXAMPLE = include_toml!("example.toml");

            static BASIC = include_toml!("basic.toml");
        };

        let mut items = items.0.into_iter();

        let images = items.next().unwrap();
        assert_eq!(images.attrs.prefix, Some(format_ident!("Cool")));
        assert!(images.attrs.suffix.is_none());
        assert_eq!(images.attrs.root_mod, Some(format_ident!("img")));
        assert!(images.attrs.values_ident.is_none());
        assert!(images.attrs.prefer_slices.is_none());
        assert!(images.other_attrs.is_empty());
        assert!(images.derive.is_empty());
        assert!(images.visibility.is_none());
        assert!(images.storage_class.is_static());
        assert_eq!(images.name, format_ident!("IMAGES"));
        assert_eq!(images.path.value().as_str(), "images.toml");

        let config = items.next().unwrap();
        assert!(config.attrs.prefix.is_none());
        assert_eq!(config.attrs.suffix, Some(format_ident!("Config")));
        assert!(config.attrs.root_mod.is_none());
        assert_eq!(config.attrs.values_ident, Some(format_ident!("items")));
        assert_eq!(
            config.attrs.prefer_slices,
            Some(LitBool::new(false, Span2::call_site()))
        );
        assert!(config.other_attrs.is_empty());
        assert_eq!(
            config.derive[0].to_token_stream().to_string(),
            quote!(#[derive(PartialEq, Eq)]).to_string()
        );
        assert_eq!(
            config.derive[1].to_token_stream().to_string(),
            quote!(#[derive(Default)]).to_string()
        );
        assert_eq!(
            config.visibility,
            Some(Visibility::Public(Token![pub](Span2::call_site())))
        );
        assert!(config.storage_class.is_const());
        assert_eq!(config.name, format_ident!("CONFIG"));
        assert_eq!(config.path.value().as_str(), "config.toml");

        let example = items.next().unwrap();
        assert!(example.attrs.prefix.is_none());
        assert!(example.attrs.suffix.is_none());
        assert!(example.attrs.root_mod.is_none());
        assert!(example.attrs.values_ident.is_none());
        assert!(example.attrs.prefer_slices.is_none());
        assert_eq!(
            example.doc[0].path().get_ident(),
            Some(&format_ident!("doc"))
        );
        assert_eq!(example.doc.len(), 1);
        assert_eq!(
            example.other_attrs[0].path().get_ident(),
            Some(&format_ident!("must_use"))
        );
        assert_eq!(example.other_attrs.len(), 1);
        let Some(Visibility::Restricted(_)) = example.visibility
        else {
            panic!("not a restricted visibility");
        };
        assert!(example.derive.is_empty());
        assert!(example.storage_class.is_static());
        assert_eq!(example.name, format_ident!("EXAMPLE"));
        assert_eq!(example.path.value().as_str(), "example.toml");

        let basic = items.next().unwrap();
        assert!(basic.attrs.prefix.is_none());
        assert!(basic.attrs.suffix.is_none());
        assert!(basic.attrs.root_mod.is_none());
        assert!(basic.attrs.values_ident.is_none());
        assert!(basic.attrs.prefer_slices.is_none());
        assert!(basic.other_attrs.is_empty());
        assert!(basic.visibility.is_none());
        assert!(basic.derive.is_empty());
        assert!(basic.storage_class.is_static());
        assert_eq!(basic.name, format_ident!("BASIC"));
        assert_eq!(basic.path.value().as_str(), "basic.toml");
    }
}
