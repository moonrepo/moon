use crate::errors::ToolchainError;
use moon_platform_runtime::{Runtime, Version};
use moon_tool::Tool;
use rustc_hash::FxHashMap;
use std::any::Any;
use std::fmt::Debug;

pub type CachedTool = Box<dyn Tool>;

#[derive(Debug)]
pub struct ToolManager {
    cache: FxHashMap<String, CachedTool>,
    runtime: Runtime, // Default workspace version
}

impl ToolManager {
    pub fn new(runtime: Runtime) -> Self {
        ToolManager {
            cache: FxHashMap::default(),
            runtime,
        }
    }

    pub fn get<T: 'static>(&self) -> Result<&T, ToolchainError> {
        self.get_for_runtime::<T>(&self.runtime)
    }

    pub fn get_for_runtime<T: 'static>(&self, runtime: &Runtime) -> Result<&T, ToolchainError> {
        match &runtime {
            Runtime::Node(version) => self.get_for_version(&version.0),
            _ => Err(ToolchainError::UnsupportedRuntime(runtime.to_owned())),
        }
    }

    pub fn get_for_version<T: 'static>(&self, version: &str) -> Result<&T, ToolchainError> {
        if !self.has(version) {
            return Err(ToolchainError::MissingTool(format!(
                "{} v{}",
                self.runtime, version
            )));
        }

        let item = self.cache.get(version).unwrap() as &dyn Any;

        Ok(item.downcast_ref::<T>().unwrap())
    }

    pub fn has(&self, version: &str) -> bool {
        self.cache.contains_key(version)
    }

    pub fn register(&mut self, tool: CachedTool, root: bool) {
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
            Some(cache) => Ok(cache.setup(last_versions).await?),
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
