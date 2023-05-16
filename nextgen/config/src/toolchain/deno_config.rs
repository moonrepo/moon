use crate::relative_path::FilePath;
use schematic::Config;

/// Docs: https://moonrepo.dev/docs/config/toolchain#deno
#[derive(Config)]
pub struct DenoConfig {
    #[setting(default_str = "deps.ts")]
    pub deps_file: FilePath,

    pub lockfile: bool,
}
