use moon_lang::{DependencyManager, Language, VersionManager};

pub const DENO: Language = Language {
    binary: "deno",
    default_version: "1.29.1",
    file_exts: &["js", "jsx", "ts", "tsx"],
    vendor_bins_dir: "",
    vendor_dir: "vendor",
};

// Version managers

pub const DVM: VersionManager = VersionManager {
    binary: "dvm",
    version_file: ".dvmrc",
};
