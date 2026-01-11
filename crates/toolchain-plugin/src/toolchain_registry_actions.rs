use crate::toolchain_plugin::ToolchainPlugin;
use crate::toolchain_registry::ToolchainRegistry;
use moon_common::Id;
use moon_config::LanguageType;
use moon_pdk_api::{
    ConfigSchema, DefineDockerMetadataInput, DefineDockerMetadataOutput, DefineRequirementsInput,
    DefineRequirementsOutput, ExtendCommandInput, ExtendCommandOutput, ExtendProjectGraphInput,
    ExtendProjectGraphOutput, ExtendTaskCommandInput, ExtendTaskScriptInput,
    ExtendTaskScriptOutput, HashTaskContentsInput, LocateDependenciesRootInput,
    LocateDependenciesRootOutput, ScaffoldDockerInput, ScaffoldDockerOutput, SetupToolchainInput,
    SetupToolchainOutput, SyncOutput, SyncProjectInput, SyncWorkspaceInput, TeardownToolchainInput,
};
use moon_plugin::CallResult;
use proto_core::UnresolvedVersionSpec;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_utils::json::JsonValue;
use std::path::{Path, PathBuf};

// These implementations aggregate the call results from all toolchains
// that were requested to be executed into a better/different format
// depending on the need of the call site.

impl ToolchainRegistry {
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

    pub async fn detect_project_language(&self, dir: &Path) -> miette::Result<LanguageType> {
        let mut detected = vec![];

        for toolchain in self.load_all().await? {
            if let Some(language) = &toolchain.metadata.language
                && toolchain.detect_project_usage(dir)?
                && !language.is_unknown()
            {
                detected.push(language.clone());
            }
        }

        if detected.is_empty() {
            return Ok(LanguageType::Unknown);
        }

        let language = detected.remove(0);

        if language == LanguageType::JavaScript && detected.contains(&LanguageType::TypeScript) {
            return Ok(LanguageType::TypeScript);
        }

        Ok(language)
    }

    pub async fn detect_project_toolchain_from_language(
        &self,
        language: &LanguageType,
    ) -> miette::Result<Vec<Id>> {
        let mut detected = vec![];

        for toolchain in self.load_all().await? {
            if toolchain
                .metadata
                .language
                .as_ref()
                .is_some_and(|lang| lang == language)
            {
                detected.push(toolchain.id.clone());
            }
        }

        Ok(detected)
    }

    pub async fn detect_project_toolchain_from_usage<InFn>(
        &self,
        dir: &Path,
        input_factory: InFn,
    ) -> miette::Result<Vec<Id>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> DefineRequirementsInput,
    {
        let mut detected = FxHashSet::default();

        for toolchain in self.load_all().await? {
            if toolchain.detect_project_usage(dir)? {
                detected.insert(toolchain.id.clone());
            }
        }

        for result in self
            .define_requirements_many(detected.iter().collect(), input_factory)
            .await?
        {
            for require_id in result.output.requires {
                detected.insert(Id::new(require_id)?);
            }
        }

        Ok(detected.into_iter().collect())
    }

    pub async fn detect_task_usage(
        &self,
        ids: Vec<&Id>,
        command: &String,
    ) -> miette::Result<Vec<Id>> {
        let mut detected = FxHashSet::default();

        for toolchain in self.load_many(ids).await? {
            if toolchain.detect_task_usage(command)? {
                detected.insert(toolchain.id.clone());
            }
        }

        Ok(detected.into_iter().collect())
    }

    pub async fn define_requirements_many<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<CallResult<ToolchainPlugin, DefineRequirementsOutput>>>
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

        Ok(results)
    }

    pub async fn define_toolchain_config_all(
        &self,
    ) -> miette::Result<FxHashMap<String, ConfigSchema>> {
        let results = self
            .call_func_all(
                "define_toolchain_config",
                self.get_plugin_ids(),
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
        let results = self
            .call_func_all_with_check(
                "define_docker_metadata",
                self.get_plugin_ids(),
                input_factory,
                |toolchain, input| async move { toolchain.define_docker_metadata(input).await },
                true,
            )
            .await?;

        Ok(results.into_iter().map(|result| result.output).collect())
    }

    pub async fn expand_task_usage<InFn>(
        &self,
        ids: Vec<Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<Id>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> DefineRequirementsInput,
    {
        let mut expanded = FxHashSet::from_iter(ids);

        for result in self
            .define_requirements_many(self.get_plugin_ids(), input_factory)
            .await?
        {
            if expanded.contains(&result.id)
                || result
                    .output
                    .requires
                    .iter()
                    .any(|id| expanded.contains(id.as_str()))
            {
                for require_id in result.output.requires {
                    expanded.insert(Id::new(require_id)?);
                }

                expanded.insert(result.id);
            }
        }

        Ok(expanded.into_iter().collect())
    }

    pub async fn extend_command_many<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<ExtendCommandOutput>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> ExtendCommandInput,
    {
        let results = self
            .call_func_all(
                "extend_command",
                ids,
                input_factory,
                |toolchain, input| async move { toolchain.extend_command(input).await },
            )
            .await?;

        Ok(results.into_iter().map(|result| result.output).collect())
    }

    pub async fn extend_project_graph_all<InFn>(
        &self,
        input_factory: InFn,
    ) -> miette::Result<Vec<CallResult<ToolchainPlugin, ExtendProjectGraphOutput>>>
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

        Ok(results)
    }

    pub async fn extend_task_command_many<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<ExtendCommandOutput>>
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
    ) -> miette::Result<Vec<CallResult<ToolchainPlugin, LocateDependenciesRootOutput>>>
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
    ) -> miette::Result<Vec<CallResult<ToolchainPlugin, SetupToolchainOutput>>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> SetupToolchainInput,
    {
        self.call_func_all_with_check(
            "setup_toolchain",
            self.get_plugin_ids(),
            input_factory,
            |toolchain, input| async move { toolchain.setup_toolchain(input, || Ok(())).await },
            true,
        )
        .await
    }

    pub async fn sync_project_many<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<CallResult<ToolchainPlugin, SyncOutput>>>
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
    ) -> miette::Result<Vec<CallResult<ToolchainPlugin, SyncOutput>>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> SyncWorkspaceInput,
    {
        self.call_func_all(
            "sync_workspace",
            self.get_plugin_ids(),
            input_factory,
            |toolchain, input| async move { toolchain.sync_workspace(input).await },
        )
        .await
    }

    pub async fn teardown_toolchain_all<InFn>(
        &self,
        input_factory: InFn,
    ) -> miette::Result<Vec<CallResult<ToolchainPlugin, ()>>>
    where
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> TeardownToolchainInput,
    {
        self.call_func_all_with_check(
            "teardown_toolchain",
            self.get_plugin_ids(),
            input_factory,
            |toolchain, input| async move { toolchain.teardown_toolchain(input).await },
            true,
        )
        .await
    }
}
