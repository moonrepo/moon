use crate::plugin::*;
use crate::plugin_error::PluginError;
use dashmap::{
    iter::{Iter, IterMut},
    DashMap,
};
use moon_env::MoonEnvironment;
use moon_pdk_api::MoonContext;
use proto_core::{is_offline, ProtoEnvironment};
use std::{
    collections::BTreeMap,
    future::Future,
    path::{Path, PathBuf},
    sync::Arc,
};
use warpgate::{
    host_funcs::*, inject_default_manifest_config, to_virtual_path, Id, PluginContainer,
    PluginLoader, PluginLocator, PluginManifest, Wasm,
};

#[allow(dead_code)]
pub struct PluginRegistry<T: Plugin> {
    loader: PluginLoader,
    moon_env: Arc<MoonEnvironment>,
    plugins: Arc<DashMap<Id, T>>,
    proto_env: Arc<ProtoEnvironment>,
    type_of: PluginType,
    virtual_paths: BTreeMap<PathBuf, PathBuf>,
}

impl<T: Plugin> PluginRegistry<T> {
    pub fn new(
        type_of: PluginType,
        moon_env: Arc<MoonEnvironment>,
        proto_env: Arc<ProtoEnvironment>,
    ) -> Self {
        let mut loader = PluginLoader::new(
            moon_env.plugins_dir.join(type_of.get_dir_name()),
            &moon_env.temp_dir,
        );
        loader.set_offline_checker(is_offline);

        let mut paths = proto_env
            .get_virtual_paths()
            .into_iter()
            // Remove proto workspace, as it collides with moon's
            .filter(|(_, g)| g != Path::new("/workspace"))
            .collect::<BTreeMap<_, _>>();

        paths.extend(moon_env.get_virtual_paths());

        Self {
            loader,
            plugins: Arc::new(DashMap::new()),
            moon_env,
            proto_env,
            type_of,
            virtual_paths: paths,
        }
    }

    pub fn create_context(&self) -> MoonContext {
        MoonContext {
            working_dir: to_virtual_path(self.get_virtual_paths(), &self.moon_env.cwd),
            workspace_root: to_virtual_path(
                self.get_virtual_paths(),
                &self.moon_env.workspace_root,
            ),
        }
    }

    pub fn create_manifest(&self, id: &Id, wasm_file: PathBuf) -> miette::Result<PluginManifest> {
        let mut manifest = PluginManifest::new([Wasm::file(wasm_file)]);

        // Allow all hosts because we don't know what endpoints plugins
        // will communicate with. Far too many to account for.
        manifest = manifest.with_allowed_host("*");

        // Inherit moon and proto virtual paths.
        manifest = manifest.with_allowed_paths(self.virtual_paths.clone().into_iter());

        // Disable timeouts as some functions, like dependency installs,
        // may take multiple minutes to complete. We also can't account
        // for network connectivity.
        manifest.timeout_ms = None;

        // Inherit default configs, like host environment and ID.
        inject_default_manifest_config(id, &self.moon_env.home, &mut manifest)?;

        Ok(manifest)
    }

    pub fn get_cache(&self) -> Arc<DashMap<Id, T>> {
        Arc::clone(&self.plugins)
    }

    pub fn get_loader(&mut self) -> &mut PluginLoader {
        &mut self.loader
    }

    pub fn get_virtual_paths(&self) -> &BTreeMap<PathBuf, PathBuf> {
        &self.virtual_paths
    }

    pub async fn access<F, Fut, R>(&self, id: &Id, op: F) -> miette::Result<R>
    where
        F: FnOnce(&T) -> Fut,
        Fut: Future<Output = miette::Result<R>> + Send + 'static,
    {
        let plugin = self.plugins.get(id).ok_or_else(|| PluginError::UnknownId {
            name: self.type_of.get_label().to_owned(),
            id: id.to_owned(),
        })?;

        op(plugin.value()).await
    }

    pub fn access_sync<F, R>(&self, id: &Id, op: F) -> miette::Result<R>
    where
        F: FnOnce(&T) -> miette::Result<R>,
    {
        let plugin = self.plugins.get(id).ok_or_else(|| PluginError::UnknownId {
            name: self.type_of.get_label().to_owned(),
            id: id.to_owned(),
        })?;

        op(plugin.value())
    }

    pub async fn perform<F, Fut, R>(&self, id: &Id, mut op: F) -> miette::Result<R>
    where
        F: FnMut(&mut T, MoonContext) -> Fut,
        Fut: Future<Output = miette::Result<R>> + Send + 'static,
    {
        let mut plugin = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| PluginError::UnknownId {
                name: self.type_of.get_label().to_owned(),
                id: id.to_owned(),
            })?;

        op(plugin.value_mut(), self.create_context()).await
    }

    pub fn perform_sync<F, R>(&self, id: &Id, mut op: F) -> miette::Result<R>
    where
        F: FnMut(&mut T, MoonContext) -> miette::Result<R>,
    {
        let mut plugin = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| PluginError::UnknownId {
                name: self.type_of.get_label().to_owned(),
                id: id.to_owned(),
            })?;

        op(plugin.value_mut(), self.create_context())
    }

    pub fn iter(&self) -> Iter<'_, Id, T> {
        self.plugins.iter()
    }

    pub fn iter_mut(&self) -> IterMut<'_, Id, T> {
        self.plugins.iter_mut()
    }

    pub async fn load<I, L>(&self, id: I, locator: L) -> miette::Result<()>
    where
        I: AsRef<Id>,
        L: AsRef<PluginLocator>,
    {
        self.load_with_config(id, locator, |_| Ok(())).await
    }

    pub async fn load_with_config<I, L, F>(
        &self,
        id: I,
        locator: L,
        mut op: F,
    ) -> miette::Result<()>
    where
        I: AsRef<Id>,
        L: AsRef<PluginLocator>,
        F: FnMut(&mut PluginManifest) -> miette::Result<()>,
    {
        let id = id.as_ref();

        if self.plugins.contains_key(id) {
            return Err(PluginError::ExistingId {
                name: self.type_of.get_label().to_owned(),
                id: id.to_owned(),
            }
            .into());
        }

        // Load the WASM file (this must happen first because of async)
        let plugin_file = self.loader.load_plugin(id, locator).await?;

        // Create host functions (provided by warpgate)
        let functions = create_host_functions(HostData {
            virtual_paths: self.virtual_paths.clone(),
            working_dir: self.moon_env.cwd.clone(),
        });

        // Create the manifest and let the consumer configure it
        let mut manifest = self.create_manifest(id, plugin_file)?;

        op(&mut manifest)?;

        // Combine everything into the container and register
        self.register(
            id.to_owned(),
            T::new(
                id.to_owned(),
                PluginContainer::new(id.to_owned(), manifest, functions)?,
            ),
        );

        Ok(())
    }

    pub fn register(&self, id: Id, plugin: T) {
        self.plugins.insert(id, plugin);
    }
}
