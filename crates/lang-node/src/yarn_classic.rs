use cached::proc_macro::cached;
use moon_error::MoonError;
use moon_lang::config_cache;
use moon_lang::LockfileDependencyVersions;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::path::{Path, PathBuf};

config_cache!(YarnLock, "yarn.lock", load_lockfile, write_lockfile);

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YarnLockDependency {
    pub version: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct YarnLock {
    pub dependencies: HashMap<String, YarnLockDependency>,

    #[serde(skip)]
    pub path: PathBuf,
}

// Package names are separated by commas in the following formats:
// "@babel/core@7.12.9":
// "@babel/code-frame@^7.0.0", "@babel/code-frame@^7.10.4", "@babel/code-frame@^7.12.13", "@babel/code-frame@^7.16.0", "@babel/code-frame@^7.18.6", "@babel/code-frame@^7.8.3":
fn extract_package_name(line: &str) -> Option<String> {
    // Remove trailing colon
    let names = &line[0..(line.len() - 1)];

    for name in names.split(", ") {
        let unquoted_name = if name.starts_with('"') {
            &name[1..(name.len() - 1)]
        } else {
            name
        };

        if let Some(at_index) = unquoted_name.rfind('@') {
            return Some(unquoted_name[0..at_index].to_owned());
        }
    }

    None
}

fn load_lockfile<P: AsRef<Path>>(path: P) -> Result<YarnLock, MoonError> {
    let path = path.as_ref();
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut current_package = None;
    let mut lockfile = YarnLock {
        dependencies: HashMap::new(),
        path: path.to_path_buf(),
    };

    for line in reader.lines().flatten() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Package name is the only line fully left aligned
        if line.starts_with('"') {
            current_package = Some(line.clone());

            // Extract only the version and skip other fields
        } else if line.starts_with("  version:") {
            if let Some(names) = current_package {
                let version = line[10..(line.len() - 1)].to_owned();

                lockfile
                    .dependencies
                    .insert(names, YarnLockDependency { version });

                current_package = None;
            }
        }
    }

    Ok(lockfile)
}

fn write_lockfile(_path: &Path, _lockfile: &YarnLock) -> Result<(), MoonError> {
    Ok(()) // Do nothing
}

#[cached(result)]
pub fn load_lockfile_dependencies(path: PathBuf) -> Result<LockfileDependencyVersions, MoonError> {
    let mut deps: LockfileDependencyVersions = HashMap::new();

    if let Some(lockfile) = YarnLock::read(path)? {
        for (names, dep) in lockfile.dependencies {
            if let Some(name) = extract_package_name(&names) {
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
