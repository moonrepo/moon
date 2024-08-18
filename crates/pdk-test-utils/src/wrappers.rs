use moon_pdk_api::{ExecuteExtensionInput, MoonContext};
use std::path::Path;
use warpgate::PluginContainer;

pub struct ExtensionTestWrapper {
    pub plugin: PluginContainer,
}

impl ExtensionTestWrapper {
    pub fn create_context(&self, sandbox: &Path) -> MoonContext {
        MoonContext {
            working_dir: self.plugin.to_virtual_path(sandbox),
            workspace_root: self.plugin.to_virtual_path(sandbox),
        }
    }

    pub async fn execute_extension(&self, input: ExecuteExtensionInput) {
        self.plugin
            .call_func_without_output("execute_extension", input)
            .await
            .unwrap();
    }
}
