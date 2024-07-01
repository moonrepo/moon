mod manifest_hash;
mod rust_platform;
mod target_hash;
mod toolchain_hash;

pub use rust_platform::*;

use moon_common::get_resolved_env_home;
use starbase_utils::fs;
use std::path::{Path, PathBuf};

fn find_cargo_lock(starting_dir: &Path, workspace_root: &Path) -> Option<PathBuf> {
    fs::find_upwards_until("Cargo.lock", starting_dir, workspace_root)
}

fn get_cargo_home() -> PathBuf {
    get_resolved_env_home("CARGO_HOME", |home_dir| home_dir.join(".cargo"))
}
