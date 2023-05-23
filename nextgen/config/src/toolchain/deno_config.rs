use crate::portable_path::FilePath;
use schematic::Config;

/// Docs: https://moonrepo.dev/docs/config/toolchain#deno
#[derive(Debug, Config)]
pub struct DenoConfig {
    #[setting(default = "deps.ts")]
    pub deps_file: FilePath,

    pub lockfile: bool,
}
