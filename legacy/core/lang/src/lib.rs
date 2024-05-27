mod config;

use rustc_hash::FxHashMap;
use std::fs;
use std::path::Path;

pub type LockfileDependencyVersions = FxHashMap<String, Vec<String>>;

#[inline]
pub fn has_vendor_installed_dependencies<T: AsRef<Path>>(dir: T, vendor_dir: &str) -> bool {
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
pub fn is_using_dependency_manager<T: AsRef<Path>>(base_dir: T, lockfile: &str) -> bool {
    base_dir.as_ref().join(lockfile).exists()
}

#[inline]
pub fn is_using_version_manager<T: AsRef<Path>>(base_dir: T, file: &str) -> bool {
    base_dir.as_ref().join(file).exists()
}
