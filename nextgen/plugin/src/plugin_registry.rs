use crate::plugin::*;
use dashmap::{
    iter::{Iter, IterMut},
    DashMap,
};
use extism::{Manifest as PluginManifest, Wasm};
use moon_env::MoonEnvironment;
use proto_core::{host_funcs::*, ProtoEnvironment};
use std::{future::Future, path::PathBuf, sync::Arc};
use warpgate::{Id, PluginContainer, PluginLoader, PluginLocator};

pub struct PluginContext {}

pub struct PluginRegistry<T: Plugin> {
    loader: PluginLoader,
    moon_env: Arc<MoonEnvironment>,
    plugins: Arc<DashMap<Id, T>>,
    proto_env: Arc<ProtoEnvironment>,
}

impl<T: Plugin> PluginRegistry<T> {
    pub fn new(
        type_of: PluginType,
        moon_env: Arc<MoonEnvironment>,
        proto_env: Arc<ProtoEnvironment>,
    ) -> Self {
        Self {
            loader: PluginLoader::new(
                &moon_env.plugins_dir.join(type_of.get_dir_name()),
                &moon_env.temp_dir,
            ),
            plugins: Arc::new(DashMap::new()),
            moon_env,
            proto_env,
        }
    }

    pub fn create_manifest(&self, wasm_file: PathBuf) -> PluginManifest {
        let mut manifest = PluginManifest::new([Wasm::file(wasm_file)]);

        // Allow all hosts because we don't know what endpoints plugins
        // will communicate with. Far too many to account for.
        manifest = manifest.with_allowed_host("*");

        // Inherit moon and proto virtual paths.
        let mut virtual_paths = self.proto_env.get_virtual_paths();
        virtual_paths.extend(self.moon_env.get_virtual_paths());

        manifest = manifest.with_allowed_paths(virtual_paths.into_iter());

        // Disable timeouts as some functions, like dependency installs,
        // may take multiple minutes to complete. We also can't account
        // for network connectivity.
        manifest.timeout_ms = None;

        manifest
    }

    pub fn get_cache(&self) -> Arc<DashMap<Id, T>> {
        Arc::clone(&self.plugins)
    }

    pub fn get_loader(&mut self) -> &mut PluginLoader {
        &mut self.loader
    }

    pub async fn access<F, Fut, R>(&self, id: &Id, op: F) -> miette::Result<R>
    where
        F: FnOnce(&T) -> Fut,
        Fut: Future<Output = miette::Result<R>> + Send + 'static,
    {
        let plugin = self
            .plugins
            .get(id)
            .ok_or_else(|| miette::miette!("TODO"))?;

        op(plugin.value()).await
    }

    pub fn access_sync<F, R>(&self, id: &Id, op: F) -> miette::Result<R>
    where
        F: FnOnce(&T) -> miette::Result<R>,
    {
        let plugin = self
            .plugins
            .get(id)
            .ok_or_else(|| miette::miette!("TODO"))?;

        op(plugin.value())
    }

    pub async fn perform<F, Fut, R>(&self, id: &Id, mut op: F) -> miette::Result<R>
    where
        F: FnMut(&mut T) -> Fut,
        Fut: Future<Output = miette::Result<R>> + Send + 'static,
    {
        let mut plugin = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| miette::miette!("TODO"))?;

        op(plugin.value_mut()).await
    }

    pub fn perform_sync<F, R>(&self, id: &Id, mut op: F) -> miette::Result<R>
    where
        F: FnMut(&mut T) -> miette::Result<R>,
    {
        let mut plugin = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| miette::miette!("TODO"))?;

        op(plugin.value_mut())
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

        // TODO error if it already exists

        let functions = create_host_functions(HostData {
            id: id.to_owned(),
            proto: Arc::clone(&self.proto_env),
        });

        let mut manifest = self.create_manifest(self.loader.load_plugin(id, locator).await?);

        op(&mut manifest)?;

        let container = PluginContainer::new(id.to_owned(), manifest, functions)?;

        self.register(id.to_owned(), T::new(id.to_owned(), container));

        Ok(())
    }

    pub fn register(&self, id: Id, plugin: T) {
        self.plugins.insert(id, plugin);
    }
}
