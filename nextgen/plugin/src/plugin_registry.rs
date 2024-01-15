use crate::plugin::Plugin;
use dashmap::{
    iter::{Iter, IterMut},
    DashMap,
};
use std::{future::Future, path::Path, sync::Arc};
use warpgate::{Id, PluginLoader, PluginLocator};

// cwd
// moon root
// moon paths
// proto paths

pub struct PluginRegistry<T: Plugin> {
    loader: PluginLoader,
    plugins: Arc<DashMap<Id, T>>,
}

impl<T: Plugin> PluginRegistry<T> {
    pub fn new(plugins_dir: &Path, temp_dir: &Path) -> Self {
        Self {
            loader: PluginLoader::new(plugins_dir, temp_dir),
            plugins: Arc::new(DashMap::new()),
        }
    }

    pub fn get_cache(&self) -> Arc<DashMap<Id, T>> {
        Arc::clone(&self.plugins)
    }

    pub fn get_loader(&mut self) -> &mut PluginLoader {
        &mut self.loader
    }

    pub fn has_plugin(&self, id: &Id) -> bool {
        self.plugins.contains_key(id)
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

    pub async fn load<I: AsRef<Id>, L: AsRef<PluginLocator>>(
        &self,
        id: I,
        locator: L,
    ) -> miette::Result<()> {
        let id = id.as_ref();

        // TODO error if it already exists

        self.register(
            id.to_owned(),
            T::new(id.to_owned(), self.loader.load_plugin(id, locator).await?)?,
        );

        Ok(())
    }

    pub fn register(&self, id: Id, plugin: T) {
        self.plugins.insert(id, plugin);
    }
}
