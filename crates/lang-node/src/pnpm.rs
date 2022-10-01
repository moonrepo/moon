use crate::PNPM;
use cached::proc_macro::cached;
use moon_error::MoonError;
use moon_lang::{config_cache, LockfileDependencyVersions};
use moon_utils::fs::sync::read_yaml;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

config_cache!(PnpmLock, PNPM.lock_filename, read_yaml);

type DependencyMap = HashMap<String, Value>;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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
    pub resolution: Option<HashMap<String, String>>,

    #[serde(flatten)]
    pub unknown: HashMap<String, Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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
                    locator = &dep_locator[1..(under_index + 1)];
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

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use moon_utils::string_vec;
    use pretty_assertions::assert_eq;
    use serde_yaml::{Mapping, Number};

    #[test]
    fn parses_lockfile() {
        let temp = assert_fs::TempDir::new().unwrap();

        temp.child("pnpm-lock.yaml")
            .write_str(
                r#"
lockfileVersion: 5.4

importers:

  .: {}

packages:

  /@ampproject/remapping/2.2.0:
    resolution: {integrity: sha512-qRmjj8nj9qmLTQXXmaR1cck3UXSRMPrbsLJAasZpF+t3riI71BXed5ebIOYwQntykeZuhjsdweEc9BxH5Jc26w==}
    engines: {node: '>=6.0.0'}
    dependencies:
      '@jridgewell/gen-mapping': 0.1.1
      '@jridgewell/trace-mapping': 0.3.14
    dev: true

  /@babel/plugin-syntax-async-generators/7.8.4_@babel+core@7.18.9:
    resolution: {integrity: sha512-tycmZxkGfZaxhMRbXlPXuVFpdWlXpir2W4AMhSJgRKzk/eDlIXOhb2LHWoLpDF7TEHylV5zNhykX6KAgHJmTNw==}
    peerDependencies:
      '@babel/core': ^7.0.0-0
    dependencies:
      '@babel/core': 7
      '@babel/helper-plugin-utils': 7.18.9
    dev: true

  /array-union/2.1.0:
    resolution: {integrity: sha512-HGyxoOTYUyCM6stUe6EJgnd4EoewAI7zMdfqO+kGjnlZmBDz/cR5pf8r/cR4Wq60sL/p0IkcjUEEPwS3GFrIyw==}
    engines: {node: '>=8'}
    dev: true

  /solid-jest/0.2.0_@babel+core@7.18.9:
    resolution: {integrity: sha512-1ILtAj+z6bh1vTvaDlcT8501vmkzkVZMk2aiexJy+XWTZ+sb9B7IWedvWadIhOwwL97fiW4eMmN6SrbaHjn12A==}
    peerDependencies:
      babel-preset-solid: ^1.0.0
    dependencies:
      '@babel/preset-env': 7.18.9_@babel+core@7.18.9
      babel-jest: 27.5.1_@babel+core@7.18.9
      enhanced-resolve-jest: 1.1.0
    transitivePeerDependencies:
      - '@babel/core'
      - supports-color
    dev: true
"#,
            )
            .unwrap();

        let lockfile: PnpmLock = read_yaml(temp.path().join("pnpm-lock.yaml")).unwrap();

        assert_eq!(
            lockfile,
            PnpmLock {
                lockfile_version: Value::Number(Number::from(5.4)),
                importers: HashMap::from([(".".into(), Value::Mapping(Mapping::new()))]),
                packages: HashMap::from([(
                    "/@ampproject/remapping/2.2.0".into(),
                    PnpmLockPackage {
                        dev: Some(true),
                        dependencies: Some(HashMap::from([
                            ("@jridgewell/gen-mapping".to_owned(), Value::String("0.1.1".to_owned())),
                            ("@jridgewell/trace-mapping".to_owned(), Value::String("0.3.14".to_owned()))
                        ])),
                        engines: Some(HashMap::from([
                            ("node".to_owned(), ">=6.0.0".to_owned())
                        ])),
                        resolution: Some(HashMap::from([
                            ("integrity".to_owned(), "sha512-qRmjj8nj9qmLTQXXmaR1cck3UXSRMPrbsLJAasZpF+t3riI71BXed5ebIOYwQntykeZuhjsdweEc9BxH5Jc26w==".to_owned())
                        ])),
                        ..PnpmLockPackage::default()
                    }
                ), (
                    "/@babel/plugin-syntax-async-generators/7.8.4_@babel+core@7.18.9".into(),
                    PnpmLockPackage {
                        dev: Some(true),
                        dependencies: Some(HashMap::from([
                            ("@babel/core".to_owned(), Value::Number(Number::from(7))),
                            ("@babel/helper-plugin-utils".to_owned(), Value::String("7.18.9".to_owned()))
                        ])),
                        peer_dependencies: Some(HashMap::from([(
                            "@babel/core".to_owned(),
                            Value::String("^7.0.0-0".to_owned())
                        )])),
                        resolution: Some(HashMap::from([
                            ("integrity".to_owned(), "sha512-tycmZxkGfZaxhMRbXlPXuVFpdWlXpir2W4AMhSJgRKzk/eDlIXOhb2LHWoLpDF7TEHylV5zNhykX6KAgHJmTNw==".to_owned())
                        ])),
                        ..PnpmLockPackage::default()
                    }
                ), (
                    "/array-union/2.1.0".into(),
                    PnpmLockPackage {
                        dev: Some(true),
                        engines: Some(HashMap::from([
                            ("node".to_owned(), ">=8".to_owned())
                        ])),
                        resolution: Some(HashMap::from([
                            ("integrity".to_owned(), "sha512-HGyxoOTYUyCM6stUe6EJgnd4EoewAI7zMdfqO+kGjnlZmBDz/cR5pf8r/cR4Wq60sL/p0IkcjUEEPwS3GFrIyw==".to_owned())
                        ])),
                        ..PnpmLockPackage::default()
                    }
                ), (
                    "/solid-jest/0.2.0_@babel+core@7.18.9".into(),
                    PnpmLockPackage {
                        dev: Some(true),
                        dependencies: Some(HashMap::from([
                            ("babel-jest".to_owned(), Value::String("27.5.1_@babel+core@7.18.9".to_owned())),
                            ("@babel/preset-env".to_owned(), Value::String("7.18.9_@babel+core@7.18.9".to_owned())),
                            ("enhanced-resolve-jest".to_owned(), Value::String("1.1.0".to_owned()))
                        ])),
                        peer_dependencies: Some(HashMap::from([(
                            "babel-preset-solid".to_owned(),
                            Value::String("^1.0.0".to_owned())
                        )])),
                        transitive_peer_dependencies: Some(string_vec!["@babel/core", "supports-color"]),
                        resolution: Some(HashMap::from([
                            ("integrity".to_owned(), "sha512-1ILtAj+z6bh1vTvaDlcT8501vmkzkVZMk2aiexJy+XWTZ+sb9B7IWedvWadIhOwwL97fiW4eMmN6SrbaHjn12A==".to_owned())
                        ])),
                        ..PnpmLockPackage::default()
                    }
                )]),
                ..PnpmLock::default()
            }
        );

        assert_eq!(
            load_lockfile_dependencies(temp.path().join("pnpm-lock.yaml")).unwrap(),
            HashMap::from([
                ("array-union".to_owned(), string_vec!["2.1.0"]),
                ("solid-jest".to_owned(), string_vec!["0.2.0"]),
                (
                    "@babel/plugin-syntax-async-generators".to_owned(),
                    string_vec!["7.8.4"]
                ),
                ("@ampproject/remapping".to_owned(), string_vec!["2.2.0"]),
            ])
        );

        temp.close().unwrap();
    }

    #[test]
    fn parses_complex_lockfile() {
        let content = reqwest::blocking::get(
            // TODO: this may change upstream
            "https://raw.githubusercontent.com/solidjs/solid/main/pnpm-lock.yaml",
        )
        .unwrap()
        .text()
        .unwrap();

        let _: PnpmLock = serde_yaml::from_str(&content).unwrap();
    }
}
