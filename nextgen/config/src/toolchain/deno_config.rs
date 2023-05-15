use schematic::Config;

/// Docs: https://moonrepo.dev/docs/config/toolchain#deno
#[derive(Config)]
pub struct DenoConfig {
    #[setting(default = "deps.ts")]
    pub deps_file: String,

    pub lockfile: bool,
}
