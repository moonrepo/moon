use crate::package_json::{PackageJsonCache, WorkspacesField};
use crate::pnpm::workspace::PnpmWorkspace;
use cached::proc_macro::cached;
use std::env;
use std::path::PathBuf;

// https://nodejs.org/api/modules.html#loading-from-the-global-folders
#[inline]
pub fn extend_node_path<T: AsRef<str>>(value: T) -> String {
    let value = value.as_ref();
    let delimiter = if cfg!(windows) { ";" } else { ":" };

    match env::var("NODE_PATH") {
        Ok(old_value) => format!("{value}{delimiter}{old_value}"),
        Err(_) => value.to_owned(),
    }
}

/// Extract the list of `workspaces` globs from the root `package.json`,
/// or if using pnpm, extract the globs from `pnpm-workspace.yaml`.
/// Furthermore, if the list is found, but is empty, return none.
#[cached(result)]
pub fn get_package_manager_workspaces(
    packages_root: PathBuf,
) -> miette::Result<Option<Vec<String>>> {
    if let Some(pnpm_workspace) = PnpmWorkspace::read(packages_root.clone())? {
        if !pnpm_workspace.packages.is_empty() {
            return Ok(Some(pnpm_workspace.packages));
        }
    }

    if let Some(package_json) = PackageJsonCache::read(packages_root)? {
        if let Some(workspaces) = package_json.data.workspaces {
            match workspaces {
                WorkspacesField::Globs(globs) => {
                    if !globs.is_empty() {
                        return Ok(Some(globs));
                    }
                }
                WorkspacesField::Config {
                    packages: globs, ..
                } => {
                    if !globs.is_empty() {
                        return Ok(Some(globs));
                    }
                }
            };
        }
    }

    Ok(None)
}
