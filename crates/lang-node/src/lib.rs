pub mod node;
pub mod npm;
pub mod package;
pub mod pnpm;
pub mod pnpm_workspace;
pub mod tsconfig;
pub mod yarn;
pub mod yarn_classic;

use moon_lang::{Language, PackageManager, VersionManager};

pub const NODE: Language = Language {
    binary: "node",
    default_version: "18.12.0",
    file_exts: &["js", "cjs", "mjs"],
    vendor_bins_dir: "node_modules/.bin",
    vendor_dir: "node_modules",
};

pub const TYPESCRIPT: Language = Language {
    binary: "tsc",
    default_version: "4.8.4",
    file_exts: &["ts", "cts", "mts", "d.ts", "d.cts", "d.mts"],
    vendor_bins_dir: "",
    vendor_dir: "node_modules/@types",
};

// Package managers

pub const NPM: PackageManager = PackageManager {
    binary: "npm",
    config_filenames: &[".npmrc"],
    default_version: "8.19.2",
    lock_filename: "package-lock.json",
    manifest_filename: "package.json",
};

pub const PNPM: PackageManager = PackageManager {
    binary: "pnpm",
    config_filenames: &[".npmrc", ".pnpmfile.cjs", "pnpm-workspace.yaml"],
    default_version: "7.14.0",
    lock_filename: "pnpm-lock.yaml",
    manifest_filename: "package.json",
};

pub const YARN: PackageManager = PackageManager {
    binary: "yarn",
    config_filenames: &[".yarn", ".yarnrc", ".yarnrc.yml"],
    default_version: "3.2.4",
    lock_filename: "yarn.lock",
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
