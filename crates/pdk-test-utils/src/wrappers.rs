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
}

pub struct ToolchainTestWrapper {
    pub metadata: RegisterToolchainOutput,
    pub plugin: PluginContainer,
    pub root: PathBuf,
}

impl ToolchainTestWrapper {
    pub fn create_context(&self) -> MoonContext {
        MoonContext {
            working_dir: self.plugin.to_virtual_path(&self.root),
            workspace_root: self.plugin.to_virtual_path(&self.root),
        }
    }

    pub async fn hash_task_contents(
        &self,
        mut input: HashTaskContentsInput,
    ) -> HashTaskContentsOutput {
        input.context = self.create_context();

        self.plugin
            .call_func_with("hash_task_contents", input)
            .await
            .unwrap()
    }

    pub async fn sync_project(&self, mut input: SyncProjectInput) -> SyncOutput {
        input.context = self.create_context();

        self.plugin
            .call_func_with("sync_project", input)
            .await
            .unwrap()
    }

    pub async fn sync_workspace(&self, mut input: SyncWorkspaceInput) -> SyncOutput {
        input.context = self.create_context();

        self.plugin
            .call_func_with("sync_workspace", input)
            .await
            .unwrap()
    }
}
