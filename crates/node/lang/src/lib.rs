pub mod node;
pub mod npm;
pub mod package;
pub mod pnpm;
pub mod yarn;

pub use moon_lang::LockfileDependencyVersions;
pub use package::PackageJson;

use moon_lang::{DependencyManager, Language, VersionManager};

pub const NODE: Language = Language {
    binary: "node",
    default_version: "18.12.0",
    file_exts: &["js", "cjs", "mjs"],
    vendor_bins_dir: Some("node_modules/.bin"),
    vendor_dir: Some("node_modules"),
};

// Package managers

pub const NPM: DependencyManager = DependencyManager {
    binary: "npm",
    config_files: &[".npmrc"],
    default_version: "8.19.2",
    lockfile: "package-lock.json",
    manifest: "package.json",
};

pub const PNPM: DependencyManager = DependencyManager {
    binary: "pnpm",
    config_files: &[".npmrc", ".pnpmfile.cjs", "pnpm-workspace.yaml"],
    default_version: "7.18.2",
    lockfile: "pnpm-lock.yaml",
    manifest: "package.json",
};

pub const YARN: DependencyManager = DependencyManager {
    binary: "yarn",
    config_files: &[".yarn", ".yarnrc", ".yarnrc.yml"],
    default_version: "3.3.0",
    lockfile: "yarn.lock",
    manifest: "package.json",
};

// Version managers

pub const NVM: VersionManager = VersionManager {
    binary: "nvm",
    version_file: ".nvmrc",
};

pub const NODENV: VersionManager = VersionManager {
    binary: "nodenv",
    version_file: ".node-version",
};
