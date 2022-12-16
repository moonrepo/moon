use moon_lang::{DependencyManager, Language, VersionManager};

pub const PHP: Language = Language {
    binary: "php",
    default_version: "8.2.0",
    file_exts: &["php"],
    vendor_bins_dir: Some("vendor/bin"),
    vendor_dir: Some("vendor"),
};

// Package managers

pub const COMPOSER: DependencyManager = DependencyManager {
    binary: "composer",
    config_files: &[],
    default_version: "2.4.4",
    lockfile: "composer.lock",
    manifest: "composer.json",
};

// Version managers

pub const PHPENV: VersionManager = VersionManager {
    binary: "phpenv",
    version_file: ".phpenv-version",
};

pub const PHPBREW: VersionManager = VersionManager {
    binary: "phpbrew",
    version_file: ".phpbrewrc", // Contains "phpbrew use <version>"
};
