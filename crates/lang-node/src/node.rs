use crate::NODE;
use lazy_static::lazy_static;
use moon_lang::LangError;
use regex::Regex;
use std::env::{self, consts};
use std::fs;
use std::path::{Path, PathBuf};

lazy_static! {
    pub static ref BIN_PATH_PATTERN: Regex =
        Regex::new("(?:(?:\\.+\\\\))+(?:(?:[a-zA-Z0-9-_@]+)\\\\)+[a-zA-Z0-9-_]+(\\.(c|m)?js)?")
            .unwrap();
}

pub fn extend_node_options_env_var(next: &str) -> String {
    match env::var("NODE_OPTIONS") {
        Ok(prev) => format!("{} {}", prev, next),
        Err(_) => next.to_owned(),
    }
}

#[track_caller]
pub fn extract_bin_from_cmd_file(bin_path: &Path, contents: &str) -> PathBuf {
    let captures = BIN_PATH_PATTERN.captures(contents).unwrap_or_else(|| {
        // This should ideally never happen!
        panic!(
            "Unable to extract binary path from {}.",
            bin_path.to_string_lossy()
        )
    });

    PathBuf::from(captures.get(0).unwrap().as_str())
}

pub fn find_package(starting_dir: &Path, package_name: &str) -> Option<PathBuf> {
    let pkg_path = starting_dir.join(NODE.vendor_dir).join(package_name);

    if pkg_path.exists() {
        return Some(pkg_path);
    }

    match starting_dir.parent() {
        Some(dir) => find_package(dir, package_name),
        None => None,
    }
}

#[track_caller]
pub fn find_package_bin(starting_dir: &Path, bin_name: &str) -> Option<PathBuf> {
    let bin_path = starting_dir
        .join(NODE.vendor_bins_dir)
        .join(get_bin_name_suffix(bin_name, "cmd", true));

    if bin_path.exists() {
        if bin_path.ends_with(".cmd") {
            let contents = fs::read_to_string(&bin_path).unwrap();

            return Some(
                bin_path
                    .parent()
                    .unwrap()
                    .join(extract_bin_from_cmd_file(&bin_path, &contents))
                    .canonicalize()
                    .unwrap(),
            );
        }

        return Some(bin_path);
    }

    match starting_dir.parent() {
        Some(dir) => find_package_bin(dir, bin_name),
        None => None,
    }
}

pub fn find_package_manager_bin(install_dir: &Path, bin_name: &str) -> PathBuf {
    install_dir.join(get_bin_name_suffix(bin_name, "cmd", false))
}

pub fn get_bin_name_suffix(name: &str, windows_ext: &str, flat: bool) -> String {
    if cfg!(windows) {
        format!("{}.{}", name, windows_ext)
    } else if flat {
        name.to_owned()
    } else {
        format!("bin/{}", name)
    }
}

pub fn get_download_file_ext() -> &'static str {
    if consts::OS == "windows" {
        "zip"
    } else {
        "tar.gz"
    }
}

pub fn get_download_file_name(version: &str) -> Result<String, LangError> {
    let platform;

    if consts::OS == "linux" {
        platform = "linux"
    } else if consts::OS == "windows" {
        platform = "win";
    } else if consts::OS == "macos" {
        platform = "darwin"
    } else {
        return Err(LangError::UnsupportedPlatform(
            consts::OS.to_string(),
            String::from("Node.js"),
        ));
    }

    let arch;

    if consts::ARCH == "x86" {
        arch = "x86"
    } else if consts::ARCH == "x86_64" {
        arch = "x64"
    } else if consts::ARCH == "arm" || consts::ARCH == "aarch64" {
        arch = "arm64"
    // } else if consts::ARCH == "powerpc64" {
    //     arch = "ppc64le"
    // } else if consts::ARCH == "s390x" {
    //     arch = "s390x"
    } else {
        return Err(LangError::UnsupportedArchitecture(
            consts::ARCH.to_string(),
            String::from("Node.js"),
        ));
    }

    Ok(format!(
        "node-v{version}-{platform}-{arch}",
        version = version,
        platform = platform,
        arch = arch,
    ))
}

