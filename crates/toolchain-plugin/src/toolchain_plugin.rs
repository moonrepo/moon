use async_trait::async_trait;
use moon_pdk_api::{
    RegisterToolchainInput, RegisterToolchainOutput, SyncProjectInput, SyncProjectOutput,
    SyncWorkspaceInput, SyncWorkspaceOutput,
};
use moon_plugin::{Plugin, PluginContainer, PluginId, PluginRegistration, PluginType};
use proto_core::Tool;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, instrument};

pub type ToolchainMetadata = RegisterToolchainOutput;

pub struct ToolchainPlugin {
    pub id: PluginId,
    pub metadata: ToolchainMetadata,

    plugin: Arc<PluginContainer>,

    #[allow(dead_code)]
    tool: Option<RwLock<Tool>>,
}

impl ToolchainPlugin {
    pub async fn has_func(&self, func: &str) -> bool {
        self.plugin.has_func(func).await
    }

    #[instrument(skip(self))]
    pub async fn sync_workspace(
        &self,
        input: SyncWorkspaceInput,
    ) -> miette::Result<Option<SyncWorkspaceOutput>> {
        if !self.plugin.has_func("sync_workspace").await {
            return Ok(None);
        }

        debug!(toolchain_id = self.id.as_str(), "Syncing workspace");

        let output: SyncWorkspaceOutput =
            self.plugin.call_func_with("sync_workspace", input).await?;

        debug!(toolchain_id = self.id.as_str(), "Synced workspace");

        Ok(Some(output))
    }

    #[instrument(skip(self))]
    pub async fn sync_project(&self, input: SyncProjectInput) -> miette::Result<Vec<PathBuf>> {
        let mut files = vec![];

        if !self.plugin.has_func("sync_project").await {
            return Ok(files);
        }

        debug!(toolchain_id = self.id.as_str(), "Syncing project");

        let output: SyncProjectOutput = self.plugin.call_func_with("sync_project", input).await?;

        for file in output.changed_files {
            files.push(
                file.real_path()
                    .unwrap_or_else(|| self.plugin.from_virtual_path(&file)),
            );
        }

        debug!(toolchain_id = self.id.as_str(), changed_files = ?files, "Synced project");

        Ok(files)
    }
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

impl fmt::Debug for ToolchainPlugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolchainPlugin")
            .field("id", &self.id)
            .field("metadata", &self.metadata)
            .finish()
    }
}
