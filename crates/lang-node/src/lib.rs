use moon_lang::{Language, PackageManager, VersionManager};

pub const NODE: Language = Language {
    default_version: "16.15.0",
    vendor_bins_dir: "node_modules/.bin",
    vendor_dir: "node_modules",
};

// Package managers

pub const NPM: PackageManager = PackageManager {
    config_filenames: &[".npmrc"],
    default_version: "8.10.0",
    lock_filenames: &["package-lock.json", "npm-shrinkwrap.json"],
    manifest_filename: "package.json",
};

pub const PNPM: PackageManager = PackageManager {
    config_filenames: &["pnpm-workspace.yaml", ".pnpmfile.cjs"],
    default_version: "7.1.5",
    lock_filenames: &["pnpm-lock.yaml"],
    manifest_filename: "package.json",
};

pub const YARN: PackageManager = PackageManager {
    config_filenames: &[".yarn", ".yarnrc", ".yarnrc.yml"],
    default_version: "3.2.1",
    lock_filenames: &["yarn.lock"],
    manifest_filename: "package.json",
};

// Version managers

pub const NVMRC: VersionManager = VersionManager {
    config_filename: None,
    version_filename: ".nvmrc",
};

pub const NODENV: VersionManager = VersionManager {
    config_filename: None,
    version_filename: ".node-version",
};
