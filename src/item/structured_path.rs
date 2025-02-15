use proc_macro2::Span;
use std::{
    fmt::{self, Display, Formatter},
    ops::{Bound, RangeBounds},
};
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    token, Ident, LitInt, LitStr, Token,
};

/// Represents a structured path composed of multiple segments.
///
/// This is used to describe paths into a TOML structure, allowing selection
/// through nested keys, indexed arrays, and wildcard selections.
///
/// # Examples
///
/// - `server.address` → `StructuredPath([Key("server"), Key("address")])`
/// - `database.tables[0]` → `StructuredPath([Key("database"), Key("tables"), Index(0..=0)])`
/// - `users[].name` → `StructuredPath([Key("users"), Index(..), Key("name")])`
/// - `users.details.*` → `StructuredPath([Key("users"), Key("details"), Wildcard])`
#[derive(Debug, Clone)]
pub struct StructuredPath {
    pub segments: Vec<StructuredPathSegment>,
    pub span: Span,
}

/// Defines a segment within a structured path.
#[derive(Debug, Clone)]
pub enum StructuredPathSegment {
    /// A named key segment.
    Key(String, Option<Span>),
    /// An index range segment.
    Index(Bound<usize>, Bound<usize>, Option<Span>),
    /// A wildcard segment matching any key element.
    ///
    /// The wildcard doesn't match against [`Index`](Self::Index).
    Wildcard(Option<Span>),
}

impl StructuredPathSegment {
    /// Creates a key segment from a string-like value.
    pub fn key(key: impl ToString, span: impl Into<Option<Span>>) -> Self {
        Self::Key(key.to_string(), span.into())
    }

    /// Creates an index segment from a range.
    pub fn index(range: impl RangeBounds<usize>, span: impl Into<Option<Span>>) -> Self {
        Self::Index(
            range.start_bound().cloned(),
            range.end_bound().cloned(),
            span.into(),
        )
    }

    /// Creates a wildcard segment.
    pub fn wildcard(span: impl Into<Option<Span>>) -> Self {
        Self::Wildcard(span.into())
    }
}

impl StructuredPath {
    pub fn new(span: Span) -> Self {
        Self {
            segments: Vec::new(),
            span,
        }
    }

    /// Check if another structured path is contained in this one.
    ///
    /// # Attention
    /// A structured path is not recursive, so `user.details.name` is not contained in
    /// `user.details`, it would require `user.details.*` to make that true.
    pub fn contains(&self, other: &Self) -> bool {
        // paths aren't recursive, so the length must match
        if self.segments.len() != other.segments.len() {
            return false;
        }

        let mut self_iter = self.segments.iter();
        let mut other_iter = other.segments.iter();

        while let (Some(self_segment), Some(other_segment)) = (self_iter.next(), other_iter.next())
        {
            if !self_segment.contains(other_segment) {
                return false;
            }
        }

        true
    }

    /// Returns a new `StructuredPath` with an additional segment appended.
    pub fn with_segment(&self, segment: StructuredPathSegment) -> Self {
        let mut segments = Vec::with_capacity(self.segments.len() + 1);
        segments.extend(self.segments.iter().cloned());
        segments.push(segment);
        Self {
            segments,
            span: self.span,
        }
    }
}

impl StructuredPathSegment {
    pub fn contains(&self, other: &Self) -> bool {
        use StructuredPathSegment as SPS;

        match (self, other) {
            (SPS::Key(this, _), SPS::Key(that, _)) if this == that => true,
            (SPS::Wildcard(_), SPS::Key(_, _)) => true,
            (SPS::Wildcard(_), SPS::Wildcard(_)) => true,
            (SPS::Index(Bound::Unbounded, Bound::Unbounded, _), SPS::Index(..)) => true,
            (this @ SPS::Index(..), that @ SPS::Index(..)) => {
                let this = this.as_range().expect("this is index");
                let that = that.as_range().expect("that is index");
                is_fully_contained(that, this)
            }
            _ => false,
        }
    }

    pub fn as_range(&self) -> Option<impl RangeBounds<usize> + use<'_>> {
        let Self::Index(start, end, _) = self else {
            return None;
        };
        Some((start.as_ref(), end.as_ref()))
    }
}

fn is_fully_contained<RB: RangeBounds<usize>>(inner: RB, outer: RB) -> bool {
    let start_bound = |bounds: &RB| match bounds.start_bound() {
        Bound::Included(val) => *val,
        Bound::Excluded(val) => val + 1,
        Bound::Unbounded => usize::MIN,
    };

    let end_bound = |bounds: &RB| match bounds.end_bound() {
        Bound::Included(val) => *val,
        Bound::Excluded(val) => val - 1,
        Bound::Unbounded => usize::MAX,
    };

    let inner_start = start_bound(&inner);
    let inner_end = end_bound(&inner);
    let outer_start = start_bound(&outer);
    let outer_end = end_bound(&outer);

    (outer_start <= inner_start) && (outer_end >= inner_end)
}

