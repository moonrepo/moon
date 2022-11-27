pub mod actions;
mod hasher;

pub use hasher::SystemTargetHasher;
use moon_config::{PlatformType, ProjectConfig, ToolchainConfig};
use moon_platform::{Platform, Runtime};

#[derive(Debug, Default)]
pub struct SystemPlatform;

impl Platform for SystemPlatform {
    fn get_type(&self) -> PlatformType {
        PlatformType::System
    }

    fn get_runtime_from_config(
        &self,
        _project_config: Option<&ProjectConfig>,
        _toolchain_config: &ToolchainConfig,
    ) -> Option<Runtime> {
        Some(Runtime::System)
    }

    fn matches(&self, platform: &PlatformType, runtime: Option<&Runtime>) -> bool {
        if matches!(platform, PlatformType::System) {
            return true;
        }

        if let Some(runtime) = &runtime {
            return matches!(runtime, Runtime::System);
        }

        false
    }
}
