use crate::manifest_hasher::RustManifestHasher;
use crate::target_hasher::RustTargetHasher;
use moon_action_context::ActionContext;
use moon_config::{
    DependencyConfig, HasherConfig, HasherOptimization, PlatformType, ProjectConfig,
    ProjectsAliasesMap, RustConfig,
};
use moon_error::MoonError;
use moon_hasher::HashSet;
use moon_lang::LockfileDependencyVersions;
use moon_platform::{Platform, Runtime, Version};
use moon_project::{Project, ProjectError};
use moon_rust_lang::{
    cargo_lock::load_lockfile_dependencies,
    cargo_toml::{CargoTomlCache, CargoTomlExt, Dependency, DependencyDetail, DepsSet},
    CARGO,
};
use moon_rust_tool::RustTool;
use moon_task::Task;
use moon_tool::{Tool, ToolError, ToolManager};
use moon_utils::{async_trait, process::Command};
use proto::{rust::RustLanguage, Executable, Proto};
use rustc_hash::FxHashMap;
use starbase_utils::{fs, glob::GlobSet};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

// const LOG_TARGET: &str = "moon:rust-platform";

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

    fn is_project_in_dependency_workspace(&self, project: &Project) -> Result<bool, MoonError> {
        let mut in_workspace = false;

        // Root package is always considered within the workspace
        if project.root == self.workspace_root {
            return Ok(true);
        }

        if let Some(cargo_toml) = CargoTomlCache::read(&self.workspace_root)? {
            if let Some(workspace) = cargo_toml.workspace {
                in_workspace = GlobSet::new_split(&workspace.members, &workspace.exclude)?
                    .matches(&project.source);
            }
        }

        Ok(in_workspace)
    }

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
        _runtime: &Runtime,
        _working_dir: &Path,
    ) -> Result<(), ToolError> {
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
        manifest_path: &Path,
        hashset: &mut HashSet,
        _hasher_config: &HasherConfig,
    ) -> Result<(), ToolError> {
        let mut hasher = RustManifestHasher::default();
        let root_cargo_toml = CargoTomlCache::read(&self.workspace_root)?;

        let mut hash_deps = |deps: DepsSet| {
            for (key, value) in deps {
                let dep = match value {
                    Dependency::Simple(version) => DependencyDetail {
                        version: Some(version),
                        ..DependencyDetail::default()
                    },
                    Dependency::Inherited(data) => {
                        let mut detail = DependencyDetail {
                            features: data.features,
                            optional: data.optional,
                            ..DependencyDetail::default()
                        };

                        if let Some(root) = &root_cargo_toml {
                            if let Some(root_dep) = root.get_detailed_workspace_dependency(&key) {
                                detail.version = root_dep.version;
                                detail.features.extend(root_dep.features);
                            }
                        }

                        detail
                    }
                    Dependency::Detailed(detail) => detail,
                };

                hasher.dependencies.insert(key, dep);
            }
        };

        if let Some(cargo_toml) = CargoTomlCache::read(manifest_path)? {
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
        let mut hasher = RustTargetHasher::new(None);
        let mut resolved_dependencies: LockfileDependencyVersions = FxHashMap::default();

        if matches!(hasher_config.optimization, HasherOptimization::Accuracy) {
            if let Some(lockfile_path) = fs::find_upwards(CARGO.lockfile, &project.root) {
                resolved_dependencies.extend(load_lockfile_dependencies(lockfile_path)?);
            }
        }

        let mut copy_deps = |deps: BTreeMap<String, Dependency>| {
            for (name, dep) in deps {
                if let Some(resolved_versions) = resolved_dependencies.get(&name) {
                    hasher
                        .locked_dependencies
                        .insert(name.to_owned(), resolved_versions.to_owned());
                } else {
                    let version = match dep {
                        Dependency::Simple(version) => version,
                        Dependency::Inherited(_) => "workspace".into(),
                        Dependency::Detailed(detail) => detail.version.unwrap_or_default(),
                    };

                    hasher
                        .locked_dependencies
                        .insert(name.to_owned(), vec![version]);
                }
            }
        };

        if let Some(cargo_toml) = CargoTomlCache::read(&project.root)? {
            copy_deps(cargo_toml.build_dependencies);
            copy_deps(cargo_toml.dev_dependencies);
            copy_deps(cargo_toml.dependencies);
        }

        hashset.hash(hasher);

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

        // Binary may be installed to ~/.cargo/bin
        if task.command != "cargo" && !task.command.starts_with("rust") {
            let globals_dir = RustLanguage::new(Proto::new()?).get_globals_bin_dir()?;
            let global_bin_path = globals_dir.join(&task.command);
            let cargo_bin_path = globals_dir.join(format!("cargo-{}", &task.command));

            // Truly global and doesn't run through cargo
            if global_bin_path.exists() {
                command = Command::new(&global_bin_path);

            // Must run through cargo
            } else if cargo_bin_path.exists() {
                command = Command::new("cargo");
                command.arg(&task.command);
            }
        }

        command.args(&task.args).envs(&task.env).cwd(working_dir);

        Ok(command)
    }
}
