use crate::{
    bins_hash::RustBinsHash, find_cargo_lock, get_cargo_home, target_hash::RustTargetHash,
};
use moon_action_context::ActionContext;
use moon_common::{is_ci, Id};
use moon_config::{
    BinEntry, HasherConfig, PlatformType, ProjectConfig, ProjectsAliasesMap, ProjectsSourcesMap,
    RustConfig,
};
use moon_hash::ContentHasher;
use moon_logger::{debug, map_list};
use moon_platform::{Platform, Runtime, Version};
use moon_process::Command;
use moon_project::Project;
use moon_rust_lang::{
    cargo_lock::load_lockfile_dependencies,
    cargo_toml::CargoTomlCache,
    toolchain_toml::{ToolchainToml, ToolchainTomlCache},
    CARGO, RUSTUP, RUSTUP_LEGACY,
};
use moon_rust_tool::RustTool;
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

const LOG_TARGET: &str = "moon:rust-platform";

pub struct RustPlatform {
    pub config: RustConfig,

    proto_env: Arc<ProtoEnvironment>,

    toolchain: ToolManager<RustTool>,

    #[allow(dead_code)]
    pub workspace_root: PathBuf,
}

impl RustPlatform {
    pub fn new(
        config: &RustConfig,
        workspace_root: &Path,
        proto_env: Arc<ProtoEnvironment>,
    ) -> Self {
        RustPlatform {
            config: config.to_owned(),
            proto_env,
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

    fn is_project_in_dependency_workspace(&self, project_source: &str) -> miette::Result<bool> {
        let Some(lockfile_path) = find_cargo_lock(&self.workspace_root.join(project_source)) else {
            return Ok(false);
        };

        let Some(cargo_toml) = CargoTomlCache::read(lockfile_path.parent().unwrap())? else {
            return Ok(false);
        };

        if let Some(workspace) = cargo_toml.workspace {
            return Ok(
                GlobSet::new_split(&workspace.members, &workspace.exclude)?.matches(project_source)
            );
        }

        Ok(false)
    }

    fn load_project_graph_aliases(
        &mut self,
        projects_map: &ProjectsSourcesMap,
        aliases_map: &mut ProjectsAliasesMap,
    ) -> miette::Result<()> {
        // Extract the alias from the Cargo project relative to the lockfile
        for (id, source) in projects_map {
            let project_root = source.to_path(&self.workspace_root);

            if let Some(cargo_toml) = CargoTomlCache::read(project_root)? {
                if let Some(package) = cargo_toml.package {
                    if package.name != id.as_str() {
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
        }

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

    fn get_tool_for_version(&self, version: Version) -> miette::Result<Box<&dyn Tool>> {
        let tool = self.toolchain.get_for_version(&version)?;

        Ok(Box::new(tool))
    }

    fn get_dependency_configs(&self) -> miette::Result<Option<(String, String)>> {
        Ok(Some((CARGO.lockfile.to_owned(), CARGO.manifest.to_owned())))
    }

    async fn setup_toolchain(&mut self) -> miette::Result<()> {
        let version = match &self.config.version {
            Some(v) => Version::new(v),
            None => Version::new_global(),
        };

        let mut last_versions = FxHashMap::default();

        if !self.toolchain.has(&version) {
            self.toolchain.register(
                &version,
                RustTool::new(&self.proto_env, &self.config, &version).await?,
            );
        }

        self.toolchain.setup(&version, &mut last_versions).await?;

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
        last_versions: &mut FxHashMap<String, String>,
    ) -> miette::Result<u8> {
        let version = runtime.version();

        if !self.toolchain.has(&version) {
            self.toolchain.register(
                &version,
                RustTool::new(&self.proto_env, &self.config, &version).await?,
            );
        }

        Ok(self.toolchain.setup(&version, last_versions).await?)
    }

    async fn install_deps(
        &self,
        _context: &ActionContext,
        runtime: &Runtime,
        working_dir: &Path,
    ) -> miette::Result<()> {
        let tool = self.toolchain.get_for_version(runtime.version())?;

        if find_cargo_lock(working_dir).is_none() {
            print_checkpoint("cargo generate-lockfile", Checkpoint::Setup);

            tool.exec_cargo(["generate-lockfile"], working_dir).await?;
        }

        if !self.config.bins.is_empty() {
            print_checkpoint("cargo binstall", Checkpoint::Setup);

            let globals_dir = tool.tool.get_globals_bin_dir();

            // Install cargo-binstall if it does not exist
            if globals_dir.is_none()
                || globals_dir.is_some_and(|p| !p.join("cargo-binstall").exists())
            {
                debug!(
                    target: LOG_TARGET,
                    "{} does not exist, installing",
                    color::shell("cargo-binstall")
                );

                tool.exec_cargo(["install", "cargo-binstall"], working_dir)
                    .await?;
            }

            // Then attempt to install binaries
            debug!(
                target: LOG_TARGET,
                "Installing Cargo binaries: {}",
                map_list(&self.config.bins, |b| color::label(b.get_name()))
            );

            for bin in &self.config.bins {
                let mut args = vec!["binstall", "--no-confirm", "--log-level", "info"];

                match bin {
                    BinEntry::Name(name) => args.push(name),
                    BinEntry::Config(cfg) => {
                        if cfg.local && is_ci() {
                            continue;
                        }

                        if cfg.force {
                            args.push("--force");
                        }

                        args.push(&cfg.bin);
                    }
                };

                tool.exec_cargo(args, working_dir).await?;
            }
        }

        Ok(())
    }

    async fn sync_project(
        &self,
        _context: &ActionContext,
        project: &Project,
        _dependencies: &FxHashMap<Id, Arc<Project>>,
    ) -> miette::Result<bool> {
        let mut mutated_files = false;

        let lockfile_path = find_cargo_lock(&project.root);
        let cargo_root = match lockfile_path {
            Some(path) => path.parent().unwrap().to_owned(),
            None => project.root.to_owned(),
        };

        let legacy_toolchain_path = cargo_root.join(RUSTUP_LEGACY.version_file);
        let toolchain_path = cargo_root.join(RUSTUP.version_file);

        // Convert rust-toolchain to rust-toolchain.toml
        if legacy_toolchain_path.exists() {
            debug!(
                target: LOG_TARGET,
                "Found legacy {} configuration file, converting to {}",
                color::file(RUSTUP_LEGACY.version_file),
                color::file(RUSTUP.version_file),
            );

            let legacy_contents = fs::read_file(&legacy_toolchain_path)?;

            if legacy_contents.contains("[toolchain]") {
                fs::rename(&legacy_toolchain_path, &toolchain_path)?;
            } else {
                fs::remove_file(&legacy_toolchain_path)?;

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
                            color::hash(&version),
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
        hasher: &mut ContentHasher,
        _hasher_config: &HasherConfig,
    ) -> miette::Result<()> {
        if !self.config.bins.is_empty() {
            hasher.hash_content(RustBinsHash {
                bins: &self.config.bins,
            })?;
        }

        // NOTE: Since Cargo has no way to install dependencies, we don't actually need this!
        // However, will leave it around incase a new cargo command is added in the future.

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
        hasher: &mut ContentHasher,
        _hasher_config: &HasherConfig,
    ) -> miette::Result<()> {
        let lockfile_path = project.root.join(CARGO.lockfile);

        // Not running in the Cargo workspace root, not sure how to handle!
        if !lockfile_path.exists() {
            return Ok(());
        }

        let mut hash = RustTargetHash::new(None);

        // Use the resolved dependencies from the lockfile directly,
        // since it also takes into account features and workspace members.
        hash.locked_dependencies = BTreeMap::from_iter(load_lockfile_dependencies(lockfile_path)?);

        hasher.hash_content(hash)?;

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
        let mut args = vec![];

        match task.command.as_str() {
            // Do nothing and run as-is
            "rls" | "rust-analyzer" | "rust-gdb" | "rust-gdbgui" | "rust-lldb" | "rustc"
            | "rustdoc" | "rustfmt" | "rustup" => {}
            // Handle toolchains for cargo commands
            "cargo" => {
                let version = runtime.version();

                if version.is_override() {
                    args.push(format!("+{}", version.number));
                }
            }
            // Binary may be installed to ~/.cargo/bin
            _ => {
                let mut globals_dir = get_cargo_home().join("bin");

                if let Ok(tool) = self.toolchain.get() {
                    if let Some(new_globals_dir) = tool.tool.get_globals_bin_dir() {
                        globals_dir = new_globals_dir.to_path_buf();
                    }
                }

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
                    args.push(cargo_bin.to_owned());

                    // Truly global and doesn't run through cargo
                } else if global_bin_path.exists() {
                    command = Command::new(&global_bin_path);

                    // Not found so error!
                } else {
                    return Err(ToolError::MissingBinary(
                        "Cargo binary".into(),
                        cargo_bin.to_owned(),
                    )
                    .into());
                }
            }
        }

        command
            .args(&args)
            .args(&task.args)
            .envs(&task.env)
            .cwd(working_dir);

        Ok(command)
    }
}
