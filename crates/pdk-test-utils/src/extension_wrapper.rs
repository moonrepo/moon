use moon_pdk_api::*;
use moon_target::Target;
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

    pub fn create_project_fragment(&self) -> ProjectFragment {
        ProjectFragment {
            id: Id::raw("project"),
            source: "project".into(),
            ..Default::default()
        }
    }

    pub fn create_task_fragment(&self) -> TaskFragment {
        TaskFragment {
            target: Target::parse("project:task").unwrap(),
            ..Default::default()
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

    pub async fn extend_command(&self, mut input: ExtendCommandInput) -> ExtendCommandOutput {
        input.context = self.create_context();

        self.plugin
            .call_func_with("extend_command", input)
            .await
            .unwrap()
    }

    pub async fn extend_project_graph(
        &self,
        mut input: ExtendProjectGraphInput,
    ) -> ExtendProjectGraphOutput {
        input.context = self.create_context();

        self.plugin
            .call_func_with("extend_project_graph", input)
            .await
            .unwrap()
    }

    pub async fn extend_task_command(
        &self,
        mut input: ExtendTaskCommandInput,
    ) -> ExtendCommandOutput {
        input.context = self.create_context();

        if input.project.id.is_empty() {
            input.project = self.create_project_fragment();
        }

        if input.task.target.id.is_empty() {
            input.task = self.create_task_fragment();
        }

        self.plugin
            .call_func_with("extend_task_command", input)
            .await
            .unwrap()
    }

    pub async fn extend_task_script(
        &self,
        mut input: ExtendTaskScriptInput,
    ) -> ExtendTaskScriptOutput {
        input.context = self.create_context();

        if input.project.id.is_empty() {
            input.project = self.create_project_fragment();
        }

        if input.task.target.id.is_empty() {
            input.task = self.create_task_fragment();
        }

        self.plugin
            .call_func_with("extend_task_script", input)
            .await
            .unwrap()
    }
}
