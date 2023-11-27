use crate::target_hash::SystemTargetHash;
use crate::tool::SystemToolStub;
use moon_action_context::ActionContext;
use moon_config::{HasherConfig, PlatformType, ProjectConfig};
use moon_hash::ContentHasher;
use moon_platform::{Platform, Runtime, RuntimeReq};
use moon_process::Command;
use moon_project::Project;
use moon_task::Task;
use moon_tool::Tool;
use moon_utils::async_trait;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct SystemPlatform {
    tool: SystemToolStub,
}

#[async_trait]
impl Platform for SystemPlatform {
    fn get_type(&self) -> PlatformType {
        PlatformType::System
    }

    fn get_runtime_from_config(&self, _project_config: Option<&ProjectConfig>) -> Runtime {
        Runtime::system()
    }

    fn matches(&self, platform: &PlatformType, runtime: Option<&Runtime>) -> bool {
        if matches!(platform, PlatformType::System) {
            return true;
        }

        if let Some(runtime) = &runtime {
            return matches!(runtime.platform, PlatformType::System);
        }

        false
    }

    // TOOLCHAIN

    fn is_toolchain_enabled(&self) -> miette::Result<bool> {
        Ok(false)
    }

    fn get_tool(&self) -> miette::Result<Box<&dyn Tool>> {
        Ok(Box::new(&self.tool))
    }

    fn get_tool_for_version(&self, _req: RuntimeReq) -> miette::Result<Box<&dyn Tool>> {
        Ok(Box::new(&self.tool))
    }

    // ACTIONS

    async fn hash_run_target(
        &self,
        _project: &Project,
        _runtime: &Runtime,
        hasher: &mut ContentHasher,
        _hasher_config: &HasherConfig,
    ) -> miette::Result<()> {
        hasher.hash_content(SystemTargetHash::new())?;

        Ok(())
    }

    async fn create_run_target_command(
        &self,
        _context: &ActionContext,
        _project: &Project,
        task: &Task,
        _runtime: &Runtime,
        working_dir: &Path,
    ) -> miette::Result<Command> {
        let mut command = Command::new(&task.command);

        // cmd/pwsh requires an absolute path to batch files
        if cfg!(windows) {
            use moon_process::shell::is_windows_script;

            for arg in &task.args {
                if is_windows_script(arg) {
                    command.arg(working_dir.join(arg));
                } else {
                    command.arg(arg);
                }
            }
        } else {
            command.args(&task.args);
        }

        command.envs(&task.env);

        Ok(command)
    }

    async fn get_env_paths(&self, _working_dir: &Path) -> miette::Result<Vec<PathBuf>> {
        Ok(vec![])
    }
}
