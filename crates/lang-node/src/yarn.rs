use cached::proc_macro::cached;
use moon_error::{map_io_to_fs_error, MoonError};
use moon_lang::config_cache;
use moon_lang::LockfileDependencyVersions;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

config_cache!(YarnLock, "yarn.lock", load_lockfile, write_lockfile);

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

    #[serde(flatten)]
    pub unknown: HashMap<String, Value>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YarnLockMetadata {
    pub cache_key: i8,
    pub version: i8,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct YarnLock {
    #[serde(rename = "__metadata")]
    pub metadata: YarnLockMetadata,

    #[serde(flatten)]
    pub dependencies: HashMap<String, YarnLockDependency>,

    #[serde(skip)]
    pub path: PathBuf,
}

fn load_lockfile<P: AsRef<Path>>(path: P) -> Result<YarnLock, MoonError> {
    let path = path.as_ref();
    let lockfile: YarnLock = serde_yaml::from_str(
        &fs::read_to_string(path).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?,
    )
    .map_err(|e| MoonError::Yaml(path.to_path_buf(), e))?;

    Ok(lockfile)
}

fn write_lockfile(_path: &Path, _lockfile: &YarnLock) -> Result<(), MoonError> {
    Ok(()) // Do nothing
}

#[cached(result)]
pub fn load_lockfile_dependencies(path: PathBuf) -> Result<LockfileDependencyVersions, MoonError> {
    let mut deps: LockfileDependencyVersions = HashMap::new();

    if let Some(lockfile) = YarnLock::read(path)? {
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
