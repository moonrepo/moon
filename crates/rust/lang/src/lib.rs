pub mod cargo_lock;
pub mod cargo_toml;
pub mod toolchain_toml;

use moon_lang::{DependencyManager, Language, VersionManager};

pub const RUST: Language = Language {
    binary: "rustc",
    file_exts: &["rs", "rlib"],
    vendor_bins_dir: None,
    vendor_dir: Some("target"),
};

// Package managers

pub const CARGO: DependencyManager = DependencyManager {
    binary: "cargo",
    config_files: &[".cargo/config.toml"],
    lockfile: "Cargo.lock",
    manifest: "Cargo.toml",
};

// Version managers

pub const RUSTUP: VersionManager = VersionManager {
    binary: "rustup",
    version_file: "rust-toolchain.toml", // A config file
};

pub const RUSTUP_LEGACY: VersionManager = VersionManager {
    binary: "rustup",
    version_file: "rust-toolchain", // A config file
};
