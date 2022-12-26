mod config;

use rustc_hash::FxHashMap;
use std::fs;
use std::path::Path;

type StaticString = &'static str;

type StaticStringList = &'static [StaticString];

pub struct Language {
    pub binary: StaticString,

    pub file_exts: StaticStringList,

    pub vendor_bins_dir: Option<StaticString>,

    pub vendor_dir: Option<StaticString>,
}

pub struct DependencyManager {
    pub binary: StaticString,

    pub config_files: StaticStringList,

    pub lockfile: StaticString,

    pub manifest: StaticString,
}

pub struct VersionManager {
    pub binary: StaticString,

    pub version_file: StaticString,
}

pub type LockfileDependencyVersions = FxHashMap<String, Vec<String>>;

#[inline]
pub fn has_vendor_installed_dependencies<T: AsRef<Path>>(dir: T, lang: &Language) -> bool {
    let Some(vendor_dir) = lang.vendor_dir else {
        return false;
    };

    let vendor_path = dir.as_ref().join(vendor_dir);

    if !vendor_path.exists() {
        return false;
    }

    match fs::read_dir(vendor_path) {
        Ok(mut contents) => contents.next().is_some(),
        Err(_) => false,
    }
}

#[inline]
pub fn is_using_dependency_manager<T: AsRef<Path>>(
    base_dir: T,
    manager: &DependencyManager,
    check_manifest: bool,
) -> bool {
    let base_dir = base_dir.as_ref();

    if base_dir.join(manager.lockfile).exists() {
        return true;
    }

    if check_manifest && base_dir.join(manager.manifest).exists() {
        return true;
    }

    for config in manager.config_files {
        if base_dir.join(config).exists() {
            return true;
        }
    }

    false
}

#[inline]
pub fn is_using_version_manager<T: AsRef<Path>>(base_dir: T, manager: &VersionManager) -> bool {
    let base_dir = base_dir.as_ref();

    if base_dir.join(manager.version_file).exists() {
        return true;
    }

    false
}
