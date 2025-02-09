use std::ops::{Bound, RangeBounds};
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    token, Ident, LitInt, LitStr, Token,
};

#[derive(Debug, PartialEq, Eq)]
pub struct StructuredPath(pub Vec<StructuredPathSegment>);

#[derive(Debug, PartialEq, Eq)]
pub enum StructuredPathSegment {
    Key(String),
    Index(Bound<usize>, Bound<usize>),
    Wildcard,
}

impl StructuredPathSegment {
    pub fn key(key: impl ToString) -> Self {
        Self::Key(key.to_string())
    }

    pub fn index(range: impl RangeBounds<usize>) -> Self {
        Self::Index(range.start_bound().cloned(), range.end_bound().cloned())
    }

    pub fn wildcard() -> Self {
        Self::Wildcard
    }
}

impl Parse for StructuredPath {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut segments = Vec::new();

        while !input.is_empty() {
            segments.push(input.parse()?);

            if input.peek(Token![..]) {
                return Err(
                    input.error("`..` is only allowed inside brackets `[...]` for indexing ranges")
                );
            }

            if input.peek(Token![.]) {
                input.parse::<Token![.]>()?;
            }
        }

        Ok(Self(segments))
    }
}

impl Parse for StructuredPathSegment {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        match input.peek(token::Bracket) {
            true => Self::parse_bracketed(&input),
            false => Self::parse_simple(&input),
        }
    }
}

impl StructuredPathSegment {
    fn parse_simple(input: ParseStream) -> syn::Result<Self> {
        // handle: input.key
        if input.peek(Ident) {
            let ident: Ident = input.parse()?;
            return Ok(Self::key(ident));
        }

        // handle: input."key"
        if input.peek(LitStr) {
            let lit: LitStr = input.parse()?;
            return Ok(Self::key(lit.value()));
        }

        // handle: input.0
        if input.peek(LitInt) {
            let lit: LitInt = input.parse()?;
            check_lit_int_suffix(&lit)?;
            let value: usize = lit.base10_parse()?;
            let range = value..=value;
            return Ok(Self::index(range));
        }

        // handle: input.*
        if input.peek(Token![*]) {
            let _: Token![*] = input.parse()?;
            return Ok(Self::wildcard());
        }

        Err(input.error("expected an identifier, string literal, integer index, or wildcard `*`"))
    }

    fn parse_bracketed(input: ParseStream) -> syn::Result<Self> {
        let delimited;
        bracketed!(delimited in input);

        let check_empty = |msg| match delimited.is_empty() {
            true => Ok(()),
            false => Err(delimited.error(msg)),
        };

        // handle: []
        if delimited.is_empty() {
            return Ok(Self::index(..));
        }

        // handle: [*]
        if delimited.peek(Token![*]) {
            let _: Token![*] = delimited.parse()?;
            check_empty("unexpected token after `*`, expected `]`")?;
            return Ok(Self::index(..));
        }

        // handle: ["key"]
        if delimited.peek(LitStr) {
            let lit: LitStr = delimited.parse()?;
            check_empty("unexpected token after string literal, expected `]`")?;
            return Ok(Self::key(lit.value()));
        }

        // handle: [0] | [0..] | [0..1] | [0..=1]
        if delimited.peek(LitInt) {
            let start: LitInt = delimited.parse()?;
            check_lit_int_suffix(&start)?;
            let start_value: usize = dbg!(start.base10_parse())?;

            if delimited.is_empty() {
                return Ok(Self::index(start_value..=start_value));
            }

            if delimited.peek(Token![..=]) {
                let _: Token![..=] = delimited.parse()?;
                let end: LitInt = delimited.parse()?;
                check_lit_int_suffix(&end)?;
                let end_value: usize = end.base10_parse()?;

                if start_value > end_value {
                    return Err(syn::Error::new(
                        end.span(),
                        "inclusive range start must not be greater than end",
                    ));
                }

                check_empty("unexpected token after range, expected `]`")?;
                return Ok(Self::index(start_value..=end_value));
            }

            if delimited.peek(Token![..]) {
                let _: Token![..] = delimited.parse()?;

                if delimited.is_empty() {
                    return Ok(Self::index(start_value..));
                }

                let end: LitInt = delimited.parse()?;
                check_lit_int_suffix(&end)?;
                let end_value: usize = end.base10_parse()?;

                if start_value > end_value {
                    return Err(syn::Error::new(
                        end.span(),
                        "range start must not be greater than range end",
                    ));
                }

                check_empty("unexpected token after range, expected `]`")?;
                return Ok(Self::index(start_value..end_value));
            }

            return Err(
                input.error("unexpected token in index brackets, expected `]`, `..`, or `..=`")
            );
        }

        // handle: [..] | [..1]
        if delimited.peek(Token![..]) {
            let _: Token![..] = delimited.parse()?;

            if delimited.is_empty() {
                return Ok(Self::index(..));
            }

            let end: LitInt = delimited.parse()?;
            check_lit_int_suffix(&end)?;
            let end_value: usize = end.base10_parse()?;
            check_empty("unexpected token after range, expected `]`")?;
            return Ok(Self::index(..end_value));
        }

        // handle: [..=1]
        if delimited.peek(Token![..=]) {
            let _: Token![..=] = delimited.parse()?;
            let end: LitInt = delimited.parse()?;
            check_lit_int_suffix(&end)?;
            let end: usize = end.base10_parse()?;
            check_empty("unexpected token after inclusive range, expected `]`")?;
            return Ok(Self::index(..=end));
        }

        Err(input
            .error("unexpected token in brackets, expected integer, range, wildcard, or string"))
    }
}

