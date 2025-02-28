use cached::proc_macro::cached;
use miette::IntoDiagnostic;
use moon_lang::{LockfileDependencyVersions, config_cache_container};
use package_lock_json_parser::{PackageLockJson, parse};
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::path::{Path, PathBuf};

fn read_file(path: &Path) -> miette::Result<PackageLockJson> {
    parse(fs::read_file(path)?).into_diagnostic()
}

config_cache_container!(
    PackageLockJsonCache,
    PackageLockJson,
    "package-lock.json",
    read_file
);

// https://docs.npmjs.com/cli/v9/configuring-npm/package-lock-json?v=true
#[cached(result)]
pub fn load_lockfile_dependencies(path: PathBuf) -> miette::Result<LockfileDependencyVersions> {
    let mut deps: LockfileDependencyVersions = FxHashMap::default();

    let mut add_dep = |name: &str, version: &str, integrity: Option<&String>| {
        if !name.is_empty() {
            let mut list = vec![];

            if let Some(int) = integrity {
                list.push(int.to_owned());
            } else {
                list.push(version.to_owned());
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

                add_dep(resolved_name, &dep.version, dep.integrity.as_ref());

                // workspaces/libnpmdiff
            } else if name.starts_with("workspaces") {
                let name_parts = name.split("workspaces/");
                let resolved_name = name_parts.last().unwrap_or_default();

                add_dep(resolved_name, &dep.version, dep.integrity.as_ref());

                // other
            } else if !name.is_empty() {
                add_dep(&name, &dep.version, dep.integrity.as_ref());
            }
        }

        // v1, v2
        if !has_packages {
            // This isn't entirely accurate as npm does not hoist all dependencies
            // to the root of the lockfile. We'd need to recursively extract everything,
            // but for now, this will get us most of the way.
            for (name, dep) in lockfile.dependencies.unwrap_or_default() {
                add_dep(&name, &dep.version, dep.integrity.as_ref());
            }
        }
    }

    Ok(deps)
}

#[cfg(test)]
#[allow(clippy::disallowed_types)]
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
                version: Some("1.2.3".into()),
                lockfile_version: 2,
                dependencies: Some(HashMap::from_iter([(
                    "@babel/helper-function-name".to_owned(),
                    V1Dependency {
                        integrity: Some("sha512-fJgWlZt7nxGksJS9a0XdSaI4XvpExnNIgRP+rVefWh5U7BL8pPuir6SJUmFKRfjWQ51OtWSzwOxhaH/EBWWc0A==".into()),
                        requires: Some(HashMap::from_iter([
                            ("@babel/template".to_owned(), "^7.18.6".to_owned()),
                            ("@babel/types".to_owned(), "^7.18.9".to_owned())
                        ])),
                        resolved: Some("https://registry.npmjs.org/@babel/helper-function-name/-/helper-function-name-7.18.9.tgz".into()),
                        version: "7.18.9".into(),
                        ..V1Dependency::default()
                    }
                ), (
                    "rollup-plugin-polyfill-node".to_owned(),
                    V1Dependency {
                        is_dev: true,
                        integrity: Some("sha512-5GMywXiLiuQP6ZzED/LO/Q0HyDi2W6b8VN+Zd3oB0opIjyRs494Me2ZMaqKWDNbGiW4jvvzl6L2n4zRgxS9cSQ==".into()),
                        requires: Some(HashMap::from_iter([
                            ("@rollup/plugin-inject".to_owned(), "^4.0.0".to_owned())
                        ])),
                        resolved: Some("https://registry.npmjs.org/rollup-plugin-polyfill-node/-/rollup-plugin-polyfill-node-0.10.2.tgz".into()),
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
                    string_vec![
                        "sha512-fJgWlZt7nxGksJS9a0XdSaI4XvpExnNIgRP+rVefWh5U7BL8pPuir6SJUmFKRfjWQ51OtWSzwOxhaH/EBWWc0A=="
                    ]
                ),
                (
                    "rollup-plugin-polyfill-node".to_owned(),
                    string_vec![
                        "sha512-5GMywXiLiuQP6ZzED/LO/Q0HyDi2W6b8VN+Zd3oB0opIjyRs494Me2ZMaqKWDNbGiW4jvvzl6L2n4zRgxS9cSQ=="
                    ]
                ),
            ])
        );

        temp.close().unwrap();
    }

    #[test]
    fn parses_v3_lockfile() {
        let temp = create_temp_dir();

        temp.child("package-lock.json")
            .write_str(
                r#"
{
    "name": "moon-examples",
    "version": "1.2.3",
    "lockfileVersion": 3,
    "requires": true,
    "packages": {
        "node_modules/tap/node_modules/yaml": {
            "version": "1.10.2",
            "dev": true,
            "inBundle": true,
            "license": "ISC",
            "engines": {
                "node": ">= 6"
            }
        },
        "node_modules/yaml": {
            "version": "2.2.2",
            "resolved": "https://registry.npmjs.org/yaml/-/yaml-2.2.2.tgz",
            "integrity": "sha512-CBKFWExMn46Foo4cldiChEzn7S7SRV+wqiluAb6xmueD/fGyRHIhX8m14vVGgeFWjN540nKCNVj6P21eQjgTuA==",
            "dev": true,
            "engines": {
                "node": ">= 14"
            }
        },
        "workspaces/libnpmdiff": {
            "version": "5.0.17",
            "license": "ISC",
            "dependencies": {
                "@npmcli/arborist": "^6.2.9",
                "@npmcli/disparity-colors": "^3.0.0",
                "@npmcli/installed-package-contents": "^2.0.2",
                "binary-extensions": "^2.2.0",
                "diff": "^5.1.0",
                "minimatch": "^9.0.0",
                "npm-package-arg": "^10.1.0",
                "pacote": "^15.0.8",
                "tar": "^6.1.13"
            },
            "devDependencies": {
                "@npmcli/eslint-config": "^4.0.0",
                "@npmcli/template-oss": "4.14.1",
                "tap": "^16.3.4"
            },
            "engines": {
                "node": "^14.17.0 || ^16.13.0 || >=18.0.0"
            }
        }
    }
}"#,
            )
            .unwrap();

        let _: PackageLockJson = read_file(&temp.path().join("package-lock.json")).unwrap();

        assert_eq!(
            load_lockfile_dependencies(temp.path().join("package-lock.json")).unwrap(),
            FxHashMap::from_iter([
                (
                    "yaml".to_owned(),
                    string_vec![
                        "sha512-CBKFWExMn46Foo4cldiChEzn7S7SRV+wqiluAb6xmueD/fGyRHIhX8m14vVGgeFWjN540nKCNVj6P21eQjgTuA=="
                    ]
                ),
                ("libnpmdiff".to_owned(), string_vec!["5.0.17"])
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
