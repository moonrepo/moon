use crate::fs;
use std::env;
use std::path::{Path, PathBuf};

pub fn get_fixtures_dir(dir: &str) -> PathBuf {
    get_fixtures_root().join(dir)
}

pub fn get_fixtures_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../tests/fixtures");

    path.canonicalize().unwrap()
}

// We need to do this so slashes are accurate and always forward
pub fn wrap_glob(path: &Path) -> PathBuf {
    PathBuf::from(fs::normalize_glob(path))
}
