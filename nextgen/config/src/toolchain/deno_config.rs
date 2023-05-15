use schematic::Config;

#[derive(Config)]
pub struct DenoConfig {
    #[setting(default = "deps.ts")]
    pub deps_file: String,

    pub lockfile: bool,
}
