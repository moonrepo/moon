use super::bin_config::BinEntry;
use schematic::Config;

/// Docs: https://moonrepo.dev/docs/config/toolchain#deno
#[derive(Clone, Config, Debug)]
pub struct DenoConfig {
    #[setting(nested)]
    pub bins: Vec<BinEntry>,

    #[setting(default = "deps.ts")]
    pub deps_file: String,

    pub lockfile: bool,

    pub plugin: Option<String>,
}
