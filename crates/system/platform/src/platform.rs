use crate::tool::SystemToolStub;
use moon_config::{PlatformType, ProjectConfig};
use moon_platform::{BoxedTool, Platform, Runtime, Version};
use moon_tool::ToolError;

#[derive(Debug)]
pub struct SystemPlatform {
    tool: BoxedTool,
}

impl SystemPlatform {
    pub fn new() -> Self {
        SystemPlatform {
            tool: Box::new(SystemToolStub::default()),
        }
    }
}

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

    fn get_language_tool(&self, _version: Version) -> Result<&BoxedTool, ToolError> {
        Ok(&self.tool)
    }
}
