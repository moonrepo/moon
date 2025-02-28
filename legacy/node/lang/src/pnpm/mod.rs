// https://github.com/pnpm/pnpm/blob/main/lockfile/lockfile-types/src/index.ts

pub mod dependency_path;
pub mod workspace;

use cached::proc_macro::cached;
use dependency_path::PnpmDependencyPath;
use moon_lang::{LockfileDependencyVersions, config_cache};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_yml::Value;
use starbase_utils::yaml::read_file as read_yaml;
use std::path::{Path, PathBuf};

config_cache!(PnpmLock, "pnpm-lock.yaml", read_yaml);

type DependencyMap = FxHashMap<String, Value>;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PnpmLockPackage {
    pub cpu: Option<Vec<String>>,
    pub dependencies: Option<DependencyMap>,
    pub deprecated: Option<String>,
    pub dev: Option<bool>,
    pub engines: Option<FxHashMap<String, String>>,
    pub has_bin: Option<bool>,
    pub libc: Option<Vec<String>>,
    pub optional: Option<bool>,
    pub optional_dependencies: Option<DependencyMap>,
    pub os: Option<Vec<String>>,
    pub patched: Option<bool>,
    pub peer_dependencies: Option<DependencyMap>,
    pub prepare: Option<bool>,
    pub requires_build: Option<bool>,
    pub transitive_peer_dependencies: Option<Vec<String>>,
    pub resolution: PnpmLockResolution,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PnpmLockResolution {
    pub commit: Option<String>, // git
    pub integrity: Option<String>,
    pub tarball: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PnpmLock {
    pub lockfile_version: Value,
    pub importers: Option<FxHashMap<String, Value>>,
    pub packages: Option<FxHashMap<String, PnpmLockPackage>>,

    #[serde(skip)]
    pub path: PathBuf,
}

#[cached(result)]
pub fn load_lockfile_dependencies(path: PathBuf) -> miette::Result<LockfileDependencyVersions> {
    let mut deps: LockfileDependencyVersions = FxHashMap::default();

    if let Some(lockfile) = PnpmLock::read(path)? {
        if let Some(packages) = lockfile.packages {
            for (package_name, details) in packages {
                let parsed_dependency = PnpmDependencyPath::parse(&package_name);
                let entry = deps
                    .entry(parsed_dependency.name.unwrap_or_default())
                    .or_default();

                if let Some(ver) = details.resolution.integrity {
                    entry.push(ver.clone());
                }

                if let Some(ver) = details.resolution.tarball {
                    entry.push(ver.clone());
                }

                if let Some(ver) = details.resolution.commit {
                    entry.push(ver.clone());
                }
            }
        }
    }

    Ok(deps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_test_utils::{assert_fs::prelude::*, create_temp_dir, pretty_assertions::assert_eq};
    use moon_utils::string_vec;
    use serde_yml::{Mapping, Number};

    #[test]
    fn parses_lockfile() {
        let temp = create_temp_dir();

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
                importers: Some(FxHashMap::from_iter([(".".into(), Value::Mapping(Mapping::new()))])),
                packages: Some(FxHashMap::from_iter([(
                    "/@ampproject/remapping/2.2.0".into(),
                    PnpmLockPackage {
                        dev: Some(true),
                        dependencies: Some(FxHashMap::from_iter([
                            ("@jridgewell/gen-mapping".to_owned(), Value::String("0.1.1".to_owned())),
                            ("@jridgewell/trace-mapping".to_owned(), Value::String("0.3.14".to_owned()))
                        ])),
                        engines: Some(FxHashMap::from_iter([
                            ("node".to_owned(), ">=6.0.0".to_owned())
                        ])),
                        resolution:
                        PnpmLockResolution { commit: None, integrity: Some("sha512-qRmjj8nj9qmLTQXXmaR1cck3UXSRMPrbsLJAasZpF+t3riI71BXed5ebIOYwQntykeZuhjsdweEc9BxH5Jc26w==".to_owned()), tarball: None },
                        ..PnpmLockPackage::default()
                    }
                ), (
                    "/@babel/plugin-syntax-async-generators/7.8.4_@babel+core@7.18.9".into(),
                    PnpmLockPackage {
                        dev: Some(true),
                        dependencies: Some(FxHashMap::from_iter([
                            ("@babel/core".to_owned(), Value::Number(Number::from(7))),
                            ("@babel/helper-plugin-utils".to_owned(), Value::String("7.18.9".to_owned()))
                        ])),
                        peer_dependencies: Some(FxHashMap::from_iter([(
                            "@babel/core".to_owned(),
                            Value::String("^7.0.0-0".to_owned())
                        )])),
                        resolution:
                        PnpmLockResolution { commit: None,integrity: Some( "sha512-tycmZxkGfZaxhMRbXlPXuVFpdWlXpir2W4AMhSJgRKzk/eDlIXOhb2LHWoLpDF7TEHylV5zNhykX6KAgHJmTNw==".to_owned()), tarball: None },
                        ..PnpmLockPackage::default()
                    }
                ), (
                    "/array-union/2.1.0".into(),
                    PnpmLockPackage {
                        dev: Some(true),
                        engines: Some(FxHashMap::from_iter([
                            ("node".to_owned(), ">=8".to_owned())
                        ])),
                        resolution:
                        PnpmLockResolution { commit: None,integrity: Some( "sha512-HGyxoOTYUyCM6stUe6EJgnd4EoewAI7zMdfqO+kGjnlZmBDz/cR5pf8r/cR4Wq60sL/p0IkcjUEEPwS3GFrIyw==".to_owned()), tarball: None },
                        ..PnpmLockPackage::default()
                    }
                ), (
                    "/solid-jest/0.2.0_@babel+core@7.18.9".into(),
                    PnpmLockPackage {
                        dev: Some(true),
                        dependencies: Some(FxHashMap::from_iter([
                            ("babel-jest".to_owned(), Value::String("27.5.1_@babel+core@7.18.9".to_owned())),
                            ("@babel/preset-env".to_owned(), Value::String("7.18.9_@babel+core@7.18.9".to_owned())),
                            ("enhanced-resolve-jest".to_owned(), Value::String("1.1.0".to_owned()))
                        ])),
                        peer_dependencies: Some(FxHashMap::from_iter([(
                            "babel-preset-solid".to_owned(),
                            Value::String("^1.0.0".to_owned())
                        )])),
                        transitive_peer_dependencies: Some(string_vec!["@babel/core", "supports-color"]),
                        resolution:
                        PnpmLockResolution { commit: None,integrity: Some( "sha512-1ILtAj+z6bh1vTvaDlcT8501vmkzkVZMk2aiexJy+XWTZ+sb9B7IWedvWadIhOwwL97fiW4eMmN6SrbaHjn12A==".to_owned()), tarball: None },
                        ..PnpmLockPackage::default()
                    }
                )])),
                ..PnpmLock::default()
            }
        );

        assert_eq!(
            load_lockfile_dependencies(temp.path().join("pnpm-lock.yaml")).unwrap(),
            FxHashMap::from_iter([
                (
                    "array-union".to_owned(),
                    string_vec![
                        "sha512-HGyxoOTYUyCM6stUe6EJgnd4EoewAI7zMdfqO+kGjnlZmBDz/cR5pf8r/cR4Wq60sL/p0IkcjUEEPwS3GFrIyw=="
                    ]
                ),
                (
                    "solid-jest".to_owned(),
                    string_vec![
                        "sha512-1ILtAj+z6bh1vTvaDlcT8501vmkzkVZMk2aiexJy+XWTZ+sb9B7IWedvWadIhOwwL97fiW4eMmN6SrbaHjn12A=="
                    ]
                ),
                (
                    "@babel/plugin-syntax-async-generators".to_owned(),
                    string_vec![
                        "sha512-tycmZxkGfZaxhMRbXlPXuVFpdWlXpir2W4AMhSJgRKzk/eDlIXOhb2LHWoLpDF7TEHylV5zNhykX6KAgHJmTNw=="
                    ]
                ),
                (
                    "@ampproject/remapping".to_owned(),
                    string_vec![
                        "sha512-qRmjj8nj9qmLTQXXmaR1cck3UXSRMPrbsLJAasZpF+t3riI71BXed5ebIOYwQntykeZuhjsdweEc9BxH5Jc26w=="
                    ]
                ),
            ])
        );

        temp.close().unwrap();
    }

    #[test]
    fn parses_complex_lockfile() {
        let content = reqwest::blocking::get(
            "https://raw.githubusercontent.com/pnpm/pnpm/main/pnpm-lock.yaml",
        )
        .unwrap()
        .text()
        .unwrap();

        let _: PnpmLock = serde_yml::from_str(&content).unwrap();
    }

    #[test]
    fn parses_empty_lockfile() {
        let _: PnpmLock = serde_yml::from_str("lockfileVersion: '6.0'").unwrap();
    }
}
