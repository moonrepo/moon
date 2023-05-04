use crate::target_hasher::RustTargetHasher;
use moon_action_context::ActionContext;
use moon_config::{
    HasherConfig, PlatformType, ProjectConfig, ProjectsAliasesMap, ProjectsSourcesMap, RustConfig,
};
use moon_error::MoonError;
use moon_hasher::HashSet;
use moon_logger::debug;
use moon_platform::{Platform, Runtime, Version};
use moon_project::{Project, ProjectError};
use moon_rust_lang::{
    cargo_lock::load_lockfile_dependencies,
    cargo_toml::CargoTomlCache,
    toolchain_toml::{ToolchainToml, ToolchainTomlCache},
    CARGO, RUSTUP, RUSTUP_LEGACY,
};
use moon_rust_tool::RustTool;
use moon_task::Task;
use moon_tool::{Tool, ToolError, ToolManager};
use moon_utils::{async_trait, process::Command};
use proto::{rust::RustLanguage, Executable, Proto};
use rustc_hash::FxHashMap;
use starbase_styles::color;
use starbase_utils::fs::{self, FsError};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

const LOG_TARGET: &str = "moon:rust-platform";

#[derive(Debug)]
pub struct RustPlatform {
    pub config: RustConfig,

    toolchain: ToolManager<RustTool>,

    #[allow(dead_code)]
    pub workspace_root: PathBuf,
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

