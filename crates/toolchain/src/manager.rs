use crate::{Tool, ToolchainError};
use std::collections::HashMap;

#[derive(Debug)]
pub struct ToolManager<T: Tool> {
    cache: HashMap<String, T>,
    version: String, // Default workspace version
}

impl<T: Tool> ToolManager<T> {
    pub fn new() -> Self {
        ToolManager {
            cache: HashMap::new(),
            version: "latest".into(),
        }
    }

    pub fn new_with(version: &str, tool: T) -> Self {
        ToolManager {
            cache: HashMap::from([(version.to_owned(), tool)]),
            version: version.into(),
        }
    }

    pub fn get(&self) -> Result<&T, ToolchainError> {
        self.get_version(&self.version)
    }

    pub fn get_version(&self, version: &str) -> Result<&T, ToolchainError> {
        if !self.cache.contains_key(version) {
            return Err(ToolchainError::RequiresNode);
        }

        Ok(self.cache.get(version).unwrap())
    }

    pub async fn setup<F>(
        &mut self,
        version: &str,
        check_versions: bool,
        factory: F,
    ) -> Result<u8, ToolchainError>
    where
        F: FnOnce() -> Result<T, ToolchainError>,
    {
        let mut tool = match self.cache.remove(version) {
            Some(tool) => tool,
            None => factory()?,
        };

        let installed = tool.run_setup(check_versions).await?;

        self.cache.insert(version.to_owned(), tool);

        Ok(installed)
    }

    pub async fn teardown(&mut self) -> Result<(), ToolchainError> {
        for (_, mut tool) in self.cache.drain() {
            tool.run_teardown().await?;
        }

        Ok(())
    }
}
