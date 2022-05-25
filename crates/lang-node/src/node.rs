use crate::NODE;
use moon_lang::LangError;
use std::env::{self, consts};
use std::path::{Path, PathBuf};

pub fn extend_node_options_env_var(next: String) -> String {
    match env::var("NODE_OPTIONS") {
        Ok(prev) => format!("{} {}", prev, next),
        Err(_) => next,
    }
}

pub fn find_package(starting_dir: &Path, package_name: &str) -> Option<PathBuf> {
    let pkg_path = starting_dir.join(NODE.vendor_dir).join(package_name);

    if pkg_path.exists() {
        return Some(pkg_path);
    }

    match starting_dir.parent() {
        Some(dir) => find_package_bin(dir, package_name),
        None => None,
    }
}

pub fn find_package_bin(starting_dir: &Path, package_name: &str) -> Option<PathBuf> {
    let bin_path = starting_dir
        .join(NODE.vendor_bins_dir)
        .join(get_bin_name_suffix(package_name, "cmd", true));

    if bin_path.exists() {
        return Some(bin_path);
    }

    match starting_dir.parent() {
        Some(dir) => find_package_bin(dir, package_name),
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
    } else if consts::ARCH == "powerpc64" {
        arch = "ppc64le"
    } else if consts::ARCH == "s390x" {
        arch = "s390x"
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