impl Eq for StructuredPathSegment {}
impl PartialEq for StructuredPathSegment {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Key(l, _), Self::Key(r, _)) => l == r,
            (Self::Index(l0, l1, _), Self::Index(r0, r1, _)) => l0 == r0 && l1 == r1,
            (Self::Wildcard(_), Self::Wildcard(_)) => true,
            _ => false,
        }
    }
}

impl Parse for StructuredPath {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut segments = Vec::new();
        let span = input.span();

        while !input.is_empty() && !input.peek(Token![,]) {
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

        Ok(Self { segments, span })
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
            return Ok(Self::key(ident.to_string(), ident.span()));
        }

        // handle: input."key"
        if input.peek(LitStr) {
            let lit: LitStr = input.parse()?;
            return Ok(Self::key(lit.value(), lit.span()));
        }

        // handle: input.0
        if input.peek(LitInt) {
            let lit: LitInt = input.parse()?;
            check_lit_int_suffix(&lit)?;
            let value: usize = lit.base10_parse()?;
            let range = value..=value;
            return Ok(Self::index(range, lit.span()));
        }

        // handle: input.*
        if input.peek(Token![*]) {
            let star: Token![*] = input.parse()?;
            return Ok(Self::wildcard(star.span));
        }

        Err(input.error("expected an identifier, string literal, integer index, or wildcard `*`"))
    }

    fn parse_bracketed(input: ParseStream) -> syn::Result<Self> {
        let delimited;
        let bracket = bracketed!(delimited in input);
        let span = bracket.span.join();

        let check_empty = |msg| match delimited.is_empty() {
            true => Ok(()),
            false => Err(delimited.error(msg)),
        };

        // handle: []
        if delimited.is_empty() {
            return Ok(Self::index(.., span));
        }

        // handle: [*]
        if delimited.peek(Token![*]) {
            let _: Token![*] = delimited.parse()?;
            check_empty("unexpected token after `*`, expected `]`")?;
            return Ok(Self::index(.., span));
        }

        // handle: ["key"]
        if delimited.peek(LitStr) {
            let lit: LitStr = delimited.parse()?;
            check_empty("unexpected token after string literal, expected `]`")?;
            return Ok(Self::key(lit.value(), span));
        }

        // handle: [0] | [0..] | [0..1] | [0..=1]
        if delimited.peek(LitInt) {
            let start: LitInt = delimited.parse()?;
            check_lit_int_suffix(&start)?;
            let start_value: usize = start.base10_parse()?;

            if delimited.is_empty() {
                return Ok(Self::index(start_value..=start_value, span));
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
                return Ok(Self::index(start_value..=end_value, span));
            }

            if delimited.peek(Token![..]) {
                let _: Token![..] = delimited.parse()?;

                if delimited.is_empty() {
                    return Ok(Self::index(start_value.., span));
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
                return Ok(Self::index(start_value..end_value, span));
            }

            return Err(
                input.error("unexpected token in index brackets, expected `]`, `..`, or `..=`")
            );
        }

        // handle: [..=1]
        if delimited.peek(Token![..=]) {
            let _: Token![..=] = delimited.parse()?;
            let end: LitInt = delimited.parse()?;
            check_lit_int_suffix(&end)?;
            let end: usize = end.base10_parse()?;
            check_empty("unexpected token after inclusive range, expected `]`")?;
            return Ok(Self::index(..=end, span));
        }

        // handle: [..] | [..1]
        if delimited.peek(Token![..]) {
            let _: Token![..] = delimited.parse()?;

            if delimited.is_empty() {
                return Ok(Self::index(.., span));
            }

            let end: LitInt = delimited.parse()?;
            check_lit_int_suffix(&end)?;
            let end_value: usize = end.base10_parse()?;
            check_empty("unexpected token after range, expected `]`")?;
            return Ok(Self::index(..end_value, span));
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

impl Eq for StructuredPath {}
impl PartialEq for StructuredPath {
    fn eq(&self, other: &Self) -> bool {
        self.segments == other.segments
    }
}

impl Display for StructuredPathSegment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Key(key, _) => write!(f, "{key}"),
            Self::Wildcard(span) => write!(f, "*"),
            Self::Index(Bound::Included(lower), Bound::Included(upper), _) if lower == upper => {
                write!(f, "[{lower}]")
            }
            Self::Index(lower, upper, _) => {
                write!(f, "[")?;
                if let Bound::Included(lower) = lower {
                    write!(f, "{lower}")?;
                }
                write!(f, "..")?;
                match upper {
                    Bound::Included(upper) => write!(f, "={upper}")?,
                    Bound::Excluded(upper) => write!(f, "{upper}")?,
                    Bound::Unbounded => {}
                }
                write!(f, "]")?;
                Ok(())
            }
        }
    }
}

