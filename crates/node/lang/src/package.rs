// package.json

use crate::NPM;
use cached::proc_macro::cached;
use json;
use moon_error::MoonError;
use moon_lang::config_cache;
use moon_utils::fs::{self, sync::read_json};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

config_cache!(
    PackageJson,
    NPM.manifest_filename,
    read_json,
    write_preserved_json
);

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Person>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bin: Option<Bin>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bugs: Option<Bug>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundled_dependencies: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub contributors: Option<Vec<Person>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<DepsSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<DepsSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub directories: Option<Directories>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub engines: Option<EnginesSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub funding: Option<Funding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub main: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub man: Option<StringOrArray<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_dependencies: Option<DepsSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<OverridesSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_dependencies: Option<DepsSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_dependencies_meta: Option<PeerDepsMetaSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub private: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish_config: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<Repository>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scripts: Option<ScriptsSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub type_of: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspaces: Option<PackageWorkspaces>,

    // Node.js specific: https://nodejs.org/api/packages.html#nodejs-packagejson-field-definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exports: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub imports: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_manager: Option<String>,

    // Pnpm specific: https://pnpm.io/package_json
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pnpm: Option<Pnpm>,

    // Yarn specific: https://yarnpkg.com/configuration/manifest
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies_meta: Option<DepsMetaSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_config: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefer_unplugged: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolutions: Option<DepsSet>,

    // Unknown fields
    #[serde(flatten)]
    pub unknown_fields: BTreeMap<String, Value>,

    // Non-standard
    #[serde(skip)]
    pub dirty: bool,

    #[serde(skip)]
    pub path: PathBuf,
}

impl PackageJson {
    pub fn save(&mut self) -> Result<(), MoonError> {
        if self.dirty {
            write_preserved_json(&self.path, self)?;
            self.dirty = false;

            PackageJson::write(self.clone())?;
        }

        Ok(())
    }

