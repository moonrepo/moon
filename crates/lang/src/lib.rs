mod config;
mod errors;

pub use errors::LangError;
use std::fmt;
use std::fs;
use std::path::Path;

type StaticString = &'static str;

type StaticStringList = &'static [StaticString];

pub struct Language {
    pub binary: StaticString,

    pub default_version: StaticString,

    pub vendor_bins_dir: StaticString,

    pub vendor_dir: StaticString,
}

pub struct PackageManager {
    pub binary: StaticString,

    pub config_filenames: StaticStringList,

    pub default_version: StaticString,

    pub lock_filenames: StaticStringList,

    pub manifest_filename: StaticString,
}

pub struct VersionManager {
    pub binary: StaticString,

    pub config_filename: Option<StaticString>,

    pub version_filename: StaticString,
}

pub fn has_vendor_installed_dependencies<T: AsRef<Path>>(dir: T, lang: &Language) -> bool {
    let vendor_path = dir.as_ref().join(lang.vendor_dir);

    if !vendor_path.exists() {
        return false;
    }

    match fs::read_dir(vendor_path) {
        Ok(mut contents) => contents.next().is_some(),
        Err(_) => false,
    }
}

pub fn is_using_package_manager<T: AsRef<Path>>(base_dir: T, pm: &PackageManager) -> bool {
    let base_dir = base_dir.as_ref();

    for lockfile in pm.lock_filenames {
        if base_dir.join(lockfile).exists() {
            return true;
        }
    }

    for config in pm.config_filenames {
        if base_dir.join(config).exists() {
            return true;
        }
    }

    false
}

pub fn is_using_version_manager<T: AsRef<Path>>(base_dir: T, vm: &VersionManager) -> bool {
    let base_dir = base_dir.as_ref();

    if base_dir.join(vm.version_filename).exists() {
        return true;
    }

    if let Some(config) = vm.config_filename {
        if base_dir.join(config).exists() {
            return true;
        }
    }

    false
}

#[derive(Clone, Eq, PartialEq)]
pub enum SupportedLanguage {
    Node,
    System,
}

impl SupportedLanguage {
    pub fn label(&self) -> String {
        match self {
            SupportedLanguage::Node => "Node.js".into(),
            SupportedLanguage::System => "system".into(),
        }
    }
}

impl fmt::Display for SupportedLanguage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SupportedLanguage::Node => write!(f, "Node"),
            SupportedLanguage::System => write!(f, "System"),
        }
    }
}
