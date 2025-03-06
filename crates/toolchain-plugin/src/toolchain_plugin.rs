use async_trait::async_trait;
use moon_pdk_api::{
    DockerMetadataInput, DockerMetadataOutput, HashTaskContentsInput, HashTaskContentsOutput,
    RegisterToolchainInput, RegisterToolchainOutput, ScaffoldDockerInput, ScaffoldDockerOutput,
    SyncOutput, SyncProjectInput, SyncWorkspaceInput, VirtualPath,
};
use moon_plugin::{Plugin, PluginContainer, PluginId, PluginRegistration, PluginType};
use proto_core::Tool;
use starbase_utils::glob;
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
            metadata,
            plugin,
        })
    }

    fn get_type(&self) -> PluginType {
        PluginType::Toolchain
    }
}

impl ToolchainPlugin {
    // Ensure we are dealing with real paths from this point onwards
    fn handle_output_files(&self, files: &mut [VirtualPath]) {
        for file in files {
            *file = VirtualPath::OnlyReal(
                file.real_path()
                    .unwrap_or_else(|| self.plugin.from_virtual_path(&file)),
            );
        }
    }

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
        let results = glob::walk(dir, &self.metadata.config_file_globs)?;

        Ok(!results.is_empty())
    }

    #[instrument(skip(self))]
    pub async fn docker_metadata(
        &self,
        input: DockerMetadataInput,
    ) -> miette::Result<DockerMetadataOutput> {
        let mut output: DockerMetadataOutput = self
            .plugin
            .cache_func_with("docker_metadata", input)
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
