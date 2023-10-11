use schematic::Config;
use system_env::SystemDependency;

/// Docs: https://moonrepo.dev/docs/config/toolchain#system
#[derive(Clone, Config, Debug)]
pub struct SystemConfig {
    pub requirements: Vec<SystemDependency>,
}
