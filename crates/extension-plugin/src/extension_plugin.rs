use async_trait::async_trait;
use moon_common::Id;
use moon_config::schematic::Schema;
use moon_pdk_api::*;
use moon_plugin::{Plugin, PluginContainer, PluginRegistration, PluginType};
use std::fmt;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::instrument;

pub type ExtensionMetadata = RegisterExtensionOutput;

pub struct ExtensionPlugin {
    pub id: Id,
    pub locator: PluginLocator,
    pub metadata: ExtensionMetadata,

    plugin: Arc<PluginContainer>,
}

#[async_trait]
impl Plugin for ExtensionPlugin {
    async fn new(registration: PluginRegistration) -> miette::Result<Self> {
        let plugin = Arc::new(registration.container);

        let metadata: RegisterExtensionOutput = plugin
            .cache_func_with(
                "register_extension",
                RegisterExtensionInput {
                    id: registration.id.clone(),
                },
            )
            .await?;

        Ok(Self {
            id: registration.id,
            locator: registration.locator,
            metadata,
            plugin,
        })
    }

    fn get_id(&self) -> &Id {
        &self.id
    }

    fn get_type(&self) -> PluginType {
        PluginType::Extension
    }

    async fn has_func(&self, name: &str) -> bool {
        self.plugin.has_func(name).await
    }
}

impl ExtensionPlugin {
    fn handle_output_file(&self, file: &mut PathBuf) {
        *file = self.plugin.to_real_path(&file).to_path_buf();
    }

    fn handle_output_files(&self, files: &mut [PathBuf]) {
        for file in files {
            self.handle_output_file(file);
        }
    }

    fn handle_virtual_file(&self, file: &mut VirtualPath) {
        *file = VirtualPath::Real(self.plugin.from_virtual_path(&file));
    }

    fn handle_virtual_files(&self, files: &mut [VirtualPath]) {
        for file in files {
            self.handle_virtual_file(file);
        }
    }

    #[instrument(skip(self))]
    pub async fn define_extension_config(&self) -> miette::Result<Option<Schema>> {
        if self.has_func("define_extension_config").await {
            let output: DefineExtensionConfigOutput =
                self.cache_func("define_extension_config").await?;

            return Ok(Some(output.schema));
        }

        Ok(None)
    }

    #[instrument(skip(self, context))]
    pub async fn execute(&self, args: Vec<String>, context: MoonContext) -> miette::Result<()> {
        if self.has_func("execute_extension").await {
            self.call_func_without_output(
                "execute_extension",
                ExecuteExtensionInput { args, context },
            )
            .await?;
        }

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn extend_command(
        &self,
        input: ExtendCommandInput,
    ) -> miette::Result<ExtendCommandOutput> {
        let mut output = ExtendCommandOutput::default();

        if self.has_func("extend_command").await {
            output = self.cache_func_with("extend_command", input).await?;

            self.handle_output_files(&mut output.paths);
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn extend_project_graph(
        &self,
        input: ExtendProjectGraphInput,
    ) -> miette::Result<ExtendProjectGraphOutput> {
        let mut output = ExtendProjectGraphOutput::default();

        if self.has_func("extend_project_graph").await {
            output = self.cache_func_with("extend_project_graph", input).await?;

            self.handle_virtual_files(&mut output.input_files);
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn extend_task_command(
        &self,
        input: ExtendTaskCommandInput,
    ) -> miette::Result<ExtendCommandOutput> {
        let mut output = ExtendCommandOutput::default();

        if self.has_func("extend_task_command").await {
            output = self.cache_func_with("extend_task_command", input).await?;

            self.handle_output_files(&mut output.paths);
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn extend_task_script(
        &self,
        input: ExtendTaskScriptInput,
    ) -> miette::Result<ExtendTaskScriptOutput> {
        let mut output = ExtendTaskScriptOutput::default();

        if self.has_func("extend_task_script").await {
            output = self.cache_func_with("extend_task_script", input).await?;

            self.handle_output_files(&mut output.paths);
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn initialize_extension(
        &self,
        input: InitializeExtensionInput,
    ) -> miette::Result<InitializeExtensionOutput> {
        // Function check happens during extension registration
        let output: InitializeExtensionOutput =
            self.cache_func_with("initialize_extension", input).await?;

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn sync_project(&self, input: SyncProjectInput) -> miette::Result<SyncOutput> {
        let mut output = SyncOutput::default();

        if self.has_func("sync_project").await {
            output = self.call_func_with("sync_project", input).await?;

            self.handle_virtual_files(&mut output.changed_files);
        }

        Ok(output)
    }

    #[instrument(skip(self))]
    pub async fn sync_workspace(&self, input: SyncWorkspaceInput) -> miette::Result<SyncOutput> {
        let mut output = SyncOutput::default();

        if self.has_func("sync_workspace").await {
            output = self.call_func_with("sync_workspace", input).await?;

            self.handle_virtual_files(&mut output.changed_files);
        }

        Ok(output)
    }
}

impl Deref for ExtensionPlugin {
    type Target = PluginContainer;

    fn deref(&self) -> &Self::Target {
        &self.plugin
    }
}

impl fmt::Debug for ExtensionPlugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExtensionPlugin")
            .field("id", &self.id)
            .field("locator", &self.locator)
            .field("metadata", &self.metadata)
            .finish()
    }
}
