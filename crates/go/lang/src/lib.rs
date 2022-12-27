use moon_lang::{DependencyManager, Language, VersionManager};

pub const GO: Language = Language {
    binary: "go",
    file_exts: &["go"],
    vendor_bins_dir: None,
    vendor_dir: Some("vendor"),
};

// Dependency managers

pub const GOMOD: DependencyManager = DependencyManager {
    binary: "go mod",
    config_files: &[],
    lockfile: "go.sum",
    manifest: "go.mod",
};

// Version managers

pub const G: VersionManager = VersionManager {
    binary: "g",
    version_file: "g.lock",
};

pub const GVM: VersionManager = VersionManager {
    binary: "gvm",
    version_file: ".gvmrc",
};

pub const GOENV: VersionManager = VersionManager {
    binary: "goenv",
    version_file: ".go-version",
};
