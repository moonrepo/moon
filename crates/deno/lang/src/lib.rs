mod deno_json;
mod deno_lock;

pub use deno_json::DenoJson;
pub use deno_lock::DenoLock;
use moon_lang::{DependencyManager, Language, VersionManager};

pub const DENO: Language = Language {
    binary: "deno",
    file_exts: &["js", "jsx", "ts", "tsx"],
    vendor_bins_dir: None,
    vendor_dir: Some("vendor"),
};

// Dependency managers

pub const DENO_DEPS: DependencyManager = DependencyManager {
    binary: "deno",
    config_files: &["deno.json", "deno.jsonc"],
    lockfile: "deno.lock",
    manifest: "deps.ts", // What to put here?
};

// Version managers

pub const DVM: VersionManager = VersionManager {
    binary: "dvm",
    version_file: ".dvmrc",
};
