use crate::toolchain_plugin::ToolchainPlugin;
use crate::toolchain_registry::ToolchainRegistry;
use indexmap::IndexSet;
use moon_common::Id;
use moon_config::LanguageType;
use moon_pdk_api::{
    ConfigSchema, DefineDockerMetadataInput, DefineDockerMetadataOutput, DefineRequirementsInput,
    DefineRequirementsOutput, ExtendCommandInput, ExtendCommandOutput, ExtendProjectGraphInput,
    ExtendProjectGraphOutput, ExtendTaskCommandInput, ExtendTaskScriptInput,
    ExtendTaskScriptOutput, LocateDependenciesRootInput, ScaffoldDockerInput, SyncOutput,
    SyncProjectInput, SyncWorkspaceInput, TeardownToolchainInput,
};
use moon_plugin::{CallOptions, CallResult};
use moon_toolchain::DependenciesWorkspace;
use proto_core::UnresolvedVersionSpec;
use proto_pdk_api::ActivateEnvironmentOutput;
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};

// These implementations aggregate the call results from all toolchains
// that were requested to be executed into a better/different format
// depending on the need of the call site.

impl ToolchainRegistry {
    pub async fn activate_environment_many<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<ActivateEnvironmentOutput>>
    where
        InFn: Fn(&ToolchainPlugin) -> Option<UnresolvedVersionSpec>,
    {
        let results = self
            .call_func(
                "activate_environment",
                ids,
                input_factory,
                |toolchain, input| async move { toolchain.activate_environment(input).await },
            )
            .await?;

        Ok(results.into_iter().map(|result| result.output).collect())
    }

