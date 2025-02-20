use async_trait::async_trait;
use moon_pdk_api::{
    HashTaskContentsInput, HashTaskContentsOutput, RegisterToolchainInput, RegisterToolchainOutput,
    SyncOutput, SyncProjectInput, SyncWorkspaceInput, VirtualPath,
};
use moon_plugin::{Plugin, PluginContainer, PluginId, PluginRegistration, PluginType};
use proto_core::Tool;
use std::fmt;
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
    pub fn handle_sync_output(&self, mut output: SyncOutput) -> SyncOutput {
        // Ensure we are dealing with real paths from this point onwards
        for file in &mut output.changed_files {
            *file = VirtualPath::OnlyReal(
                file.real_path()
                    .unwrap_or_else(|| self.plugin.from_virtual_path(&file)),
            );
        }

        output
    }

    pub async fn has_func(&self, func: &str) -> bool {
        self.plugin.has_func(func).await
    }

    #[instrument(skip(self))]
    pub async fn hash_task_contents(
        &self,
        input: HashTaskContentsInput,
    ) -> miette::Result<HashTaskContentsOutput> {
        debug!(toolchain_id = self.id.as_str(), "Hashing task contents");

        let output: HashTaskContentsOutput = self
            .plugin
            .call_func_with("hash_task_contents", input)
            .await?;

        debug!(toolchain_id = self.id.as_str(), "Hashed task contents");

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn sync_project(&self, input: SyncProjectInput) -> miette::Result<SyncOutput> {
        debug!(toolchain_id = self.id.as_str(), "Syncing project");

        let output: SyncOutput = self.plugin.call_func_with("sync_project", input).await?;
        let output = self.handle_sync_output(output);

        debug!(
            toolchain_id = self.id.as_str(),
            changed_files = ?output.changed_files,
            "Synced project",
        );

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn sync_workspace(&self, input: SyncWorkspaceInput) -> miette::Result<SyncOutput> {
        debug!(toolchain_id = self.id.as_str(), "Syncing workspace");

        let output: SyncOutput = self.plugin.call_func_with("sync_workspace", input).await?;
        let output = self.handle_sync_output(output);

        debug!(toolchain_id = self.id.as_str(), "Synced workspace");

        Ok(output)
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
