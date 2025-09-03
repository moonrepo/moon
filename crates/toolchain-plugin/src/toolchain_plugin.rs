use async_trait::async_trait;
use moon_config::schematic::schema::indexmap::IndexSet;
use moon_feature_flags::glob_walk;
use moon_pdk_api::*;
use moon_plugin::{Plugin, PluginContainer, PluginId, PluginRegistration, PluginType};
use proto_core::flow::install::InstallOptions;
use proto_core::{
    PluginLocator, PluginType as ProtoPluginType, Tool, ToolContext, ToolSpec,
    UnresolvedVersionSpec, locate_plugin,
};
use starbase_utils::glob::GlobSet;
use std::fmt;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::instrument;

pub type ToolchainMetadata = RegisterToolchainOutput;

pub struct ToolchainPlugin {
    pub id: PluginId,
    pub locator: PluginLocator,
    pub metadata: ToolchainMetadata,

    plugin: Arc<PluginContainer>,

    #[allow(dead_code)]
    tool: Option<RwLock<Tool>>,
}

#[async_trait]
impl Plugin for ToolchainPlugin {
    async fn new(registration: PluginRegistration) -> miette::Result<Self> {
        let plugin = Arc::new(registration.container);

        let metadata: RegisterToolchainOutput = plugin
            .cache_func_with(
                "register_toolchain",
                RegisterToolchainInput {
                    id: registration.id.to_string(),
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
            metadata,
            plugin,
        })
    }

    fn get_type(&self) -> PluginType {
        PluginType::Toolchain
    }
}

impl ToolchainPlugin {
    fn handle_output_file(&self, file: &mut PathBuf) {
        *file = self.plugin.from_virtual_path(&file);
    }

    fn handle_output_files(&self, files: &mut [PathBuf]) {
        for file in files {
            self.handle_output_file(file);
        }
    }

