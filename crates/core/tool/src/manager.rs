use crate::errors::ToolError;
use crate::tool::Tool;
use moon_platform_runtime::{Runtime, Version};
use rustc_hash::FxHashMap;
use std::fmt::Debug;

#[derive(Debug)]
pub struct ToolManager<T: Tool> {
    cache: FxHashMap<String, T>,
    runtime: Runtime, // Default workspace version
}

impl<T: Tool> ToolManager<T> {
    pub fn new(runtime: Runtime) -> Self {
        ToolManager {
            cache: FxHashMap::default(),
            runtime,
        }
    }

    pub fn get(&self) -> Result<&T, ToolError> {
        self.get_for_runtime(&self.runtime)
    }

    pub fn get_for_runtime(&self, runtime: &Runtime) -> Result<&T, ToolError> {
        match &runtime {
            Runtime::Node(version) => self.get_for_version(&version.0),
            _ => Err(ToolError::UnsupportedRuntime(runtime.to_owned())),
        }
    }

    pub fn get_for_version(&self, version: &str) -> Result<&T, ToolError> {
        if !self.has(version) {
            return Err(ToolError::UnknownTool(format!(
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
    ) -> Result<u8, ToolError> {
        match self.cache.get_mut(version) {
            Some(cache) => Ok(cache.setup(last_versions).await?),
            None => Err(ToolError::UnknownTool(self.runtime.to_string())),
        }
    }

    pub async fn teardown(&mut self, version: &str) -> Result<(), ToolError> {
        if let Some(mut tool) = self.cache.remove(version) {
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
