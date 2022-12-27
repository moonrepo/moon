use moon_lang::{DependencyManager, Language, VersionManager};

pub const RUBY: Language = Language {
    binary: "ruby",
    file_exts: &["rb"],
    vendor_bins_dir: None,
    vendor_dir: Some("vendor"),
};

// Package managers

pub const BUNDLER: DependencyManager = DependencyManager {
    binary: "bundle",
    config_files: &[".bundle/config"],
    lockfile: "Gemfile.lock",
    manifest: "Gemfile",
};

// Version managers

pub const RVM: VersionManager = VersionManager {
    binary: "rvm",
    version_file: ".ruby-version",
};

pub const RBENV: VersionManager = VersionManager {
    binary: "rbenv",
    version_file: ".ruby-version",
};
