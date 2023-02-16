use moon_action_context::ActionContext;
use moon_config::{
    DenoConfig, DependencyConfig, HasherConfig, PlatformType, ProjectConfig, ProjectsAliasesMap,
    TypeScriptConfig,
};
use moon_deno_lang::DENO_DEPS;
use moon_deno_tool::DenoTool;
use moon_error::MoonError;
use moon_hasher::HashSet;
use moon_logger::debug;
use moon_platform::{Platform, Runtime, Version};
use moon_project::{Project, ProjectError};
use moon_task::Task;
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_tool::{Tool, ToolError, ToolManager};
use moon_utils::{async_trait, process::Command};
use proto::Proto;
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};

const LOG_TARGET: &str = "moon:deno-platform";

#[derive(Debug)]
pub struct DenoPlatform {
    config: DenoConfig,

    toolchain: ToolManager<DenoTool>,

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

    fn get_runtime_from_config(&self, _project_config: Option<&ProjectConfig>) -> Option<Runtime> {
        Some(Runtime::Deno(Version::default()))
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
        Ok(Some((
            DENO_DEPS.lockfile.to_owned(),
            DENO_DEPS.manifest.to_owned(),
        )))
    }

    async fn setup_toolchain(&mut self) -> Result<(), ToolError> {
        // if let Some(version) = &self.config.version {
        //     let version = Version::new(version);
        //     let mut last_versions = FxHashMap::default();

        //     if !self.toolchain.has(&version) {
        //         self.toolchain.register(
        //             &version,
        //             DenoTool::new(&Proto::new()?, &self.config, &version.0)?,
        //         );
        //     }

        //     self.toolchain.setup(&version, &mut last_versions).await?;
        // }

        Ok(())
    }

    async fn teardown_toolchain(&mut self) -> Result<(), ToolError> {
        self.toolchain.teardown_all().await?;

        Ok(())
    }

    // ACTIONS

    async fn setup_tool(
        &mut self,
        _context: &ActionContext,
        runtime: &Runtime,
        last_versions: &mut FxHashMap<String, String>,
    ) -> Result<u8, ToolError> {
        let version = runtime.version();

        if !self.toolchain.has(&version) {
            self.toolchain.register(
                &version,
                DenoTool::new(&Proto::new()?, &self.config, &version.0)?,
            );
        }

        Ok(self.toolchain.setup(&version, last_versions).await?)
    }

    async fn install_deps(
        &self,
        _context: &ActionContext,
        runtime: &Runtime,
        working_dir: &Path,
    ) -> Result<(), ToolError> {
        let tool = self.toolchain.get_for_version(runtime.version())?;

        debug!(target: LOG_TARGET, "Installing dependencies");

        print_checkpoint("deno cache", Checkpoint::Setup);

        Command::new(tool.get_bin_path()?)
            .args(["cache", "--lock", "--lock-write", "src/deps.ts"])
            .cwd(working_dir)
            .exec_stream_output()
            .await?;

        Ok(())
    }

    async fn sync_project(
        &self,
        _context: &ActionContext,
        _project: &Project,
        _dependencies: &FxHashMap<String, &Project>,
    ) -> Result<bool, ProjectError> {
        Ok(false)
    }

    async fn hash_manifest_deps(
        &self,
        _manifest_path: &Path,
        _hashset: &mut HashSet,
        _hasher_config: &HasherConfig,
    ) -> Result<(), ToolError> {
        Ok(())
    }

    async fn hash_run_target(
        &self,
        _project: &Project,
        _runtime: &Runtime,
        _hashset: &mut HashSet,
        _hasher_config: &HasherConfig,
    ) -> Result<(), ToolError> {
        Ok(())
    }

    async fn create_run_target_command(
        &self,
        _context: &ActionContext,
        _project: &Project,
        _task: &Task,
        _runtime: &Runtime,
        _working_dir: &Path,
    ) -> Result<Command, ToolError> {
        Ok(Command::new("deno"))
    }
}
