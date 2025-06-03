use moon_pdk_api::*;
use std::path::PathBuf;
use warpgate::PluginContainer;

pub struct ExtensionTestWrapper {
    pub metadata: RegisterExtensionOutput,
    pub plugin: PluginContainer,
    pub root: PathBuf,
}

impl ExtensionTestWrapper {
    pub fn create_context(&self) -> MoonContext {
        MoonContext {
            working_dir: self.plugin.to_virtual_path(&self.root),
            workspace_root: self.plugin.to_virtual_path(&self.root),
        }
    }

    pub async fn execute_extension(&self, mut input: ExecuteExtensionInput) {
        input.context = self.create_context();

        self.plugin
            .call_func_without_output("execute_extension", input)
            .await
            .unwrap();
    }

    pub async fn register_extension(
        &self,
        input: RegisterExtensionInput,
    ) -> RegisterExtensionOutput {
        self.plugin
            .call_func_with("register_extension", input)
            .await
            .unwrap()
    }
}
