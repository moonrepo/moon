// package.json

use crate::NPM;
use cached::proc_macro::cached;
use moon_lang::config_cache;
use serde::{Deserialize, Serialize};
use starbase_utils::json::{self, read_file as read_json, JsonValue};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

config_cache!(PackageJson, NPM.manifest, read_json, write_preserved_json);

// Only define fields we interact with and care about!
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<DepsSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<DepsSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub engines: Option<EnginesSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_manager: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_dependencies: Option<DepsSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scripts: Option<ScriptsSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspaces: Option<PackageWorkspaces>,

    // Non-standard
    #[serde(skip)]
    pub dirty: Vec<String>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl PackageJson {
    pub fn save(&mut self) -> miette::Result<()> {
        if !self.dirty.is_empty() {
            write_preserved_json(&self.path, self)?;
            self.dirty.clear();

            PackageJson::write(self.clone())?;
        }

        Ok(())
    }

    /// Add a package and version range to the `dependencies` field.
    pub fn add_dependency<T: AsRef<str>>(&mut self, name: T, range: T, if_missing: bool) -> bool {
        if let Some(deps) = self.internal_add_dependency(
            "dependencies",
            self.dependencies.clone(),
            name,
            range,
            if_missing,
        ) {
            self.dependencies = Some(deps);

            return true;
        }

        false
    }

    /// Add a package and version range to the `devDependencies` field.
    pub fn add_dev_dependency<T: AsRef<str>>(
        &mut self,
        name: T,
        range: T,
        if_missing: bool,
    ) -> bool {
        if let Some(deps) = self.internal_add_dependency(
            "devDependencies",
            self.dev_dependencies.clone(),
            name,
            range,
            if_missing,
        ) {
            self.dev_dependencies = Some(deps);

            return true;
        }

        false
    }

    /// Add a package and version range to the `peerDependencies` field.
    pub fn add_peer_dependency<T: AsRef<str>>(
        &mut self,
        name: T,
        range: T,
        if_missing: bool,
    ) -> bool {
        if let Some(deps) = self.internal_add_dependency(
            "peerDependencies",
            self.peer_dependencies.clone(),
            name,
            range,
            if_missing,
        ) {
            self.peer_dependencies = Some(deps);

            return true;
        }

        false
    }

    /// Add a version range to the `engines` field.
    /// Return true if the new value is different from the old value.
    pub fn add_engine<T: AsRef<str>>(&mut self, engine: T, range: T) -> bool {
        let engine = engine.as_ref();
        let range = range.as_ref();

        if let Some(engines) = &mut self.engines {
            if engines.contains_key(engine) && engines.get(engine).unwrap() == range {
                return false;
            }

            engines.insert(engine.to_owned(), range.to_owned());
        } else {
            self.engines = Some(BTreeMap::from([(engine.to_owned(), range.to_owned())]));
        }

        self.dirty.push("engines".into());

        true
    }

    /// Set the `packageManager` field.
    /// Return true if the new value is different from the old value.
    pub fn set_package_manager<T: AsRef<str>>(&mut self, value: T) -> bool {
        let value = value.as_ref();

        if self.package_manager.is_some() && self.package_manager.as_ref().unwrap() == value {
            return false;
        }

        self.dirty.push("packageManager".into());
        self.package_manager = Some(value.to_owned());

        true
    }

    /// Set the `scripts` field.
    /// Return true if the new value is different from the old value.
    pub fn set_scripts(&mut self, scripts: ScriptsSet) -> bool {
        if self.scripts.is_none() && scripts.is_empty() {
            return false;
        }

        self.dirty.push("scripts".into());

        if scripts.is_empty() {
            self.scripts = None;
        } else {
            self.scripts = Some(scripts);
        }

        true
    }

    /// Add a package and version range to a dependencies field.
    /// If `is_missing` is true, only add if it doesn't already exist.
    /// Return true if the new value is different from the old value.
    fn internal_add_dependency<T: AsRef<str>>(
        &mut self,
        deps_name: &str,
        deps_map: Option<DepsSet>,
        name: T,
        range: T,
        if_missing: bool,
    ) -> Option<DepsSet> {
        let name = name.as_ref();
        let range = range.as_ref();

        if name.is_empty() {
            return None;
        }

        let mut dependencies = match deps_map {
            Some(deps) => deps,
            None => BTreeMap::new(),
        };

        // Only add if the dependency doesnt already exist
        if if_missing && dependencies.contains_key(name) {
            return None;
        }

        dependencies.insert(name.to_owned(), range.to_owned());

        self.dirty.push(deps_name.to_owned());

        Some(dependencies)
    }
}

pub type DepsSet = BTreeMap<String, String>;
pub type EnginesSet = BTreeMap<String, String>;
pub type ScriptsSet = BTreeMap<String, String>;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PackageWorkspacesExpanded {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nohoist: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub packages: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum PackageWorkspaces {
    Array(Vec<String>),
    Object(PackageWorkspacesExpanded),
}

// https://github.com/serde-rs/json/issues/858
// `serde-json` does NOT preserve original order when serializing the struct,
// so we need to hack around this by using the `json` crate and manually
// making the changes. For this to work correctly, we need to read the json
// file again and parse it with `json`, then stringify it with `json`.
#[track_caller]
fn write_preserved_json(path: &Path, package: &PackageJson) -> miette::Result<()> {
    let mut data: JsonValue = json::read_file(path)?;

    // We only need to set fields that we modify within moon,
    // otherwise it's a ton of overhead and maintenance!
    for field in &package.dirty {
        match field.as_ref() {
            "dependencies" => {
                if let Some(dependencies) = &package.dependencies {
                    data[field] = JsonValue::from_iter(dependencies.clone());
                } else if let Some(root) = data.as_object_mut() {
                    root.remove(field);
                }
            }
            "devDependencies" => {
                if let Some(dev_dependencies) = &package.dev_dependencies {
                    data[field] = JsonValue::from_iter(dev_dependencies.clone());
                } else if let Some(root) = data.as_object_mut() {
                    root.remove(field);
                }
            }
            "peerDependencies" => {
                if let Some(peer_dependencies) = &package.peer_dependencies {
                    data[field] = JsonValue::from_iter(peer_dependencies.clone());
                } else if let Some(root) = data.as_object_mut() {
                    root.remove(field);
                }
            }
            "engines" => {
                if let Some(engines) = &package.engines {
                    data[field] = JsonValue::from_iter(engines.clone());
                }
            }
            "packageManager" => {
                if let Some(package_manager) = &package.package_manager {
                    data[field] = JsonValue::from(package_manager.clone());
                }
            }
            "scripts" => {
                if let Some(scripts) = &package.scripts {
                    data[field] = JsonValue::from_iter(scripts.clone());
                } else if let Some(root) = data.as_object_mut() {
                    root.remove(field);
                }
            }
            _ => {}
        };
    }

    json::write_file_with_config(path, data, true)?;

    Ok(())
}
