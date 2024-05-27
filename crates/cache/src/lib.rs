mod cache_engine;
mod hash_engine;
mod state_engine;

pub use cache_engine::*;
pub use hash_engine::*;
pub use moon_cache_item::*;
pub use state_engine::*;

use starbase_utils::fs::RemoveDirContentsResult;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub(crate) fn resolve_path(base_dir: &Path, path: impl AsRef<OsStr>) -> PathBuf {
    let path = PathBuf::from(path.as_ref());

    let mut path = if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    };

    path.set_extension("json");
    path
}

pub(crate) fn merge_clean_results(
    mut left: RemoveDirContentsResult,
    right: RemoveDirContentsResult,
) -> RemoveDirContentsResult {
    left.bytes_saved += right.bytes_saved;
    left.files_deleted += right.files_deleted;
    left
}
