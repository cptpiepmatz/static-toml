use std::path::PathBuf;

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::{
    parenthesized,
    parse::{discouraged::Speculative, Parse, ParseStream, Parser},
    punctuated, Attribute, Ident, LitBool, LitStr, Token, Visibility,
};

mod options;
pub use options::*;

mod structured_path;
pub use structured_path::*;

mod storage_class;
pub use storage_class::*;

mod include_toml_token;
pub use include_toml_token::*;

mod type_hint;
pub use type_hint::*;

pub struct Items(Vec<Item>);

/// Represents a single TOML file and its associated configurations and
/// attributes.
pub struct Item {
    /// Configuration attributes specific to static_toml macro.
    pub options: Options,
    /// Documentation attributes.
    pub doc_attrs: Vec<Attribute>,
    /// Derive attributes.
    pub derive_attrs: Vec<Attribute>,
    /// Attributes other than doc and derive.
    pub other_attrs: Vec<Attribute>,
    /// Visibility of the static value (e.g., `pub`, `pub(crate)`).
    pub visibility: Option<Visibility>,
    /// Storage class of the variable (`static` or `const`).
    pub storage_class: StorageClass,
    /// The name of the static value.
    pub name: Ident,
    /// The path to the TOML file.
    pub path: (PathBuf, Span),
}

impl IntoIterator for Items {
    type Item = Item;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Parse for Items {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = vec![];

        while !input.is_empty() {
            items.push(input.parse()?);
        }

        Ok(Self(items))
    }
}

impl Parse for Item {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut options = Options::default();
        let mut doc_attrs = Vec::<Attribute>::default();
        let mut derive_attrs = Vec::<Attribute>::default();
        let mut other_attrs = Vec::<Attribute>::default();

        while input.peek(Token![#]) {
            let attributes = input.call(Attribute::parse_outer)?;
            for attribute in attributes {
                Item::parse_attribute(
                    attribute,
                    &mut options,
                    &mut doc_attrs,
                    &mut derive_attrs,
                    &mut other_attrs,
                )?;
            }
        }

        let visibility = match input.peek(Token![pub]) {
            false => None,
            true => Some(input.parse()?),
        };

        let storage_class = input.parse()?;
        let name = input.parse()?;
        let _: Token![=] = input.parse()?;
        let _: IncludeTomlToken = input.parse()?;
        let _: Token![!] = input.parse()?;
        let content;
        syn::parenthesized!(content in input);
        let path_span = content.span();
        let path: LitStr = content.parse()?;
        let path = PathBuf::from(path.value());
        let path = (path, path_span);
        let _: Token![;] = input.parse()?;

        Ok(Item {
            options,
            doc_attrs,
            derive_attrs,
            other_attrs,
            visibility,
            storage_class,
            name,
            path,
        })
    }
}

impl Item {
    fn parse_attribute(
        attribute: Attribute,
        options: &mut Options,
        doc_attrs: &mut Vec<Attribute>,
        derive_attrs: &mut Vec<Attribute>,
        other_attrs: &mut Vec<Attribute>,
    ) -> syn::Result<()> {
        if attribute.path().is_ident("doc") {
            doc_attrs.push(attribute);
            return Ok(());
        }

        if attribute.path().is_ident("derive") {
            derive_attrs.push(attribute);
            return Ok(());
        }

        if !attribute.path().is_ident("static_toml") {
            other_attrs.push(attribute);
            return Ok(());
        }

        attribute.parse_nested_meta(|meta| {
            let Some(key) = meta.path.get_ident() else {
                return Ok(());
            };

            let value_or_empty = || match meta.input.is_empty() || meta.input.peek(Token![,]) {
                true => Ok(LitBool::new(true, Span::call_site())),
                false => meta.value()?.parse(),
            };

            match key.to_string().as_str() {
                "prefix" => options.prefix = Some(meta.value()?.parse()?),
                "suffix" => options.suffix = Some(meta.value()?.parse()?),
                "root_mod" => options.root_mod = Some(meta.value()?.parse()?),
                "values_ident" => options.values_ident = Some(meta.value()?.parse()?),
                "prefer_slices" => options.prefer_slices = Some(value_or_empty()?),
                "auto_doc" => options.auto_doc = Some(value_or_empty()?),
                "cow" => options.cow = Some(value_or_empty()?),
                "optional" => options
                    .optional
                    .push(Item::parse_optional_input(meta.input)?),
                _ => {
                    return Err(meta.error(
                        "unexpected attribute, expected one of `prefix`, `suffix`, `root_mod`, \
                        `values_ident`, `prefer_slices`, `auto_doc`, `cow` or `optional`",
                    ));
                }
            }

            Ok(())
        })
    }

    fn parse_optional_input(input: ParseStream) -> syn::Result<(StructuredPath, Option<TypeHint>)> {
        let inner;
        parenthesized!(inner in input);

        let path = inner.parse()?;
        let mut optional_type = None;
        if inner.peek(Token![,]) {
            let _: Token![,] = inner.parse()?;
            let _: Token![type] = inner.parse()?;
            let _: Token![=] = inner.parse()?;
            optional_type = Some(inner.parse()?);
        }

        if !inner.is_empty() {
            return Err(syn::Error::new(
                inner.span(),
                "unexpected tokens, use a new `optional` item for another value",
            ));
        }

        Ok((path, optional_type))
    }
}

#[cfg(test)]
mod tests {
    use quote::{format_ident, quote};
    use syn::{parse::Parser, parse_quote};

    use super::*;