    pub async fn get_command_paths<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<PathBuf>>
    where
        InFn: Fn(&ToolchainPlugin) -> Option<UnresolvedVersionSpec>,
    {
        let results = self
            .call_func_with_options(
                "get_command_paths",
                ids,
                input_factory,
                |toolchain, input| async move { toolchain.get_command_paths(input).await },
                CallOptions {
                    check_func_exists: false,
                },
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
                && !language.is_unknown()
                && toolchain.detect_project_usage(dir)?
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

    pub async fn detect_project_toolchain_from_usage(&self, dir: &Path) -> miette::Result<Vec<Id>> {
        let mut detected = IndexSet::<Id>::default();

        for toolchain in self.load_all().await? {
            if toolchain.detect_project_usage(dir)? {
                detected.insert(toolchain.id.clone());
            }
        }

        Ok(detected.into_iter().collect())
    }

    pub async fn detect_task_usage(
        &self,
        ids: Vec<&Id>,
        command: &String,
    ) -> miette::Result<Vec<Id>> {
        let mut detected = IndexSet::<Id>::default();

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
        InFn: Fn(&ToolchainPlugin) -> DefineRequirementsInput,
    {
        self.call_func(
            "define_requirements",
            ids,
            input_factory,
            |toolchain, input| async move { toolchain.define_requirements(input).await },
        )
        .await
    }

    pub async fn define_toolchain_config_all(
        &self,
    ) -> miette::Result<FxHashMap<String, ConfigSchema>> {
        let results = self
            .call_func_all(
                "define_toolchain_config",
                |_| (),
                |toolchain, _| async move { toolchain.define_toolchain_config().await },
            )
            .await?;

        Ok(results
            .into_iter()
            .flat_map(|result| result.output.map(|schema| (result.id.to_string(), schema)))
            .collect())
    }

    pub async fn define_docker_metadata_all<InFn>(
        &self,
        input_factory: InFn,
    ) -> miette::Result<Vec<DefineDockerMetadataOutput>>
    where
        InFn: Fn(&ToolchainPlugin) -> DefineDockerMetadataInput,
    {
        let results = self
            .call_func_all_with_options(
                "define_docker_metadata",
                input_factory,
                |toolchain, input| async move { toolchain.define_docker_metadata(input).await },
                CallOptions {
                    check_func_exists: false,
                },
            )
            .await?;

        Ok(results.into_iter().map(|result| result.output).collect())
    }

    /// Expand the toolchains with the toolchains that they require, and the
    /// toolchains that those require, and so on. The configured toolchains that
    /// require any of the provided toolchains (or of those dependents) are also
    /// added, with their own requirements, like Node.js also using the JavaScript
    /// toolchain and its configured package manager, which require it. The
    /// dependents of required toolchains are not added, as they're siblings,
    /// like pnpm for npm, as both require Node.js.
    ///
    /// The provided toolchains stay first and in order, as consumers treat the
    /// first as the primary (the `$taskToolchain` token), and the list is hashed
    /// as-is. Identifiers are resolved to how the toolchains were configured
    /// (stable or unstable), and toolchains that aren't configured are kept, so
    /// that the consumer can filter them.
    pub async fn expand_requirements<InFn>(
        &self,
        ids: Vec<Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<Id>>
    where
        InFn: Fn(&ToolchainPlugin) -> DefineRequirementsInput,
    {
        let mut expanded = ids
            .into_iter()
            .map(|id| self.resolve_toolchain_id(id))
            .collect::<IndexSet<_>>();

        // The provided toolchains and their dependents
        let mut dependents = expanded.clone();

        // The requirements of every configured toolchain, as any may be a dependent,
        // while toolchains that aren't configured can't define requirements
        let mut requirements = vec![];

        for result in self
            .define_requirements_many(self.get_plugin_ids(), &input_factory)
            .await?
        {
            let mut required = vec![];

            for requirement in result.output.requires {
                // An invalid identifier can't be a configured toolchain, and is
                // reported, naming this toolchain, when the action graph sets it up
                if let Ok(require_id) = Id::new(requirement.id) {
                    required.push(self.resolve_toolchain_id(require_id));
                }
            }

            requirements.push((result.id, required));
        }

        loop {
            let counts = (expanded.len(), dependents.len());

            for (id, required) in &requirements {
                if !dependents.contains(id) && required.iter().any(|req| dependents.contains(req)) {
                    dependents.insert(id.to_owned());
                    expanded.insert(id.to_owned());
                }

                if expanded.contains(id) {
                    expanded.extend(required.iter().cloned());
                }
            }

            // Nothing was added, so every dependent and requirement is known
            if (expanded.len(), dependents.len()) == counts {
                break;
            }
        }

        Ok(expanded.into_iter().collect())
    }

    /// Resolve the identifier to how the toolchain was configured, which may be
    /// its stable or unstable ("unstable_" prefixed) form. If not configured,
    /// it's returned as-is.
    pub fn resolve_toolchain_id(&self, id: Id) -> Id {
        match self.config_data.get_plugin_id(&id) {
            Some(configured_id) => configured_id.to_owned(),
            None => id,
        }
    }

    pub async fn extend_command_many<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<ExtendCommandOutput>>
    where
        InFn: Fn(&ToolchainPlugin) -> ExtendCommandInput,
    {
        let results = self
            .call_func(
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
        InFn: Fn(&ToolchainPlugin) -> ExtendProjectGraphInput,
    {
        self.call_func_all(
            "extend_project_graph",
            input_factory,
            |toolchain, input| async move { toolchain.extend_project_graph(input).await },
        )
        .await
    }

    pub async fn extend_task_command_many<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<ExtendCommandOutput>>
    where
        InFn: Fn(&ToolchainPlugin) -> ExtendTaskCommandInput,
    {
        let results = self
            .call_func(
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
        InFn: Fn(&ToolchainPlugin) -> ExtendTaskScriptInput,
    {
        let results = self
            .call_func(
                "extend_task_script",
                ids,
                input_factory,
                |toolchain, input| async move { toolchain.extend_task_script(input).await },
            )
            .await?;

        Ok(results.into_iter().map(|result| result.output).collect())
    }

    pub async fn locate_dependencies_root_many<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<CallResult<ToolchainPlugin, DependenciesWorkspace>>>
    where
        InFn: Fn(&ToolchainPlugin) -> LocateDependenciesRootInput,
    {
        let results = self
            .call_func(
                "locate_dependencies_root",
                ids,
                input_factory,
                |toolchain, input| async move { toolchain.locate_dependencies_root(input).await },
            )
            .await?;

        Ok(results
            .into_iter()
            .filter_map(|result| {
                if result.output.is_some() {
                    Some(result.map_output(|out| out.unwrap()))
                } else {
                    None
                }
            })
            .collect())
    }

    pub async fn scaffold_docker_many<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<()>
    where
        InFn: Fn(&ToolchainPlugin) -> ScaffoldDockerInput,
    {
        self.call_func(
            "scaffold_docker",
            ids,
            input_factory,
            |toolchain, input| async move { toolchain.scaffold_docker(input).await },
        )
        .await?;

        Ok(())
    }

    pub async fn sync_project_many<InFn>(
        &self,
        ids: Vec<&Id>,
        input_factory: InFn,
    ) -> miette::Result<Vec<CallResult<ToolchainPlugin, SyncOutput>>>
    where
        InFn: Fn(&ToolchainPlugin) -> SyncProjectInput,
    {
        self.call_func(
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
        InFn: Fn(&ToolchainPlugin) -> SyncWorkspaceInput,
    {
        self.call_func_all(
            "sync_workspace",
            input_factory,
            |toolchain, input| async move { toolchain.sync_workspace(input).await },
        )
        .await
    }

    pub async fn teardown_toolchain_all<InFn>(&self, input_factory: InFn) -> miette::Result<()>
    where
        InFn: Fn(&ToolchainPlugin) -> TeardownToolchainInput,
    {
        self.call_func_all_with_options(
            "teardown_toolchain",
            input_factory,
            |toolchain, input| async move { toolchain.teardown_toolchain(input).await },
            CallOptions {
                check_func_exists: false,
            },
        )
        .await?;

        Ok(())
    }
}
