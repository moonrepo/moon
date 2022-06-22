use crate::NODE;
use moon_lang::LangError;
use std::env::{self, consts};
use std::path::{Path, PathBuf};

pub fn extend_node_options_env_var(next: &str) -> String {
    match env::var("NODE_OPTIONS") {
        Ok(prev) => format!("{} {}", prev, next),
        Err(_) => String::from(next),
    }
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

pub fn find_package_bin(starting_dir: &Path, bin_name: &str) -> Option<PathBuf> {
    let bin_path = starting_dir
        .join(NODE.vendor_bins_dir)
        .join(get_bin_name_suffix(bin_name, "cmd", true));

    if bin_path.exists() {
        return Some(bin_path);
    }

    match starting_dir.parent() {
        Some(dir) => find_package_bin(dir, bin_name),
        None => None,
    }
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

    // Working dir is within the crate
    fn get_workspace_root() -> PathBuf {
        env::current_dir()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    mod extend_node_options_env_var {
        use super::*;

        #[test]
        fn returns_value_if_not_set() {
            assert_eq!(extend_node_options_env_var("--arg"), String::from("--arg"));
        }

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
            let path = find_package(&env::current_dir().unwrap(), "@moonrepo/cli");

            assert_eq!(
                path.unwrap(),
                get_workspace_root()
                    .join("node_modules")
                    .join("@moonrepo/cli")
            );
        }

        #[test]
        fn returns_path_without_package_scope() {
            let path = find_package(&env::current_dir().unwrap(), "packemon");

            assert_eq!(
                path.unwrap(),
                get_workspace_root().join("node_modules").join("packemon")
            );
        }

        #[test]
        fn returns_none_for_missing() {
            let path = find_package(&env::current_dir().unwrap(), "@moonrepo/unknown-package");

            assert_eq!(path, None);
        }
    }

    mod find_package_bin {
        use super::*;

        #[test]
        fn returns_path_if_found() {
            let path = find_package_bin(&env::current_dir().unwrap(), "packemon");

            assert_eq!(
                path.unwrap(),
                get_workspace_root()
                    .join("node_modules/.bin")
                    .join(if consts::OS == "windows" {
                        "packemon.cmd"
                    } else {
                        "packemon"
                    })
            );
        }

        #[test]
        fn returns_none_for_missing() {
            let path = find_package_bin(&env::current_dir().unwrap(), "unknown-binary");

            assert_eq!(path, None);
        }
    }
}
