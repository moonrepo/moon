use crate::NPM;
use cached::proc_macro::cached;
use moon_error::MoonError;
use moon_lang::{config_cache_container, LockfileDependencyVersions};
use package_lock_json_parser::{parse, PackageLockJson};
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::path::{Path, PathBuf};

fn read_file(path: &Path) -> Result<PackageLockJson, MoonError> {
    Ok(parse(fs::read_file(path)?).map_err(|e| MoonError::Generic(e.to_string()))?)
}

config_cache_container!(
    PackageLockJsonCache,
    PackageLockJson,
    NPM.lockfile,
    read_file
);

// https://docs.npmjs.com/cli/v9/configuring-npm/package-lock-json?v=true
#[cached(result)]
pub fn load_lockfile_dependencies(path: PathBuf) -> Result<LockfileDependencyVersions, MoonError> {
    let mut deps: LockfileDependencyVersions = FxHashMap::default();

    let mut add_dep = |name: &str, version: &str, integrity: &str| {
        if !name.is_empty() {
            let mut list = vec![];

            if integrity.is_empty() {
                list.push(version.to_owned());
            } else {
                list.push(integrity.to_owned());
            }

            deps.entry(name.to_owned())
                .and_modify(|data| {
                    data.extend(list.clone());
                })
                .or_insert(list);
        }
    };

    if let Some(lockfile) = PackageLockJsonCache::read(path)? {
        let has_packages = lockfile.packages.is_some();

        // v2, v3
        for (name, dep) in lockfile.packages.unwrap_or_default() {
            // node_modules/cacache
            // node_modules/node-gyp/node_modules/cacache
            if name.starts_with("node_modules") {
                let name_parts = name.split("node_modules/");
                let resolved_name = name_parts.last().unwrap_or_default();

                add_dep(resolved_name, &dep.version, &dep.integrity);

                // workspaces/libnpmdiff
            } else if name.starts_with("workspaces") {
                let name_parts = name.split("workspaces/");
                let resolved_name = name_parts.last().unwrap_or_default();

                add_dep(resolved_name, &dep.version, &dep.integrity);
            }
        }

        // v1, v2
        if !has_packages {
            // This isn't entirely accurate as npm does not hoist all dependencies
            // to the root of the lockfile. We'd need to recursively extract everything,
            // but for now, this will get us most of the way.
            for (name, dep) in lockfile.dependencies.unwrap_or_default() {
                add_dep(&name, &dep.version, &dep.integrity);
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
    use package_lock_json_parser::V1Dependency;
    use std::collections::HashMap;

    #[test]
    fn parses_lockfile() {
        let temp = create_temp_dir();

        temp.child("package-lock.json")
            .write_str(r#"
{
    "name": "moon-examples",
    "version": "1.2.3",
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

        let lockfile: PackageLockJson = read_file(&temp.path().join("package-lock.json")).unwrap();

        assert_eq!(
            lockfile,
            PackageLockJson {
                name: "moon-examples".into(),
                version: "1.2.3".into(),
                lockfile_version: 2,
                dependencies: Some(HashMap::from_iter([(
                    "@babel/helper-function-name".to_owned(),
                    V1Dependency {
                        integrity: "sha512-fJgWlZt7nxGksJS9a0XdSaI4XvpExnNIgRP+rVefWh5U7BL8pPuir6SJUmFKRfjWQ51OtWSzwOxhaH/EBWWc0A==".into(),
                        requires: Some(HashMap::from_iter([
                            ("@babel/template".to_owned(), "^7.18.6".to_owned()),
                            ("@babel/types".to_owned(), "^7.18.9".to_owned())
                        ])),
                        resolved: "https://registry.npmjs.org/@babel/helper-function-name/-/helper-function-name-7.18.9.tgz".into(),
                        version: "7.18.9".into(),
                        ..V1Dependency::default()
                    }
                ), (
                    "rollup-plugin-polyfill-node".to_owned(),
                    V1Dependency {
                        is_dev: true,
                        integrity: "sha512-5GMywXiLiuQP6ZzED/LO/Q0HyDi2W6b8VN+Zd3oB0opIjyRs494Me2ZMaqKWDNbGiW4jvvzl6L2n4zRgxS9cSQ==".into(),
                        requires: Some(HashMap::from_iter([
                            ("@rollup/plugin-inject".to_owned(), "^4.0.0".to_owned())
                        ])),
                        resolved: "https://registry.npmjs.org/rollup-plugin-polyfill-node/-/rollup-plugin-polyfill-node-0.10.2.tgz".into(),
                        version: "0.10.2".into(),
                        ..V1Dependency::default()
                    }
                )])),
                ..PackageLockJson::default()
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
            "https://raw.githubusercontent.com/npm/cli/latest/package-lock.json",
        )
        .unwrap()
        .text()
        .unwrap();

        let _: PackageLockJson = serde_json::from_str(&content).unwrap();
    }
}
