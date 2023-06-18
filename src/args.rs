use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Ident as Ident2, LitBool, LitStr, Token};

pub struct Args {
    pub file_path: LitStr,
    pub named_args: NamedArgs
}

#[derive(Default)]
pub struct NamedArgs {
    pub prefix: Option<Ident2>,
    pub suffix: Option<Ident2>,
    pub entry: Option<Ident2>,
    pub values_ident: Option<Ident2>,
    pub prefer_slices: Option<LitBool>
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let file_path: LitStr = input.parse()?;
        if input.is_empty() {
            return Ok(Args {
                file_path,
                named_args: NamedArgs::default()
            });
        }

        let _: Token![,] = input.parse()?;
        let named_args = NamedArgs::parse(input)?;

        Ok(Args {
            file_path,
            named_args
        })
    }
}

impl Parse for NamedArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut prefix = None;
        let mut suffix = None;
        let mut entry = None;
        let mut values_ident = None;
        let mut prefer_slices = None;

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(Ident2::peek_any) {
                let name: Ident2 = input.parse()?;
                let _: Token![,] = input.parse()?;

                match name.to_string().as_str() {
                    "prefix" => prefix = Some(input.parse()?),
                    "suffix" => suffix = Some(input.parse()?),
                    "entry" => entry = Some(input.parse()?),
                    "values_ident" => values_ident = Some(input.parse()?),
                    "prefer_slices" => prefer_slices = Some(input.parse()?),
                    _ => {
                        return Err(input.error(
                            "expected `prefix`, `suffix`, `entry`, `values_ident` or \
                             `prefer_slices`"
                        ))
                    }
                }
            }
            else {
                return Err(lookahead.error());
            }

            if !input.is_empty() {
                let _: Token![,] = input.parse()?;
            }
        }

        Ok(NamedArgs {
            prefix,
            suffix,
            entry,
            values_ident,
            prefer_slices
        })
    }
}
