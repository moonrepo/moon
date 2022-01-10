use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

// This implementation is forked from the wonderful crate "npm-package-json", as we need full
// control for integration with the rest of the crates. We also can't wait for upsteam for new
// updates. https://github.com/mainrs/npm-package-json-rs

// Original license: Copyright 2020 Sven Lechner

// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
    pub author: Option<Person>,
    pub bin: Option<Bin>,
    pub browser: Option<String>,
    pub bugs: Option<Bug>,
    pub bundled_dependencies: Option<Vec<String>>,
    pub config: Option<Value>,
    pub contributors: Option<Vec<Person>>,
    pub cpu: Option<Vec<String>>,
    pub dependencies: Option<DepsSet>,
    pub description: Option<String>,
    pub dev_dependencies: Option<DepsSet>,
    pub directories: Option<Directories>,
    pub engines: Option<EnginesSet>,
    pub files: Option<Vec<String>>,
    pub funding: Option<Funding>,
    pub homepage: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub license: Option<License>,
    pub main: Option<String>,
    pub man: Option<StringOrArray<String>>,
    pub module: Option<String>,
    pub name: Option<String>,
    pub optional_dependencies: Option<DepsSet>,
    pub os: Option<Vec<String>>,
    pub overrides: Option<OverridesSet>,
    pub peer_dependencies: Option<DepsSet>,
    pub peer_dependencies_meta: Option<PeerDepsMetaSet>,
    pub private: Option<bool>,
    pub publish_config: Option<Value>,
    pub repository: Option<Repository>,
    pub scripts: Option<ScriptsSet>,
    #[serde(rename = "type")]
    pub type_of: Option<String>,
    pub version: Option<String>,
    pub workspaces: Option<Workspaces>,

    // Node.js specific: https://nodejs.org/api/packages.html#nodejs-packagejson-field-definitions
    pub exports: Option<Value>,
    pub imports: Option<Value>,
    pub package_manager: Option<String>,

    // Pnpm specific: https://pnpm.io/package_json
    pub pnpm: Option<Pnpm>,

    // Yarn specific: https://yarnpkg.com/configuration/manifest
    pub dependencies_meta: Option<DepsMetaSet>,
    pub language_name: Option<String>,
    pub install_config: Option<Value>,
    pub prefer_unplugged: Option<bool>,
    pub resolutions: Option<DepsSet>,

    // Unknown fields
    #[serde(flatten)]
    pub unknown_fields: BTreeMap<String, Value>,

    // Non-standard
    #[serde(skip)]
    pub path: PathBuf,
}

pub type BinSet = BTreeMap<String, String>;
pub type DepsMetaSet = BTreeMap<String, DependencyMeta>;
pub type DepsSet = BTreeMap<String, String>;
pub type EnginesSet = BTreeMap<String, String>;
pub type OverridesSet = BTreeMap<String, StringOrObject<DepsSet>>;
pub type PeerDepsMetaSet = BTreeMap<String, PeerDependencyMeta>;
pub type ScriptsSet = BTreeMap<String, String>;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
enum StringOrArray<T> {
    String(String),
    Object(T),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
enum StringOrObject<T> {
    String(String),
    Object(T),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
enum StringObjectAndArray<T> {
    String(String),
    Object(T),
    Array(T),
}

pub type Bin = StringOrArray<BinSet>;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Bug {
    pub email: Option<String>,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DependencyMeta {
    pub optional: Option<bool>,

    // Yarn
    pub built: Option<bool>,
    pub unplugged: Option<bool>,

    // pnpm
    pub injected: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Directories {
    pub bin: Option<String>,
    pub man: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FundingMetadata {
    #[serde(rename = "type")]
    pub type_of: String,
    pub url: String,
}

pub type Funding = StringObjectAndArray<FundingMetadata>;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct LicenseMetadata {
    #[serde(rename = "type")]
    pub type_of: String,
    pub url: String,
}

pub type License = StringObjectAndArray<LicenseMetadata>;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PersonMetadata {
    pub name: String,
    pub email: Option<String>,
    pub url: Option<String>,
}

pub type Person = StringOrObject<PersonMetadata>;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PeerDependencyMeta {
    pub optional: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Pnpm {
    pub never_built_dependencies: Option<Vec<String>>,
    pub overrides: Option<OverridesSet>,
    pub package_extensions: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RepositoryMetadata {
    pub directory: Option<String>,
    #[serde(rename = "type")]
    pub type_of: String,
    pub url: String,
}

pub type Repository = StringOrObject<RepositoryMetadata>;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorkspacesExpanded {
    pub nohoist: Option<Vec<String>>,
    pub packages: Option<Vec<String>>,
}

pub type Workspaces = StringOrObject<WorkspacesExpanded>;
