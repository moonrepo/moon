use crate::toolchain_plugin::ToolchainPlugin;
use crate::toolchain_registry::{CallResult, ToolchainRegistry};
use moon_common::Id;
use moon_pdk_api::{
    ConfigSchema, DefineDockerMetadataInput, DefineDockerMetadataOutput, ExtendProjectInput,
    ExtendProjectOutput, HashTaskContentsInput, ScaffoldDockerInput, ScaffoldDockerOutput,
    SyncOutput, SyncProjectInput, SyncWorkspaceInput, TeardownToolchainInput,
};
use rustc_hash::FxHashMap;
use starbase_utils::json::JsonValue;
use std::path::Path;

// These implementations aggregate the call results from all toolchains
// that were requested to be executed into a better/different format
// depending on the need of the call site.

// TODO: Remove the Ok(toolchain) checks once everything is on the registry!

impl ToolchainRegistry {
    pub async fn detect_project_usage(&self, dir: &Path) -> miette::Result<Vec<Id>> {
        let mut detected = vec![];

        for id in self.get_plugin_ids() {
            if let Ok(toolchain) = self.load(id).await {
                if toolchain.detect_project_usage(dir)? {
                    detected.push(Id::raw(id));
                }
            }
        }

        Ok(detected)
    }

    pub async fn detect_task_usage(
        &self,
        ids: Vec<&Id>,
        command: &String,
        args: &[String],
    ) -> miette::Result<Vec<Id>> {
        let mut detected = vec![];

        for id in ids {
            if let Ok(toolchain) = self.load(id).await {
                if toolchain.detect_task_usage(command, args)? {
                    detected.push(Id::raw(id));
                }
            }
        }

        Ok(detected)
    }

    pub async fn define_toolchain_config_all(
        &self,
    ) -> miette::Result<FxHashMap<String, ConfigSchema>> {
        let ids = self.get_plugin_ids();

        let results = self
            .call_func_all(
                "define_toolchain_config",
                ids,
                |_, _| (),
                |toolchain, _| async move { toolchain.define_toolchain_config().await },
            )
            .await?;

        Ok(results
            .into_iter()
            .map(|result| (result.id.to_string(), result.output.schema))
            .collect())
    }

    pub async fn define_docker_metadata_all<InFn>(
        &self,
        input_factory: InFn,
    ) -> miette::Result<Vec<DefineDockerMetadataOutput>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> DefineDockerMetadataInput,
    {
        let ids = self.get_plugin_ids();

        let results = self
            .call_func_all(
                "define_docker_metadata",
                ids,
                input_factory,
                |toolchain, input| async move { toolchain.define_docker_metadata(input).await },
            )
            .await?;

        Ok(results.into_iter().map(|result| result.output).collect())
    }

    pub async fn extend_project_many<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<ExtendProjectOutput>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> ExtendProjectInput,
    {
        let results = self
            .call_func_all(
                "extend_project",
                ids,
                input_factory,
                |toolchain, input| async move { toolchain.extend_project(input).await },
            )
            .await?;

        Ok(results.into_iter().map(|result| result.output).collect())
    }

    pub async fn hash_task_contents_many<InFn>(
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

    pub async fn scaffold_docker_many<InFn>(
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

    pub async fn sync_project_many<InFn>(
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

    pub async fn sync_workspace_all<InFn>(
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

    pub async fn teardown_all<InFn>(
        &self,
        input_factory: InFn,
    ) -> miette::Result<Vec<CallResult<()>>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> TeardownToolchainInput,
    {
        let ids = self.get_plugin_ids();

        self.call_func_all(
            "teardown_toolchain",
            ids,
            input_factory,
            |toolchain, input| async move { toolchain.teardown_toolchain(input).await },
        )
        .await
    }
}