fn check_lit_int_suffix(lit: &LitInt) -> syn::Result<()> {
    match lit.suffix() {
        "" => Ok(()),
        suffix => Err(syn::Error::new(
            lit.span(),
            format!("invalid numeric literal `{lit}`, unexpected suffix {suffix:?}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::ops::RangeBounds;

    use proc_macro2::{TokenStream as TokenStream2, TokenTree as TokenTree2};
    use quote::quote;
    use syn::spanned::Spanned;

    use crate::options::{StructuredPath, StructuredPathSegment};

    #[test]
    fn parse_structured_path() {
        fn k(key: impl ToString) -> StructuredPathSegment {
            StructuredPathSegment::key(key)
        }

        fn i(range: impl RangeBounds<usize>) -> StructuredPathSegment {
            StructuredPathSegment::index(range)
        }

        fn w() -> StructuredPathSegment {
            StructuredPathSegment::wildcard()
        }

        fn p(segments: impl IntoIterator<Item = StructuredPathSegment>) -> StructuredPath {
            StructuredPath(segments.into_iter().collect())
        }

        #[rustfmt::skip]
        let test_cases: Vec<(TokenStream2, StructuredPath)> = vec![
            (
                quote!(*),
                p([w()]),
            ),
            (
                quote!(array[1..=5]),
                p([k("array"), i(1..=5)]),
            ),
            (
                quote!(array[3]),
                p([k("array"), i(3..=3)]),
            ),
            (
                quote!(array[5..]),
                p([k("array"), i(5..)]),
            ),
            (
                quote!(array[..5]),
                p([k("array"), i(..5)]),
            ),
            (
                quote!(data["escaped\\\"key"]),
                p([k("data"), k("escaped\\\"key")]),
            ),
            (
                quote!(identifier),
                p([k("identifier")]),
            ),
            (
                quote!(items[*].id),
                p([k("items"), i(..), k("id")]),
            ),
            (
                quote!(items.1.id),
                p([k("items"), i(1..=1), k("id")]),
            ),
            (
                quote!(items[1..4].id),
                p([k("items"), i(1..4), k("id")]),
            ),
            (
                quote!(items[2].details),
                p([k("items"), i(2..=2), k("details")]),
            ),
            (
                quote!(items[].name), 
                p([k("items"), i(..), k("name")]),
            ),
            (
                quote!(logs[].*), 
                p([k("logs"), i(..), w()]),
            ),
            (
                quote!(nested["one"]["two"]),
                p([k("nested"), k("one"), k("two")]),
            ),
            (
                quote!(numbers["1234"]),
                p([k("numbers"), k("1234")]),
            ),
            (
                quote!(settings.debug_mode),
                p([k("settings"), k("debug_mode")]),
            ),
            (
                quote!(some["bracketed"].details),
                p([k("some"), k("bracketed"), k("details")]),
            ),
            (
                quote!(some."non-rust-identifier".details),
                p([k("some"), k("non-rust-identifier"), k("details")]),
            ),
            (
                quote!(users[].profile.details.nickname),
                p([k("users"), i(..), k("profile"), k("details"), k("nickname")]),
            ),            
        ];

        for (ts, expected) in test_cases {
            let input = ts.to_string();
            let parsed = match syn::parse2::<StructuredPath>(ts) {
                Ok(parsed) => parsed,
                Err(err) => panic!("could not parse {input:?}, {err}"),
            };
            assert_eq!(parsed, expected);
        }
    }

    #[test]
    fn error_on_invalid_structured_path() {
        fn find_tt(tokens: TokenStream2, expected: &str) -> Option<Box<dyn Spanned>> {
            tokens.into_iter().find_map(|tt| {
                match tt {
                    TokenTree2::Group(group) => {
                        if let Some(tt) = find_tt(group.stream(), expected) {
                            return Some(tt);
                        }
                    }
                    TokenTree2::Ident(ident) => {
                        if ident.to_string().as_str() == expected {
                            return Some(Box::new(ident) as Box<dyn Spanned>);
                        }
                    }
                    TokenTree2::Punct(punct) => {
                        if punct.as_char().to_string().as_str() == expected {
                            return Some(Box::new(punct) as Box<dyn Spanned>);
                        }
                    }
                    TokenTree2::Literal(literal) => {
                        if literal.to_string().as_str() == expected {
                            return Some(Box::new(literal) as Box<dyn Spanned>);
                        }
                    }
                }

                None
            })
        }

        #[rustfmt::skip]
        let error_cases: Vec<(&str, TokenStream2, &str)> = vec![
            (
                "hyphen in path", 
                quote!(some.very - invalid.key), 
                "-",
            ),
            (
                "double dot without key", 
                quote!(some..other), 
                ".",
            ),
            (
                "triple dot instead of range", 
                quote!(items[1...5]), 
                ".",
            ),
            (
                "invalid range end", 
                quote!(items[1..foo]), 
                "foo",
            ),
            (
                "non-numeric range end",
                quote!(array[2..bar]),
                "bar",
            ),
            (
                "invalid inclusive range end",
                quote!(array[1..=]),
                "=",
            ),
            (
                "invalid range start",
                quote!(array[foo..5]),
                "foo",
            ),
            (
                "unexpected ellipsis",
                quote!(items[1...=5]),
                ".",
            ),
            (
                "range start greater than end",
                quote!(items[5..2]),
                "5",
            ),
            (
                "range start greater than end (inclusive)",
                quote!(items[5..=2]),
                "5",
            ),
            (
                "double bracketed identifiers",
                quote!(data["one"]["two"]["three" "four"]),
                "\"four\"",
            ),
            (
                "invalid key as suffixed number without quotes",
                quote!(data[123abc]),
                "123abc",
            ),
        ];

        for (desc, ts, expected_punct) in error_cases {
            let tokens = ts.to_string();
            let tt = match find_tt(ts.clone(), expected_punct) {
                Some(tt) => tt,
                None => panic!("expected token tree {expected_punct:?} not found in {tokens}"),
            };

            let err = match syn::parse2::<StructuredPath>(ts) {
                Err(err) => err,
                Ok(_) => panic!("parsed {tokens:?} as valid"),
            };

            // ensure the span is at the expected position
            assert_eq!(
                (tt.span().start(), tt.span().end()),
                (err.span().start(), err.span().end()),
                "span mismatch for test case: {desc}"
            );
        }
    }
}
