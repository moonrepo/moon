use crate::NPM;
use cached::proc_macro::cached;
use moon_error::MoonError;
use moon_lang::{config_cache, LockfileDependencyVersions};
use moon_utils::json::read as read_json;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

config_cache!(PackageLock, NPM.lock_filename, read_json);

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageLockDependency {
    pub dependencies: Option<FxHashMap<String, PackageLockDependency>>,
    pub dev: Option<bool>,
    pub integrity: Option<String>,
    pub requires: Option<FxHashMap<String, String>>,
    pub resolved: Option<String>,
    pub version: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageLock {
    pub name: String,
    pub dependencies: Option<FxHashMap<String, PackageLockDependency>>,

    #[serde(skip)]
    pub path: PathBuf,
}

#[cached(result)]
pub fn load_lockfile_dependencies(path: PathBuf) -> Result<LockfileDependencyVersions, MoonError> {
    let mut deps: LockfileDependencyVersions = FxHashMap::default();

    if let Some(lockfile) = PackageLock::read(path)? {
        // TODO: This isn't entirely accurate as npm does not hoist all dependencies
        // to the root of the lockfile. We'd need to recursively extract everything,
        // but for now, this will get us most of the way.
        for (name, dep) in lockfile.dependencies.unwrap_or_default() {
            // This is a workspace project, so we don't care about its version
            if dep.version.contains(':') {
                continue;
            }

            // Most packages have `integrity` field. Exception is when you install from
            // source directly: `npm install --save-dev
            // https://github.com/fergiemcdowall/search-index.git`. In that case, we use
            // the `version` field instead.
            let to_push = if let Some(integrity) = dep.integrity {
                integrity
            } else {
                dep.version
            };
            if let Some(versions) = deps.get_mut(&name) {
                versions.push(to_push);
            } else {
                deps.insert(name, vec![to_push]);
            }
        }
    }
    Ok(deps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_test_utils::{assert_fs::prelude::*, create_temp_dir};
    use moon_utils::string_vec;
    use pretty_assertions::assert_eq;

    #[test]
    fn parses_lockfile() {
        let temp = create_temp_dir();

        temp.child("package-lock.json")
            .write_str(r#"
{
    "name": "moon-examples",
    "lockfileVersion": 2,
    "requires": true,
    "dependencies": {
        "@babel/helper-function-name": {
            "version": "7.18.9",
            "resolved": "https://registry.npmjs.org/@babel/helper-function-name/-/helper-function-name-7.18.9.tgz",
            "integrity": "sha512-fJgWlZt7nxGksJS9a0XdSaI4XvpExnNIgRP+rVefWh5U7BL8pPuir6SJUmFKRfjWQ51OtWSzwOxhaH/EBWWc0A==",
            "requires": {
              "@babel/template": "^7.18.6",
              "@babel/types": "^7.18.9"
            }
        },
        "rollup-plugin-polyfill-node": {
            "version": "0.10.2",
            "resolved": "https://registry.npmjs.org/rollup-plugin-polyfill-node/-/rollup-plugin-polyfill-node-0.10.2.tgz",
            "integrity": "sha512-5GMywXiLiuQP6ZzED/LO/Q0HyDi2W6b8VN+Zd3oB0opIjyRs494Me2ZMaqKWDNbGiW4jvvzl6L2n4zRgxS9cSQ==",
            "dev": true,
            "requires": {
                "@rollup/plugin-inject": "^4.0.0"
            }
        }
    }
}"#,
            )
            .unwrap();

        let lockfile: PackageLock = read_json(temp.path().join("package-lock.json")).unwrap();

        assert_eq!(
            lockfile,
            PackageLock {
                name: "moon-examples".into(),
                dependencies: Some(FxHashMap::from_iter([(
                    "@babel/helper-function-name".to_owned(),
                    PackageLockDependency {
                        integrity: Some("sha512-fJgWlZt7nxGksJS9a0XdSaI4XvpExnNIgRP+rVefWh5U7BL8pPuir6SJUmFKRfjWQ51OtWSzwOxhaH/EBWWc0A==".into()),
                        requires: Some(FxHashMap::from_iter([
                            ("@babel/template".to_owned(), "^7.18.6".to_owned()),
                            ("@babel/types".to_owned(), "^7.18.9".to_owned())
                        ])),
                        resolved: Some("https://registry.npmjs.org/@babel/helper-function-name/-/helper-function-name-7.18.9.tgz".into()),
                        version: "7.18.9".into(),
                        ..PackageLockDependency::default()
                    }
                ), (
                    "rollup-plugin-polyfill-node".to_owned(),
                    PackageLockDependency {
                        dev: Some(true),
                        integrity: Some("sha512-5GMywXiLiuQP6ZzED/LO/Q0HyDi2W6b8VN+Zd3oB0opIjyRs494Me2ZMaqKWDNbGiW4jvvzl6L2n4zRgxS9cSQ==".into()),
                        requires: Some(FxHashMap::from_iter([
                            ("@rollup/plugin-inject".to_owned(), "^4.0.0".to_owned())
                        ])),
                        resolved: Some("https://registry.npmjs.org/rollup-plugin-polyfill-node/-/rollup-plugin-polyfill-node-0.10.2.tgz".into()),
                        version: "0.10.2".into(),
                        ..PackageLockDependency::default()
                    }
                )])),
                ..PackageLock::default()
            }
        );

        assert_eq!(
            load_lockfile_dependencies(temp.path().join("package-lock.json")).unwrap(),
            FxHashMap::from_iter([
                (
                    "@babel/helper-function-name".to_owned(),
                    string_vec!["sha512-fJgWlZt7nxGksJS9a0XdSaI4XvpExnNIgRP+rVefWh5U7BL8pPuir6SJUmFKRfjWQ51OtWSzwOxhaH/EBWWc0A=="]

                ),
                (
                    "rollup-plugin-polyfill-node".to_owned(),
                    string_vec!["sha512-5GMywXiLiuQP6ZzED/LO/Q0HyDi2W6b8VN+Zd3oB0opIjyRs494Me2ZMaqKWDNbGiW4jvvzl6L2n4zRgxS9cSQ=="]
                ),
            ])
        );

        temp.close().unwrap();
    }

    #[test]
    fn parses_complex_lockfile() {
        let content = reqwest::blocking::get(
            "https://raw.githubusercontent.com/moonrepo/examples/master/package-lock.json",
        )
        .unwrap()
        .text()
        .unwrap();

        let _: PackageLock = serde_json::from_str(&content).unwrap();
    }
}
