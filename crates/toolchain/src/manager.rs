use crate::{Tool, ToolchainError};
use moon_contract::Runtime;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ToolManager<T: Tool> {
    cache: HashMap<String, T>,
    runtime: Runtime, // Default workspace version
}

impl<T: Tool> ToolManager<T> {
    pub fn new(runtime: Runtime) -> Self {
        ToolManager {
            cache: HashMap::new(),
            runtime,
        }
    }

    pub fn get(&self) -> Result<&T, ToolchainError> {
        self.get_for_runtime(&self.runtime)
    }

    pub fn get_for_runtime(&self, runtime: &Runtime) -> Result<&T, ToolchainError> {
        match &runtime {
            Runtime::Node(version) => self.get_for_version(version),
            _ => panic!("Unsupported toolchain runtime."),
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
                    *version = tool.get_version();
                }
                _ => {
                    // Ignore
                }
            };
        }

        self.cache.insert(tool.get_version(), tool);
    }

    pub async fn setup(
        &mut self,
        version: &str,
        check_versions: bool,
    ) -> Result<u8, ToolchainError> {
        self.cache
            .get_mut(version)
            .expect("Missing tool")
            .run_setup(check_versions)
            .await
    }

    pub async fn teardown(&mut self, version: &str) -> Result<(), ToolchainError> {
        if let Some(mut tool) = self.cache.remove(version) {
            tool.run_teardown().await?;
        }

        Ok(())
    }

    pub async fn teardown_all(&mut self) -> Result<(), ToolchainError> {
        for (_, mut tool) in self.cache.drain() {
            tool.run_teardown().await?;
        }

        Ok(())
    }
}
