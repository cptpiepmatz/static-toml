static_toml::static_toml! {
    /// this is a doc comment
    #[derive(Debug)]
    static EXAMPLE = include_toml!("example.toml");
}

fn main() {
    dbg!(&EXAMPLE);
}
