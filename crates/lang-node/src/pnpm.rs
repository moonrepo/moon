use cached::proc_macro::cached;
use moon_error::{map_io_to_fs_error, MoonError};
use moon_lang::config_cache;
use moon_lang::LockfileDependencyVersions;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

config_cache!(PnpmLock, "pnpm-lock.yaml", load_lockfile, write_lockfile);

type DependencyMap = HashMap<String, Value>;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PnpmLockPackage {
    pub cpu: Option<Vec<String>>,
    pub dependencies: Option<DependencyMap>,
    pub dev: Option<bool>,
    pub engines: Option<HashMap<String, String>>,
    pub has_bin: Option<bool>,
    pub optional: Option<bool>,
    pub optional_dependencies: Option<DependencyMap>,
    pub os: Option<Vec<String>>,
    pub peer_dependencies: Option<DependencyMap>,
    pub requires_build: Option<bool>,
    pub transitive_peer_dependencies: Option<Vec<String>>,
    pub resolution: Value,

    #[serde(flatten)]
    pub unknown: HashMap<String, Value>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PnpmLock {
    pub lockfile_version: Value,
    pub importers: HashMap<String, Value>,
    pub packages: HashMap<String, PnpmLockPackage>,

    #[serde(flatten)]
    pub unknown: HashMap<String, Value>,

    #[serde(skip)]
    pub path: PathBuf,
}

fn load_lockfile<P: AsRef<Path>>(path: P) -> Result<PnpmLock, MoonError> {
    let path = path.as_ref();
    let lockfile: PnpmLock = serde_yaml::from_str(
        &fs::read_to_string(path).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?,
    )
    .map_err(|e| MoonError::Yaml(path.to_path_buf(), e))?;

    Ok(lockfile)
}

fn write_lockfile(_path: &Path, _lockfile: &PnpmLock) -> Result<(), MoonError> {
    Ok(()) // Do nothing
}

#[cached(result)]
pub fn load_lockfile_dependencies(path: PathBuf) -> Result<LockfileDependencyVersions, MoonError> {
    let mut deps: LockfileDependencyVersions = HashMap::new();

    if let Some(lockfile) = PnpmLock::read(path)? {
        // Dependencies are defined in the following formats:
        // /p-limit/2.3.0
        // /jest/28.1.3_@types+node@18.0.6
        // /@jest/core/28.1.3
        // /@babel/plugin-transform-block-scoping/7.18.9_@babel+core@7.18.9
        for dep_locator in lockfile.packages.keys() {
            // Remove the leading slash
            let mut locator = &dep_locator[1..];

            // Find an underscore and return the 1st portion
            if locator.contains('_') {
                if let Some(under_index) = locator.find('_') {
                    locator = &dep_locator[1..under_index];
                }
            }

            // Find the last slash before the version
            if let Some(slash_index) = locator.rfind('/') {
                let name = &locator[0..slash_index];
                let version = &locator[(slash_index + 1)..];

                if let Some(versions) = deps.get_mut(name) {
                    versions.push(version.to_owned());
                } else {
                    deps.insert(name.to_owned(), vec![version.to_owned()]);
                }
            }
        }
    }

    Ok(deps)
}
