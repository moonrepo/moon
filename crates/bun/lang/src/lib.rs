use moon_lang::{DependencyManager, Language};

pub const BUN: Language = Language {
    binary: "bun",
    file_exts: &["js", "jsx", "cjs", "mjs", "ts", "tsx", "cts", "mts"],
    vendor_bins_dir: Some("node_modules/.bin"),
    vendor_dir: Some("node_modules"),
};

// Dependency managers

pub const BUN_INSTALL: DependencyManager = DependencyManager {
    binary: "bun install",
    config_files: &["bunfig.toml"],
    lockfile: "bun.lockb",
    manifest: "package.json",
};
