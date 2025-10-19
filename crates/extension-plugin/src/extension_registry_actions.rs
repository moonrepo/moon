use crate::extension_plugin::ExtensionPlugin;
use crate::extension_registry::ExtensionRegistry;
use moon_pdk_api::{
    ExtendProjectGraphInput, ExtendProjectGraphOutput, ExtendTaskCommandInput,
    ExtendTaskCommandOutput, ExtendTaskScriptInput, ExtendTaskScriptOutput,
};

impl ExtensionRegistry {
    pub async fn extend_project_graph_all<InFn>(
        &self,
        input_factory: InFn,
    ) -> miette::Result<Vec<ExtendProjectGraphOutput>>
    where
        InFn: Fn(&ExtensionRegistry, &ExtensionPlugin) -> ExtendProjectGraphInput,
    {
        let results = self
            .call_func_all(
                "extend_project_graph",
                self.get_plugin_ids(),
                input_factory,
                |toolchain, input| async move { toolchain.extend_project_graph(input).await },
            )
            .await?;

        Ok(results.into_iter().map(|result| result.output).collect())
    }

    pub async fn extend_task_command_all<InFn>(
        &self,
        input_factory: InFn,
    ) -> miette::Result<Vec<ExtendTaskCommandOutput>>
    where
        InFn: Fn(&ExtensionRegistry, &ExtensionPlugin) -> ExtendTaskCommandInput,
    {
        let results = self
            .call_func_all(
                "extend_task_command",
                self.get_plugin_ids(),
                input_factory,
                |toolchain, input| async move { toolchain.extend_task_command(input).await },
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
                |toolchain, input| async move { toolchain.extend_task_script(input).await },
            )
            .await?;

        Ok(results.into_iter().map(|result| result.output).collect())
    }
}
