use cached::proc_macro::cached;
use cargo_lock::Lockfile as CargoLock;
use miette::IntoDiagnostic;
use moon_lang::{LockfileDependencyVersions, config_cache_container};
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};

fn read_lockfile(path: &Path) -> miette::Result<CargoLock> {
    CargoLock::load(path).into_diagnostic()
}

config_cache_container!(CargoLockCache, CargoLock, "Cargo.lock", read_lockfile);

#[cached(result)]
pub fn load_lockfile_dependencies(path: PathBuf) -> miette::Result<LockfileDependencyVersions> {
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
