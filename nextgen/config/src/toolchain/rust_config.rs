use crate::validate::validate_semver;
use schematic::Config;

/// Docs: https://moonrepo.dev/docs/config/toolchain#rust
#[derive(Config)]
pub struct RustConfig {
    pub bins: Vec<String>,

    pub sync_toolchain_config: bool,

    #[setting(validate = validate_semver)]
    pub version: Option<String>,
}
