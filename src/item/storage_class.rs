use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    Token,
};

/// Storage class for the literal value.
#[derive(Debug)]
pub enum StorageClass {
    Static(Token![static]),
    Const(Token![const]),
}

impl StorageClass {
    pub fn is_static(&self) -> bool {
        match self {
            Self::Static(..) => true,
            _ => false,
        }
    }

    pub fn is_const(&self) -> bool {
        match self {
            Self::Const(..) => true,
            _ => false
        }
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

impl ToTokens for StorageClass {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            StorageClass::Static(inner) => inner.to_tokens(tokens),
            StorageClass::Const(inner) => inner.to_tokens(tokens),
        }
    }
}
