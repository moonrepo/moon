use crate::{
    find_cargo_lock, get_cargo_home, target_hash::RustTargetHash, toolchain_hash::RustToolchainHash,
};
use miette::IntoDiagnostic;
use moon_action::Operation;
use moon_action_context::ActionContext;
use moon_common::{
    Id, is_ci,
    path::{WorkspaceRelativePath, WorkspaceRelativePathBuf, exe_name, is_root_level_source},
};
use moon_config::{
    BinEntry, DependencyConfig, DependencyScope, DependencySource, HasherConfig, PlatformType,
    ProjectConfig, ProjectsAliasesList, ProjectsSourcesList, RustConfig, UnresolvedVersionSpec,
};
use moon_console::{Checkpoint, Console};
use moon_hash::ContentHasher;
use moon_logger::{debug, map_list};
use moon_platform::{Platform, Runtime, RuntimeReq};
use moon_process::Command;
use moon_project::Project;
use moon_rust_lang::{
    cargo_lock::load_lockfile_dependencies,
    cargo_toml::{CargoTomlCache, DepsSet},
    toolchain_toml::{ToolchainToml, ToolchainTomlCache},
};
use moon_rust_tool::{RustTool, get_rust_env_paths};
use moon_task::Task;
use moon_tool::{Tool, ToolManager, prepend_path_env_var};
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
use tracing::instrument;

const LOG_TARGET: &str = "moon:rust-platform";

pub struct RustPlatform {
    pub config: RustConfig,

    console: Arc<Console>,

