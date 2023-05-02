use crate::CARGO;
use cached::proc_macro::cached;
use cargo_lock::Lockfile as CargoLock;
use moon_error::MoonError;
use moon_lang::{config_cache_container, LockfileDependencyVersions};
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};

fn read_lockfile(path: &Path) -> Result<CargoLock, MoonError> {
    CargoLock::load(path).map_err(|e| MoonError::Generic(e.to_string()))
}

config_cache_container!(CargoLockCache, CargoLock, CARGO.lockfile, read_lockfile);

// trait CargoLockExt {
//     async fn get_resolved_dependencies(
//         &self,
//         project_root: &Path,
//     ) -> Result<LockfileDependencyVersions, MoonError>;
// }

#[cached(result)]
pub fn load_lockfile_dependencies(path: PathBuf) -> Result<LockfileDependencyVersions, MoonError> {
    let mut deps: LockfileDependencyVersions = FxHashMap::default();

    if let Some(lockfile) = CargoLockCache::read(path)? {
        for package in lockfile.packages {
            let version = if let Some(checksum) = package.checksum {
                checksum.to_string()
            } else {
                package.version.to_string()
            };

            deps.entry(package.name.as_str().to_string())
                .and_modify(|dep| {
                    dep.push(version.clone());
                })
                .or_insert_with(|| vec![version]);
        }
    }

    Ok(deps)
}
