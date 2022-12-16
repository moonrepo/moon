pub mod actions;
mod hasher;

pub use hasher::SystemTargetHasher;
use moon_config::{PlatformType, ProjectConfig};
use moon_platform::{Platform, Runtime};
use moon_utils::regex::{UNIX_SYSTEM_COMMAND, WINDOWS_SYSTEM_COMMAND};

#[derive(Debug, Default)]
pub struct SystemPlatform;

impl Platform for SystemPlatform {
    fn get_type(&self) -> PlatformType {
        PlatformType::System
    }

    fn get_runtime_from_config(&self, _project_config: Option<&ProjectConfig>) -> Option<Runtime> {
        Some(Runtime::System)
    }

    fn is_task_command(&self, command: &str) -> bool {
        UNIX_SYSTEM_COMMAND.is_match(command) || WINDOWS_SYSTEM_COMMAND.is_match(command)
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
