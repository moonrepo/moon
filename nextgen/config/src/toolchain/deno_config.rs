use schematic::Config;
use serde::Serialize;

/// Docs: https://moonrepo.dev/docs/config/toolchain#deno
#[derive(Debug, Clone, Config, Serialize)]
pub struct DenoConfig {
    #[setting(default = "deps.ts")]
    pub deps_file: String,

    pub lockfile: bool,
}
