use crate::validate::validate_semver;
use schematic::Config;

/// Docs: https://moonrepo.dev/docs/config/toolchain#rust
#[derive(Debug, Clone, Config)]
pub struct RustConfig {
    pub bins: Vec<String>,

    pub sync_toolchain_config: bool,

    #[setting(env = "MOON_RUST_VERSION", validate = validate_semver)]
    pub version: Option<String>,
}
