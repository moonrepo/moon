use crate::host::*;
use crate::plugin::*;
use crate::plugin_error::PluginError;
use moon_common::Id;
use moon_pdk_api::MoonContext;
use proto_core::is_offline;
use starbase_utils::{fs, json::JsonValue};
use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;
use warpgate::sort_virtual_paths;
use warpgate::{
    PluginLoader, PluginLocator, PluginManifest, VirtualPath, Wasm, from_virtual_path,
    inject_default_manifest_config, to_virtual_path,
};

pub trait PluginsConfig: Debug + Send + Sync + 'static {
    fn configure_manifest(
        &self,
        _id: &Id,
        _host_data: &MoonHostData,
        _manifest: &mut PluginManifest,
    ) -> miette::Result<()> {
        Ok(())
    }

    fn get_ids(&self) -> Vec<&Id>;

    fn get_json_config(&self, _id: &Id) -> Option<JsonValue> {
        None
    }

    fn get_locator(&self, id: &Id) -> Option<&PluginLocator>;
}

pub struct PluginRegistry<Cfg: PluginsConfig, Inst: Plugin> {
    pub config_data: Arc<Cfg>,
    pub host_data: MoonHostData,

    pub(crate) loader: Arc<PluginLoader>,
    pub(crate) plugins: Arc<scc::HashMap<Id, Arc<Inst>>>,
    pub(crate) type_of: PluginType,
    pub(crate) virtual_paths: Vec<(PathBuf, PathBuf)>,
}

impl<Cfg: PluginsConfig, Inst: Plugin> PluginRegistry<Cfg, Inst> {
    pub fn new(
        type_of: PluginType,
        host_data: MoonHostData,
        config_data: Cfg,
    ) -> miette::Result<Self> {
        debug!(
            plugin_type = type_of.get_label(),
            "Creating plugin registry"
        );

        let config = host_data.proto_env.load_config()?;

        // Create the loader
        let mut loader = PluginLoader::new(
            host_data.moon_env.plugins_dir.join(type_of.get_dir_name()),
            &host_data.moon_env.temp_dir,
        );

        if let Some(secs) = config.settings.cache_duration {
            loader.set_cache_duration(Duration::from_secs(secs));
        }

        loader.set_offline_checker(is_offline);
        loader.add_registries(config.settings.registries.iter().cloned().collect());

        // Merge proto and moon virtual paths
        let mut paths = BTreeMap::new();
        paths.extend(host_data.proto_env.get_virtual_paths());
        paths.extend(host_data.moon_env.get_virtual_paths());

        let mut paths = paths.into_iter().collect::<Vec<_>>();

        sort_virtual_paths(&mut paths);

        Ok(Self {
            loader: Arc::new(loader),
            config_data: Arc::new(config_data),
            plugins: Arc::new(scc::HashMap::default()),
            host_data,
            type_of,
            virtual_paths: paths,
        })
    }

    pub fn create_context(&self) -> MoonContext {
        MoonContext {
            working_dir: self.to_virtual_path(&self.host_data.moon_env.working_dir),
            workspace_root: self.to_virtual_path(&self.host_data.moon_env.workspace_root),
        }
    }

    pub fn create_manifest(&self, id: &Id, wasm_file: PathBuf) -> miette::Result<PluginManifest> {
        debug!(
            plugin_type = self.type_of.get_label(),
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
        for (host_path, _) in &self.virtual_paths {
            fs::create_dir_all(host_path)?;
        }

        Ok(manifest)
    }

    pub fn get_cache(&self) -> Arc<scc::HashMap<Id, Arc<Inst>>> {
        Arc::clone(&self.plugins)
    }

    pub async fn get_instance(&self, id: &Id) -> miette::Result<Arc<Inst>> {
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

    pub fn get_plugin_ids(&self) -> Vec<&Id> {
        self.config_data.get_ids()
    }

    pub fn get_virtual_paths(&self) -> &Vec<(PathBuf, PathBuf)> {
        &self.virtual_paths
    }

    pub fn has_plugin_configs(&self) -> bool {
        !self.config_data.get_ids().is_empty()
    }

    pub async fn is_registered(&self, id: &Id) -> bool {
        self.plugins.contains_async(id).await
    }

    pub async fn register(&self, id: Id, plugin: Inst) -> miette::Result<()> {
        if self.is_registered(&id).await {
            return Err(PluginError::ExistingId {
                id: id.to_string(),
                ty: self.type_of,
            }
            .into());
        }

        debug!(
            plugin_type = self.type_of.get_label(),
            id = id.as_str(),
            "Registered plugin",
        );

        let _ = self.plugins.insert_async(id, Arc::new(plugin)).await;

        Ok(())
    }

    pub fn from_virtual_path(&self, path: impl AsRef<Path>) -> PathBuf {
        from_virtual_path(&self.virtual_paths, path.as_ref())
    }

    pub fn to_virtual_path(&self, path: impl AsRef<Path>) -> VirtualPath {
        to_virtual_path(&self.virtual_paths, path.as_ref())
    }
}

// Implemented manually as `derive(Clone)` requires `Cfg` and `Inst` to
// also be `Clone`, even though they are wrapped in `Arc`s.
impl<Cfg: PluginsConfig, Inst: Plugin> Clone for PluginRegistry<Cfg, Inst> {
    fn clone(&self) -> Self {
        Self {
            config_data: Arc::clone(&self.config_data),
            host_data: self.host_data.clone(),
            loader: Arc::clone(&self.loader),
            plugins: Arc::clone(&self.plugins),
            type_of: self.type_of,
            virtual_paths: self.virtual_paths.clone(),
        }
    }
}

impl<Cfg: PluginsConfig, Inst: Plugin> fmt::Debug for PluginRegistry<Cfg, Inst> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PluginRegistry")
            .field("config_data", &self.config_data)
            .field("host_data", &self.host_data)
            .field("plugins", &self.plugins)
            .field("type_of", &self.type_of)
            .field("virtual_paths", &self.virtual_paths)
            .finish()
    }
}
