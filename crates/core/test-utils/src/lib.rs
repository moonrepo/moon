mod cli;
mod sandbox;

pub use cli::*;
pub use sandbox::*;

use clean_path::Clean;
use std::path::PathBuf;

pub fn get_fixtures_root() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.push("../../../tests/fixtures");
    root.clean()
}

pub fn get_fixtures_dir<T: AsRef<str>>(name: T) -> PathBuf {
    get_fixtures_root().join(name.as_ref())
}
