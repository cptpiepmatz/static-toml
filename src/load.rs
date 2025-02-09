use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

pub fn open(path: impl AsRef<Path>) -> io::Result<String> {
    let mut file_path = PathBuf::new();
    file_path.push(env::var("CARGO_MANIFEST_DIR").expect("set by Cargo"));
    file_path.push(path);
    fs::read_to_string(file_path)
}

pub fn load(s: &str) -> Result<toml::Value, toml::de::Error> {
    toml::from_str(s)
}
