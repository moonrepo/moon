use std::path::Path;

pub type StaticString = &'static str;

pub type StaticStringList = &'static [StaticString];

pub struct Language {
    pub default_version: StaticString,

    pub vendor_bins_dir: StaticString,

    pub vendor_dir: StaticString,
}

pub struct PackageManager {
    pub config_filenames: StaticStringList,

    pub default_version: StaticString,

    pub lock_filenames: StaticStringList,

    pub manifest_filename: StaticString,
}

pub struct VersionManager {
    pub config_filename: Option<StaticString>,

    pub version_filename: StaticString,
}

pub fn is_using_package_manager(base_dir: &Path, pm: &PackageManager) -> bool {
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

pub fn is_using_version_manager(base_dir: &Path, vm: &VersionManager) -> bool {
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
