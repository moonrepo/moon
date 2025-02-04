use moon_lang::LockfileDependencyVersions;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use starbase_utils::json;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct BunLockPackageJson {
    pub name: String,
    pub dependencies: BTreeMap<String, String>,
    pub dev_dependencies: BTreeMap<String, String>,
    pub peer_dependencies: BTreeMap<String, String>,
    pub optional_dependencies: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum BunLockPackage {
    Dependency(
        String,             // identifier
        String,             // ???
        BunLockPackageJson, // dependencies
        String,             // sha
    ),

    DependencyAlt(
        String,             // identifier
        BunLockPackageJson, // dependencies
    ),

    // Must be last!
    Workspace(Vec<String>),
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct BunLock {
    pub lockfile_version: u32,
    pub packages: FxHashMap<String, BunLockPackage>,
    pub patched_dependencies: BTreeMap<String, String>,
    pub overrides: BTreeMap<String, String>,
    pub workspaces: Option<FxHashMap<String, BunLockPackageJson>>,
}

pub fn load_text_lockfile_dependencies(
    lockfile_text: Arc<String>,
) -> miette::Result<LockfileDependencyVersions> {
    let mut deps: LockfileDependencyVersions = FxHashMap::default();
    let lockfile: BunLock = json::parse(lockfile_text.as_str())?;

    for (_name, package) in lockfile.packages {
        match package {
            BunLockPackage::Workspace(_) => {}
            BunLockPackage::DependencyAlt(id, _data) => {
                let Some((name, version)) = id.rsplit_once('@') else {
                    continue;
                };

                let dep = deps.entry(name.to_owned()).or_default();
                dep.push(version.to_owned());
            }
            BunLockPackage::Dependency(id, _unknown, _data, integrity) => {
                let Some((name, version)) = id.rsplit_once('@') else {
                    continue;
                };

                let dep = deps.entry(name.to_owned()).or_default();
                dep.push(version.to_owned());
                dep.push(integrity);
            }
        }
    }

    Ok(deps)
}
