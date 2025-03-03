use async_trait::async_trait;
use moon_pdk_api::*;
use moon_plugin::{Plugin, PluginContainer, PluginId, PluginRegistration, PluginType};
use std::fmt;
use std::sync::Arc;
use tracing::instrument;

pub type ExtensionMetadata = RegisterExtensionOutput;

pub struct ExtensionPlugin {
    pub id: PluginId,
    pub metadata: ExtensionMetadata,

    plugin: Arc<PluginContainer>,
}

impl ExtensionPlugin {
    #[instrument(skip(self, context))]
    pub async fn execute(&self, args: Vec<String>, context: MoonContext) -> miette::Result<()> {
        self.plugin
            .call_func_without_output("execute_extension", ExecuteExtensionInput { args, context })
            .await?;

        Ok(())
    }
}

#[async_trait]
impl Plugin for ExtensionPlugin {
    async fn new(registration: PluginRegistration) -> miette::Result<Self> {
        let plugin = Arc::new(registration.container);

        let metadata: RegisterExtensionOutput = plugin
            .cache_func_with(
                "register_extension",
                RegisterExtensionInput {
                    id: registration.id.to_string(),
                },
            )
            .await?;

        Ok(Self {
            id: registration.id,
            metadata,
            plugin,
        })
    }

    fn get_type(&self) -> PluginType {
        PluginType::Extension
    }
}

impl fmt::Debug for ExtensionPlugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExtensionPlugin")
            .field("id", &self.id)
            .field("metadata", &self.metadata)
            .finish()
    }
}
