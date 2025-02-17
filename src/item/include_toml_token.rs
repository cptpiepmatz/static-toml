use syn::{
    parse::{Parse, ParseStream},
    Ident,
};

const EXPECTED_INCLUDE_TOML: &str = "expected `include_toml`";

/// A token representing the 'include_toml' keyword.
pub struct IncludeTomlToken;

/// Parse implementation for `IncludeTomlToken`.
///
/// Ensures that the token is the 'include_toml' keyword.
impl Parse for IncludeTomlToken {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse the token and ensure it matches 'include_toml'.
        let include_toml: Ident =
            input.parse().map_err(|e| syn::Error::new(e.span(), EXPECTED_INCLUDE_TOML))?;
        if include_toml != "include_toml" {
            return Err(syn::Error::new_spanned(include_toml, EXPECTED_INCLUDE_TOML));
        }

        Ok(IncludeTomlToken)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn parse_include_toml_token() {
        let input = quote!(include_toml);
        assert!(syn::parse2::<IncludeTomlToken>(input).is_ok());

        let input = quote!(include_json);
        match syn::parse2::<IncludeTomlToken>(input) {
            Err(e) => assert_eq!(e.to_string(), EXPECTED_INCLUDE_TOML),
            Ok(_) => panic!("should be error variant"),
        }
    }
}
