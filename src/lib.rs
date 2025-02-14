use error::Error;
use ir::AnnotateIr;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error2::{proc_macro_error, Diagnostic};

mod error;
mod ir;
mod item;
mod load;

#[proc_macro_error]
#[proc_macro]
pub fn static_toml(input: TokenStream) -> TokenStream {
    let input = TokenStream2::from(input);
    match static_toml2(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => Diagnostic::from(err).abort(),
    }
}

fn static_toml2(input: TokenStream2) -> Result<TokenStream2, Error> {
    let items: item::Items = syn::parse2(input)?;
    for item in items {
        let file = load::open(item.path.0).map_err(|err| Error::Io(err, item.path.1))?;
        let file = load::load(&file).map_err(|err| Error::Toml(err, item.path.1))?;
        let annotated = ir::annotate(file, item.path.1)?;
        // do something here
    }
    todo!()
}
