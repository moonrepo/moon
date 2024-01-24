use moon_pdk_api::extension::ExecuteExtensionInput;
use moon_pdk_api::MoonContext;
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

    pub fn prepare_context(&self, context: MoonContext) -> MoonContext {
        MoonContext {
            working_dir: self.plugin.to_virtual_path(context.working_dir),
            workspace_root: self.plugin.to_virtual_path(context.workspace_root),
        }
    }

    pub fn execute_extension(&self, mut input: ExecuteExtensionInput) {
        input.context = self.prepare_context(input.context);

        self.plugin
            .call_func_without_output("execute_extension", input)
            .unwrap();
    }
}
