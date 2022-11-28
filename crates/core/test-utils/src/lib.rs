mod cli;
mod configs;
mod sandbox;

pub use assert_cmd;
pub use assert_fs;
pub use cli::*;
pub use configs::*;
pub use insta::*;
pub use sandbox::*;

use clean_path::Clean;
use std::path::PathBuf;

pub fn get_fixtures_root() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.push("../../../tests/fixtures");
    root.clean()
}

pub fn get_fixtures_path<T: AsRef<str>>(name: T) -> PathBuf {
    let path = get_fixtures_root().join(name.as_ref());

    if !path.exists() {
        panic!(
            "{}",
            format!("Fixture {} does no exist.", path.to_string_lossy())
        );
    }

    path
}
