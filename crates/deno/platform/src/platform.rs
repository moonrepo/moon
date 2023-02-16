use moon_config::{
    DenoConfig, DependencyConfig, PlatformType, ProjectConfig, ProjectsAliasesMap, TypeScriptConfig,
};
use moon_error::MoonError;
use moon_platform::{Platform, Runtime, Version};
use moon_project::Project;
use moon_tool::{Tool, ToolError, ToolManager};
use moon_utils::{async_trait, process::Command};
use std::path::{Path, PathBuf};

const LOG_TARGET: &str = "moon:deno-platform";

#[derive(Debug)]
pub struct DenoPlatform {
    config: DenoConfig,

    toolchain: ToolManager<NodeTool>,

    typescript_config: Option<TypeScriptConfig>,

    workspace_root: PathBuf,
}

impl DenoPlatform {
    pub fn new(
        config: &DenoConfig,
        typescript_config: &Option<TypeScriptConfig>,
        workspace_root: &Path,
    ) -> Self {
        DenoPlatform {
            config: config.to_owned(),
            toolchain: ToolManager::new(Runtime::Deno(Version::default())),
            typescript_config: typescript_config.to_owned(),
            workspace_root: workspace_root.to_path_buf(),
        }
    }
}

#[async_trait]
impl Platform for DenoPlatform {
    fn get_type(&self) -> PlatformType {
        PlatformType::Deno
    }

    fn get_runtime_from_config(&self, project_config: Option<&ProjectConfig>) -> Option<Runtime> {
        None
    }

    fn matches(&self, platform: &PlatformType, runtime: Option<&Runtime>) -> bool {
        if matches!(platform, PlatformType::Deno) {
            return true;
        }

        if let Some(runtime) = &runtime {
            return matches!(runtime, Runtime::Deno(_));
        }

        false
    }

    // PROJECT GRAPH

    fn load_project_implicit_dependencies(
        &self,
        _project: &Project,
        _aliases_map: &ProjectsAliasesMap,
    ) -> Result<Vec<DependencyConfig>, MoonError> {
        let implicit_deps = vec![];

        Ok(implicit_deps)
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

    fn get_dependency_configs(&self) -> Result<Option<(String, String)>, ToolError> {
        let tool = self.toolchain.get()?;
        let depman = tool.get_package_manager();

        Ok(Some((
            depman.get_lock_filename(),
            depman.get_manifest_filename(),
        )))
    }

    async fn setup_toolchain(&mut self) -> Result<(), ToolError> {
        Ok(())
    }

    async fn teardown_toolchain(&mut self) -> Result<(), ToolError> {
        Ok(())
    }

    // ACTIONS

    async fn setup_tool(
        &mut self,
        _context: &ActionContext,
        runtime: &Runtime,
        last_versions: &mut FxHashMap<String, String>,
    ) -> Result<u8, ToolError> {
        Ok(0)
    }

    async fn install_deps(
        &self,
        _context: &ActionContext,
        runtime: &Runtime,
        working_dir: &Path,
    ) -> Result<(), ToolError> {
        Ok(())
    }

    async fn sync_project(
        &self,
        _context: &ActionContext,
        project: &Project,
        dependencies: &FxHashMap<String, &Project>,
    ) -> Result<bool, ProjectError> {
        Ok(false)
    }

    async fn hash_manifest_deps(
        &self,
        manifest_path: &Path,
        hashset: &mut HashSet,
        _hasher_config: &HasherConfig,
    ) -> Result<(), ToolError> {
        Ok(())
    }

    async fn hash_run_target(
        &self,
        project: &Project,
        runtime: &Runtime,
        hashset: &mut HashSet,
        hasher_config: &HasherConfig,
    ) -> Result<(), ToolError> {
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
        Ok(command)
    }
}
