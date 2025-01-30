mod deno_json;
mod deno_lock;

pub use deno_json::*;
pub use deno_lock::*;
pub use moon_lang::LockfileDependencyVersions;

use cached::proc_macro::cached;
use std::path::PathBuf;

#[cached(result)]
pub fn find_package_manager_workspaces_root(
    starting_dir: PathBuf,
) -> miette::Result<Option<PathBuf>> {
    let mut current_dir = Some(starting_dir.as_path());

    while let Some(dir) = current_dir {
        if let Some(deno_json) = DenoJson::read(dir)? {
            if deno_json.workspace.is_some() {
                return Ok(Some(dir.to_path_buf()));
            }
        }

        current_dir = dir.parent();
    }

    Ok(None)
}
