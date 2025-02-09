use std::{
    iter::Peekable,
    ops::{Bound, RangeBounds},
    str::FromStr,
};

use proc_macro2::Span as Span2;
use syn::spanned::Spanned;

mod structured_path;
pub use structured_path::*;

#[derive(Debug)]
pub struct Options {
    prefix: Option<syn::Ident>,
    suffix: Option<syn::Ident>,
    root_mod: Option<syn::Ident>,
    values_ident: Option<syn::Ident>,
    prefer_slices: Option<bool>,
    auto_doc: Option<bool>,
    cow: Option<bool>,
    optional: Vec<(StructuredPath, Span2)>,
}
