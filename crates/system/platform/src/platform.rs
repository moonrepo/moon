use crate::target_hasher::SystemTargetHasher;
use crate::tool::SystemToolStub;
use moon_action_context::ActionContext;
use moon_config::{HasherConfig, PlatformType, ProjectConfig};
use moon_hasher::HashSet;
use moon_platform::{Platform, Runtime, Version};
use moon_project::Project;
use moon_task::Task;
use moon_tool::{Tool, ToolError};
use moon_utils::{async_trait, process::Command};
use std::path::Path;

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

    fn get_tool(&self) -> Result<Box<&dyn Tool>, ToolError> {
        Ok(Box::new(&self.tool))
    }

    fn get_tool_for_version(&self, _version: Version) -> Result<Box<&dyn Tool>, ToolError> {
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

    async fn create_run_target_command(
        &self,
        _context: &ActionContext,
        _project: &Project,
        task: &Task,
        working_dir: &Path,
    ) -> Result<Command, ToolError> {
        let mut command = Command::new(&task.command);

        // cmd/pwsh requires an absolute path to batch files
        if cfg!(windows) {
            use moon_utils::process::is_windows_script;

            for arg in &task.args {
                if is_windows_script(arg) {
                    command.arg(working_dir.join(arg));
                } else {
                    command.arg(arg);
                }
            }
        } else {
            command.args(&task.args).envs(&task.env);
        }

        Ok(command)
    }
}
