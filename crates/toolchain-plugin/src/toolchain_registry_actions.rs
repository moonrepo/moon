use crate::toolchain_plugin::ToolchainPlugin;
use crate::toolchain_registry::{CallResult, ToolchainRegistry};
use moon_common::Id;
use moon_config::ProjectConfig;
use moon_env_var::GlobalEnvBag;
use moon_pdk_api::{
    ConfigSchema, DefineDockerMetadataInput, DefineDockerMetadataOutput, DefineRequirementsInput,
    DefineRequirementsOutput, ExtendProjectGraphInput, ExtendProjectGraphOutput,
    ExtendTaskCommandInput, ExtendTaskCommandOutput, ExtendTaskScriptInput, ExtendTaskScriptOutput,
    HashTaskContentsInput, LocateDependenciesRootInput, LocateDependenciesRootOutput,
    ScaffoldDockerInput, ScaffoldDockerOutput, SetupToolchainInput, SetupToolchainOutput,
    SyncOutput, SyncProjectInput, SyncWorkspaceInput, TeardownToolchainInput,
};
use moon_process::Command;
use moon_toolchain::{
    get_version_env_key, get_version_env_value, is_using_global_toolchain,
    is_using_global_toolchains,
};
use proto_core::UnresolvedVersionSpec;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_utils::json::JsonValue;
use std::path::{Path, PathBuf};

// These implementations aggregate the call results from all toolchains
// that were requested to be executed into a better/different format
// depending on the need of the call site.

// TODO: Remove the Ok(toolchain) checks once everything is on the registry!

pub struct CommandAugment<'a> {
    pub add_env: bool,
    pub add_path: bool,
    pub version: &'a UnresolvedVersionSpec,
}

impl ToolchainRegistry {
    pub async fn augment_command(
        &self,
        command: &mut Command,
        bag: &GlobalEnvBag,
        augments: FxHashMap<Id, CommandAugment<'_>>,
    ) -> miette::Result<()> {
        let proto_version = self.config.proto.version.to_string();

        // Inherit common proto env vars
        command.env("PROTO_AUTO_INSTALL", "false");
        command.env("PROTO_IGNORE_MIGRATE_WARNING", "true");
        command.env("PROTO_NO_PROGRESS", "true");
        command.env("PROTO_VERSION", &proto_version);
        command.env("STARBASE_FORCE_TTY", "true");

        // If no versions defined, then proto shouldn't be used
        if augments.is_empty() {
            return Ok(());
        }

        // Otherwise inherit the version for each toolchain as an env var
        let mut toolchain_ids = vec![];

        for (id, augment) in &augments {
            if is_using_global_toolchain(bag, id) {
                continue;
            }

            if augment.add_env {
                command.env(
                    get_version_env_key(id),
                    get_version_env_value(augment.version),
                );
            }

            if augment.add_path {
                toolchain_ids.push(id);
            }
        }

        // If forced to globals, don't inject any paths but keep env vars
        if is_using_global_toolchains(bag) {
            return Ok(());
        }

        // Otherwise inherit common proto paths
        let moon = &self.host_data.moon_env;
        let proto = &self.host_data.proto_env;

        command.prepend_paths([
            proto.store.inventory_dir.join("proto").join(proto_version),
            proto.store.shims_dir.clone(),
            proto.store.bin_dir.clone(),
            moon.store_root.join("bin"),
        ]);

        // Then inherit toolchain specific paths
        if !toolchain_ids.is_empty() {
            command.prepend_paths(
                self.get_command_paths(toolchain_ids, |_, toolchain| {
                    augments
                        .get(toolchain.id.as_str())
                        .map(|augment| (*augment.version).to_owned())
                })
                .await?,
            );
        }

        Ok(())
    }

    pub async fn augment_command_for_project(
        &self,
        command: &mut Command,
        bag: &GlobalEnvBag,
        config: &ProjectConfig,
    ) -> miette::Result<()> {
        self.augment_command(command, bag, self.create_command_augments(Some(config)))
            .await
    }

    pub async fn augment_command_for_workspace(
        &self,
        command: &mut Command,
        bag: &GlobalEnvBag,
    ) -> miette::Result<()> {
        self.augment_command(command, bag, self.create_command_augments(None))
            .await
    }

    pub fn create_command_augments<'a, 'b: 'a>(
        &'a self,
        project_config: Option<&'b ProjectConfig>,
    ) -> FxHashMap<Id, CommandAugment<'a>> {
        let mut map = FxHashMap::default();

        for (id, config) in &self.plugins {
            if let Some(version) = &config.version {
                map.insert(
                    Id::raw(id),
                    CommandAugment {
                        add_env: true,
                        add_path: true,
                        version,
                    },
                );
            }
        }

        if let Some(project_config) = project_config {
            for (id, config) in &project_config.toolchain.plugins {
                if !config.is_enabled() {
                    map.remove(id);
                    continue;
                }

                if let Some(version) = config.get_version() {
                    map.insert(
                        id.to_owned(),
                        CommandAugment {
                            add_env: true,
                            add_path: true,
                            version,
                        },
                    );
                }
            }
        }

        map
    }

    pub async fn get_command_paths<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<PathBuf>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> Option<UnresolvedVersionSpec>,
    {
        let results = self
            .call_func_all_with_check(
                "get_command_paths",
                ids,
                input_factory,
                |toolchain, input| async move { toolchain.get_command_paths(input).await },
                true,
            )
            .await?;

        Ok(results
            .into_iter()
            .flat_map(|result| result.output)
            .collect())
    }

    pub async fn detect_project_usage<InFn>(
        &self,
        dir: &Path,
        input_factory: InFn,
    ) -> miette::Result<Vec<Id>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> DefineRequirementsInput,
    {
        let mut detected = FxHashSet::default();

        for id in self.get_plugin_ids() {
            if let Ok(toolchain) = self.load(id).await
                && toolchain.detect_project_usage(dir)?
            {
                detected.insert(Id::raw(id));
            }
        }

        for output in self
            .define_requirements_many(detected.iter().collect(), input_factory)
            .await?
        {
            for require_id in output.requires {
                detected.insert(Id::new(require_id)?);
            }
        }

        Ok(detected.into_iter().collect())
    }

    pub async fn detect_task_usage<InFn>(
        &self,
        ids: Vec<&Id>,
        command: &String,
        args: &[String],
        input_factory: InFn,
    ) -> miette::Result<Vec<Id>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> DefineRequirementsInput,
    {
        let mut detected = FxHashSet::default();

        for id in ids {
            if let Ok(toolchain) = self.load(id).await
                && toolchain.detect_task_usage(command, args)?
            {
                detected.insert(Id::raw(id));
            }
        }

        for output in self
            .define_requirements_many(detected.iter().collect(), input_factory)
            .await?
        {
            for require_id in output.requires {
                detected.insert(Id::new(require_id)?);
            }
        }

        Ok(detected.into_iter().collect())
    }

    pub async fn define_requirements_many<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<DefineRequirementsOutput>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> DefineRequirementsInput,
    {
        let results = self
            .call_func_all(
                "define_requirements",
                ids,
                input_factory,
                |toolchain, input| async move { toolchain.define_requirements(input).await },
            )
            .await?;

        Ok(results.into_iter().map(|result| result.output).collect())
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
