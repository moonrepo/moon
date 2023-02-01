use crate::target_hasher::GoTargetHasher;
use moon_action_context::ActionContext;
use moon_config::{HasherConfig, PlatformType, ProjectConfig, GoConfig};
use moon_hasher::HashSet;
use moon_platform::{Platform, Runtime, Version};
use proto::Proto;
use rustc_hash::FxHashMap;
use moon_project::Project;
use moon_task::Task;
use moon_tool::{Tool, ToolError, ToolManager};
use moon_go_tool::GoTool;
use moon_utils::{async_trait, process::Command};
use std::path::Path;

#[derive(Debug)]
pub struct GoPlatform {
    config: GoConfig,
    toolchain: ToolManager<GoTool>,
}

impl GoPlatform {
    pub fn new(config: &GoConfig) -> Self {
        GoPlatform{
            toolchain: ToolManager::new(Runtime::Go(Version::default())),
            config: config.to_owned(),
        }
    }
}


#[async_trait]
impl Platform for GoPlatform {
    fn get_type(&self) -> PlatformType {
        PlatformType::Go
    }

    fn get_runtime_from_config(&self, _project_config: Option<&ProjectConfig>) -> Option<Runtime> {
        if let Some(go_version) = &self.config.version {
            return Some(Runtime::Go(Version::new(go_version)));
        }

        None
    }

    fn matches(&self, platform: &PlatformType, runtime: Option<&Runtime>) -> bool {
        if matches!(platform, PlatformType::Go) {
            return true;
        }

        if let Some(runtime) = &runtime {
            return matches!(runtime, Runtime::Go(_));
        }

        false
    }

    // TOOLCHAIN

    fn get_tool(&self) -> Result<Box<&dyn Tool>, ToolError> {
        let tool = self.toolchain.get()?;

        Ok(Box::new(tool))
    }

    fn get_tool_for_version(&self, version: Version) -> Result<Box<&dyn Tool>, ToolError> {
        let tool = self.toolchain.get_for_version(&version)?;

        Ok(Box::new(tool))
    }

    // ACTIONS

    async fn setup_tool(
        &mut self,
        _context: &ActionContext,
        runtime: &Runtime,
        last_versions: &mut FxHashMap<String, String>,
    ) -> Result<u8, ToolError> {
        let version = runtime.version();

        dbg!(&version);

        if !self.toolchain.has(&version) {
            self.toolchain.register(
                &version,
                GoTool::new(&Proto::new()?, &self.config, &version.0)?,
            );
        }

        Ok(self.toolchain.setup(&version, last_versions).await?)
    }

    async fn hash_run_target(
        &self,
        _project: &Project,
        hashset: &mut HashSet,
        _hasher_config: &HasherConfig,
    ) -> Result<(), ToolError> {
        hashset.hash(GoTargetHasher::new());

        Ok(())
    }

    async fn create_run_target_command(
        &self,
        context: &ActionContext,
        project: &Project,
        task: &Task,
        runtime: &Runtime,
        working_dir: &Path,
    ) -> Result<Command, ToolError> {
        dbg!(runtime);
        let tool = self.toolchain.get_for_version(runtime.version())?;
        // let command = actions::create_target_command(tool, context, project, task, working_dir)?;
        let go_bin = tool.get_bin_path()?;

        let command = Command::new(go_bin);

        Ok(command)
    }
}