pub fn get_download_file(version: &str) -> Result<String, LangError> {
    Ok(format!(
        "{}.{}",
        get_download_file_name(version)?,
        get_download_file_ext()
    ))
}

pub fn get_nodejs_url(version: &str, host: &str, path: &str) -> String {
    format!(
        "{host}/dist/v{version}/{path}",
        host = host,
        version = version,
        path = path,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use assert_fs::TempDir;

    fn create_node_modules_sandbox() -> TempDir {
        let sandbox = TempDir::new().unwrap();

        sandbox
            .child("node_modules/@scope/pkg-foo/package.json")
            .write_str("{}")
            .unwrap();

        sandbox
            .child("node_modules/pkg-bar/package.json")
            .write_str("{}")
            .unwrap();

        sandbox
            .child("node_modules/.bin/baz")
            .write_str("{}")
            .unwrap();

        sandbox
            .child("node_modules/.bin/baz.cmd")
            .write_str("{}")
            .unwrap();

        sandbox.child("nested/file.js").write_str("{}").unwrap();

        sandbox
    }

    mod extend_node_options_env_var {
        use super::*;
        use serial_test::serial;

        #[serial]
        #[test]
        fn returns_value_if_not_set() {
            env::remove_var("NODE_OPTIONS");

            assert_eq!(extend_node_options_env_var("--arg"), String::from("--arg"));
        }

        #[serial]
        #[test]
        fn combines_value_if_set() {
            env::set_var("NODE_OPTIONS", "--base");

            assert_eq!(
                extend_node_options_env_var("--arg"),
                String::from("--base --arg")
            );

            env::remove_var("NODE_OPTIONS");
        }
    }

    mod extract_bin_from_cmd_file {
        use super::*;

        fn create_cmd(path: &str) -> String {
            format!(
                r#"
@IF EXIST "%~dp0\node.exe" (
    "%~dp0\node.exe" "%~dp0\{path}" %*
) ELSE (
    SETLOCAL
    SET PATHEXT=%PATHEXT:;.JS;=;%
    node "%~dp0\{path}" %*
)"#,
                path = path
            )
        }

        #[test]
        fn basic_path() {
            assert_eq!(
                extract_bin_from_cmd_file(
                    &PathBuf::from("test.cmd"),
                    &create_cmd(r"..\typescript\bin\tsc"),
                ),
                PathBuf::from(r"..\typescript\bin\tsc")
            );
        }

        #[test]
        fn relative_paths() {
            assert_eq!(
                extract_bin_from_cmd_file(
                    &PathBuf::from("test.cmd"),
                    &create_cmd(r".\eslint\bin\eslint"),
                ),
                PathBuf::from(r".\eslint\bin\eslint")
            );

            assert_eq!(
                extract_bin_from_cmd_file(
                    &PathBuf::from("test.cmd"),
                    &create_cmd(r"..\..\eslint\bin\eslint"),
                ),
                PathBuf::from(r"..\..\eslint\bin\eslint")
            );
        }

        #[test]
        fn with_exts() {
            assert_eq!(
                extract_bin_from_cmd_file(
                    &PathBuf::from("test.cmd"),
                    &create_cmd(r"..\babel\index.js"),
                ),
                PathBuf::from(r"..\babel\index.js")
            );

            assert_eq!(
                extract_bin_from_cmd_file(
                    &PathBuf::from("test.cmd"),
                    &create_cmd(r".\webpack\dist\cli.cjs"),
                ),
                PathBuf::from(r".\webpack\dist\cli.cjs")
            );

            assert_eq!(
                extract_bin_from_cmd_file(
                    &PathBuf::from("test.cmd"),
                    &create_cmd(r".\..\rollup\build\rollup.mjs"),
                ),
                PathBuf::from(r".\..\rollup\build\rollup.mjs")
            );
        }

        #[test]
        fn with_scopes() {
            assert_eq!(
                extract_bin_from_cmd_file(
                    &PathBuf::from("test.cmd"),
                    &create_cmd(r"..\@scope\foo\bin"),
                ),
                PathBuf::from(r"..\@scope\foo\bin")
            );

            assert_eq!(
                extract_bin_from_cmd_file(
                    &PathBuf::from("test.cmd"),
                    &create_cmd(r"..\@scope\foo-bar\bin.js"),
                ),
                PathBuf::from(r"..\@scope\foo-bar\bin.js")
            );

            assert_eq!(
                extract_bin_from_cmd_file(
                    &PathBuf::from("test.cmd"),
                    &create_cmd(r"..\@scope-long\foo-bar\bin_file.js"),
                ),
                PathBuf::from(r"..\@scope-long\foo-bar\bin_file.js")
            );
        }
    }

    mod get_bin_name_suffix {
        use super::*;

        #[test]
        #[cfg(windows)]
        fn supports_cmd() {
            assert_eq!(
                get_bin_name_suffix("foo", "cmd", false),
                "foo.cmd".to_owned()
            );
        }

        #[test]
        #[cfg(windows)]
        fn supports_ps1() {
            assert_eq!(
                get_bin_name_suffix("foo", "ps1", false),
                "foo.ps1".to_owned()
            );
        }

        #[test]
        #[cfg(windows)]
        fn supports_exe() {
            assert_eq!(
                get_bin_name_suffix("foo", "exe", true),
                "foo.exe".to_owned()
            );
        }

        #[test]
        #[cfg(not(windows))]
        fn returns_nested_bin() {
            assert_eq!(
                get_bin_name_suffix("foo", "ext", false),
                "bin/foo".to_owned()
            );
        }

        #[test]
        #[cfg(not(windows))]
        fn returns_flat_bin() {
            assert_eq!(get_bin_name_suffix("foo", "ext", true), "foo".to_owned());
        }
    }

    mod find_package {
        use super::*;

        #[test]
        fn returns_path_with_package_scope() {
            let sandbox = create_node_modules_sandbox();
            let path = find_package(sandbox.path(), "@scope/pkg-foo");

            assert_eq!(
                path.unwrap(),
                sandbox.path().join("node_modules").join("@scope/pkg-foo")
            );
        }

        #[test]
        fn returns_path_without_package_scope() {
            let sandbox = create_node_modules_sandbox();
            let path = find_package(sandbox.path(), "pkg-bar");

            assert_eq!(
                path.unwrap(),
                sandbox.path().join("node_modules").join("pkg-bar")
            );
        }

        #[test]
        fn returns_path_from_nested_file() {
            let sandbox = create_node_modules_sandbox();
            let path = find_package(&sandbox.path().join("nested"), "@scope/pkg-foo");

            assert_eq!(
                path.unwrap(),
                sandbox.path().join("node_modules").join("@scope/pkg-foo")
            );
        }

        #[test]
        fn returns_none_for_missing() {
            let sandbox = create_node_modules_sandbox();
            let path = find_package(sandbox.path(), "unknown-pkg");

            assert_eq!(path, None);
        }
    }

    mod find_package_bin {
        use super::*;

        #[test]
        fn returns_path_if_found() {
            let sandbox = create_node_modules_sandbox();
            let path = find_package_bin(sandbox.path(), "baz");

            assert_eq!(
                path.unwrap(),
                sandbox
                    .path()
                    .join("node_modules/.bin")
                    .join(if consts::OS == "windows" {
                        "baz.cmd"
                    } else {
                        "baz"
                    })
            );
        }

        #[test]
        fn returns_path_from_nested_file() {
            let sandbox = create_node_modules_sandbox();
            let path = find_package_bin(&sandbox.path().join("nested"), "baz");

            assert_eq!(
                path.unwrap(),
                sandbox
                    .path()
                    .join("node_modules/.bin")
                    .join(if consts::OS == "windows" {
                        "baz.cmd"
                    } else {
                        "baz"
                    })
            );
        }

        #[test]
        fn returns_none_for_missing() {
            let sandbox = create_node_modules_sandbox();
            let path = find_package_bin(sandbox.path(), "unknown-binary");

            assert_eq!(path, None);
        }
    }
}
