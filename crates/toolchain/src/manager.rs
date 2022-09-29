use crate::{Tool, ToolchainError};
use std::collections::HashMap;

#[derive(Debug)]
pub struct ToolManager<T: Tool> {
    cache: HashMap<String, T>,
    version: String, // Default workspace version
}

impl<T: Tool> Default for ToolManager<T> {
    fn default() -> Self {
        ToolManager {
            cache: HashMap::new(),
            version: "latest".into(),
        }
    }
}

impl<T: Tool> ToolManager<T> {
    pub fn from(tool: T) -> Self {
        let version = tool.get_version();

        ToolManager {
            cache: HashMap::from([(tool.get_version(), tool)]),
            version,
        }
    }

    pub fn get(&self) -> Result<&T, ToolchainError> {
        self.get_version(&self.version)
    }

    pub fn get_version(&self, version: &str) -> Result<&T, ToolchainError> {
        if !self.has(version) {
            return Err(ToolchainError::RequiresNode);
        }

        Ok(self.cache.get(version).unwrap())
    }

    pub fn has(&self, version: &str) -> bool {
        self.cache.contains_key(version)
    }

    pub fn register(&mut self, tool: T) {
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