    fn get_runtime_from_config(&self, project_config: Option<&ProjectConfig>) -> Runtime {
        if let Some(config) = &project_config {
            if let Some(rust_config) = &config.toolchain.rust {
                if let Some(version) = &rust_config.version {
                    return Runtime::Rust(Version::new_override(version));
                }
            }
        }

        if let Some(version) = &self.config.version {
            return Runtime::Rust(Version::new(version));
        }

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

    fn is_project_in_dependency_workspace(&self, _project: &Project) -> Result<bool, MoonError> {
        // Always assume Cargo is running from the root
        Ok(true)
    }

    fn load_project_graph_aliases(
        &mut self,
        projects_map: &ProjectsSourcesMap,
        aliases_map: &mut ProjectsAliasesMap,
    ) -> Result<(), MoonError> {
        // Extract the alias from the Cargo project relative to the lockfile
        for (id, source) in projects_map {
            let project_root = self.workspace_root.join(source);

            if !project_root.join(CARGO.lockfile).exists() {
                continue;
            }

            if let Some(cargo_toml) = CargoTomlCache::read(project_root)? {
                if let Some(package) = cargo_toml.package {
                    if &package.name != id {
                        debug!(
                            target: LOG_TARGET,
                            "Inheriting alias {} for project {}",
                            color::label(&package.name),
                            color::id(id)
                        );

                        aliases_map.insert(package.name, id.to_owned());
                    }
                }
            }

            break;
        }

        Ok(())
    }

    // TOOLCHAIN

    fn is_toolchain_enabled(&self) -> Result<bool, ToolError> {
        Ok(self.config.version.is_some())
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
        let version = match &self.config.version {
            Some(v) => Version::new(v),
            None => Version::new_global(),
        };

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
        let lockfile_path = working_dir.join(CARGO.lockfile);

        if !lockfile_path.exists() {
            let tool = self.toolchain.get_for_version(runtime.version())?;

            tool.exec_cargo(&["generate-lockfile"], working_dir).await?;
        }

        Ok(())
    }

    async fn sync_project(
        &self,
        _context: &ActionContext,
        project: &Project,
        _dependencies: &FxHashMap<String, &Project>,
    ) -> Result<bool, ProjectError> {
        let mut mutated_files = false;
        let legacy_toolchain_path = project.root.join(RUSTUP_LEGACY.version_file);
        let toolchain_path = project.root.join(RUSTUP.version_file);

        // Convert rust-toolchain to rust-toolchain.toml
        if legacy_toolchain_path.exists() {
            debug!(
                target: LOG_TARGET,
                "Found legacy {} configuration file, converting to {}",
                color::file(RUSTUP_LEGACY.version_file),
                color::file(RUSTUP.version_file),
            );

            let handle_error = |error: FsError| ProjectError::Moon(MoonError::StarFs(error));
            let legacy_contents = fs::read_file(&legacy_toolchain_path).map_err(handle_error)?;

            if legacy_contents.contains("[toolchain]") {
                fs::rename(&legacy_toolchain_path, &toolchain_path).map_err(handle_error)?;
            } else {
                fs::remove_file(&legacy_toolchain_path).map_err(handle_error)?;

                ToolchainTomlCache::write(
                    &toolchain_path,
                    ToolchainToml::new_with_channel(&legacy_contents),
                )?;
            }

            mutated_files = true;
        }

        // Sync version into `toolchain.channel`
        if self.config.sync_toolchain_config && self.config.version.is_some() {
            let version = self.config.version.clone().unwrap();

            if toolchain_path.exists() {
                ToolchainTomlCache::sync(toolchain_path, |cfg| {
                    if cfg.toolchain.channel != self.config.version {
                        debug!(
                            target: LOG_TARGET,
                            "Syncing {} configuration file with version {}",
                            color::file(RUSTUP.version_file),
                            color::symbol(&version),
                        );

                        cfg.toolchain.channel = Some(version);
                        mutated_files = true;

                        return Ok(true);
                    }

                    Ok(false)
                })?;
            } else {
                debug!(
                    target: LOG_TARGET,
                    "Creating {} configuration file",
                    color::file(RUSTUP.version_file),
                );

                ToolchainTomlCache::write(
                    toolchain_path,
                    ToolchainToml::new_with_channel(&version),
                )?;

                mutated_files = true;
            }
        }

        Ok(mutated_files)
    }

    async fn hash_manifest_deps(
        &self,
        _manifest_path: &Path,
        _hashset: &mut HashSet,
        _hasher_config: &HasherConfig,
    ) -> Result<(), ToolError> {
        // NOTE: Since Cargo has no way to install dependencies, we don't actually need this!

        // let mut hasher = RustManifestHasher::default();
        // let root_cargo_toml = CargoTomlCache::read(&self.workspace_root)?;

        // let mut hash_deps = |deps: DepsSet| {
        //     for (key, value) in deps {
        //         let dep = match value {
        //             Dependency::Simple(version) => DependencyDetail {
        //                 version: Some(version),
        //                 ..DependencyDetail::default()
        //             },
        //             Dependency::Inherited(data) => {
        //                 let mut detail = DependencyDetail {
        //                     features: data.features,
        //                     optional: data.optional,
        //                     ..DependencyDetail::default()
        //                 };

        //                 if let Some(root) = &root_cargo_toml {
        //                     if let Some(root_dep) = root.get_detailed_workspace_dependency(&key) {
        //                         detail.version = root_dep.version;
        //                         detail.features.extend(root_dep.features);
        //                     }
        //                 }

        //                 detail
        //             }
        //             Dependency::Detailed(detail) => detail,
        //         };

        //         hasher.dependencies.insert(key, dep);
        //     }
        // };

        // if let Some(cargo_toml) = CargoTomlCache::read(manifest_path)? {
        //     hash_deps(cargo_toml.build_dependencies);
        //     hash_deps(cargo_toml.dev_dependencies);
        //     hash_deps(cargo_toml.dependencies);

        //     if let Some(package) = cargo_toml.package {
        //         hasher.name = package.name;
        //     } else if cargo_toml.workspace.is_some() {
        //         hasher.name = "workspace".into();
        //     }
        // }

        // hashset.hash(hasher);

        Ok(())
    }

    async fn hash_run_target(
        &self,
        project: &Project,
        _runtime: &Runtime,
        hashset: &mut HashSet,
        _hasher_config: &HasherConfig,
    ) -> Result<(), ToolError> {
        let lockfile_path = project.root.join(CARGO.lockfile);

        // Not running in the Cargo workspace root, not sure how to handle!
        if !lockfile_path.exists() {
            return Ok(());
        }

        let mut hasher = RustTargetHasher::new(None);

        // Use the resolved dependencies from the lockfile directly,
        // since it also takes into account features and workspace members.
        hasher.locked_dependencies =
            BTreeMap::from_iter(load_lockfile_dependencies(lockfile_path)?);

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

            let cargo_bin = if task.command.starts_with("cargo-") {
                &task.command[6..]
            } else {
                &task.command
            };
            let cargo_bin_path = globals_dir.join(format!("cargo-{}", cargo_bin));

            // Must run through cargo
            if cargo_bin_path.exists() {
                command = Command::new("cargo");
                command.arg(cargo_bin);

                // Truly global and doesn't run through cargo
            } else if global_bin_path.exists() {
                command = Command::new(&global_bin_path);

                // Not found so error!
            } else {
                return Err(ToolError::MissingBinary(
                    "Cargo binary".into(),
                    cargo_bin.to_owned(),
                ));
            }
        }

        command.args(&task.args).envs(&task.env).cwd(working_dir);

        Ok(command)
    }
}
