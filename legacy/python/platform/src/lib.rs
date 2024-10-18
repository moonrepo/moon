pub mod actions;
mod manifest_hash;
mod python_platform;
mod target_hash;
mod toolchain_hash;

pub use python_platform::*;

use starbase_utils::fs;
use std::path::{Path, PathBuf};

fn find_requirements_txt(starting_dir: &Path, workspace_root: &Path) -> Option<PathBuf> {
    fs::find_upwards_until("requirements.txt", starting_dir, workspace_root)
}
