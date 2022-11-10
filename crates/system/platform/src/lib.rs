pub mod actions;
mod hasher;

pub use hasher::SystemTargetHasher;
use moon_config::{ProjectConfig, WorkspaceConfig};
use moon_platform::{Platform, Runtime};

#[derive(Debug, Default)]
pub struct SystemPlatform;

impl Platform for SystemPlatform {
    fn get_default_runtime(&self) -> Runtime {
        Runtime::System
    }

    fn get_runtime_from_config(
        &self,
        _project_config: Option<&ProjectConfig>,
        _workspace_config: &WorkspaceConfig,
    ) -> Option<Runtime> {
        Some(Runtime::System)
    }

    fn matches(&self, project_config: &ProjectConfig, runtime: Option<&Runtime>) -> bool {
        if project_config.language.is_system_platform() {
            return true;
        }

        if let Some(runtime) = &runtime {
            return matches!(runtime, Runtime::System);
        }

        false
    }
}
