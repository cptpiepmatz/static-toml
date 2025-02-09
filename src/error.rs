use std::io;
use toml_edit::TomlError;

pub enum Error {
    Syn(syn::Error),
    Io(io::Error),
    Toml(TomlError),

    Parse(ParseError),
    Analyze(AnalyzeError),
    Transform(TransformError),
    Codegen(CodegenError),
}

pub enum ParseError {}

pub enum AnalyzeError {}

pub enum TransformError {}

pub enum CodegenError {}
