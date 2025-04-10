use crate::host::*;
use crate::plugin::*;
use crate::plugin_error::PluginError;
use moon_pdk_api::MoonContext;
use proto_core::is_offline;
use scc::hash_map::Entry;
use starbase_utils::fs;
use std::fmt;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use tracing::{debug, instrument};
use warpgate::{
    Id as PluginId, PluginContainer, PluginLoader, PluginLocator, PluginManifest, Wasm,
    host::HostData, inject_default_manifest_config, to_virtual_path,
};

#[allow(dead_code)]
pub struct PluginRegistry<T: Plugin> {
    pub host_data: PluginHostData,

    loader: PluginLoader,
    plugins: Arc<scc::HashMap<PluginId, Arc<T>>>,
    type_of: PluginType,
    virtual_paths: BTreeMap<PathBuf, PathBuf>,
}

impl<T: Plugin> PluginRegistry<T> {
    pub fn new(type_of: PluginType, host_data: PluginHostData) -> Self {
        debug!(plugin = type_of.get_label(), "Creating plugin registry");

        // Create the loader
        let mut loader = PluginLoader::new(
            host_data.moon_env.plugins_dir.join(type_of.get_dir_name()),
            &host_data.moon_env.temp_dir,
        );

        loader.set_offline_checker(is_offline);

        // Merge proto and moon virtual paths
        let mut paths = BTreeMap::new();
        paths.extend(host_data.proto_env.get_virtual_paths());
        paths.extend(host_data.moon_env.get_virtual_paths());

        Self {
            loader,
            plugins: Arc::new(scc::HashMap::default()),
            host_data,
            type_of,
            virtual_paths: paths,
        }
    }

    pub fn create_context(&self) -> MoonContext {
        MoonContext {
            working_dir: to_virtual_path(
                self.get_virtual_paths(),
                &self.host_data.moon_env.working_dir,
            ),
            workspace_root: to_virtual_path(
                self.get_virtual_paths(),
                &self.host_data.moon_env.workspace_root,
            ),
        }
    }

    pub fn create_manifest(
        &self,
        id: &PluginId,
        wasm_file: PathBuf,
    ) -> miette::Result<PluginManifest> {
        debug!(
            plugin = self.type_of.get_label(),
            id = id.as_str(),
            path = ?wasm_file,
            "Creating plugin manifest from WASM file",
        );

        let mut manifest = PluginManifest::new([Wasm::file(wasm_file)]);

        // Allow all hosts because we don't know what endpoints plugins
        // will communicate with. Far too many to account for.
        manifest = manifest.with_allowed_host("*");

        // Inherit moon and proto virtual paths.
        manifest = manifest.with_allowed_paths(
            self.virtual_paths
                .iter()
                .map(|(key, value)| (key.to_string_lossy().to_string(), value.to_owned())),
        );

        // Disable timeouts as some functions, like dependency installs,
        // may take multiple minutes to complete. We also can't account
        // for network connectivity.
        manifest.timeout_ms = None;

        // Inherit default configs, like host environment and ID.
        inject_default_manifest_config(id, &self.host_data.moon_env.home_dir, &mut manifest)?;

        // Ensure virtual host paths exist, otherwise WASI (via extism)
        // will throw a cryptic file/directory not found error.
        for host_path in self.virtual_paths.keys() {
            fs::create_dir_all(host_path)?;
        }

        Ok(manifest)
    }

    pub fn get_cache(&self) -> Arc<scc::HashMap<PluginId, Arc<T>>> {
        Arc::clone(&self.plugins)
    }

    pub fn get_virtual_paths(&self) -> &BTreeMap<PathBuf, PathBuf> {
        &self.virtual_paths
    }

    pub async fn get_instance(&self, id: &PluginId) -> miette::Result<Arc<T>> {
        Ok(self
            .plugins
            .get_async(id)
            .await
            .map(|entry| Arc::clone(entry.get()))
            .ok_or_else(|| PluginError::UnknownId {
                id: id.to_string(),
                ty: self.type_of,
            })?)
    }

