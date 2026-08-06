use crate::host::*;
use crate::plugin::{Plugin, PluginRegistration};
use crate::plugin_error::PluginError;
use crate::plugin_registry_new::*;
use miette::IntoDiagnostic;
use moon_common::{Id, IdExt};
use scc::hash_map::Entry;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, instrument};
use warpgate::host::HostData;
use warpgate::{PluginContainer, PluginLocator};

impl<Cfg: PluginsConfig, Inst: Plugin> PluginRegistry<Cfg, Inst> {
    pub async fn load<I>(&self, id: I) -> miette::Result<Arc<Inst>>
    where
        I: AsRef<str>,
    {
        let id = Id::raw(id.as_ref());

        if !self.is_registered(&id).await {
            if self.config_data.get_locator(&id).is_none() {
                return Err(PluginError::UnknownId {
                    id: id.to_string(),
                    ty: self.type_of,
                }
                .into());
            }

            return Ok(self.load_many([&id]).await?.remove(0));
        }

        self.get_instance(&id).await
    }

    pub async fn load_all(&self) -> miette::Result<Vec<Arc<Inst>>> {
        let ids = self.config_data.get_ids();

        if ids.is_empty() {
            return Ok(vec![]);
        }

        self.load_many(ids).await
    }

    pub async fn load_many<It, I>(&self, ids: It) -> miette::Result<Vec<Arc<Inst>>>
    where
        It: IntoIterator<Item = I>,
        I: AsRef<str>,
    {
        let ids = ids
            .into_iter()
            .map(|id| Id::raw(id.as_ref()))
            .collect::<Vec<_>>();
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
        let mut set = JoinSet::<miette::Result<Arc<Inst>>>::new();

        for id in ids {
            let Some(locator) = self.config_data.get_locator(&id) else {
                continue;
            };

            let registry = self.to_owned();
            let locator = locator.to_owned();

            set.spawn(Box::pin(async move {
                registry.internal_load(&id, locator).await
            }));
        }

        while let Some(result) = set.join_next().await {
            list.push(result.into_diagnostic()??);
        }

        Ok(list)
    }

    #[instrument(skip(self))]
    async fn internal_load<I, L>(&self, id: I, locator: L) -> miette::Result<Arc<Inst>>
    where
        I: AsRef<str> + Debug,
        L: AsRef<PluginLocator> + Debug,
    {
        let id = Id::raw(id.as_ref());
        let locator = locator.as_ref();

        // Return early if already registered. We must NOT hold a map lock (an
        // scc entry guard) across the expensive, multi-second WASM load below:
        // doing so serializes loads that collide on a bucket and can deadlock
        // under concurrent loads (e.g. `load_many`), since a guard held across
        // an `.await` blocks other tasks (and map resizes) from making progress.
        if let Some(existing) = self.plugins.get_async(&id).await {
            return Ok(Arc::clone(existing.get()));
        }

        debug!(
            plugin_type = self.type_of.get_label(),
            id = id.as_str(),
            "Attempting to load and register plugin",
        );

        // Load the WASM file (this must happen first because of async)
        let plugin_file = self.loader.load_plugin(&id, locator).await?;

        // Create host functions (provided by warpgate)
        let functions = create_host_functions(
            self.host_data.clone(),
            HostData {
                cache_dir: self.host_data.moon_env.cache_dir.clone(),
                http_client: self.loader.get_http_client()?.clone(),
                virtual_paths: self.virtual_paths.clone(),
                working_dir: self.host_data.moon_env.working_dir.clone(),
            },
        );

        // Create the manifest and let the consumer configure it
        let mut manifest = self.create_manifest(&id, plugin_file.clone())?;

        self.config_data.configure_manifest(&id, &mut manifest)?;

        debug!(
            plugin_type = self.type_of.get_label(),
            id = id.as_str(),
            "Updated plugin manifest, attempting to register plugin",
        );

        // Create a new ID for the WASM manifest if it's prefixed with
        // "unstable_". The reason for this is that proto's built-in tools
        // expect a specific ID, for example "rust", and if we provide
        // "unstable_rust", it breaks in weird ways.
        let stable_id = Id::stable(id.as_str());

        // Combine everything into the container and register
        let plugin = Inst::new(PluginRegistration {
            container: PluginContainer::new(stable_id.clone(), manifest, functions)?,
            locator: locator.to_owned(),
            id: id.clone(),
            id_stable: stable_id,
            moon_env: Arc::clone(&self.host_data.moon_env),
            proto_env: Arc::clone(&self.host_data.proto_env),
            wasm_file: plugin_file,
        })
        .await?;

        debug!(
            plugin_type = self.type_of.get_label(),
            id = id.as_str(),
            "Registered plugin",
        );

        let instance = Arc::new(plugin);

        // Insert into the registry, holding the bucket lock only around the
        // synchronous insert (never across an `.await`). If another task loaded
        // the same plugin concurrently, discard ours and use the race winner.
        Ok(match self.plugins.entry_async(id).await {
            Entry::Occupied(entry) => Arc::clone(entry.get()),
            Entry::Vacant(entry) => {
                entry.insert_entry(Arc::clone(&instance));
                instance
            }
        })
    }
}
