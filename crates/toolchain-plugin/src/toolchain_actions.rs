use crate::toolchain_plugin::ToolchainPlugin;
use crate::toolchain_registry::{CallResult, ToolchainRegistry};
use moon_common::Id;
use moon_env_var::GlobalEnvBag;
use moon_pdk_api::{
    ConfigSchema, DefineDockerMetadataInput, DefineDockerMetadataOutput, ExtendProjectGraphInput,
    ExtendProjectGraphOutput, ExtendTaskCommandInput, ExtendTaskCommandOutput,
    ExtendTaskScriptInput, ExtendTaskScriptOutput, HashTaskContentsInput,
    LocateDependenciesRootInput, LocateDependenciesRootOutput, ScaffoldDockerInput,
    ScaffoldDockerOutput, SetupToolchainInput, SetupToolchainOutput, SyncOutput, SyncProjectInput,
    SyncWorkspaceInput, TeardownToolchainInput,
};
use moon_process::Command;
use moon_toolchain::{get_version_env_key, get_version_env_value, is_using_global_toolchains};
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
            if let Ok(toolchain) = self.load(id).await
                && toolchain.detect_task_usage(command, args)?
            {
                detected.push(Id::raw(id));
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

    pub async fn extend_project_graph_all<InFn>(
        &self,
        input_factory: InFn,
    ) -> miette::Result<Vec<ExtendProjectGraphOutput>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> ExtendProjectGraphInput,
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

    pub async fn extend_task_command_many<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<ExtendTaskCommandOutput>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> ExtendTaskCommandInput,
    {
        let results = self
            .call_func_all(
                "extend_task_command",
                ids,
                input_factory,
                |toolchain, input| async move { toolchain.extend_task_command(input).await },
            )
            .await?;

        Ok(results.into_iter().map(|result| result.output).collect())
    }

    pub async fn extend_task_script_many<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<ExtendTaskScriptOutput>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> ExtendTaskScriptInput,
    {
        let results = self
            .call_func_all(
                "extend_task_script",
                ids,
                input_factory,
                |toolchain, input| async move { toolchain.extend_task_script(input).await },
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

    pub async fn locate_dependencies_root_many<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<CallResult<LocateDependenciesRootOutput>>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> LocateDependenciesRootInput,
    {
        let results = self
            .call_func_all(
                "locate_dependencies_root",
                ids,
                input_factory,
                |toolchain, input| async move { toolchain.locate_dependencies_root(input).await },
            )
            .await?;

        Ok(results.into_iter().collect())
    }

    pub fn prepare_process_command(&self, command: &mut Command, bag: &GlobalEnvBag) {
        let moon = &self.host_data.moon_env;
        let proto = &self.host_data.proto_env;
        let proto_version = self.config.proto.version.to_string();

        // Inherit env vars
        command.env("PROTO_AUTO_INSTALL", "false");
        command.env("PROTO_IGNORE_MIGRATE_WARNING", "true");
        command.env("PROTO_NO_PROGRESS", "true");
        command.env("PROTO_VERSION", &proto_version);
        command.env("STARBASE_FORCE_TTY", "true");

        // Inherit versions for each toolchain
        for (id, config) in &self.plugins {
            if let Some(version) = &config.version {
                command.env_if_missing(get_version_env_key(id), get_version_env_value(version));
            }
        }

        // Abort early if using globals
        if is_using_global_toolchains(bag) {
            command.prepend_paths([moon.store_root.join("bin")]);
            return;
        }

        // Inherit lookup paths
        command.prepend_paths([
            // Always use a versioned proto first
            proto.store.inventory_dir.join("proto").join(proto_version),
            // Then fallback to shims/bins
            proto.store.shims_dir.clone(),
            proto.store.bin_dir.clone(),
            // And ensure non-proto managed moon comes last
            moon.store_root.join("bin"),
        ]);
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

    pub async fn setup_toolchain_all<InFn>(
        &self,
        input_factory: InFn,
    ) -> miette::Result<Vec<CallResult<SetupToolchainOutput>>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> SetupToolchainInput,
    {
        let ids = self.get_plugin_ids();

        self.call_func_all(
            "setup_toolchain",
            ids,
            input_factory,
            |toolchain, input| async move { toolchain.setup_toolchain(input, || Ok(())).await },
        )
        .await
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

    pub async fn teardown_toolchain_all<InFn>(
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
