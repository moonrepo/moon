use async_trait::async_trait;
use moon_feature_flags::glob_walk;
use moon_pdk_api::*;
use moon_plugin::{Plugin, PluginContainer, PluginId, PluginRegistration, PluginType};
use proto_core::flow::install::InstallOptions;
use proto_core::{PluginLocator, Tool, UnresolvedVersionSpec};
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
    pub async fn locate_dependencies_root(
        &self,
        input: LocateDependenciesRootInput,
    ) -> miette::Result<LocateDependenciesRootOutput> {
        let output: LocateDependenciesRootOutput = self
            .plugin
            .cache_func_with("locate_dependencies_root", input)
            .await?;

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

            // Resolve the version first so that it is available
            input.version = Some(
                tool.resolve_version(&input.configured_version, false)
                    .await?,
            );

            // Only setup if not already been
            if !tool.is_setup(&input.configured_version).await? {
                on_setup()?;

                output.installed = tool
                    .setup(
                        &input.configured_version,
                        InstallOptions {
                            skip_prompts: true,
                            skip_ui: true,
                            ..Default::default()
                        },
                    )
                    .await?;
            }

            // Locate pieces that we'll need
            tool.locate_exes_dirs().await?;
            tool.locate_globals_dirs().await?;
        }

        if self.has_func("setup_toolchain").await {
            let mut post_output: SetupToolchainOutput =
                self.plugin.call_func_with("setup_toolchain", input).await?;

            self.handle_output_files(&mut post_output.changed_files);

            output.changed_files.extend(post_output.changed_files);
        }

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
    pub async fn teardown_toolchain(&self, input: TeardownToolchainInput) -> miette::Result<()> {
        let spec = input.configured_version.clone();

        self.plugin
            .call_func_without_output("teardown_toolchain", input)
            .await?;

        if let (Some(tool), Some(spec)) = (&self.tool, &spec) {
            let mut tool = tool.write().await;
            tool.resolve_version(spec, true).await?;
            tool.teardown().await?;
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
            .field("metadata", &self.metadata)
            .finish()
    }
}
