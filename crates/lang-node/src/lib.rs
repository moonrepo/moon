pub mod node;
pub mod npm;
pub mod package;
pub mod pnpm;
pub mod tsconfig;
pub mod yarn;
pub mod yarn_classic;

use moon_lang::{Language, PackageManager, VersionManager};

pub const NODE: Language = Language {
    binary: "node",
    default_version: "16.16.0",
    vendor_bins_dir: "node_modules/.bin",
    vendor_dir: "node_modules",
};

// Package managers

pub const NPM: PackageManager = PackageManager {
    binary: "npm",
    config_filenames: &[".npmrc"],
    default_version: "8.16.0",
    lock_filenames: &["package-lock.json", "npm-shrinkwrap.json"],
    manifest_filename: "package.json",
};

pub const PNPM: PackageManager = PackageManager {
    binary: "pnpm",
    config_filenames: &["pnpm-workspace.yaml", ".pnpmfile.cjs"],
    default_version: "7.9.0",
    lock_filenames: &["pnpm-lock.yaml"],
    manifest_filename: "package.json",
};

pub const YARN: PackageManager = PackageManager {
    binary: "yarn",
    config_filenames: &[".yarn", ".yarnrc", ".yarnrc.yml"],
    default_version: "3.2.1",
    lock_filenames: &["yarn.lock"],
    manifest_filename: "package.json",
};

// Version managers

pub const NVMRC: VersionManager = VersionManager {
    binary: "nvm",
    config_filename: None,
    version_filename: ".nvmrc",
};

pub const NODENV: VersionManager = VersionManager {
    binary: "nodenv",
    config_filename: None,
    version_filename: ".node-version",
};
