use crate::toolchain_plugin::ToolchainPlugin;
use futures::{StreamExt, stream::FuturesOrdered};
use miette::IntoDiagnostic;
use moon_common::Id;
use moon_config::{ProjectConfig, ProjectToolchainEntry, ToolchainConfig, ToolchainPluginConfig};
use moon_pdk_api::Operation;
use moon_plugin::{
    MoonHostData, PluginError, PluginId, PluginRegistry, PluginType, serialize_config,
};
use proto_core::inject_proto_manifest_config;
use rustc_hash::FxHashMap;
use starbase_utils::json::{self, JsonValue};
use std::future::Future;
use std::ops::Deref;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, trace};

#[derive(Debug)]
pub struct ToolchainRegistry {
    pub config: Arc<ToolchainConfig>,
    pub plugins: FxHashMap<PluginId, ToolchainPluginConfig>,
    registry: Arc<PluginRegistry<ToolchainPlugin>>,
}

impl Default for ToolchainRegistry {
    fn default() -> Self {
        Self {
            config: Default::default(),
            plugins: FxHashMap::default(),
            registry: Arc::new(PluginRegistry::new(
                PluginType::Toolchain,
                MoonHostData::default(),
            )),
        }
    }
}

impl ToolchainRegistry {
    pub fn new(host_data: MoonHostData, config: Arc<ToolchainConfig>) -> Self {
        Self {
            config,
            plugins: FxHashMap::default(),
            registry: Arc::new(PluginRegistry::new(PluginType::Toolchain, host_data)),
        }
    }

    pub fn inherit_configs(&mut self, configs: &FxHashMap<Id, ToolchainPluginConfig>) {
        for (id, config) in configs {
            // Convert moon IDs to plugin IDs
            self.plugins.insert(PluginId::raw(id), config.to_owned());
        }
    }

    pub fn create_config(&self, id: &str, toolchain_config: &ToolchainConfig) -> JsonValue {
        if let Some(config) = toolchain_config.get_plugin_config(id) {
            return config.to_json();
        }

        JsonValue::default()
    }

    pub fn create_merged_config(
        &self,
        id: &str,
        toolchain_config: &ToolchainConfig,
        project_config: &ProjectConfig,
    ) -> JsonValue {
        let mut data = self.create_config(id, toolchain_config);

        if let Some(ProjectToolchainEntry::Config(leaf_config)) =
            project_config.toolchain.get_plugin_config(id)
        {
            let next = leaf_config.to_json();

            data = json::merge(&data, &next);
        }

        data
    }

    pub fn get_plugin_ids(&self) -> Vec<&PluginId> {
        self.plugins.keys().collect()
    }

    pub fn has_plugins(&self) -> bool {
        !self.plugins.is_empty()
    }

    pub async fn load<Id>(&self, id: Id) -> miette::Result<Arc<ToolchainPlugin>>
    where
        Id: AsRef<str>,
    {
        let id = PluginId::raw(id.as_ref());

        if !self.is_registered(&id) {
            if !self.plugins.contains_key(&id) {
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
        if !self.has_plugins() {
            return Ok(vec![]);
        }

        debug!("Loading all toolchain plugins");

        self.load_many(self.get_plugin_ids()).await
    }

    pub async fn load_many<I, Id>(&self, ids: I) -> miette::Result<Vec<Arc<ToolchainPlugin>>>
    where
        I: IntoIterator<Item = Id>,
        Id: AsRef<str>,
    {
        let mut set = JoinSet::<miette::Result<Arc<ToolchainPlugin>>>::new();
        let mut list = vec![];

        for id in ids {
            let id = PluginId::raw(id.as_ref());

            if self.registry.is_registered(&id) {
                list.push(self.get_instance(&id).await?);
                continue;
            }

            let Some(config) = self.plugins.get(&id) else {
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

                        inject_proto_manifest_config(&id, &registry.host_data.proto_env, manifest)?;

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
    ) -> miette::Result<Vec<CallResult<Out>>>
    where
        I: IntoIterator<Item = Id>,
        Id: AsRef<str> + Clone,
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> In,
        OutFn: Fn(Arc<ToolchainPlugin>, In) -> OutFut,
        OutFut: Future<Output = miette::Result<Out>> + Send + 'static,
        Out: Send + 'static,
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
    ) -> miette::Result<Vec<CallResult<Out>>>
    where
        I: IntoIterator<Item = Id>,
        Id: AsRef<str> + Clone,
        InFn: Fn(&ToolchainRegistry, &ToolchainPlugin) -> In,
        OutFn: Fn(Arc<ToolchainPlugin>, In) -> OutFut,
        OutFut: Future<Output = miette::Result<Out>> + Send + 'static,
        Out: Send + 'static,
    {
        let mut results = vec![];

        if !self.has_plugins() {
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
            if let Ok(toolchain) = self.load(toolchain_id).await
                && (skip_func_check || toolchain.has_func(func_name).await)
            {
                let mut operation = Operation::new(func_name);
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
                        toolchain,
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

pub struct CallResult<T> {
    pub id: PluginId,
    pub operation: Operation,
    pub output: T,
    pub toolchain: Arc<ToolchainPlugin>,
}
