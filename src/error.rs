use std::io;
use toml_edit::TomlError;

pub enum Error {
    Parse(ParseError),
    Validate(),
    Transform(),
    Codegen(),
}

pub enum ParseError {
    Syn(syn::Error),
    Io(io::Error),
    Toml(TomlError),
}
