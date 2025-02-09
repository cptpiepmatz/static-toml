use super::{TypeHint, StructuredPath};
use syn::{Ident, LitBool};

/// Configuration options for the `static_toml` macro.
#[derive(Debug, Default)]
pub struct Options {
    pub prefix: Option<Ident>,
    pub suffix: Option<Ident>,
    pub root_mod: Option<Ident>,
    pub values_ident: Option<Ident>,
    pub prefer_slices: Option<LitBool>,
    pub auto_doc: Option<LitBool>,
    pub cow: Option<LitBool>,
    pub optional: Vec<(StructuredPath, Option<TypeHint>)>,
}