    package_names: FxHashMap<String, Id>,

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
        console: Arc<Console>,
    ) -> Self {
        RustPlatform {
            config: config.to_owned(),
            package_names: FxHashMap::default(),
            proto_env,
            toolchain: ToolManager::new(Runtime::new(Id::raw("rust"), RuntimeReq::Global)),
            workspace_root: workspace_root.to_path_buf(),
            console,
        }
    }

    fn get_globals_dir(&self, tool: Option<&RustTool>) -> PathBuf {
        let mut globals_dir = get_cargo_home().join("bin");

        if let Some(tool) = tool {
            if let Some(new_globals_dir) = tool.tool.get_globals_dir() {
                globals_dir = new_globals_dir.to_path_buf();
            }
        }

        globals_dir
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
                    return Runtime::new_override(
                        Id::raw("rust"),
                        RuntimeReq::Toolchain(version.to_owned()),
                    );
                }
            }
        }

        if let Some(version) = &self.config.version {
            return Runtime::new(Id::raw("rust"), RuntimeReq::Toolchain(version.to_owned()));
        }

        Runtime::new(Id::raw("rust"), RuntimeReq::Global)
    }

    fn matches(&self, platform: &PlatformType, runtime: Option<&Runtime>) -> bool {
        if matches!(platform, PlatformType::Rust) {
            return true;
        }

        if let Some(runtime) = &runtime {
            return runtime.toolchain == "rust";
        }

        false
    }

    // PROJECT GRAPH

    fn find_dependency_workspace_root(
        &self,
        starting_dir: &str,
    ) -> miette::Result<WorkspaceRelativePathBuf> {
        let root = find_cargo_lock(
            &self.workspace_root.join(starting_dir),
            &self.workspace_root,
        )
        .map(|lockfile| lockfile.parent().unwrap().to_path_buf())
        .unwrap_or(self.workspace_root.clone());

        if let Some(cargo_toml) = CargoTomlCache::read(root.clone())? {
            if cargo_toml.workspace.is_some() {
                if let Ok(root) = root.strip_prefix(&self.workspace_root) {
                    return WorkspaceRelativePathBuf::from_path(root).into_diagnostic();
                }
            }
        }

        Ok(WorkspaceRelativePathBuf::default())
    }

    fn is_project_in_dependency_workspace(
        &self,
        deps_root: &WorkspaceRelativePath,
        project_source: &str,
    ) -> miette::Result<bool> {
        let deps_root_path = deps_root.to_logical_path(&self.workspace_root);

        if is_root_level_source(project_source) && deps_root_path == self.workspace_root
            || deps_root.as_str() == project_source
        {
            return Ok(true);
        }

        let Some(cargo_toml) = CargoTomlCache::read(&deps_root_path)? else {
            return Ok(false);
        };

        if let Some(workspace) = cargo_toml.workspace {
            return Ok(
                GlobSet::new_split(&workspace.members, &workspace.exclude)?.matches(project_source)
            );
        }

        Ok(false)
    }

    #[instrument(skip_all)]
    fn load_project_graph_aliases(
        &mut self,
        projects_list: &ProjectsSourcesList,
        aliases_list: &mut ProjectsAliasesList,
    ) -> miette::Result<()> {
        debug!(
            target: LOG_TARGET,
            "Loading names (aliases) from project {}'s",
            color::file("Cargo.toml")
        );

        // Extract the alias from the Cargo project relative to the lockfile
        for (id, source) in projects_list {
            let project_root = source.to_path(&self.workspace_root);

            if let Some(cargo_toml) = CargoTomlCache::read(project_root)? {
                if let Some(package) = cargo_toml.package {
                    self.package_names
                        .insert(package.name.clone(), id.to_owned());

                    if package.name != id.as_str() {
                        debug!(
                            target: LOG_TARGET,
                            "Inheriting alias {} for project {}",
                            color::label(&package.name),
                            color::id(id)
                        );

                        aliases_list.push((id.to_owned(), package.name));
                    }
                }
            }
        }

        Ok(())
    }

    #[instrument(skip(self))]
    fn load_project_implicit_dependencies(
        &self,
        project_id: &str,
        project_source: &str,
    ) -> miette::Result<Vec<DependencyConfig>> {
        let mut implicit_deps = vec![];

        debug!(
            target: LOG_TARGET,
            "Scanning {} for implicit dependency relations",
            color::id(project_id),
        );

        if let Some(cargo_toml) = CargoTomlCache::read(self.workspace_root.join(project_source))? {
            let mut find_implicit_relations = |package_deps: &DepsSet, scope: &DependencyScope| {
                for (dep_name, dep) in package_deps {
                    // Only inherit if the dependency is using the local `path = "..."` syntax
                    if dep.detail().is_some_and(|d| d.path.is_some()) {
                        if let Some(dep_project_id) = self.package_names.get(dep_name) {
                            implicit_deps.push(DependencyConfig {
                                id: dep_project_id.to_owned(),
                                scope: *scope,
                                source: DependencySource::Implicit,
                                via: Some(format!("crate {dep_name}")),
                            });
                        }
                    }
                }
            };

            find_implicit_relations(&cargo_toml.dependencies, &DependencyScope::Production);
            find_implicit_relations(&cargo_toml.dev_dependencies, &DependencyScope::Development);
            find_implicit_relations(&cargo_toml.build_dependencies, &DependencyScope::Build);
        }

        Ok(implicit_deps)
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
        Ok(Some(("Cargo.lock".to_owned(), "Cargo.toml".to_owned())))
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
                RustTool::new(
                    Arc::clone(&self.proto_env),
                    Arc::clone(&self.console),
                    &self.config,
                    &req,
                )
                .await?,
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

    #[instrument(skip_all)]
    async fn setup_tool(
        &mut self,
        _context: &ActionContext,
        runtime: &Runtime,
        last_versions: &mut FxHashMap<String, UnresolvedVersionSpec>,
    ) -> miette::Result<u8> {
        let req = &runtime.requirement;

        if !self.toolchain.has(req) {
            self.toolchain.register(
                req,
                RustTool::new(
                    Arc::clone(&self.proto_env),
                    Arc::clone(&self.console),
                    &self.config,
                    req,
                )
                .await?,
            );
        }

        Ok(self.toolchain.setup(req, last_versions).await?)
    }

    #[instrument(skip_all)]
    async fn install_deps(
        &self,
        _context: &ActionContext,
        runtime: &Runtime,
        working_dir: &Path,
    ) -> miette::Result<Vec<Operation>> {
        let tool = self.toolchain.get_for_version(&runtime.requirement)?;
        let mut operations = vec![];

        if !self.config.components.is_empty() {
            debug!(
                target: LOG_TARGET,
                "Installing rustup components: {}",
                map_list(&self.config.components, |c| color::label(c))
            );

            let mut args = vec!["component", "add"];
            args.extend(self.config.components.iter().map(|c| c.as_str()));

            operations.push(
                Operation::task_execution(format!("rustup {}", args.join(" ")))
                    .track_async(|| async {
                        self.console
                            .print_checkpoint(Checkpoint::Setup, "rustup component")?;

                        tool.exec_rustup(args, working_dir).await
                    })
                    .await?,
            );
        }

        if !self.config.targets.is_empty() {
            debug!(
                target: LOG_TARGET,
                "Installing rustup targets: {}",
                map_list(&self.config.targets, |c| color::label(c))
            );

            let mut args = vec!["target", "add"];
            args.extend(self.config.targets.iter().map(|c| c.as_str()));

            operations.push(
                Operation::task_execution(format!("rustup {}", args.join(" ")))
                    .track_async(|| async {
                        self.console
                            .print_checkpoint(Checkpoint::Setup, "rustup target")?;

                        tool.exec_rustup(args, working_dir).await
                    })
                    .await?,
            );
        }

        if find_cargo_lock(working_dir, &self.workspace_root).is_none() {
            operations.push(
                Operation::task_execution("cargo generate-lockfile")
                    .track_async(|| async {
                        self.console
                            .print_checkpoint(Checkpoint::Setup, "cargo generate-lockfile")?;

                        tool.exec_cargo(["generate-lockfile"], working_dir).await
                    })
                    .await?,
            );
        }

        let globals_dir = self.get_globals_dir(Some(tool));

        if !self.config.bins.is_empty() {
            // Install cargo-binstall if it does not exist
            if !globals_dir.join(exe_name("cargo-binstall")).exists() {
                debug!(
                    target: LOG_TARGET,
                    "{} does not exist, installing",
                    color::shell("cargo-binstall")
                );

                let package = if let Some(version) = &self.config.binstall_version {
                    format!("cargo-binstall@{version}")
                } else {
                    "cargo-binstall".into()
                };

                operations.push(
                    Operation::task_execution("cargo install cargo-binstall --force")
                        .track_async(|| {
                            tool.exec_cargo(["install", &package, "--force"], working_dir)
                        })
                        .await?,
                );
            }

            // Then attempt to install binaries
            debug!(
                target: LOG_TARGET,
                "Installing Cargo binaries: {}",
                map_list(&self.config.bins, |b| color::label(b.get_name()))
            );

            for bin in &self.config.bins {
                let mut args = vec!["binstall", "--no-confirm", "--log-level", "info"];
                let name = match bin {
                    BinEntry::Name(inner) => {
                        args.push(inner);
                        inner
                    }
                    BinEntry::Config(cfg) => {
                        if cfg.local && is_ci() {
                            continue;
                        }

                        if cfg.force {
                            args.push("--force");
                            // force = cfg.force;
                        }

                        args.push(&cfg.bin);
                        &cfg.bin
                    }
                };

                operations.push(
                    Operation::task_execution(format!("cargo {}", args.join(" ")))
                        .track_async(|| async {
                            self.console.print_checkpoint(
                                Checkpoint::Setup,
                                format!("cargo binstall {name}"),
                            )?;

                            tool.exec_cargo(args, working_dir).await
                        })
                        .await?,
                );
            }
        }

        Ok(operations)
    }

    #[instrument(skip_all)]
    async fn sync_project(
        &self,
        _context: &ActionContext,
        project: &Project,
        _dependencies: &FxHashMap<Id, Arc<Project>>,
    ) -> miette::Result<bool> {
        let mut mutated_files = false;

        let lockfile_path = find_cargo_lock(&project.root, &self.workspace_root);
        let cargo_root = match lockfile_path {
            Some(path) => path.parent().unwrap().to_owned(),
            None => project.root.to_owned(),
        };

        let legacy_toolchain_path = cargo_root.join("rust-toolchain");
        let toolchain_path = cargo_root.join("rust-toolchain.toml");

        // Convert rust-toolchain to rust-toolchain.toml
        if legacy_toolchain_path.exists() {
            debug!(
                target: LOG_TARGET,
                "Found legacy {} configuration file, converting to {}",
                color::file("rust-toolchain"),
                color::file("rust-toolchain.toml"),
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
            let version = self.config.version.as_ref().map(|v| v.to_string()).unwrap();

            if toolchain_path.exists() {
                ToolchainTomlCache::sync(toolchain_path, |cfg| {
                    if cfg.toolchain.channel.as_ref() != Some(&version) {
                        debug!(
                            target: LOG_TARGET,
                            "Syncing {} configuration file with version {}",
                            color::file("rust-toolchain.toml"),
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
                    color::file("rust-toolchain.toml"),
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

    #[instrument(skip_all)]
    async fn hash_manifest_deps(
        &self,
        _manifest_path: &Path,
        hasher: &mut ContentHasher,
        _hasher_config: &HasherConfig,
    ) -> miette::Result<()> {
        hasher.hash_content(RustToolchainHash {
            bins: &self.config.bins,
            components: &self.config.components,
            targets: &self.config.targets,
            version: self.config.version.as_ref(),
        })?;

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

    #[instrument(skip_all)]
    async fn hash_run_target(
        &self,
        project: &Project,
        _runtime: &Runtime,
        hasher: &mut ContentHasher,
        _hasher_config: &HasherConfig,
    ) -> miette::Result<()> {
        let lockfile_path = project.root.join("Cargo.lock");

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

    #[instrument(skip_all)]
    async fn create_run_target_command(
        &self,
        _context: &ActionContext,
        _project: &Project,
        task: &Task,
        runtime: &Runtime,
        _working_dir: &Path,
    ) -> miette::Result<Command> {
        let mut command = Command::new(&task.command);
        let mut args = vec![];

        match task.command.as_str() {
            // Do nothing and run as-is
            "rls" | "rust-analyzer" | "rust-gdb" | "rust-gdbgui" | "rust-lldb" | "rustc"
            | "rustdoc" | "rustfmt" | "rustup" => {}
            // Handle toolchains for cargo commands
            "cargo" => {
                if runtime.overridden {
                    args.push(format!("+{}", runtime.requirement));
                }
            }
            // Binary may be installed to ~/.cargo/bin
            _ => {
                let globals_dir = self.get_globals_dir(self.toolchain.get().ok());
                let cargo_bin = task.command.strip_prefix("cargo-").unwrap_or(&task.command);
                let cargo_bin_path = globals_dir.join(exe_name(format!("cargo-{cargo_bin}")));

                // Must run through cargo
                if cargo_bin_path.exists() {
                    command = Command::new("cargo");
                    args.push(cargo_bin.to_owned());
                }
            }
        }

        command.with_console(self.console.clone());
        command.args(&args);
        command.args(&task.args);
        command.envs_if_not_global(&task.env);
        command.env(
            "PATH",
            prepend_path_env_var(get_rust_env_paths(
                &self.proto_env,
                runtime.requirement.is_global(),
            )),
        );

        Ok(command)
    }
}
