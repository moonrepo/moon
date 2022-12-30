use crate::errors::ToolError;
use crate::tool::Tool;
use moon_platform_runtime::{Runtime, Version};
use rustc_hash::FxHashMap;
use std::fmt::Debug;

#[derive(Debug)]
pub struct ToolManager<T: Tool> {
    cache: FxHashMap<String, T>,
    default_version: Version,
    runtime: Runtime,
}

impl<T: Tool> ToolManager<T> {
    pub fn new(runtime: Runtime) -> Self {
        ToolManager {
            cache: FxHashMap::default(),
            default_version: runtime.version(),
            runtime,
        }
    }

    pub fn get(&self) -> Result<&T, ToolError> {
        self.get_for_version(&self.default_version)
    }

    pub fn get_for_version<V: AsRef<Version>>(&self, version: V) -> Result<&T, ToolError> {
        let version = version.as_ref();

        if !self.has(version) {
            return Err(ToolError::UnknownTool(format!(
                "{} v{}",
                self.runtime, version.0
            )));
        }

        Ok(self.cache.get(&version.0).unwrap())
    }

    pub fn has(&self, version: &Version) -> bool {
        self.cache.contains_key(&version.0)
    }

    pub fn register(&mut self, version: &Version, tool: T) {
        // Nothing exists in the cache yet, so this tool must be the top-level
        // workspace tool. If so, update the default version within the platform.
        if self.cache.is_empty() && !version.is_override() {
            self.default_version = version.to_owned();
        }

        self.cache.insert(version.0.to_owned(), tool);
    }

    pub async fn setup(
        &mut self,
        version: &Version,
        last_versions: &mut FxHashMap<String, String>,
    ) -> Result<u8, ToolError> {
        match self.cache.get_mut(&version.0) {
            Some(cache) => Ok(cache.setup(last_versions).await?),
            None => Err(ToolError::UnknownTool(self.runtime.to_string())),
        }
    }

    pub async fn teardown(&mut self, version: &Version) -> Result<(), ToolError> {
        if let Some(mut tool) = self.cache.remove(&version.0) {
            tool.teardown().await?;
        }

        Ok(())
    }

    pub async fn teardown_all(&mut self) -> Result<(), ToolError> {
        for (_, mut tool) in self.cache.drain() {
            tool.teardown().await?;
        }

        Ok(())
    }
}
