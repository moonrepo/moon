use moon_common::Id;
use moon_pdk_api::{MoonContext, SyncProjectInput, SyncProjectRecord, SyncWorkspaceInput};
use moon_plugin::{Plugin, PluginContainer, PluginId, PluginRegistration, PluginType};
use proto_core::Tool;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use tracing::{debug, instrument};

pub struct PlatformPlugin {
    pub id: PluginId,

    plugin: Arc<PluginContainer>,

    #[allow(dead_code)]
    tool: Option<Tool>,
}

impl PlatformPlugin {
    #[instrument(skip_all)]
    pub fn sync_workspace(&self, context: MoonContext) -> miette::Result<()> {
        if !self.plugin.has_func("sync_workspace") {
            return Ok(());
        }

        debug!(platform = self.id.as_str(), "Syncing workspace");

        self.plugin
            .call_func_without_output("sync_workspace", SyncWorkspaceInput { context })?;

        Ok(())
    }

    #[instrument(skip_all)]
    pub fn sync_project(
        &self,
        project: SyncProjectRecord,
        dependencies: FxHashMap<Id, SyncProjectRecord>,
        context: MoonContext,
    ) -> miette::Result<()> {
        if !self.plugin.has_func("sync_project") {
            return Ok(());
        }

        debug!(platform = self.id.as_str(), "Syncing project");

        self.plugin.call_func_without_output(
            "sync_project",
            SyncProjectInput {
                context,
                dependencies,
                project,
            },
        )?;

        Ok(())
    }
}

impl Plugin for PlatformPlugin {
    fn new(registration: PluginRegistration) -> miette::Result<Self> {
        let plugin = Arc::new(registration.container);

        Ok(Self {
            // Only create the proto tool instance if we know that
            // the WASM file has support for it!
            tool: if plugin.has_func("register_tool") {
                Some(Tool::new(
                    registration.id.clone(),
                    Arc::clone(&registration.proto_env),
                    Arc::clone(&plugin),
                )?)
            } else {
                None
            },
            id: registration.id,
            plugin,
        })
    }

    fn get_type(&self) -> PluginType {
        PluginType::Platform
    }
}
