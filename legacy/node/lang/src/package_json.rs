// package.json

use cached::proc_macro::cached;
use moon_lang::config_cache_model;
use starbase_utils::json::{self, JsonValue, read_file as read_json};
use std::path::{Path, PathBuf};

pub use nodejs_package_json::*;

config_cache_model!(
    PackageJsonCache,
    PackageJson,
    "package.json",
    read_json,
    write_preserved_json
);

impl PackageJsonCache {
    /// Add a package and version range to the `dependencies` field.
    pub fn add_dependency<T: AsRef<str>>(&mut self, name: T, range: T, if_missing: bool) -> bool {
        if let Some(deps) = self.internal_add_dependency(
            "dependencies",
            self.data.dependencies.clone(),
            name,
            range,
            if_missing,
        ) {
            self.data.dependencies = Some(deps);

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
            self.data.dev_dependencies.clone(),
            name,
            range,
            if_missing,
        ) {
            self.data.dev_dependencies = Some(deps);

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
            self.data.peer_dependencies.clone(),
            name,
            range,
            if_missing,
        ) {
            self.data.peer_dependencies = Some(deps);

            return true;
        }

        false
    }

    /// Add a version range to the `engines` field.
    /// Return true if the new value is different from the old value.
    pub fn add_engine<K: AsRef<str>, V: AsRef<str>>(&mut self, engine: K, range: V) -> bool {
        let engine = engine.as_ref();
        let range = range.as_ref();

        if let Some(engines) = &mut self.data.engines {
            if engines.contains_key(engine) && engines.get(engine).unwrap() == range {
                return false;
            }

            engines.insert(engine.to_owned(), range.to_owned());
        } else {
            self.data.engines = Some(EnginesMap::from_iter([(
                engine.to_owned(),
                range.to_owned(),
            )]));
        }

        self.dirty.push("engines".into());

        true
    }

    /// Set the `packageManager` field.
    /// Return true if the new value is different from the old value.
    pub fn set_package_manager<T: AsRef<str>>(&mut self, value: T) -> bool {
        let value = value.as_ref();

        if self
            .data
            .package_manager
            .as_ref()
            .is_some_and(|v| v == value)
        {
            return false;
        }

        self.dirty.push("packageManager".into());

        if value.is_empty() {
            self.data.package_manager = None;
        } else {
            self.data.package_manager = Some(value.to_owned());
        }

        true
    }

    /// Set the `scripts` field.
    /// Return true if the new value is different from the old value.
    pub fn set_scripts(&mut self, scripts: ScriptsMap) -> bool {
        if self.data.scripts.is_none() && scripts.is_empty() {
            return false;
        }

        self.dirty.push("scripts".into());

        if scripts.is_empty() {
            self.data.scripts = None;
        } else {
            self.data.scripts = Some(scripts);
        }

        true
    }

    /// Add a package and version range to a dependencies field.
    /// If `is_missing` is true, only add if it doesn't already exist.
    /// Return true if the new value is different from the old value.
    fn internal_add_dependency<T: AsRef<str>>(
        &mut self,
        deps_name: &str,
        deps_map: Option<DependenciesMap<String>>,
        name: T,
        range: T,
        if_missing: bool,
    ) -> Option<DependenciesMap<String>> {
        let name = name.as_ref();
        let range = range.as_ref();

        if name.is_empty() {
            return None;
        }

        let mut dependencies = deps_map.unwrap_or_default();

        // Only add if the dependency doesnt already exist
        if if_missing && dependencies.contains_key(name) {
            return None;
        }

        dependencies.insert(name.to_owned(), range.to_owned());

        self.dirty.push(deps_name.to_owned());

        Some(dependencies)
    }
}

fn write_preserved_json(path: &Path, package: &PackageJsonCache) -> miette::Result<()> {
    let mut data: JsonValue = json::read_file(path)?;

    // We only need to set fields that we modify within moon,
    // otherwise it's a ton of overhead and maintenance!
    for field in &package.dirty {
        match field.as_ref() {
            "dependencies" => {
                if let Some(dependencies) = &package.data.dependencies {
                    data[field] = JsonValue::from_iter(dependencies.clone());
                } else if let Some(root) = data.as_object_mut() {
                    root.remove(field);
                }
            }
            "devDependencies" => {
                if let Some(dev_dependencies) = &package.data.dev_dependencies {
                    data[field] = JsonValue::from_iter(dev_dependencies.clone());
                } else if let Some(root) = data.as_object_mut() {
                    root.remove(field);
                }
            }
            "peerDependencies" => {
                if let Some(peer_dependencies) = &package.data.peer_dependencies {
                    data[field] = JsonValue::from_iter(peer_dependencies.clone());
                } else if let Some(root) = data.as_object_mut() {
                    root.remove(field);
                }
            }
            "engines" => {
                if let Some(engines) = &package.data.engines {
                    data[field] = JsonValue::from_iter(engines.clone());
                } else if let Some(root) = data.as_object_mut() {
                    root.remove(field);
                }
            }
            "packageManager" => {
                if let Some(package_manager) = &package.data.package_manager {
                    data[field] = JsonValue::from(package_manager.clone());
                } else if let Some(root) = data.as_object_mut() {
                    root.remove(field);
                }
            }
            "scripts" => {
                if let Some(scripts) = &package.data.scripts {
                    data[field] = JsonValue::from_iter(scripts.clone());
                } else if let Some(root) = data.as_object_mut() {
                    root.remove(field);
                }
            }
            _ => {}
        };
    }

    json::write_file_with_config(path, &data, true)?;

    Ok(())
}
