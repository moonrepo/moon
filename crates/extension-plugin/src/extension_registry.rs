use crate::extension_plugin::ExtensionPlugin;
use futures::StreamExt;
use futures::stream::FuturesOrdered;
use miette::IntoDiagnostic;
use moon_common::Id;
use moon_config::ExtensionsConfig;
use moon_pdk_api::Operation;
use moon_plugin::{
    CallResult, MoonHostData, PluginError, PluginRegistry, PluginType, serialize_config,
};
use starbase_utils::json::JsonValue;
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, trace};

#[derive(Debug)]
pub struct ExtensionRegistry {
    pub config: Arc<ExtensionsConfig>,
    registry: Arc<PluginRegistry<ExtensionPlugin>>,
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self {
            config: Arc::new(ExtensionsConfig::default()),
            registry: Arc::new(PluginRegistry::new(
                PluginType::Extension,
                MoonHostData::default(),
            )),
        }
    }
}

impl ExtensionRegistry {
    pub fn new(host_data: MoonHostData, config: Arc<ExtensionsConfig>) -> Self {
        Self {
            config,
            registry: Arc::new(PluginRegistry::new(PluginType::Extension, host_data)),
        }
    }

    pub fn create_config(&self, id: &str) -> JsonValue {
        if let Some(config) = self.config.get_plugin_config(id) {
            return config.to_json();
        }

        JsonValue::default()
    }

    pub fn get_plugin_ids(&self) -> Vec<&Id> {
        self.config.plugins.keys().collect()
    }

    pub fn has_plugin_configs(&self) -> bool {
        !self.config.plugins.is_empty()
    }

    pub async fn load<T>(&self, id: T) -> miette::Result<Arc<ExtensionPlugin>>
    where
        T: AsRef<str>,
    {
        let id = Id::raw(id.as_ref());

        if !self.is_registered(&id).await {
            if !self.config.plugins.contains_key(&id) {
                return Err(PluginError::UnknownId {
                    id: id.to_string(),
                    ty: PluginType::Extension,
                }
                .into());
            }

            return Ok(self.load_many([&id]).await?.remove(0));
        }

        self.get_instance(&id).await
    }

    pub async fn load_all(&self) -> miette::Result<Vec<Arc<ExtensionPlugin>>> {
        if !self.has_plugin_configs() {
            return Ok(vec![]);
        }

        debug!("Loading all extension plugins");

        self.load_many(self.get_plugin_ids()).await
    }

    pub async fn load_many<I, T>(&self, ids: I) -> miette::Result<Vec<Arc<ExtensionPlugin>>>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let ids = ids
            .into_iter()
            .map(|id| Id::raw(id.as_ref()))
            .collect::<Vec<_>>();
        let mut set = JoinSet::<miette::Result<Arc<ExtensionPlugin>>>::new();
        let mut list = vec![];

        // First check if all of the requested plugins are already registered,
        // and if so, return them immediately
        for id in &ids {
            if self.is_registered(id).await {
                list.push(self.get_instance(id).await?);
            }
        }

        if list.len() == ids.len() {
            return Ok(list);
        } else {
            list.clear();
        }

        // Otherwise load all the plugins in parallel, and return them in the
        // order they were requested
        for id in ids {
            let Some(config) = self.config.plugins.get(&id) else {
                continue;
            };

            let registry = Arc::clone(&self.registry);
            let config = config.to_owned();

            set.spawn(Box::pin(async move {
                let instance = registry
                    .load_with_config(&id, config.plugin.as_ref().unwrap(), |manifest| {
                        let value = serialize_config(config.config.iter())?;

                        trace!(
                            extension_id = id.as_str(),
                            config = %value,
                            "Storing moon extension configuration",
                        );

                        manifest
                            .config
                            .insert("moon_extension_config".to_owned(), value);

                        Ok(())
                    })
                    .await?;

                Ok(instance)
            }));
        }

        while let Some(result) = set.join_next().await {
            list.push(result.into_diagnostic()??);
        }

        Ok(list)
    }

    // This method looks crazy, but it basically loads and executes each requested
    // extension in parallel, and returns the results in the order they were
    // requested. We had to utilize generics and factory functions to make this
    // easy to use at each call site.
    pub(crate) async fn call_func_all<I, Id, InFn, In, OutFn, OutFut, Out>(
        &self,
        func_name: &str,
        extension_ids: I,
        input_factory: InFn,
        output_factory: OutFn,
    ) -> miette::Result<Vec<CallResult<ExtensionPlugin, Out>>>
    where
        I: IntoIterator<Item = Id>,
        Id: AsRef<str> + Clone,
        InFn: Fn(&ExtensionRegistry, &ExtensionPlugin) -> In,
        OutFn: Fn(Arc<ExtensionPlugin>, In) -> OutFut,
        OutFut: Future<Output = miette::Result<Out>> + Send + 'static,
        Out: Debug + Send + 'static,
    {
        let mut results = vec![];

        if !self.has_plugin_configs() {
            return Ok(results);
        }

        // Load the plugins on-demand when we need them
        let extensions = self.load_many(extension_ids).await?;

        // Use ordered futures because we need the results to
        // be in a deterministic order for operations to work
        // correctly, like hashing
        let mut futures = FuturesOrdered::new();

        for extension in extensions {
            if extension.has_func(func_name).await {
                let mut operation = Operation::new(func_name).unwrap();
                let input = input_factory(self, &extension);
                let future = output_factory(extension.clone(), input);

                futures.push_back(tokio::spawn(Box::pin(async move {
                    let result = future.await;

                    operation.finish_with_result(&result);

                    Ok::<_, miette::Report>(CallResult {
                        id: extension.id.clone(),
                        operation,
                        output: result?,
                        plugin: extension,
                    })
                })));
            }
        }

        while let Some(result) = futures.next().await {
            results.push(result.into_diagnostic()??);
        }

        Ok(results)
    }
}

impl Deref for ExtensionRegistry {
    type Target = PluginRegistry<ExtensionPlugin>;

    fn deref(&self) -> &Self::Target {
        &self.registry
    }
}