impl Display for StructuredPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut iter = self.segments.iter();

        // write first segment
        if let Some(next) = iter.next() {
            write!(f, "{next}")?;
        }

        // write following segments
        for segment in iter {
            if matches!(
                segment,
                StructuredPathSegment::Key(..) | StructuredPathSegment::Wildcard(..)
            ) {
                write!(f, ".")?;
            }

            write!(f, "{segment}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::{TokenStream as TokenStream2, TokenTree as TokenTree2};
    use quote::quote;
    use std::ops::RangeBounds;
    use syn::{parse_quote, spanned::Spanned};

    #[test]
    fn parse_structured_path() {
        fn k(key: impl ToString) -> StructuredPathSegment {
            StructuredPathSegment::key(key, None)
        }

        fn i(range: impl RangeBounds<usize>) -> StructuredPathSegment {
            StructuredPathSegment::index(range, None)
        }

        fn w() -> StructuredPathSegment {
            StructuredPathSegment::wildcard(None)
        }

        fn p(segments: impl IntoIterator<Item = StructuredPathSegment>) -> StructuredPath {
            StructuredPath {
                segments: segments.into_iter().collect(),
                span: Span::call_site(),
            }
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
                quote!(array[..=5]),
                p([k("array"), i(..=5)]),
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
                quote!(nested.arrays[0][0]),
                p([k("nested"), k("arrays"), i(0..=0), i(0..=0)]),
            ),
            (
                quote!(nested.arrays.[0].0),
                p([k("nested"), k("arrays"), i(0..=0), i(0..=0)]),
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

    #[test]
    fn test_structured_path_contains() {
        #[rustfmt::skip]
        let cases = vec![
            (
                quote!(server.address),
                quote!(server.address),
                true,
            ),
            (
                quote!(database.tables[0]),
                quote!(database.tables[0]),
                true,
            ),
            (
                quote!(users[].name),
                quote!(users[0].name),
                true,
            ),
            (
                quote!(users.details.*),
                quote!(users.details.address),
                true,
            ),
            (
                quote!(server.address),
                quote!(server.port),
                false,
            ),
            (
                quote!(database.tables[0]),
                quote!(database.tables[1]),
                false,
            ),
            (
                quote!(users[].name),
                quote!(users.details.name),
                false,
            ),
            (
                quote!(users.details.*),
                quote!(users.info.address),
                false,
            ),
            (
                quote!(logs[].*),
                quote!(logs[1].error),
                true,
            ),
            (
                quote!(logs[].*),
                quote!(logs[].debug),
                true,
            ),
            (
                quote!(items[1..4].id),
                quote!(items[2].id),
                true,
            ),
            (
                quote!(items[1..4].id),
                quote!(items[4].id),
                false,
            ),
        ];

        for (path, other, expected) in cases {
            let path: StructuredPath = syn::parse2(path.clone())
                .expect(&format!("failed to parse path {:?}", path.to_string()));

            let other: StructuredPath = syn::parse2(other.clone()).expect(&format!(
                "failed to parse other path {:?}",
                other.to_string()
            ));

            assert_eq!(
                path.contains(&other),
                expected,
                "{:?}.contains({:?}) expected {:?}",
                path,
                other,
                expected
            );
        }
    }

    #[test]
    fn test_display_impl() {
        macro_rules! assert_display {
            ($display:literal, $($path:tt)*) => {
                let path: StructuredPath = parse_quote!($($path)*);
                assert_eq!(path.to_string().as_str(), $display);
            };
        }

        assert_display!("x", x);
        assert_display!("foo.bar", foo.bar);
        assert_display!("config.settings.option", config.settings.option);
        assert_display!("foo.bar_42", foo.bar_42);
        assert_display!("version_1[0].data", version_1.0.data);
        assert_display!("snake_case.path_here", snake_case.path_here);
        assert_display!("camelCase.mixedUP", camelCase.mixedUP);
        assert_display!("UPPER_CASE.HELLO_WORLD", UPPER_CASE.HELLO_WORLD);
        assert_display!("data.items[0]", data.items[0]);
        assert_display!("config.list[42].property", config.list[42].property);
        assert_display!("single", single);
        assert_display!("only_one", only_one);
        assert_display!("data[..]", data[]);
        assert_display!("nested.items[..].name", nested.items[*].name);
        assert_display!("data.weird-key", data."weird-key");
        assert_display!("nested.123abc.property", nested."123abc".property);
    }
}
