// the example includes an approximate value for pi
#![allow(clippy::approx_constant)]

static_toml::static_toml! {
    /// this is a doc comment
    #[static_toml(
        prefix = Prefix,
        suffix = Suffix,
        root_mod = cfg,
        values_ident = items,
        prefer_slices = false,
        auto_doc = true
    )]
    #[derive(Debug)]
    static EXAMPLE = include_toml!("example.toml");
}

fn main() {
    dbg!(&EXAMPLE);
}
