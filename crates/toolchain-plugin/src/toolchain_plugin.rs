use async_trait::async_trait;
use moon_common::Id;
use moon_config::is_glob_like;
use moon_config::schematic::Schema;
use moon_config::schematic::schema::indexmap::IndexSet;
use moon_pdk_api::*;
use moon_plugin::{Plugin, PluginContainer, PluginRegistration, PluginType, inherit_path_methods};
use moon_toolchain::{DependenciesWorkspace, DependenciesWorkspaceRole};
use proto_core::flow::detect::Detector;
use proto_core::flow::install::InstallOptions;
use proto_core::flow::locate::{Locator, LocatorResponse};
use proto_core::flow::manage::Manager;
use proto_core::flow::resolve::Resolver;
use proto_core::reporter::ProtoConsole;
use proto_core::utils::log::LogWriter;
use proto_core::{
    PluginLocator, PluginType as ProtoPluginType, Tool, ToolContext, ToolSpec,
    UnresolvedVersionSpec, locate_plugin,
};
use proto_pdk_api::InstallStrategy;
use scc::hash_map::Entry;
use starbase_utils::glob::{self, GlobSet};
use std::fmt;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;
use tracing::instrument;

pub type ToolchainMetadata = RegisterToolchainOutput;

pub struct ToolchainPlugin {
    pub id: Id,
    pub locator: PluginLocator,
    pub metadata: ToolchainMetadata,

    plugin: Arc<PluginContainer>,
    tool: Option<RwLock<Tool>>,
    setup: AtomicBool,

    globals_cache: scc::HashMap<UnresolvedVersionSpec, Option<PathBuf>>,
    locations_cache: scc::HashMap<UnresolvedVersionSpec, LocatorResponse>,
}

#[async_trait]
impl Plugin for ToolchainPlugin {
    async fn new(registration: PluginRegistration) -> miette::Result<Self> {
        let plugin = Arc::new(registration.container);

        let metadata: RegisterToolchainOutput = plugin
            .cache_func_with(
                "register_toolchain",
                RegisterToolchainInput {
                    id: registration.id.clone(),
                },
            )
            .await?;

        Ok(Self {
            // Only create the proto tool instance if we know that
            // the WASM file has support for it!
            tool: if plugin.has_func("register_tool").await {
                Some(RwLock::new(
                    Tool::new(
                        ToolContext::new(registration.id_stable),
                        Arc::clone(&registration.proto_env),
                        Arc::clone(&plugin),
                    )
                    .await?,
                ))
            } else {
                None
            },
            id: registration.id,
            locator: registration.locator,
            globals_cache: scc::HashMap::new(),
            locations_cache: scc::HashMap::new(),
            setup: AtomicBool::new(false),
            metadata,
            plugin,
        })
    }

    fn get_id(&self) -> &Id {
        &self.id
    }

    fn get_type(&self) -> PluginType {
        PluginType::Toolchain
    }

    async fn has_func(&self, name: &str) -> bool {
        self.plugin.has_func(name).await
    }
}

impl ToolchainPlugin {
    inherit_path_methods!(plugin);

    fn handle_exec_command(&self, command: &mut ExecCommand) {
        if let Some(cwd) = &mut command.command.cwd {
            self.convert_to_absolute_real_path(cwd);
        }

        self.convert_output_files(&mut command.command.paths);

        for input in &mut command.inputs {
            if let Some(path) = input.get_virtual_path() {
                input.set_virtual_path(VirtualPath::new(self.plugin.to_real_path(path)));
            }
        }
    }

    async fn cache_globals_dir(&self) -> miette::Result<Option<PathBuf>> {
        if let Some(tool) = &self.tool {
            return match self
                .globals_cache
                .entry_async(UnresolvedVersionSpec::default())
                .await
            {
                Entry::Occupied(entry) => Ok(entry.get().to_owned()),
                Entry::Vacant(entry) => {
                    let tool = tool.read().await;
                    let spec = ToolSpec::default();
                    let locations = Locator::new(&tool, &spec).locate_globals_dir().await?;

                    entry.insert_entry(locations.clone());

                    Ok(locations)
                }
            };
        }

        Ok(None)
    }

