use moon_pdk_api::*;
use moon_target::Target;
use proto_pdk_test_utils::WasmTestWrapper as ToolTestWrapper;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use warpgate::PluginContainer;

pub struct ToolchainTestWrapper {
    pub metadata: RegisterToolchainOutput,
    pub plugin: Arc<PluginContainer>,
    pub root: PathBuf,
    pub tool: Option<ToolTestWrapper>,
}

impl ToolchainTestWrapper {
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

    pub async fn define_docker_metadata(
        &self,
        mut input: DefineDockerMetadataInput,
    ) -> DefineDockerMetadataOutput {
        input.context = self.create_context();

        self.plugin
            .call_func_with("define_docker_metadata", input)
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
    ) -> ExtendTaskCommandOutput {
        input.context = self.create_context();

        input.globals_dir = input
            .globals_dir
            .map(|path| self.plugin.to_virtual_path(path));

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

        input.globals_dir = input
            .globals_dir
            .map(|path| self.plugin.to_virtual_path(path));

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

    pub async fn hash_task_contents(
        &self,
        mut input: HashTaskContentsInput,
    ) -> HashTaskContentsOutput {
        input.context = self.create_context();

        if input.project.id.is_empty() {
            input.project = self.create_project_fragment();
        }

        if input.task.target.id.is_empty() {
            input.task = self.create_task_fragment();
        }

        self.plugin
            .call_func_with("hash_task_contents", input)
            .await
            .unwrap()
    }

    pub async fn initialize_toolchain(
        &self,
        mut input: InitializeToolchainInput,
    ) -> InitializeToolchainOutput {
        input.context = self.create_context();

        self.plugin
            .call_func_with("initialize_toolchain", input)
            .await
            .unwrap()
    }

    pub async fn install_dependencies(
        &self,
        mut input: InstallDependenciesInput,
    ) -> InstallDependenciesOutput {
        input.context = self.create_context();
        input.root = self.plugin.to_virtual_path(input.root);

        if input
            .project
            .as_ref()
            .is_some_and(|project| project.id.is_empty())
        {
            input.project = Some(self.create_project_fragment());
        }

        self.plugin
            .call_func_with("install_dependencies", input)
            .await
            .unwrap()
    }

    pub async fn locate_dependencies_root(
        &self,
        mut input: LocateDependenciesRootInput,
    ) -> LocateDependenciesRootOutput {
        input.context = self.create_context();
        input.starting_dir = self.plugin.to_virtual_path(input.starting_dir);

        self.plugin
            .call_func_with("locate_dependencies_root", input)
            .await
            .unwrap()
    }

    pub async fn parse_lock(&self, mut input: ParseLockInput) -> ParseLockOutput {
        input.context = self.create_context();
        input.path = self.plugin.to_virtual_path(input.path);
        input.root = self.plugin.to_virtual_path(input.root);

        self.plugin
            .call_func_with("parse_lock", input)
            .await
            .unwrap()
    }

    pub async fn parse_manifest(&self, mut input: ParseManifestInput) -> ParseManifestOutput {
        input.context = self.create_context();
        input.path = self.plugin.to_virtual_path(input.path);
        input.root = self.plugin.to_virtual_path(input.root);

        self.plugin
            .call_func_with("parse_manifest", input)
            .await
            .unwrap()
    }

    pub async fn register_toolchain(
        &self,
        input: RegisterToolchainInput,
    ) -> RegisterToolchainOutput {
        self.plugin
            .call_func_with("register_toolchain", input)
            .await
            .unwrap()
    }

    pub async fn prune_docker(&self, mut input: PruneDockerInput) -> PruneDockerOutput {
        input.context = self.create_context();
        input.root = self.plugin.to_virtual_path(input.root);

        self.plugin
            .call_func_with("prune_docker", input)
            .await
            .unwrap()
    }

    pub async fn scaffold_docker(&self, mut input: ScaffoldDockerInput) -> ScaffoldDockerOutput {
        input.context = self.create_context();
        input.input_dir = self.plugin.to_virtual_path(input.input_dir);
        input.output_dir = self.plugin.to_virtual_path(input.output_dir);

        if input.project.id.is_empty() {
            input.project = self.create_project_fragment();
        }

        self.plugin
            .call_func_with("scaffold_docker", input)
            .await
            .unwrap()
    }

    pub async fn setup_environment(
        &self,
        mut input: SetupEnvironmentInput,
    ) -> SetupEnvironmentOutput {
        input.context = self.create_context();
        input.root = self.plugin.to_virtual_path(input.root);

        input.globals_dir = input
            .globals_dir
            .map(|path| self.plugin.to_virtual_path(path));

        if input
            .project
            .as_ref()
            .is_some_and(|project| project.id.is_empty())
        {
            input.project = Some(self.create_project_fragment());
        }

        self.plugin
            .call_func_with("setup_environment", input)
            .await
            .unwrap()
    }

    pub async fn setup_toolchain(&self, mut input: SetupToolchainInput) -> SetupToolchainOutput {
        input.context = self.create_context();

        self.plugin
            .call_func_with("setup_toolchain", input)
            .await
            .unwrap()
    }

    pub async fn sync_project(&self, mut input: SyncProjectInput) -> SyncOutput {
        input.context = self.create_context();

        if input.project.id.is_empty() {
            input.project = self.create_project_fragment();
        }

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

    pub async fn teardown_toolchain(&self, mut input: TeardownToolchainInput) {
        input.context = self.create_context();

        self.plugin
            .call_func_without_output("teardown_toolchain", input)
            .await
            .unwrap();
    }
}

impl Deref for ToolchainTestWrapper {
    type Target = ToolTestWrapper;

    fn deref(&self) -> &Self::Target {
        self.tool
            .as_ref()
            .expect("Toolchain requires `register_tool` (tier 3) to use this API.")
    }
}
