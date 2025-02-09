use proc_macro2::Span;
use proc_macro_error2::Diagnostic;
use std::io;

pub enum Error {
    Syn(syn::Error),
    Io(io::Error, Span),
    Toml(toml::de::Error, Span),

    Parse(ParseError),
    Analyze(AnalyzeError),
    Transform(TransformError),
    Codegen(CodegenError),
}

pub enum ParseError {}

pub enum AnalyzeError {}

pub enum TransformError {}

pub enum CodegenError {}

impl From<Error> for Diagnostic {
    fn from(value: Error) -> Self {
        todo!()
    }
}
