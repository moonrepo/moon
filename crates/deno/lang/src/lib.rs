use moon_lang::{Language, VersionManager};

pub const DENO: Language = Language {
    binary: "deno",
    file_exts: &["js", "jsx", "ts", "tsx"],
    vendor_bins_dir: None,
    vendor_dir: Some("vendor"),
};

// Version managers

pub const DVM: VersionManager = VersionManager {
    binary: "dvm",
    version_file: ".dvmrc",
};
