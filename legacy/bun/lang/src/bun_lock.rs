use moon_lang::LockfileDependencyVersions;
use nodejs_package_json::{DependenciesMap, PackageJson};
use rustc_hash::FxHashMap;
use serde::Deserialize;
use starbase_utils::json;
use std::sync::Arc;

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct BunLockPackageDependencies {
    pub dependencies: DependenciesMap<String>,
    pub dev_dependencies: DependenciesMap<String>,
    pub peer_dependencies: DependenciesMap<String>,
    pub optional_dependencies: DependenciesMap<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct BunLockPackage(
    String,                     // identifier
    String,                     // ???
    BunLockPackageDependencies, // dependencies
    String,                     // sha
);

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct BunLock {
    pub lockfile_version: u32,
    pub packages: FxHashMap<String, BunLockPackage>,
    pub workspaces: Option<FxHashMap<String, PackageJson>>,
}

pub fn load_text_lockfile_dependencies(
    lockfile_text: Arc<String>,
) -> miette::Result<LockfileDependencyVersions> {
    let mut deps: LockfileDependencyVersions = FxHashMap::default();
    let lockfile: BunLock = json::parse(lockfile_text.as_str())?;

    for (name, package) in lockfile.packages {
        let version = package.0.replace("{name}@", "");
        let integrity = package.3;

        let dep = deps.entry(name).or_default();
        dep.push(version);
        dep.push(integrity);
    }

    Ok(deps)
}
