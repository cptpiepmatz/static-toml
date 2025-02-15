use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};

#[derive(Debug)]
pub enum TypeHint {
    String,
    Integer,
    Float,
    Boolean,
}

impl Parse for TypeHint {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match ident.to_string().to_lowercase().as_str() {
            "s" | "str" | "string" => Ok(TypeHint::String),
            "i" | "i64" | "int" | "integer" => Ok(TypeHint::Integer),
            "f" | "f64" | "float" => Ok(TypeHint::Float),
            "b" | "bool" | "boolean" => Ok(TypeHint::Boolean),
            _ => Err(syn::Error::new(
                ident.span(),
                "unexpected type, use `String`, `Integer`, `Float` or `Boolean`",
            )),
        }
    }
}
