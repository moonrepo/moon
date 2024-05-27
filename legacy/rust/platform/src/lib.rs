mod manifest_hash;
mod rust_platform;
mod target_hash;
mod toolchain_hash;

pub use rust_platform::*;

use starbase_utils::{dirs, fs};
use std::env;
use std::path::{Path, PathBuf};

fn find_cargo_lock(starting_dir: &Path, workspace_root: &Path) -> Option<PathBuf> {
    fs::find_upwards_until("Cargo.lock", starting_dir, workspace_root)
}

fn get_cargo_home() -> PathBuf {
    env::var("CARGO_HOME")
        .map(|p| p.into())
        .unwrap_or_else(|_| dirs::home_dir().unwrap().join(".cargo"))
}
