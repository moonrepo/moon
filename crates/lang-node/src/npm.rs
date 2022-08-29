use cached::proc_macro::cached;
use moon_error::MoonError;
use moon_lang::config_cache;
use moon_lang::LockfileDependencyVersions;
use moon_utils::fs::sync_read_json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

config_cache!(
    PackageLock,
    "package-lock.json",
    sync_read_json,
    write_lockfile
);

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageLockDependency {
    pub dependencies: Option<HashMap<String, PackageLockDependency>>,
    pub dev: Option<bool>,
    pub integrity: Option<String>,
    pub requires: Option<HashMap<String, String>>,
    pub resolved: Option<String>,
    pub version: String,

    #[serde(flatten)]
    pub unknown: HashMap<String, Value>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageLock {
    pub lockfile_version: i32,
    pub name: String,
    pub dependencies: HashMap<String, PackageLockDependency>,
    pub packages: Option<HashMap<String, Value>>,
    pub requires: Option<bool>,

    #[serde(flatten)]
    pub unknown: HashMap<String, Value>,

    #[serde(skip)]
    pub path: PathBuf,
}

fn write_lockfile(_path: &Path, _lockfile: &PackageLock) -> Result<(), MoonError> {
    Ok(()) // Do nothing
}

#[cached(result)]
pub fn load_lockfile_dependencies(path: PathBuf) -> Result<LockfileDependencyVersions, MoonError> {
    let mut deps: LockfileDependencyVersions = HashMap::new();

    if let Some(lockfile) = PackageLock::read(path)? {
        for (name, dep) in lockfile.dependencies {
            if let Some(versions) = deps.get_mut(&name) {
                versions.push(dep.version.clone());
            } else {
                deps.insert(name, vec![dep.version.clone()]);
            }
        }
    }

    Ok(deps)
}
