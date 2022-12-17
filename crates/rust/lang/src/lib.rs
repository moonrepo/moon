use moon_lang::{DependencyManager, Language, VersionManager};

pub const RUST: Language = Language {
    binary: "rustc",
    default_version: "1.66.0",
    file_exts: &["rs", "rlib"],
    vendor_bins_dir: None,
    vendor_dir: None,
};

// Package managers

pub const CARGO: DependencyManager = DependencyManager {
    binary: "cargo",
    config_files: &[".cargo/config.toml"],
    default_version: "1.66.0",
    lockfile: "Cargo.lock",
    manifest: "Cargo.toml",
};

// Version managers

pub const RUSTUP: VersionManager = VersionManager {
    binary: "rustup",
    version_file: "rust-toolchain.toml", // A config file
};