    async fn cache_locations(
        &self,
        version: &UnresolvedVersionSpec,
    ) -> miette::Result<Option<LocatorResponse>> {
        if let Some(tool) = &self.tool {
            return match self.locations_cache.entry_async(version.to_owned()).await {
                Entry::Occupied(entry) => Ok(Some(entry.get().to_owned())),
                Entry::Vacant(entry) => {
                    let tool = tool.read().await;
                    let mut spec = ToolSpec::new(version.to_owned());

                    Resolver::resolve(&tool, &mut spec, false).await?;

                    let locations = Locator::locate(&tool, &spec).await?;

                    entry.insert_entry(locations.clone());

                    Ok(Some(locations))
                }
            };
        }

        Ok(None)
    }

    pub fn in_dependencies_workspace(
        &self,
        workspace: &DependenciesWorkspace,
        path: &Path,
    ) -> miette::Result<Option<DependenciesWorkspaceRole>> {
        Ok(
            // Root always in the workspace
            if path == workspace.root {
                if workspace.members.is_some() {
                    Some(DependenciesWorkspaceRole::WorkspaceRoot)
                } else {
                    Some(DependenciesWorkspaceRole::PackageRoot)
                }
            }
            // Match against the provided member globs
            else if let Some(members) = &workspace.members {
                GlobSet::new(members)?
                    .matches(path.strip_prefix(&workspace.root).unwrap_or(path))
                    .then_some(DependenciesWorkspaceRole::WorkspaceMember)
            }
            // No members means there's no workspace at all, only the root
            // package, and the path is not it
            else {
                None
            },
        )
    }

    /// Returns true if this toolchain has been setup (installed/located) during
    /// the current process. Toolchains that were never setup have no executables
    /// on disk, so locating them would fail.
    pub fn is_setup(&self) -> bool {
        self.setup.load(Ordering::Acquire)
    }

    // Detection
    pub async fn supports_tier_1(&self) -> bool {
        self.has_func("register_toolchain").await || self.has_func("detect_version_files").await
    }

    // Install dependencies
    pub async fn supports_tier_2(&self) -> bool {
        self.has_func("locate_dependencies_root").await
    }

    // Setup toolchain
    pub async fn supports_tier_3(&self) -> bool {
        self.has_func("setup_toolchain").await
            || self.tool.is_some()
                && (self.has_func("download_prebuilt").await
                    || self.has_func("native_install").await
                    || self.has_func("build_instructions").await)
    }

    #[instrument(skip(self))]
    pub async fn get_command_paths(
        &self,
        version: Option<UnresolvedVersionSpec>,
    ) -> miette::Result<Vec<PathBuf>> {
        let mut paths = IndexSet::<PathBuf>::default();

        // Toolchains that are merely declared in the workspace
        // (and were never installed) have no executables on disk,
        // and attempting to locate them would fail
        if let Some(version) = &version
            && self.is_setup()
            && let Some(locations) = self.cache_locations(version).await?
        {
            if let Some(dir) = locations.exe_file.parent() {
                paths.insert(dir.to_path_buf());
            }

            paths.extend(locations.exes_dirs);
            paths.extend(locations.globals_dirs);
        }

        Ok(paths.into_iter().collect())
    }

    #[instrument(skip(self))]
    pub async fn define_toolchain_config(&self) -> miette::Result<Option<Schema>> {
        if self.has_func("define_toolchain_config").await {
            let output: DefineToolchainConfigOutput =
                self.cache_func("define_toolchain_config").await?;

            return Ok(Some(output.schema));
        }

        Ok(None)
    }

