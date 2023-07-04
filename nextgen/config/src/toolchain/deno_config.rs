use schematic::Config;

/// Docs: https://moonrepo.dev/docs/config/toolchain#deno
#[derive(Clone, Config, Debug)]
pub struct DenoConfig {
    #[setting(default = "deps.ts")]
    pub deps_file: String,

    pub lockfile: bool,
}