    /// Add a package and version range to the `dependencies` field.
    pub fn add_dependency<T: AsRef<str>>(&mut self, name: T, range: T, if_missing: bool) -> bool {
        if let Some(deps) =
            self.internal_add_dependency(self.dependencies.clone(), name, range, if_missing)
        {
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
        if let Some(deps) =
            self.internal_add_dependency(self.dev_dependencies.clone(), name, range, if_missing)
        {
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
        if let Some(deps) =
            self.internal_add_dependency(self.peer_dependencies.clone(), name, range, if_missing)
        {
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

        self.dirty = true;

        true
    }

    /// Set the `packageManager` field.
    /// Return true if the new value is different from the old value.
    pub fn set_package_manager<T: AsRef<str>>(&mut self, value: T) -> bool {
        let value = value.as_ref();

        if self.package_manager.is_some() && self.package_manager.as_ref().unwrap() == value {
            return false;
        }

        self.dirty = true;
        self.package_manager = Some(value.to_owned());

        true
    }

    /// Set the `scripts` field.
    /// Return true if the new value is different from the old value.
    pub fn set_scripts(&mut self, scripts: ScriptsSet) -> bool {
        if self.scripts.is_none() && scripts.is_empty() {
            return false;
        }

        self.dirty = true;

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

        self.dirty = true;

        Some(dependencies)
    }
}

pub type BinSet = BTreeMap<String, String>;
pub type DepsMetaSet = BTreeMap<String, DependencyMeta>;
pub type DepsSet = BTreeMap<String, String>;
pub type EnginesSet = BTreeMap<String, String>;
pub type OverridesSet = BTreeMap<String, StringOrObject<DepsSet>>;
pub type PeerDepsMetaSet = BTreeMap<String, PeerDependencyMeta>;
pub type ScriptsSet = BTreeMap<String, String>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StringOrArray<T> {
    String(String),
    Array(Vec<T>),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StringOrObject<T> {
    String(String),
    Object(T),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StringArrayOrObject<T> {
    String(String),
    Array(Vec<T>),
    Object(T),
}

pub type Bin = StringOrObject<BinSet>;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bug {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct DependencyMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,

    // Yarn
    #[serde(skip_serializing_if = "Option::is_none")]
    pub built: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub unplugged: Option<bool>,

    // Pnpm
    #[serde(skip_serializing_if = "Option::is_none")]
    pub injected: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Directories {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bin: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub man: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct FundingMetadata {
    #[serde(rename = "type")]
    pub type_of: String,
    pub url: String,
}

pub type Funding = StringArrayOrObject<FundingMetadata>;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct LicenseMetadata {
    #[serde(rename = "type")]
    pub type_of: String,
    pub url: String,
}

pub type License = StringArrayOrObject<LicenseMetadata>;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PersonMetadata {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

pub type Person = StringOrObject<PersonMetadata>;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PeerDependencyMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Pnpm {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub never_built_dependencies: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<OverridesSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_extensions: Option<Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RepositoryMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,

    #[serde(rename = "type")]
    pub type_of: String,

    pub url: String,
}

pub type Repository = StringOrObject<RepositoryMetadata>;

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
fn write_preserved_json(path: &Path, package: &PackageJson) -> Result<(), MoonError> {
    let contents = fs::sync::read_json_string(path)?;
    let mut data = json::parse(&contents).expect("Unable to parse package.json");

    // We only need to set fields that we modify within Moon,
    // otherwise it's a ton of overhead and maintenance!
    if let Some(dependencies) = &package.dependencies {
        data["dependencies"] = json::from(dependencies.clone());
    } else {
        data.remove("dependencies");
    }

    if let Some(dev_dependencies) = &package.dev_dependencies {
        data["devDependencies"] = json::from(dev_dependencies.clone());
    } else {
        data.remove("devDependencies");
    }

    if let Some(peer_dependencies) = &package.peer_dependencies {
        data["peerDependencies"] = json::from(peer_dependencies.clone());
    } else {
        data.remove("peerDependencies");
    }

    if let Some(engines) = &package.engines {
        data["engines"] = json::from(engines.clone());
    }

    if let Some(package_manager) = &package.package_manager {
        data["packageManager"] = json::from(package_manager.clone());
    }

    if let Some(scripts) = &package.scripts {
        data["scripts"] = json::from(scripts.clone());
    } else {
        data.remove("scripts");
    }

    let mut data = json::stringify_pretty(data, 2);
    data += "\n"; // Always add trailing newline

    std::fs::write(path, data)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use assert_fs::prelude::*;

    #[tokio::test]
    async fn preserves_order_when_de_to_ser() {
        let json = r#"{"name": "hello", "description": "world", "private": true}"#;

        let dir = assert_fs::TempDir::new().unwrap();
        let file = dir.child("package.json");
        file.write_str(json).unwrap();

        let mut package = PackageJson::read(dir.path()).unwrap().unwrap();

        package.save().unwrap();

        assert_eq!(fs::read_json_string(file.path()).await.unwrap(), json,);
    }

    mod add_dependency {
        use super::*;

        #[test]
        fn adds_if_not_set() {
            let mut pkg = PackageJson::default();

            assert_eq!(pkg.dependencies, None);

            assert!(pkg.add_dependency("foo", "1.2.3", false));

            assert_eq!(pkg.dependencies.unwrap().get("foo").unwrap(), &"1.2.3");
        }

        #[test]
        fn adds_if_not_set_and_missing_true() {
            let mut pkg = PackageJson::default();

            assert_eq!(pkg.dependencies, None);

            assert!(pkg.add_dependency("foo", "1.2.3", true));

            assert_eq!(pkg.dependencies.unwrap().get("foo").unwrap(), &"1.2.3");
        }

        #[test]
        fn adds_if_set() {
            let mut pkg = PackageJson {
                dependencies: Some(BTreeMap::from([("foo".to_owned(), "1.2.3".to_owned())])),
                ..PackageJson::default()
            };

            assert!(pkg.add_dependency("foo", "4.5.6", false));

            assert_eq!(pkg.dependencies.unwrap().get("foo").unwrap(), &"4.5.6");
        }

        #[test]
        fn doesnt_add_if_set_and_missing_true() {
            let mut pkg = PackageJson {
                dependencies: Some(BTreeMap::from([("foo".to_owned(), "1.2.3".to_owned())])),
                ..PackageJson::default()
            };

            assert!(!pkg.add_dependency("foo", "4.5.6", true));

            assert_eq!(pkg.dependencies.unwrap().get("foo").unwrap(), &"1.2.3");
        }
    }

    mod add_engine {
        use super::*;

        #[test]
        fn adds_if_not_set() {
            let mut pkg = PackageJson::default();

            assert_eq!(pkg.engines, None);

            assert!(pkg.add_engine("node", "1.2.3"));

            assert_eq!(pkg.engines.unwrap().get("node").unwrap(), &"1.2.3");
        }

        #[test]
        fn adds_if_set() {
            let mut pkg = PackageJson {
                engines: Some(BTreeMap::from([("node".to_owned(), "1.2.3".to_owned())])),
                ..PackageJson::default()
            };

            assert!(pkg.add_engine("node", "4.5.6"));

            assert_eq!(pkg.engines.unwrap().get("node").unwrap(), &"4.5.6");
        }

        #[test]
        fn returns_false_for_same_value() {
            let mut pkg = PackageJson {
                engines: Some(BTreeMap::from([("node".to_owned(), "1.2.3".to_owned())])),
                ..PackageJson::default()
            };

            assert!(!pkg.add_engine("node", "1.2.3"));
        }
    }

    mod set_package_manager {
        use super::*;

        #[test]
        fn adds_if_not_set() {
            let mut pkg = PackageJson::default();

            assert_eq!(pkg.package_manager, None);

            assert!(pkg.set_package_manager("npm@1.2.3"));

            assert_eq!(pkg.package_manager.unwrap(), "npm@1.2.3".to_owned());
        }

        #[test]
        fn adds_if_set() {
            let mut pkg = PackageJson {
                package_manager: Some(String::from("npm@1.2.3")),
                ..PackageJson::default()
            };

            assert!(pkg.set_package_manager("npm@4.5.6"));

            assert_eq!(pkg.package_manager.unwrap(), "npm@4.5.6".to_owned());
        }

        #[test]
        fn returns_false_for_same_value() {
            let mut pkg = PackageJson {
                package_manager: Some(String::from("npm@1.2.3")),
                ..PackageJson::default()
            };

            assert!(!pkg.set_package_manager("npm@1.2.3"));
        }
    }
}
