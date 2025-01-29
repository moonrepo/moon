mod bun_lock;
mod bun_lockb;

pub use bun_lock::*;
pub use bun_lockb::*;
pub use moon_lang::LockfileDependencyVersions;

use cached::proc_macro::cached;
use std::path::PathBuf;
use std::sync::Arc;

#[cached(result)]
pub fn load_lockfile_dependencies(
    lockfile_text: Arc<String>,
    path: PathBuf,
) -> miette::Result<LockfileDependencyVersions> {
    if path.ends_with("bun.lock") {
        load_text_lockfile_dependencies(lockfile_text)
    } else {
        load_binary_lockfile_dependencies(lockfile_text, path)
    }
}