    pub fn in_dependencies_workspace(
        &self,
        output: &LocateDependenciesRootOutput,
        path: &Path,
    ) -> miette::Result<bool> {
        let Some(root) = &output.root else {
            return Ok(false);
        };

        Ok(
            // Root always in the workspace
            if path == root {
                true
            }
            // Match against the provided member globs
            else if let Some(globs) = &output.members {
                GlobSet::new(globs)?.matches(path.strip_prefix(root).unwrap_or(path))
            }
            // Otherwise a stand alone project?
            else {
                true
            },
        )
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
                    || self.has_func("native_install").await)
    }

    #[instrument(skip(self))]
    pub async fn get_command_paths(
        &self,
        version: Option<UnresolvedVersionSpec>,
    ) -> miette::Result<Vec<PathBuf>> {
        let mut paths = IndexSet::<PathBuf>::default();

        if let Some(version) = &version
            && let Some(tool) = &self.tool
        {
            let mut tool = tool.write().await;
            let spec = ToolSpec::new(version.to_owned());

            tool.resolve_version(&spec, false).await?;

            if let Some(dir) = tool.locate_exe_file().await?.parent() {
                paths.insert(dir.to_path_buf());
            }

            paths.extend(tool.locate_exes_dirs().await?);
            paths.extend(tool.locate_globals_dirs().await?);
        }

        Ok(paths.into_iter().collect())
    }

    #[instrument(skip(self))]
    pub async fn define_toolchain_config(&self) -> miette::Result<DefineToolchainConfigOutput> {
        let output: DefineToolchainConfigOutput =
            self.plugin.cache_func("define_toolchain_config").await?;

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn define_docker_metadata(
        &self,
        input: DefineDockerMetadataInput,
    ) -> miette::Result<DefineDockerMetadataOutput> {
        let mut output: DefineDockerMetadataOutput = self
            .plugin
            .cache_func_with("define_docker_metadata", input)
            .await?;

        // Include toolchain metadata in docker
        let mut add_globs = |globs: &[String]| {
            for glob in globs {
                if !output.scaffold_globs.iter().any(|g| g == glob) {
                    output.scaffold_globs.push(glob.to_owned());
                }
            }
        };

        add_globs(&self.metadata.lock_file_names);
        add_globs(&self.metadata.manifest_file_names);

        if let Some(name) = &self.metadata.vendor_dir_name {
            add_globs(&[format!("!{name}/**/*")]);
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn define_requirements(
        &self,
        input: DefineRequirementsInput,
    ) -> miette::Result<DefineRequirementsOutput> {
        let output: DefineRequirementsOutput = self
            .plugin
            .cache_func_with("define_requirements", input)
            .await?;

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

        // Oh no, heavy lookup...
        let results = glob_walk(dir, &self.metadata.config_file_globs)?;

        Ok(!results.is_empty())
    }

    #[instrument(skip(self))]
    pub fn detect_task_usage(&self, command: &String, _args: &[String]) -> miette::Result<bool> {
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

        let tool = tool.read().await;

        if let Some((version, _)) = tool.detect_version_from(dir).await? {
            return Ok(Some(version));
        }

        Ok(None)
    }

    #[instrument(skip(self))]
    pub async fn extend_project_graph(
        &self,
        input: ExtendProjectGraphInput,
    ) -> miette::Result<ExtendProjectGraphOutput> {
        let mut output: ExtendProjectGraphOutput = self
            .plugin
            .cache_func_with("extend_project_graph", input)
            .await?;

        self.handle_output_files(&mut output.input_files);

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn extend_task_command(
        &self,
        mut input: ExtendTaskCommandInput,
    ) -> miette::Result<ExtendTaskCommandOutput> {
        if let Some(tool) = &self.tool {
            input.globals_dir = tool
                .write()
                .await
                .locate_globals_dir()
                .await?
                .map(|dir| self.to_virtual_path(dir));
        }

        let output: ExtendTaskCommandOutput = self
            .plugin
            .cache_func_with("extend_task_command", input)
            .await?;

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn extend_task_script(
        &self,
        mut input: ExtendTaskScriptInput,
    ) -> miette::Result<ExtendTaskScriptOutput> {
        if let Some(tool) = &self.tool {
            input.globals_dir = tool
                .write()
                .await
                .locate_globals_dir()
                .await?
                .map(|dir| self.to_virtual_path(dir));
        }

        let output: ExtendTaskScriptOutput = self
            .plugin
            .cache_func_with("extend_task_script", input)
            .await?;

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn hash_task_contents(
        &self,
        input: HashTaskContentsInput,
    ) -> miette::Result<HashTaskContentsOutput> {
        let output: HashTaskContentsOutput = self
            .plugin
            .cache_func_with("hash_task_contents", input)
            .await?;

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn initialize_toolchain(
        &self,
        input: InitializeToolchainInput,
    ) -> miette::Result<InitializeToolchainOutput> {
        let output: InitializeToolchainOutput = self
            .plugin
            .cache_func_with("initialize_toolchain", input)
            .await?;

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn install_dependencies(
        &self,
        input: InstallDependenciesInput,
    ) -> miette::Result<InstallDependenciesOutput> {
        let output: InstallDependenciesOutput = self
            .plugin
            .call_func_with("install_dependencies", input)
            .await?;

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn locate_dependencies_root(
        &self,
        input: LocateDependenciesRootInput,
    ) -> miette::Result<LocateDependenciesRootOutput> {
        let mut output: LocateDependenciesRootOutput = self
            .plugin
            .cache_func_with("locate_dependencies_root", input)
            .await?;

        if let Some(root) = &mut output.root {
            self.handle_output_file(root);
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn parse_lock(&self, input: ParseLockInput) -> miette::Result<ParseLockOutput> {
        let output: ParseLockOutput = self.plugin.cache_func_with("parse_lock", input).await?;

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn parse_manifest(
        &self,
        input: ParseManifestInput,
    ) -> miette::Result<ParseManifestOutput> {
        let output: ParseManifestOutput =
            self.plugin.cache_func_with("parse_manifest", input).await?;

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn prune_docker(&self, input: PruneDockerInput) -> miette::Result<PruneDockerOutput> {
        let mut output: PruneDockerOutput =
            self.plugin.call_func_with("prune_docker", input).await?;

        self.handle_output_files(&mut output.changed_files);

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn scaffold_docker(
        &self,
        input: ScaffoldDockerInput,
    ) -> miette::Result<ScaffoldDockerOutput> {
        let mut output: ScaffoldDockerOutput =
            self.plugin.call_func_with("scaffold_docker", input).await?;

        self.handle_output_files(&mut output.copied_files);

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn setup_environment(
        &self,
        mut input: SetupEnvironmentInput,
    ) -> miette::Result<SetupEnvironmentOutput> {
        if let Some(tool) = &self.tool {
            input.globals_dir = tool
                .write()
                .await
                .locate_globals_dir()
                .await?
                .map(|dir| self.to_virtual_path(dir));
        }

        let mut output: SetupEnvironmentOutput = self
            .plugin
            .cache_func_with("setup_environment", input)
            .await?;

        self.handle_output_files(&mut output.changed_files);

        Ok(output)
    }

    #[instrument(skip(self, on_setup))]
    pub async fn setup_toolchain(
        &self,
        mut input: SetupToolchainInput,
        on_setup: impl FnOnce() -> miette::Result<()>,
    ) -> miette::Result<SetupToolchainOutput> {
        let mut output = SetupToolchainOutput::default();

        if let Some(tool) = &self.tool {
            let mut tool = tool.write().await;

            // Only install if a version has been configured
            if let Some(version) = &input.configured_version {
                let spec = ToolSpec::new(version.to_owned());

                // Resolve the version first so that it is available
                input.version = Some(tool.resolve_version(&spec, false).await?);

                // Only setup if not already been
                if !tool.is_setup(&spec).await? {
                    on_setup()?;

                    output.installed = tool
                        .setup(
                            &spec,
                            InstallOptions {
                                skip_prompts: true,
                                skip_ui: true,
                                ..Default::default()
                            },
                        )
                        .await?
                        .is_some();
                }

                // Locate pieces that we'll need
                tool.locate_exes_dirs().await?;
                tool.locate_globals_dirs().await?;
            }

            // Pre-load the tool plugin so that task executions
            // avoid network race conditions and collisions
            if let Ok(loader) = tool.proto.get_plugin_loader()
                && let Some(locator) = tool.locator.clone().or_else(|| {
                    locate_plugin(&tool.context.id, &tool.proto, ProtoPluginType::Tool).ok()
                })
            {
                let _ = loader.load_plugin(&tool.context.id, &locator).await;
            }
        }

        // This should always run, regardless of the install outcome
        if self.has_func("setup_toolchain").await {
            let mut post_output: SetupToolchainOutput =
                self.plugin.call_func_with("setup_toolchain", input).await?;

            self.handle_output_files(&mut post_output.changed_files);

            output.operations.extend(post_output.operations);
            output.changed_files.extend(post_output.changed_files);
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn sync_project(&self, input: SyncProjectInput) -> miette::Result<SyncOutput> {
        let mut output: SyncOutput = self.plugin.call_func_with("sync_project", input).await?;

        self.handle_output_files(&mut output.changed_files);

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn sync_workspace(&self, input: SyncWorkspaceInput) -> miette::Result<SyncOutput> {
        let mut output: SyncOutput = self.plugin.call_func_with("sync_workspace", input).await?;

        self.handle_output_files(&mut output.changed_files);

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn teardown_toolchain(
        &self,
        mut input: TeardownToolchainInput,
    ) -> miette::Result<()> {
        if let (Some(version), Some(tool)) = (&input.configured_version, &self.tool) {
            let mut tool = tool.write().await;
            let spec = ToolSpec::new(version.to_owned());

            input.version = Some(tool.resolve_version(&spec, false).await?);

            tool.teardown(&spec).await?;
        }

        self.plugin
            .call_func_without_output("teardown_toolchain", input)
            .await?;

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
            .field("metadata", &self.metadata)
            .finish()
    }
}