    pub fn is_registered(&self, id: &PluginId) -> bool {
        self.plugins.contains(id)
    }

    #[instrument(skip(self, op))]
    pub async fn load<I, L, F>(&self, id: I, locator: L, mut op: F) -> miette::Result<()>
    where
        I: AsRef<str> + fmt::Debug,
        L: AsRef<PluginLocator> + fmt::Debug,
        F: FnMut(&mut PluginManifest) -> miette::Result<()>,
    {
        let id = PluginId::raw(id.as_ref());
        let locator = locator.as_ref();

        // Use an entry so that it creates a lock,
        // and hopefully avoids parallel registrations
        match self.plugins.entry_async(id).await {
            Entry::Occupied(_) => {
                // Already loaded
            }
            Entry::Vacant(entry) => {
                debug!(
                    plugin = self.type_of.get_label(),
                    id = entry.key().as_str(),
                    "Attempting to load and register plugin",
                );

                // Load the WASM file (this must happen first because of async)
                let plugin_file = self.loader.load_plugin(entry.key(), locator).await?;

                // Create host functions (provided by warpgate)
                let functions = create_host_functions(
                    self.host_data.clone(),
                    HostData {
                        cache_dir: self.host_data.moon_env.cache_dir.clone(),
                        http_client: self.loader.get_client()?.clone(),
                        virtual_paths: self.virtual_paths.clone(),
                        working_dir: self.host_data.moon_env.working_dir.clone(),
                    },
                );

                // Create the manifest and let the consumer configure it
                let mut manifest = self.create_manifest(entry.key(), plugin_file)?;

                op(&mut manifest)?;

                debug!(
                    plugin = self.type_of.get_label(),
                    id = entry.key().as_str(),
                    "Updated plugin manifest, attempting to register plugin",
                );

                // Combine everything into the container and register
                let plugin = T::new(PluginRegistration {
                    container: PluginContainer::new(entry.key().to_owned(), manifest, functions)?,
                    locator: locator.to_owned(),
                    id: entry.key().to_owned(),
                    moon_env: Arc::clone(&self.host_data.moon_env),
                    proto_env: Arc::clone(&self.host_data.proto_env),
                })
                .await?;

                debug!(
                    plugin = self.type_of.get_label(),
                    id = entry.key().as_str(),
                    "Registered plugin",
                );

                entry.insert_entry(Arc::new(plugin));
            }
        };

        Ok(())
    }

    pub async fn load_without_config<I, L>(&self, id: I, locator: L) -> miette::Result<()>
    where
        I: AsRef<str> + fmt::Debug,
        L: AsRef<PluginLocator> + fmt::Debug,
    {
        self.load(id, locator, |_| Ok(())).await
    }

    pub fn register(&self, id: PluginId, plugin: T) -> miette::Result<()> {
        if self.is_registered(&id) {
            return Err(PluginError::ExistingId {
                id: id.to_string(),
                ty: self.type_of,
            }
            .into());
        }

        debug!(
            plugin = self.type_of.get_label(),
            id = id.as_str(),
            "Registered plugin",
        );

        let _ = self.plugins.insert(id, Arc::new(plugin));

        Ok(())
    }
}

impl<T: Plugin> fmt::Debug for PluginRegistry<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PluginRegistry")
            .field("host_data", &self.host_data)
            .field("plugins", &self.plugins)
            .field("type_of", &self.type_of)
            .field("virtual_paths", &self.virtual_paths)
            .finish()
    }
}

// pub struct PluginInstance<'l, T: Plugin> {
//     entry: OccupiedEntry<'l, Id, T>,
// }

// impl<T: Plugin> Deref for PluginInstance<'_, T> {
//     type Target = T;

//     fn deref(&self) -> &Self::Target {
//         self.entry.get()
//     }
// }

// impl<T: Plugin> DerefMut for PluginInstance<'_, T> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         self.entry.get_mut()
//     }
// }
