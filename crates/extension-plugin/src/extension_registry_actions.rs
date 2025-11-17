use crate::extension_plugin::ExtensionPlugin;
use crate::extension_registry::ExtensionRegistry;
use moon_pdk_api::{
    ExtendCommandOutput, ExtendProjectGraphInput, ExtendProjectGraphOutput, ExtendTaskCommandInput,
    ExtendTaskScriptInput, ExtendTaskScriptOutput, SyncOutput, SyncProjectInput,
    SyncWorkspaceInput,
};
use moon_plugin::CallResult;

impl ExtensionRegistry {
    pub async fn extend_project_graph_all<InFn>(
        &self,
        input_factory: InFn,
    ) -> miette::Result<Vec<CallResult<ExtensionPlugin, ExtendProjectGraphOutput>>>
    where
        InFn: Fn(&ExtensionRegistry, &ExtensionPlugin) -> ExtendProjectGraphInput,
    {
        let results = self
            .call_func_all(
                "extend_project_graph",
                self.get_plugin_ids(),
                input_factory,
                |extension, input| async move { extension.extend_project_graph(input).await },
            )
            .await?;

        Ok(results)
    }

    pub async fn extend_task_command_all<InFn>(
        &self,
        input_factory: InFn,
    ) -> miette::Result<Vec<ExtendCommandOutput>>
    where
        InFn: Fn(&ExtensionRegistry, &ExtensionPlugin) -> ExtendTaskCommandInput,
    {
        let results = self
            .call_func_all(
                "extend_task_command",
                self.get_plugin_ids(),
                input_factory,
                |extension, input| async move { extension.extend_task_command(input).await },
            )
            .await?;

        Ok(results.into_iter().map(|result| result.output).collect())
    }

    pub async fn extend_task_script_all<InFn>(
        &self,
        input_factory: InFn,
    ) -> miette::Result<Vec<ExtendTaskScriptOutput>>
    where
        InFn: Fn(&ExtensionRegistry, &ExtensionPlugin) -> ExtendTaskScriptInput,
    {
        let results = self
            .call_func_all(
                "extend_task_script",
                self.get_plugin_ids(),
                input_factory,
                |extension, input| async move { extension.extend_task_script(input).await },
            )
            .await?;

        Ok(results.into_iter().map(|result| result.output).collect())
    }

    pub async fn sync_project_all<InFn>(
        &self,
        input_factory: InFn,
    ) -> miette::Result<Vec<CallResult<ExtensionPlugin, SyncOutput>>>
    where
        InFn: Fn(&ExtensionRegistry, &ExtensionPlugin) -> SyncProjectInput,
    {
        self.call_func_all(
            "sync_project",
            self.get_plugin_ids(),
            input_factory,
            |extension, input| async move { extension.sync_project(input).await },
        )
        .await
    }

    pub async fn sync_workspace_all<InFn>(
        &self,
        input_factory: InFn,
    ) -> miette::Result<Vec<CallResult<ExtensionPlugin, SyncOutput>>>
    where
        InFn: Fn(&ExtensionRegistry, &ExtensionPlugin) -> SyncWorkspaceInput,
    {
        self.call_func_all(
            "sync_workspace",
            self.get_plugin_ids(),
            input_factory,
            |extension, input| async move { extension.sync_workspace(input).await },
        )
        .await
    }
}
