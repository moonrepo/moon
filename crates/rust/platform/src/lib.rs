mod bins_hasher;
mod manifest_hasher;
mod rust_platform;
mod target_hasher;

pub use rust_platform::*;

use moon_rust_lang::CARGO;
use starbase_utils::fs;
use std::path::{Path, PathBuf};

fn find_cargo_lock(starting_dir: &Path) -> Option<PathBuf> {
    fs::find_upwards(CARGO.lockfile, starting_dir)
}
