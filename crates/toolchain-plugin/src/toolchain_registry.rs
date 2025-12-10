use crate::toolchain_plugin::ToolchainPlugin;
use futures::{StreamExt, stream::FuturesOrdered};
use miette::IntoDiagnostic;
use moon_common::Id;
use moon_config::{ProjectConfig, ProjectToolchainEntry, ToolchainsConfig};
use moon_pdk_api::Operation;
use moon_plugin::{
    CallResult, MoonHostData, PluginError, PluginRegistry, PluginType, serialize_config,
};
use proto_core::{ToolContext, inject_proto_manifest_config};
use starbase_utils::json::{self, JsonValue};
use std::fmt::Debug;
use std::future::Future;
use std::ops::Deref;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::trace;

#[derive(Debug)]
pub struct ToolchainRegistry {
    pub config: Arc<ToolchainsConfig>,
    registry: Arc<PluginRegistry<ToolchainPlugin>>,
}

impl Default for ToolchainRegistry {
    fn default() -> Self {
        Self {
            config: Default::default(),
            registry: Arc::new(PluginRegistry::new(
                PluginType::Toolchain,
                MoonHostData::default(),
            )),
        }
    }
}

impl ToolchainRegistry {
    pub fn new(host_data: MoonHostData, config: Arc<ToolchainsConfig>) -> Self {
        Self {
            config,
            registry: Arc::new(PluginRegistry::new(PluginType::Toolchain, host_data)),
        }
    }

    pub fn create_config(&self, id: &str) -> JsonValue {
        if let Some(config) = self.config.get_plugin_config(id) {
            return config.to_json();
        }

        JsonValue::default()
    }

    pub fn create_merged_config(&self, id: &str, project_config: &ProjectConfig) -> JsonValue {
        let mut data = self.create_config(id);

        if let Some(ProjectToolchainEntry::Config(leaf_config)) =
            project_config.toolchains.get_plugin_config(id)
        {
            let next = leaf_config.to_json();

            data = json::merge(&data, &next);
        }

        data
    }

    pub fn get_plugin_ids(&self) -> Vec<&Id> {
        self.config.plugins.keys().collect()
    }

    pub fn has_plugin_configs(&self) -> bool {
        !self.config.plugins.is_empty()
    }

    pub async fn load<T>(&self, id: T) -> miette::Result<Arc<ToolchainPlugin>>
    where
        T: AsRef<str>,
    {
        let id = Id::raw(id.as_ref());

        if !self.is_registered(&id) {
            if !self.config.plugins.contains_key(&id) {
                return Err(PluginError::UnknownId {
                    id: id.to_string(),
                    ty: PluginType::Toolchain,
                }
                .into());
            }

            self.load_many([&id]).await?;
        }

        self.get_instance(&id).await
    }

    pub async fn load_all(&self) -> miette::Result<Vec<Arc<ToolchainPlugin>>> {
        if !self.has_plugin_configs() {
            return Ok(vec![]);
        }

        self.load_many(self.get_plugin_ids()).await
    }

    pub async fn load_many<I, T>(&self, ids: I) -> miette::Result<Vec<Arc<ToolchainPlugin>>>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let mut set = JoinSet::<miette::Result<Arc<ToolchainPlugin>>>::new();
        let mut list = vec![];

        for id in ids {
            let id = Id::raw(id.as_ref());

            if self.registry.is_registered(&id) {
                list.push(self.get_instance(&id).await?);
                continue;
            }

            let Some(config) = self.config.get_plugin_config(&id) else {
                continue;
            };

            let registry = Arc::clone(&self.registry);
            let config = config.to_owned();

            set.spawn(async move {
                let instance = registry
                    .load_with_config(&id, config.plugin.as_ref().unwrap(), |manifest| {
                        let value = serialize_config(config.config.iter())?;

                        trace!(
                            toolchain_id = id.as_str(),
                            config = %value,
                            "Storing moon toolchain configuration",
                        );

                        manifest
                            .config
                            .insert("moon_toolchain_config".to_owned(), value);

                        inject_proto_manifest_config(
                            &ToolContext::new(id.clone()),
                            &registry.host_data.proto_env,
                            manifest,
                        )?;

                        Ok(())
                    })
                    .await?;

                Ok(instance)
            });
        }

        if !set.is_empty() {
            while let Some(result) = set.join_next().await {
                list.push(result.into_diagnostic()??);
            }
        }

        Ok(list)
    }

    // This method looks crazy, but it basically loads and executes each requested
    // toolchain in parallel, and returns the results in the order they were
    // requested. We had to utilize generics and factory functions to make this
    // easy to use at each call site.
    pub(crate) async fn call_func_all<I, Id, InFn, In, OutFn, OutFut, Out>(
        &self,
        func_name: &str,
        toolchain_ids: I,
        input_factory: InFn,
        output_factory: OutFn,
    ) -> miette::Result<Vec<CallResult<ToolchainPlugin, Out>>>
    where
        I: IntoIterator<Item = Id>,
        Id: AsRef<str> + Clone,
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> In,
        OutFn: Fn(Arc<ToolchainPlugin>, In) -> OutFut,
        OutFut: Future<Output = miette::Result<Out>> + Send + 'static,
        Out: Debug + Send + 'static,
    {
        self.call_func_all_with_check(
            func_name,
            toolchain_ids,
            input_factory,
            output_factory,
            false,
        )
        .await
    }

    pub(crate) async fn call_func_all_with_check<I, Id, InFn, In, OutFn, OutFut, Out>(
        &self,
        func_name: &str,
        toolchain_ids: I,
        input_factory: InFn,
        output_factory: OutFn,
        skip_func_check: bool,
    ) -> miette::Result<Vec<CallResult<ToolchainPlugin, Out>>>
    where
        I: IntoIterator<Item = Id>,
        Id: AsRef<str> + Clone,
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> In,
        OutFn: Fn(Arc<ToolchainPlugin>, In) -> OutFut,
        OutFut: Future<Output = miette::Result<Out>> + Send + 'static,
        Out: Debug + Send + 'static,
    {
        let mut results = vec![];

        if !self.has_plugin_configs() {
            return Ok(results);
        }

        let toolchain_ids = toolchain_ids.into_iter().collect::<Vec<_>>();

        // Load the plugins on-demand when we need them
        self.load_many(toolchain_ids.clone()).await?;

        // Use ordered futures because we need the results to
        // be in a deterministic order for operations to work
        // correct, like hashing
        let mut futures = FuturesOrdered::new();

        for toolchain_id in toolchain_ids {
            let toolchain = self.load(toolchain_id).await?;

            if skip_func_check || toolchain.has_func(func_name).await {
                let mut operation = Operation::new(func_name).unwrap();
                let id = toolchain.id.clone();
                let input = input_factory(self, &toolchain);
                let future = output_factory(toolchain.clone(), input);

                futures.push_back(tokio::spawn(async move {
                    let result = future.await;
                    operation.finish_with_result(&result);

                    Ok::<_, miette::Report>(CallResult {
                        id,
                        operation,
                        output: result?,
                        plugin: toolchain,
                    })
                }));
            }
        }

        while let Some(result) = futures.next().await {
            results.push(result.into_diagnostic()??);
        }

        Ok(results)
    }
}

impl Deref for ToolchainRegistry {
    type Target = PluginRegistry<ToolchainPlugin>;

    fn deref(&self) -> &Self::Target {
        &self.registry
    }
}
