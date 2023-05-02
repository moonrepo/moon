use crate::target_hasher::RustTargetHasher;
use moon_action_context::ActionContext;
use moon_config::{
    DependencyConfig, HasherConfig, HasherOptimization, PlatformType, ProjectConfig,
    ProjectsAliasesMap, RustConfig, TypeScriptConfig,
};
use moon_error::MoonError;
use moon_hasher::{DepsHasher, HashSet};
use moon_logger::debug;
use moon_platform::{Platform, Runtime, Version};
use moon_project::{Project, ProjectError};
use moon_rust_lang::{
    cargo_toml::{CargoTomlCache, DepsSet},
    CARGO,
};
use moon_rust_tool::RustTool;
use moon_task::Task;
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_tool::{Tool, ToolError, ToolManager};
use moon_utils::{async_trait, process::Command};
use proto::{get_sha256_hash_of_file, Proto};
use rustc_hash::FxHashMap;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

const LOG_TARGET: &str = "moon:rust-platform";

#[derive(Debug)]
pub struct RustPlatform {
    config: RustConfig,

    toolchain: ToolManager<RustTool>,

    workspace_root: PathBuf,
}

impl RustPlatform {
    pub fn new(config: &RustConfig, workspace_root: &Path) -> Self {
        RustPlatform {
            config: config.to_owned(),
            toolchain: ToolManager::new(Runtime::Rust(Version::new_global())),
            workspace_root: workspace_root.to_path_buf(),
        }
    }
}

#[async_trait]
impl Platform for RustPlatform {
    fn get_type(&self) -> PlatformType {
        PlatformType::Rust
    }

    fn get_runtime_from_config(&self, _project_config: Option<&ProjectConfig>) -> Runtime {
        Runtime::Rust(Version::new_global())
    }

    fn matches(&self, platform: &PlatformType, runtime: Option<&Runtime>) -> bool {
        if matches!(platform, PlatformType::Rust) {
            return true;
        }

        if let Some(runtime) = &runtime {
            return matches!(runtime, Runtime::Rust(_));
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

    fn is_toolchain_enabled(&self) -> Result<bool, ToolError> {
        Ok(false)
    }

    fn get_tool(&self) -> Result<Box<&dyn Tool>, ToolError> {
        let tool = self.toolchain.get()?;

        Ok(Box::new(tool))
    }

    fn get_tool_for_version(&self, version: Version) -> Result<Box<&dyn Tool>, ToolError> {
        let tool = self.toolchain.get_for_version(&version)?;

        Ok(Box::new(tool))
    }

    fn get_dependency_configs(&self) -> Result<Option<(String, String)>, ToolError> {
        Ok(Some((CARGO.lockfile.to_owned(), CARGO.manifest.to_owned())))
    }

    async fn setup_toolchain(&mut self) -> Result<(), ToolError> {
        // let version = match &self.config.version {
        //     Some(v) => Version::new(v),
        //     None => Version::new_global(),
        // };

        let version = Version::new_global();
        let mut last_versions = FxHashMap::default();

        if !self.toolchain.has(&version) {
            self.toolchain.register(
                &version,
                RustTool::new(&Proto::new()?, &self.config, &version)?,
            );
        }

        self.toolchain.setup(&version, &mut last_versions).await?;

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
                RustTool::new(&Proto::new()?, &self.config, &version)?,
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
        let mut hasher = DepsHasher::new("cargo".into());

        let mut hash_deps = |deps: DepsSet| {
            for (key, value) in deps {
                hasher.hash_dep(key, serde_json::to_string(&value).unwrap());
            }
        };

        if let Ok(Some(cargo_toml)) = CargoTomlCache::read(manifest_path) {
            hash_deps(cargo_toml.build_dependencies);
            hash_deps(cargo_toml.dev_dependencies);
            hash_deps(cargo_toml.dependencies);

            if let Some(package) = cargo_toml.package {
                hasher.name = package.name;
            } else if cargo_toml.workspace.is_some() {
                hasher.name = "workspace".into();
            }
        }

        hashset.hash(hasher);

        Ok(())
    }

    async fn hash_run_target(
        &self,
        project: &Project,
        _runtime: &Runtime,
        hashset: &mut HashSet,
        hasher_config: &HasherConfig,
    ) -> Result<(), ToolError> {
        // let mut rust_hasher = RustTargetHasher::new(None);

        // if matches!(hasher_config.optimization, HasherOptimization::Accuracy)
        //     && self.config.lockfile
        // {
        //     let resolved_dependencies =
        //         load_lockfile_dependencies(project.root.join(DENO_DEPS.lockfile))?;

        //     rust_hasher.hash_deps(BTreeMap::from_iter(resolved_dependencies));
        // };

        // hashset.hash(rust_hasher);

        // if let Ok(Some(rust_json)) = RustJson::read(&project.root) {
        //     if let Some(compiler_options) = &rust_json.compiler_options {
        //         let mut ts_hasher = TypeScriptTargetHasher::default();
        //         ts_hasher.hash_compiler_options(compiler_options);

        //         hashset.hash(ts_hasher);
        //     }
        // }

        // // Do we need this if we're using compiler options from rust.json?
        // if let Some(typescript_config) = &self.typescript_config {
        //     let ts_hasher = TypeScriptTargetHasher::generate(
        //         typescript_config,
        //         &self.workspace_root,
        //         &project.root,
        //     )?;

        //     hashset.hash(ts_hasher);
        // }

        Ok(())
    }

    async fn create_run_target_command(
        &self,
        _context: &ActionContext,
        _project: &Project,
        task: &Task,
        _runtime: &Runtime,
        working_dir: &Path,
    ) -> Result<Command, ToolError> {
        let mut command = Command::new(&task.command);

        command.args(&task.args).envs(&task.env).cwd(working_dir);

        Ok(command)
    }
}