    #[test]
    fn test_item_parse_attribute_options() {
        let test_cases = [
            // empty configuration
            (quote!(#[static_toml()]), Options::default()),
            // test auto_doc and cow
            (
                quote!(#[static_toml(auto_doc = true, cow = false)]),
                Options {
                    auto_doc: Some(LitBool::new(true, Span::call_site())),
                    cow: Some(LitBool::new(false, Span::call_site())),
                    ..Default::default()
                },
            ),
            // test boolean options without explicit values (should default to true)
            (
                quote!(#[static_toml(prefer_slices, auto_doc)]),
                Options {
                    prefer_slices: Some(LitBool::new(true, Span::call_site())),
                    auto_doc: Some(LitBool::new(true, Span::call_site())),
                    ..Default::default()
                },
            ),
            // test multiple identifiers
            (
                quote!(#[static_toml(prefix = Pfx, suffix = Sfx, root_mod = root, values_ident = items)]),
                Options {
                    prefix: Some(format_ident!("Pfx")),
                    suffix: Some(format_ident!("Sfx")),
                    root_mod: Some(format_ident!("root")),
                    values_ident: Some(format_ident!("items")),
                    ..Default::default()
                },
            ),
            // test multiple optional paths
            (
                quote!(#[static_toml(optional(logs[].*), optional(items[].name, type = String))]),
                Options {
                    optional: vec![
                        (parse_quote!(logs[].*), None),
                        (parse_quote!(items[].name), Some(TypeHint::String)),
                    ],
                    ..Default::default()
                },
            ),
        ];

        let mut doc_attrs = Vec::<Attribute>::default();
        let mut derive_attrs = Vec::<Attribute>::default();
        let mut other_attrs = Vec::<Attribute>::default();
        for (ts, expected) in test_cases {
            let mut options = Options::default();

            let attributes = Attribute::parse_outer.parse2(ts.clone()).unwrap();
            for attribute in attributes {
                if let Err(err) = Item::parse_attribute(
                    attribute,
                    &mut options,
                    &mut doc_attrs,
                    &mut derive_attrs,
                    &mut other_attrs,
                ) {
                    let ts = ts.to_string();
                    panic!("failed parsing attribute {ts}, {err}");
                }
            }

            assert_eq!(options.prefix, expected.prefix);
            assert_eq!(options.suffix, expected.suffix);
            assert_eq!(options.root_mod, expected.root_mod);
            assert_eq!(options.values_ident, expected.values_ident);
            assert_eq!(options.prefer_slices, expected.prefer_slices);
            assert_eq!(options.auto_doc, expected.auto_doc);
            assert_eq!(options.cow, expected.cow);

            assert_eq!(
                options
                    .optional
                    .iter()
                    .map(|(path, _)| path)
                    .collect::<Vec<_>>(),
                expected
                    .optional
                    .iter()
                    .map(|(path, _)| path)
                    .collect::<Vec<_>>(),
            );
        }
    }

    #[test]
    fn test_parse_items() {
        let items: Items = parse_quote! {
            #[static_toml()]
            static IMAGES = include_toml!("images.toml");

            #[derive(PartialEq, Eq)]
            #[derive(Default)]
            #[static_toml()]
            pub const CONFIG = include_toml!("config.toml");

            /// Documentation comment
            #[must_use]
            pub(crate) static EXAMPLE = include_toml!("example.toml");

            static BASIC = include_toml!("basic.toml");
        };

        let mut items = items.0.into_iter();

        // we skip checking options here as it doesn't implement PartialEq

        let images = items.next().unwrap();
        assert!(images.doc_attrs.is_empty());
        assert!(images.derive_attrs.is_empty());
        assert!(images.other_attrs.is_empty());
        assert_eq!(images.visibility, None);
        assert!(images.storage_class.is_static());
        assert_eq!(images.name, format_ident!("IMAGES"));
        assert_eq!(images.path.0, PathBuf::from("images.toml"));

        let config = items.next().unwrap();
        assert!(config.doc_attrs.is_empty());
        assert_eq!(config.derive_attrs.len(), 2);
        // it's pretty annoying testing the inner value here exactly, so we just assert the length
        assert!(config.other_attrs.is_empty());
        assert_eq!(config.visibility, Some(parse_quote!(pub)));
        assert!(config.storage_class.is_const());
        assert_eq!(config.name, format_ident!("CONFIG"));
        assert_eq!(config.path.0, PathBuf::from("config.toml"));

        let example = items.next().unwrap();
        assert_eq!(example.doc_attrs.len(), 1);
        assert!(example.doc_attrs[0].path().is_ident("doc"));
        assert_eq!(example.derive_attrs.len(), 0);
        assert_eq!(example.other_attrs.len(), 1);
        assert!(example.other_attrs[0].path().is_ident("must_use"));
        assert_eq!(example.visibility, Some(parse_quote!(pub(crate))));
        assert!(example.storage_class.is_static());
        assert_eq!(example.name, format_ident!("EXAMPLE"));
        assert_eq!(example.path.0, PathBuf::from("example.toml"));

        let basic = items.next().unwrap();
        assert!(basic.doc_attrs.is_empty());
        assert!(basic.derive_attrs.is_empty());
        assert!(basic.other_attrs.is_empty());
        assert_eq!(basic.visibility, None);
        assert!(basic.storage_class.is_static());
        assert_eq!(basic.name, format_ident!("BASIC"));
        assert_eq!(basic.path.0, PathBuf::from("basic.toml"));

        assert!(items.next().is_none());
    }
}
