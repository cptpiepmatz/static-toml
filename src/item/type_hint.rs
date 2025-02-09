use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};

#[derive(Debug)]
pub enum TypeHint {
    String,
}

impl Parse for TypeHint {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match ident.to_string().to_lowercase().as_str() {
            "str" | "string" => Ok(TypeHint::String),
            _ => Err(syn::Error::new(
                ident.span(),
                "unexpected type, use `String` or ...",
            )),
        }
    }
}
