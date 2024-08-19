use async_trait::async_trait;
use moon_pdk_api::{
    MoonContext, SyncWorkspaceInput, ToolchainMetadataInput, ToolchainMetadataOutput,
};
use moon_plugin::{Plugin, PluginContainer, PluginId, PluginRegistration, PluginType};
use proto_core::Tool;
use std::fmt;
use std::sync::Arc;
use tracing::{debug, instrument};

pub struct ToolchainPlugin {
    pub id: PluginId,
    pub metadata: ToolchainMetadataOutput,

    plugin: Arc<PluginContainer>,

    #[allow(dead_code)]
    tool: Option<Tool>,
}

impl ToolchainPlugin {
    #[instrument(skip_all)]
    pub async fn sync_workspace(&self, context: MoonContext) -> miette::Result<()> {
        if !self.plugin.has_func("sync_workspace").await {
            return Ok(());
        }

        debug!(toolchain = self.id.as_str(), "Syncing workspace");

        self.plugin
            .call_func_without_output("sync_workspace", SyncWorkspaceInput { context })
            .await?;

        Ok(())
    }

    // #[instrument(skip_all)]
    // pub async fn sync_project(
    //     &self,
    //     project: SyncProjectRecord,
    //     dependencies: FxHashMap<Id, SyncProjectRecord>,
    //     context: MoonContext,
    // ) -> miette::Result<()> {
    //     if !self.plugin.has_func("sync_project").await {
    //         return Ok(());
    //     }

    //     debug!(toolchain = self.id.as_str(), "Syncing project");

    //     self.plugin
    //         .call_func_without_output(
    //             "sync_project",
    //             SyncProjectInput {
    //                 context,
    //                 dependencies,
    //                 project,
    //             },
    //         )
    //         .await?;

    //     Ok(())
    // }
}

#[async_trait]
impl Plugin for ToolchainPlugin {
    async fn new(registration: PluginRegistration) -> miette::Result<Self> {
        let plugin = Arc::new(registration.container);

        let metadata: ToolchainMetadataOutput = plugin
            .cache_func_with(
                "register_toolchain",
                ToolchainMetadataInput {
                    id: registration.id.to_string(),
                },
            )
            .await?;

        Ok(Self {
            // Only create the proto tool instance if we know that
            // the WASM file has support for it!
            tool: if plugin.has_func("register_tool").await {
                Some(
                    Tool::new(
                        registration.id.clone(),
                        Arc::clone(&registration.proto_env),
                        Arc::clone(&plugin),
                    )
                    .await?,
                )
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
