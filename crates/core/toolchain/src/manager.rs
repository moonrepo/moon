use crate::{RuntimeTool, ToolchainError};
use moon_platform::{Runtime, Version};
use rustc_hash::FxHashMap;
use std::fmt::Debug;

#[derive(Debug)]
pub struct ToolManager<T: RuntimeTool + Debug> {
    cache: FxHashMap<String, T>,
    runtime: Runtime, // Default workspace version
}

impl<T: RuntimeTool + Debug> ToolManager<T> {
    pub fn new(runtime: Runtime) -> Self {
        ToolManager {
            cache: FxHashMap::default(),
            runtime,
        }
    }

    pub fn get(&self) -> Result<&T, ToolchainError> {
        self.get_for_runtime(&self.runtime)
    }

    pub fn get_for_runtime(&self, runtime: &Runtime) -> Result<&T, ToolchainError> {
        match &runtime {
            Runtime::Node(version) => self.get_for_version(&version.0),
            _ => Err(ToolchainError::UnsupportedRuntime(runtime.to_owned())),
        }
    }

    pub fn get_for_version(&self, version: &str) -> Result<&T, ToolchainError> {
        if !self.has(version) {
            return Err(ToolchainError::MissingTool(format!(
                "{} v{}",
                self.runtime, version
            )));
        }

        Ok(self.cache.get(version).unwrap())
    }

    pub fn has(&self, version: &str) -> bool {
        self.cache.contains_key(version)
    }

    pub fn register(&mut self, tool: T, root: bool) {
        // Nothing exists in the cache yet, so this tool must be the top-level
        // workspace tool. If so, update the default version within the platform.
        if self.cache.is_empty() && root {
            #[allow(clippy::single_match)]
            match &mut self.runtime {
                Runtime::Node(ref mut version) => {
                    *version = Version(tool.get_version().to_owned(), false);
                }
                _ => {
                    // Ignore
                }
            };
        }

        self.cache.insert(tool.get_version().to_owned(), tool);
    }

    pub async fn setup(
        &mut self,
        version: &str,
        last_versions: &mut FxHashMap<String, String>,
    ) -> Result<u8, ToolchainError> {
        match self.cache.get_mut(version) {
            Some(cache) => cache.setup(last_versions).await,
            None => Err(ToolchainError::MissingTool(self.runtime.to_string())),
        }
    }

    pub async fn teardown(&mut self, version: &str) -> Result<(), ToolchainError> {
        if let Some(mut tool) = self.cache.remove(version) {
            tool.teardown().await?;
        }

        Ok(())
    }

    pub async fn teardown_all(&mut self) -> Result<(), ToolchainError> {
        for (_, mut tool) in self.cache.drain() {
            tool.teardown().await?;
        }

        Ok(())
    }
}
