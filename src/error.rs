use derive_more::From;
use proc_macro2::Span;
use proc_macro_error2::Diagnostic;
use std::io;

use crate::item::{StructuredPath, StructuredPathSegment, TypeHint};

#[derive(Debug, From)]
pub enum Error {
    Syn(syn::Error),

    #[from(skip)]
    Io(io::Error, Span),

    #[from(skip)]
    Toml(toml::de::Error, Span),

    Annotate(AnnotateError),
    Analyze(AnalyzeError),
    Transform(TransformError),
    Codegen(CodegenError),
}

#[derive(Debug)]
pub struct AnnotateError {
    pub kind: AnnotateErrorKind,
    pub path: StructuredPath,
}

#[derive(Debug)]
pub enum AnnotateErrorKind {
    RootNotTable(toml::Value),
    DatetimeFound(toml::value::Datetime),
}

#[derive(Debug)]
pub enum AnalyzeError {
    EmptyStructuredPath(StructuredPath),
    KeyOfPrimitive(StructuredPath, StructuredPathSegment),
    // the segment could not be found
    OptionalSegmentNotFound(StructuredPath, StructuredPathSegment),
    // found segment but the structured path segment doesn't match to the type
    UnmatchedField(StructuredPath, StructuredPathSegment),
    WildcardNotAtEnd(StructuredPath, StructuredPath),
    TypeUnionConflict(StructuredPath, StructuredPath),
}

#[derive(Debug)]
pub enum TransformError {}

#[derive(Debug)]
pub enum CodegenError {}

impl From<Error> for Diagnostic {
    fn from(value: Error) -> Self {
        todo!()
    }
}
