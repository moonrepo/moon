use moon_lang::{Language, VersionManager};

pub const GO: Language = Language {
    binary: "go",
    default_version: "1.19.4",
    file_exts: &["go"],
    vendor_bins_dir: None,
    vendor_dir: Some("vendor"),
};

// Version managers

pub const GVM: VersionManager = VersionManager {
    binary: "gvm",
    version_file: ".gvmrc",
};
