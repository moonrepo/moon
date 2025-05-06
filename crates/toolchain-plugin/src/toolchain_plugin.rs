use async_trait::async_trait;
use moon_feature_flags::glob_walk;
use moon_pdk_api::*;
use moon_plugin::{Plugin, PluginContainer, PluginId, PluginRegistration, PluginType};
use proto_core::flow::install::InstallOptions;
use proto_core::{PluginLocator, Tool, UnresolvedVersionSpec};
use starbase_utils::glob::GlobSet;
use starbase_utils::json::JsonValue;
use std::fmt;
use std::ops::Deref;
use std::path::Path;
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
                        registration.id.clone(),
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
    fn handle_output_file(&self, file: &mut VirtualPath) {
        *file = VirtualPath::OnlyReal(
            file.real_path()
                .unwrap_or_else(|| self.plugin.from_virtual_path(&file)),
        );
    }

    fn handle_output_files(&self, files: &mut [VirtualPath]) {
        for file in files {
            self.handle_output_file(file);
        }
    }

    pub fn in_dependencies_workspace(
        &self,
        output: &LocateDependenciesRootOutput,
        path: &Path,
    ) -> miette::Result<bool> {
        let Some(root) = output.root.as_ref().and_then(|root| root.real_path()) else {
            return Ok(false);
        };

        Ok(
            // Root always in the workspace
            if path == root {
                true
            }
            // Match against the provided member globs
            else if let Some(globs) = &output.members {
                GlobSet::new(globs)?.matches(path.strip_prefix(&root).unwrap_or(path))
            }
            // Otherwise a stand alone project?
            else {
                true
            },
        )
    }

    // Detection
    pub async fn supports_tier_1(&self) -> bool {
        self.has_func("parse_lockfile").await || self.has_func("parse_manifest").await
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
}

impl ToolchainPlugin {
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
        let mut add_glob = |glob: &str| {
            if !output.scaffold_globs.iter().any(|g| g == glob) {
                output.scaffold_globs.push(glob.to_owned());
            }
        };

        if let Some(name) = &self.metadata.lock_file_name {
            add_glob(name);
        }

        if let Some(name) = &self.metadata.manifest_file_name {
            add_glob(name);
        }

        if let Some(name) = &self.metadata.vendor_dir_name {
            add_glob(&format!("!{name}/**/*"));
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub fn detect_project_usage(&self, dir: &Path) -> miette::Result<bool> {
        // Do simple checks first to avoid glob overhead
        if let Some(file) = &self.metadata.manifest_file_name {
            if dir.join(file).exists() {
                return Ok(true);
            }
        }

        if let Some(file) = &self.metadata.lock_file_name {
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
    pub async fn extend_project(
        &self,
        input: ExtendProjectInput,
    ) -> miette::Result<ExtendProjectOutput> {
        let output: ExtendProjectOutput =
            self.plugin.cache_func_with("extend_project", input).await?;

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn extend_task_command(
        &self,
        mut input: ExtendTaskCommandInput,
    ) -> miette::Result<ExtendTaskCommandOutput> {
        if let Some(tool) = &self.tool {
            input.globals_dir = tool
                .read()
                .await
                .get_globals_dir()
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
                .read()
                .await
                .get_globals_dir()
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
        let mut output: HashTaskContentsOutput = self
            .plugin
            .call_func_with("hash_task_contents", input)
            .await?;

        // Include the ID for easier debugging
        for content in &mut output.contents {
            if let Some(obj) = content.as_object_mut() {
                obj.insert("@toolchain".into(), JsonValue::String(self.id.to_string()));
            }
        }

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
        input: SetupEnvironmentInput,
    ) -> miette::Result<SetupEnvironmentOutput> {
        let mut output: SetupEnvironmentOutput = self
            .plugin
            .call_func_with("setup_environment", input)
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

        // Only install if a version has been configured,
        // and the plugin provides the required APIs
        if let (Some(spec), Some(tool)) = (&input.configured_version, &self.tool) {
            let mut tool = tool.write().await;

            // Resolve the version first so that it is available
            input.version = Some(tool.resolve_version(spec, false).await?);

            // Only setup if not already been
            if !tool.is_setup(spec).await? {
                on_setup()?;

                output.installed = tool
                    .setup(
                        spec,
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
        if let (Some(spec), Some(tool)) = (&input.configured_version, &self.tool) {
            let mut tool = tool.write().await;

            input.version = Some(tool.resolve_version(spec, false).await?);

            tool.teardown().await?;
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
