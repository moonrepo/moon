use cached::proc_macro::cached;
use moon_lang::LockfileDependencyVersions;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YarnLockDependency {
    pub bin: Option<HashMap<String, String>>,
    pub checksum: Option<String>,
    pub dependencies: Option<HashMap<String, String>>,
    pub language_name: String,
    pub link_type: String,
    pub peer_dependencies: Option<HashMap<String, String>>,
    pub peer_dependencies_meta: Option<serde_yaml::Value>,
    pub resolution: String,
    pub version: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YarnLockMetadata {
    pub cache_key: i8,
    pub version: i8,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum YarnLockEntry {
    Dependency(YarnLockDependency),
    Metadata(YarnLockMetadata),
}

#[cached(result)]
pub fn load_lockfile(path: PathBuf) -> Result<HashMap<String, YarnLockEntry>, serde_yaml::Error> {
    serde_yaml::from_str(&fs::read_to_string(path).unwrap())
}

#[cached(result)]
pub fn load_lockfile_dependencies(
    path: PathBuf,
) -> Result<LockfileDependencyVersions, serde_yaml::Error> {
    let mut deps: LockfileDependencyVersions = HashMap::new();

    for entry in load_lockfile(path)?.values() {
        if let YarnLockEntry::Dependency(dep) = entry {
            if let Some(at_index) = dep.resolution.rfind('@') {
                let name = dep.resolution[0..at_index].to_owned();

                if let Some(versions) = deps.get_mut(&name) {
                    versions.push(dep.version.clone());
                } else {
                    deps.insert(name, vec![dep.version.clone()]);
                }
            }
        }
    }

    Ok(deps)
}
