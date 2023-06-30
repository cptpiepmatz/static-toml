use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Error, Ident as Ident2, LitBool, LitStr, Token, Visibility};

pub struct StaticToml(pub Vec<StaticTomlItem>);

pub struct StaticTomlItem {
    pub attrs: StaticTomlAttributes,
    pub other_attrs: Vec<Attribute>,
    pub derive: Vec<Attribute>,
    pub visibility: Option<Visibility>,
    pub name: Ident2,
    pub path: LitStr
}

#[derive(Default)]
pub struct StaticTomlAttributes {
    pub prefix: Option<Ident2>,
    pub suffix: Option<Ident2>,
    pub root_mod: Option<Ident2>,
    pub values_ident: Option<Ident2>,
    pub prefer_slices: Option<LitBool>
}

struct IncludeTomlToken;

impl Parse for StaticToml {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = vec![];

        while !input.is_empty() {
            items.push(input.parse()?);
        }

        Ok(Self(items))
    }
}

impl Parse for StaticTomlItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let all_attrs = match input.peek(Token![#]) {
            false => None,
            true => Some(input.call(Attribute::parse_outer)?)
        };

        let mut attrs = StaticTomlAttributes::default();
        let mut other_attrs = Vec::new();
        let mut derive = Vec::new();
        if let Some(all_attrs) = all_attrs {
            for attr in all_attrs {
                if attr.path().is_ident("derive") {
                    derive.push(attr);
                    continue;
                }

                if !attr.path().is_ident("static_toml") {
                    other_attrs.push(attr);
                    continue;
                }

                attr.parse_nested_meta(|meta| {
                    let Some(key) = meta.path.get_ident() else {
                        return Ok(())
                    };

                    match key.to_string().as_str() {
                        "prefix" => attrs.prefix = Some(meta.value()?.parse()?),
                        "suffix" => attrs.suffix = Some(meta.value()?.parse()?),
                        "root_mod" => attrs.root_mod = Some(meta.value()?.parse()?),
                        "values_ident" => attrs.values_ident = Some(meta.value()?.parse()?),
                        "prefer_slices" => attrs.prefer_slices = Some(meta.value()?.parse()?),
                        _ => {
                            return Err(meta.error(
                                "unexpected attribute, expected one of `prefix`, `suffix`, \
                                 `root_mod`, `values_ident` or `prefer_slices`"
                            ))
                        }
                    }

                    Ok(())
                })?;
            }
        }

        let visibility = match input.peek(Token![pub]) {
            false => None,
            true => Some(input.parse()?)
        };

        input.parse::<Token![static]>()?;
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
            derive,
            visibility,
            name,
            path
        })
    }
}

const EXPECTED_INCLUDE_TOML: &str = "expected `include_toml`";

impl Parse for IncludeTomlToken {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let include_toml: Ident2 = input
            .parse()
            .map_err(|e| syn::Error::new(e.span(), EXPECTED_INCLUDE_TOML))?;
        if include_toml.to_string() != "include_toml" {
            return Err(Error::new_spanned(include_toml, EXPECTED_INCLUDE_TOML));
        }

        Ok(IncludeTomlToken)
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span as Span2;
    use quote::{format_ident, quote, ToTokens};
    use syn::{parse_quote, LitBool, Token, Visibility};

    use crate::parse::{IncludeTomlToken, StaticToml, EXPECTED_INCLUDE_TOML};

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
            pub static CONFIG = include_toml!("config.toml");

            /// Documentation comment
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
        assert_eq!(config.name, format_ident!("CONFIG"));
        assert_eq!(config.path.value().as_str(), "config.toml");

        let example = items.next().unwrap();
        assert!(example.attrs.prefix.is_none());
        assert!(example.attrs.suffix.is_none());
        assert!(example.attrs.root_mod.is_none());
        assert!(example.attrs.values_ident.is_none());
        assert!(example.attrs.prefer_slices.is_none());
        assert_eq!(
            example.other_attrs[0].path().get_ident(),
            Some(&format_ident!("doc"))
        );
        assert_eq!(example.other_attrs.len(), 1);
        let Some(Visibility::Restricted(_)) = example.visibility else {
            panic!("not a restricted visibility");
        };
        assert!(example.derive.is_empty());
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
        assert_eq!(basic.name, format_ident!("BASIC"));
        assert_eq!(basic.path.value().as_str(), "basic.toml");
    }
}
