use moon_action_context::ActionContext;
use moon_bun_lang::{load_lockfile_dependencies, BUN, BUNPM};
use moon_bun_tool::BunTool;
use moon_common::{is_ci, Id};
use moon_config::{
    BunConfig, HasherConfig, PlatformType, ProjectConfig, ProjectsAliasesMap, ProjectsSourcesMap,
    TypeScriptConfig, UnresolvedVersionSpec,
};
use moon_hash::ContentHasher;
use moon_logger::{debug, map_list};
use moon_platform::{Platform, Runtime, RuntimeReq};
use moon_process::Command;
use moon_project::Project;
use moon_task::Task;
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_tool::{Tool, ToolError, ToolManager};
use moon_utils::async_trait;
use proto_core::ProtoEnvironment;
use rustc_hash::FxHashMap;
use starbase_styles::color;
use starbase_utils::{fs, glob::GlobSet};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

const LOG_TARGET: &str = "moon:bun-platform";

pub struct BunPlatform {
    pub config: BunConfig,

    proto_env: Arc<ProtoEnvironment>,

    toolchain: ToolManager<BunTool>,

    typescript_config: Option<TypeScriptConfig>,

    #[allow(dead_code)]
    pub workspace_root: PathBuf,
}

impl BunPlatform {
    pub fn new(
        config: &BunConfig,
        typescript_config: &Option<TypeScriptConfig>,
        workspace_root: &Path,
        proto_env: Arc<ProtoEnvironment>,
    ) -> Self {
        BunPlatform {
            config: config.to_owned(),
            proto_env,
            toolchain: ToolManager::new(Runtime::new(PlatformType::Bun, RuntimeReq::Global)),
            typescript_config: typescript_config.to_owned(),
            workspace_root: workspace_root.to_path_buf(),
        }
    }
}

#[async_trait]
impl Platform for BunPlatform {
    fn get_type(&self) -> PlatformType {
        PlatformType::Bun
    }

    fn get_runtime_from_config(&self, project_config: Option<&ProjectConfig>) -> Runtime {
        if let Some(config) = &project_config {
            if let Some(bun_config) = &config.toolchain.bun {
                if let Some(version) = &bun_config.version {
                    return Runtime::new_override(
                        PlatformType::Bun,
                        RuntimeReq::Toolchain(version.to_owned()),
                    );
                }
            }
        }

        if let Some(version) = &self.config.version {
            return Runtime::new(PlatformType::Bun, RuntimeReq::Toolchain(version.to_owned()));
        }

        Runtime::new(PlatformType::Bun, RuntimeReq::Global)
    }

    fn matches(&self, platform: &PlatformType, runtime: Option<&Runtime>) -> bool {
        if matches!(platform, PlatformType::Bun) {
            return true;
        }

        if let Some(runtime) = &runtime {
            return matches!(runtime.platform, PlatformType::Bun);
        }

        false
    }

    // PROJECT GRAPH

    fn is_project_in_dependency_workspace(&self, project_source: &str) -> miette::Result<bool> {
        // TODO

        Ok(false)
    }

    fn load_project_graph_aliases(
        &mut self,
        projects_map: &ProjectsSourcesMap,
        aliases_map: &mut ProjectsAliasesMap,
    ) -> miette::Result<()> {
        Ok(())
    }

    // TOOLCHAIN

    fn is_toolchain_enabled(&self) -> miette::Result<bool> {
        Ok(self.config.version.is_some())
    }

    fn get_tool(&self) -> miette::Result<Box<&dyn Tool>> {
        let tool = self.toolchain.get()?;

        Ok(Box::new(tool))
    }

    fn get_tool_for_version(&self, req: RuntimeReq) -> miette::Result<Box<&dyn Tool>> {
        let tool = self.toolchain.get_for_version(&req)?;

        Ok(Box::new(tool))
    }

    fn get_dependency_configs(&self) -> miette::Result<Option<(String, String)>> {
        Ok(Some((BUNPM.lockfile.to_owned(), BUNPM.manifest.to_owned())))
    }

    async fn setup_toolchain(&mut self) -> miette::Result<()> {
        let req = match &self.config.version {
            Some(v) => RuntimeReq::Toolchain(v.to_owned()),
            None => RuntimeReq::Global,
        };

        let mut last_versions = FxHashMap::default();

        if !self.toolchain.has(&req) {
            self.toolchain.register(
                &req,
                BunTool::new(&self.proto_env, &self.config, &req).await?,
            );
        }

        self.toolchain.setup(&req, &mut last_versions).await?;

        Ok(())
    }

    async fn teardown_toolchain(&mut self) -> miette::Result<()> {
        self.toolchain.teardown_all().await?;

        Ok(())
    }

    // ACTIONS

    async fn setup_tool(
        &mut self,
        _context: &ActionContext,
        runtime: &Runtime,
        last_versions: &mut FxHashMap<String, UnresolvedVersionSpec>,
    ) -> miette::Result<u8> {
        let req = &runtime.requirement;

        if !self.toolchain.has(req) {
            self.toolchain
                .register(req, BunTool::new(&self.proto_env, &self.config, req).await?);
        }

        Ok(self.toolchain.setup(req, last_versions).await?)
    }

    async fn install_deps(
        &self,
        _context: &ActionContext,
        runtime: &Runtime,
        working_dir: &Path,
    ) -> miette::Result<()> {
        let tool = self.toolchain.get_for_version(&runtime.requirement)?;

        // TODO

        Ok(())
    }

    async fn sync_project(
        &self,
        _context: &ActionContext,
        project: &Project,
        _dependencies: &FxHashMap<Id, Arc<Project>>,
    ) -> miette::Result<bool> {
        Ok(false)
    }

    async fn hash_manifest_deps(
        &self,
        _manifest_path: &Path,
        hasher: &mut ContentHasher,
        _hasher_config: &HasherConfig,
    ) -> miette::Result<()> {
        // TODO

        Ok(())
    }

    async fn hash_run_target(
        &self,
        project: &Project,
        _runtime: &Runtime,
        hasher: &mut ContentHasher,
        _hasher_config: &HasherConfig,
    ) -> miette::Result<()> {
        // TODO

        Ok(())
    }

    async fn create_run_target_command(
        &self,
        _context: &ActionContext,
        _project: &Project,
        task: &Task,
        runtime: &Runtime,
        working_dir: &Path,
    ) -> miette::Result<Command> {
        let mut command = Command::new(&task.command);

        command.args(&task.args).envs(&task.env).cwd(working_dir);

        Ok(command)
    }
}
