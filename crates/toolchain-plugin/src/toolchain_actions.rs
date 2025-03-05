use crate::toolchain_plugin::ToolchainPlugin;
use crate::toolchain_registry::{CallResult, ToolchainRegistry};
use moon_common::Id;
use moon_pdk_api::{
    DockerMetadataInput, DockerMetadataOutput, HashTaskContentsInput, ScaffoldDockerInput,
    ScaffoldDockerOutput, SyncOutput, SyncProjectInput, SyncWorkspaceInput,
};
use starbase_utils::json::JsonValue;
use std::path::Path;

// These implementations aggregate the call results from all toolchains
// that were requested to be executed into a better/different format
// depending on the need of the call site.

impl ToolchainRegistry {
    pub async fn detect_usage(&self, dir: &Path) -> miette::Result<Vec<Id>> {
        let mut detected = vec![];

        for id in self.get_plugin_ids() {
            if let Ok(toolchain) = self.load(id).await {
                if toolchain.detect_usage(dir)? {
                    detected.push(Id::raw(id));
                }
            }
        }

        Ok(detected)
    }

    pub async fn docker_metadata<InFn>(
        &self,
        input_factory: InFn,
    ) -> miette::Result<Vec<DockerMetadataOutput>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> DockerMetadataInput,
    {
        let ids = self.get_plugin_ids();

        let results = self
            .call_func_all(
                "docker_metadata",
                ids,
                input_factory,
                |toolchain, input| async move { toolchain.docker_metadata(input).await },
            )
            .await?;

        Ok(results.into_iter().map(|result| result.output).collect())
    }

    pub async fn hash_task_contents<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<JsonValue>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> HashTaskContentsInput,
    {
        let results = self
            .call_func_all(
                "hash_task_contents",
                ids,
                input_factory,
                |toolchain, input| async move { toolchain.hash_task_contents(input).await },
            )
            .await?;

        Ok(results
            .into_iter()
            .flat_map(|result| result.output.contents)
            .collect())
    }

    pub async fn scaffold_docker<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<ScaffoldDockerOutput>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> ScaffoldDockerInput,
    {
        let results = self
            .call_func_all(
                "scaffold_docker",
                ids,
                input_factory,
                |toolchain, input| async move { toolchain.scaffold_docker(input).await },
            )
            .await?;

        Ok(results.into_iter().map(|result| result.output).collect())
    }

    pub async fn sync_project<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<CallResult<SyncOutput>>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> SyncProjectInput,
    {
        self.call_func_all(
            "sync_project",
            ids,
            input_factory,
            |toolchain, input| async move { toolchain.sync_project(input).await },
        )
        .await
    }

    pub async fn sync_workspace<InFn>(
        &self,
        input_factory: InFn,
    ) -> miette::Result<Vec<CallResult<SyncOutput>>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> SyncWorkspaceInput,
    {
        let ids = self.get_plugin_ids();

        self.call_func_all(
            "sync_workspace",
            ids,
            input_factory,
            |toolchain, input| async move { toolchain.sync_workspace(input).await },
        )
        .await
    }
}