    #[instrument(skip(self))]
    pub async fn define_docker_metadata(
        &self,
        input: DefineDockerMetadataInput,
    ) -> miette::Result<DefineDockerMetadataOutput> {
        let mut output = DefineDockerMetadataOutput::default();

        if self.has_func("define_docker_metadata").await {
            output = self
                .cache_func_with("define_docker_metadata", input)
                .await?
        };

        // Include toolchain metadata in docker
        let mut add_globs = |globs: &[String]| {
            for glob in globs {
                if !output.scaffold_globs.iter().any(|g| g == glob) {
                    output.scaffold_globs.push(glob.to_owned());
                }
            }
        };

        add_globs(&self.metadata.config_file_globs);
        add_globs(&self.metadata.lock_file_names);
        add_globs(&self.metadata.manifest_file_names);

        if let Some(name) = &self.metadata.vendor_dir_name {
            add_globs(&[format!("!{name}/**/*"), format!("!**/{name}/**/*")]);
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn define_requirements(
        &self,
        input: DefineRequirementsInput,
    ) -> miette::Result<DefineRequirementsOutput> {
        let mut output = DefineRequirementsOutput::default();

        if self.has_func("define_requirements").await {
            output = self.cache_func_with("define_requirements", input).await?;
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub fn detect_project_usage(&self, dir: &Path) -> miette::Result<bool> {
        // Do simple checks first to avoid glob overhead
        for file in &self.metadata.manifest_file_names {
            if dir.join(file).exists() {
                return Ok(true);
            }
        }

        for file in &self.metadata.lock_file_names {
            if dir.join(file).exists() {
                return Ok(true);
            }
        }

        if self.metadata.config_file_globs.is_empty() {
            return Ok(false);
        }

        // Before we glob, extract non-globs from the list
        let mut globs = vec![];

        for glob in &self.metadata.config_file_globs {
            if is_glob_like(glob) {
                globs.push(glob);
            } else if dir.join(glob).exists() {
                return Ok(true);
            }
        }

        // Oh no, heavy lookup...
        let results = glob::walk_fast(dir, globs)?;

        Ok(!results.is_empty())
    }

    #[instrument(skip(self))]
    pub fn detect_task_usage(&self, command: &String) -> miette::Result<bool> {
        if self.metadata.exe_names.contains(command) {
            return Ok(true);
        }

        // Support proto binaries like `node-20.1` or `python-3`
        for exe in &self.metadata.exe_names {
            if let Some((name, version)) = exe.split_once('-')
                && name == exe
                && version.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    #[instrument(skip(self))]
    pub async fn detect_version(
        &self,
        dir: &Path,
    ) -> miette::Result<Option<UnresolvedVersionSpec>> {
        let Some(tool) = &self.tool else {
            return Ok(None);
        };

        if self.has_func("detect_version_files").await {
            let tool = tool.read().await;

            if let Some((version, _)) = Detector::new(&tool).detect_version_from(dir).await? {
                return Ok(Some(version));
            }
        }

        Ok(None)
    }

    #[instrument(skip(self))]
    pub async fn extend_command(
        &self,
        input: ExtendCommandInput,
    ) -> miette::Result<ExtendCommandOutput> {
        let mut output = ExtendCommandOutput::default();

        if self.has_func("extend_command").await {
            output = self.cache_func_with("extend_command", input).await?;

            self.convert_output_files(&mut output.paths);
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn extend_project_graph(
        &self,
        input: ExtendProjectGraphInput,
    ) -> miette::Result<ExtendProjectGraphOutput> {
        let mut output = ExtendProjectGraphOutput::default();

        if self.has_func("extend_project_graph").await {
            output = self.cache_func_with("extend_project_graph", input).await?;

            self.convert_virtual_files(&mut output.input_files);
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn extend_task_command(
        &self,
        mut input: ExtendTaskCommandInput,
    ) -> miette::Result<ExtendCommandOutput> {
        input.globals_dir = self
            .cache_globals_dir()
            .await?
            .map(|dir| self.to_virtual_path(dir));

        let mut output = ExtendCommandOutput::default();

        if self.has_func("extend_task_command").await {
            output = self.cache_func_with("extend_task_command", input).await?;

            self.convert_output_files(&mut output.paths);
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn extend_task_script(
        &self,
        mut input: ExtendTaskScriptInput,
    ) -> miette::Result<ExtendTaskScriptOutput> {
        input.globals_dir = self
            .cache_globals_dir()
            .await?
            .map(|dir| self.to_virtual_path(dir));

        let mut output = ExtendTaskScriptOutput::default();

        if self.has_func("extend_task_script").await {
            output = self.cache_func_with("extend_task_script", input).await?;

            self.convert_output_files(&mut output.paths);
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn hash_task_contents(
        &self,
        input: HashTaskContentsInput,
    ) -> miette::Result<HashTaskContentsOutput> {
        let mut output = HashTaskContentsOutput::default();

        if self.has_func("hash_task_contents").await {
            output = self.cache_func_with("hash_task_contents", input).await?;
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn initialize_toolchain(
        &self,
        input: InitializeToolchainInput,
    ) -> miette::Result<InitializeToolchainOutput> {
        // Function exists check happens in the CLI!
        let output: InitializeToolchainOutput =
            self.cache_func_with("initialize_toolchain", input).await?;

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn install_dependencies(
        &self,
        input: InstallDependenciesInput,
    ) -> miette::Result<InstallDependenciesOutput> {
        let mut output = InstallDependenciesOutput::default();

        if self.has_func("install_dependencies").await {
            output = self.call_func_with("install_dependencies", input).await?;

            if let Some(command) = &mut output.install_command {
                self.handle_exec_command(command);
            }

            if let Some(command) = &mut output.dedupe_command {
                self.handle_exec_command(command);
            }
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn is_installed_in_proto(
        &self,
        spec: Option<&UnresolvedVersionSpec>,
    ) -> miette::Result<bool> {
        if let (Some(tool), Some(spec)) = (&self.tool, spec) {
            let tool = tool.read().await;
            let mut spec = ToolSpec::new(spec.to_owned());

            Resolver::resolve(&tool, &mut spec, false).await?;

            return Ok(tool.is_installed(&spec));
        }

        Ok(false)
    }

    #[instrument(skip(self))]
    pub async fn locate_dependencies_root(
        &self,
        input: LocateDependenciesRootInput,
    ) -> miette::Result<Option<DependenciesWorkspace>> {
        if self.has_func("locate_dependencies_root").await {
            let output: LocateDependenciesRootOutput = self
                .cache_func_with("locate_dependencies_root", input)
                .await?;

            if let Some(root) = output.root {
                return Ok(Some(DependenciesWorkspace {
                    root: self.to_real_path(&root).to_path_buf(),
                    members: output.members,
                }));
            }
        }

        Ok(None)
    }

    #[instrument(skip(self))]
    pub async fn parse_lock(&self, input: ParseLockInput) -> miette::Result<ParseLockOutput> {
        let mut output = ParseLockOutput::default();

        if self.has_func("parse_lock").await {
            output = self.cache_func_with("parse_lock", input).await?;
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn parse_manifest(
        &self,
        input: ParseManifestInput,
    ) -> miette::Result<ParseManifestOutput> {
        let mut output = ParseManifestOutput::default();

        if self.has_func("parse_manifest").await {
            output = self.cache_func_with("parse_manifest", input).await?;
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn prune_docker(&self, input: PruneDockerInput) -> miette::Result<PruneDockerOutput> {
        let mut output = PruneDockerOutput::default();

        if self.has_func("prune_docker").await {
            output = self.call_func_with("prune_docker", input).await?;

            self.convert_virtual_files(&mut output.changed_files);
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn scaffold_docker(
        &self,
        input: ScaffoldDockerInput,
    ) -> miette::Result<ScaffoldDockerOutput> {
        let mut output = ScaffoldDockerOutput::default();

        if self.has_func("scaffold_docker").await {
            output = self.call_func_with("scaffold_docker", input).await?;

            self.convert_virtual_files(&mut output.copied_files);
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn setup_environment(
        &self,
        mut input: SetupEnvironmentInput,
    ) -> miette::Result<SetupEnvironmentOutput> {
        input.globals_dir = self
            .cache_globals_dir()
            .await?
            .map(|dir| self.to_virtual_path(dir));

        // Function exists check happens in the action!
        let mut output: SetupEnvironmentOutput =
            self.cache_func_with("setup_environment", input).await?;

        self.convert_virtual_files(&mut output.changed_files);

        for command in &mut output.commands {
            self.handle_exec_command(command);
        }

        Ok(output)
    }

    #[instrument(skip(self, console, on_setup))]
    pub async fn setup_toolchain(
        &self,
        mut input: SetupToolchainInput,
        console: Option<ProtoConsole>,
        on_setup: impl FnOnce() -> miette::Result<()>,
    ) -> miette::Result<SetupToolchainOutput> {
        let mut output = SetupToolchainOutput::default();

        if let Some(tool) = &self.tool {
            let mut tool = tool.write().await;

            // Only install if a version has been configured
            if let Some(version) = &input.configured_version {
                let mut spec = ToolSpec::new(version.to_owned());

                // Resolve the version first so that it is available
                input.version = Some(Resolver::resolve(&tool, &mut spec, false).await?);

                // Only setup if not already been
                if !tool.is_installed(&spec) {
                    on_setup()?;

                    // Honor the tool's declared install strategy (e.g. Ruby
                    // builds from source); otherwise proto defaults to a
                    // prebuilt download and errors for source-only tools.
                    let strategy = tool.metadata.default_install_strategy;

                    // Only the build-from-source path routes through proto's
                    // Builder, which requires a console + log writer. Prebuilt
                    // installs don't, so leave them `None` to avoid the
                    // allocation and any change to prebuilt logging behavior.
                    let building = matches!(strategy, InstallStrategy::BuildFromSource);
                    let mut manager = Manager::new(&mut tool);

                    output.installed = manager
                        .install(
                            &mut spec,
                            InstallOptions {
                                skip_prompts: true,
                                skip_ui: true,
                                strategy,
                                console: building.then_some(console).flatten(),
                                log_writer: building.then(LogWriter::default),
                                ..Default::default()
                            },
                        )
                        .await?
                        .is_some();

                    // We must sync the manifest for tool's not managed by
                    // proto, like Rust (via rustup)
                    manager.sync_manifest().await?;
                }

                // Track used at so that proto's auto-clean doesn't remove it
                if let Some(version) = &spec.version {
                    tool.inventory.create_product(version).track_used_at()?;
                }

                self.setup.store(true, Ordering::Relaxed);
            }

            // Pre-load the tool plugin so that task executions
            // avoid network race conditions and collisions
            if let Ok(loader) = tool.proto.get_plugin_loader()
                && let Some(locator) = tool.locator.clone().or_else(|| {
                    locate_plugin(&tool.context, &tool.proto, ProtoPluginType::Tool).ok()
                })
            {
                let _ = loader.load_plugin(&tool.context.id, &locator).await;
            }
        }

        // This should always run, regardless of the install outcome
        if self.has_func("setup_toolchain").await {
            let mut post_output: SetupToolchainOutput =
                self.call_func_with("setup_toolchain", input).await?;

            self.convert_virtual_files(&mut post_output.changed_files);

            output.operations.extend(post_output.operations);
            output.changed_files.extend(post_output.changed_files);
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn sync_project(&self, input: SyncProjectInput) -> miette::Result<SyncOutput> {
        let mut output = SyncOutput::default();

        if self.has_func("sync_project").await {
            output = self.call_func_with("sync_project", input).await?;

            self.convert_virtual_files(&mut output.changed_files);
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn sync_workspace(&self, input: SyncWorkspaceInput) -> miette::Result<SyncOutput> {
        let mut output = SyncOutput::default();

        if self.has_func("sync_workspace").await {
            output = self.call_func_with("sync_workspace", input).await?;

            self.convert_virtual_files(&mut output.changed_files);
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn teardown_toolchain(
        &self,
        mut input: TeardownToolchainInput,
    ) -> miette::Result<()> {
        if let (Some(version), Some(tool)) = (&input.configured_version, &self.tool) {
            let mut tool = tool.write().await;
            let mut spec = ToolSpec::new(version.to_owned());

            input.version = Some(Resolver::resolve(&tool, &mut spec, false).await?);

            Manager::new(&mut tool).uninstall(&mut spec).await?;
        }

        if self.has_func("teardown_toolchain").await {
            self.call_func_without_output("teardown_toolchain", input)
                .await?;
        }

        Ok(())
    }
}

impl Deref for ToolchainPlugin {
    type Target = PluginContainer;

    fn deref(&self) -> &Self::Target {
        &self.plugin
    }
}

impl fmt::Debug for ToolchainPlugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolchainPlugin")
            .field("id", &self.id)
            .field("locator", &self.locator)
            .field("metadata", &self.metadata)
            .finish()
    }
}
