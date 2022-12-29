use crate::hasher::SystemTargetHasher;
use crate::tool::SystemToolStub;
use moon_config::{HasherConfig, PlatformType, ProjectConfig};
use moon_hasher::HashSet;
use moon_platform::{Platform, Runtime, Version};
use moon_project::Project;
use moon_tool::{Tool, ToolError};
use moon_utils::async_trait;

#[derive(Debug, Default)]
pub struct SystemPlatform {
    tool: SystemToolStub,
}

#[async_trait]
impl Platform for SystemPlatform {
    fn get_type(&self) -> PlatformType {
        PlatformType::System
    }

    fn get_runtime_from_config(&self, _project_config: Option<&ProjectConfig>) -> Option<Runtime> {
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

    // TOOLCHAIN

    fn get_language_tool(&self, _version: Version) -> Result<Box<&dyn Tool>, ToolError> {
        Ok(Box::new(&self.tool))
    }

    // ACTIONS

    async fn hash_run_target(
        &self,
        _project: &Project,
        hashset: &mut HashSet,
        _hasher_config: &HasherConfig,
    ) -> Result<(), ToolError> {
        hashset.hash(SystemTargetHasher::new());

        Ok(())
    }
}
