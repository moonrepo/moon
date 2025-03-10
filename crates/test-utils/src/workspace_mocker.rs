use moon_cache::CacheEngine;
use moon_config::*;
use moon_toolchain_plugin::ToolchainRegistry;
use moon_vcs::{BoxedVcs, Git};
use moon_workspace::*;
use moon_workspace_graph::WorkspaceGraph;
use proto_core::ProtoConfig;
use starbase_events::Emitter;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Default)]
pub struct WorkspaceMocker {
    pub config_loader: ConfigLoader,
    pub inherited_tasks: InheritedTasksManager,
    pub toolchain_config: ToolchainConfig,
    pub workspace_config: WorkspaceConfig,
    pub workspace_root: PathBuf,
    pub vcs: Option<Arc<BoxedVcs>>,
}

impl WorkspaceMocker {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            workspace_root: root.as_ref().to_path_buf(),
            ..Default::default()
        }
    }

    pub fn with_default_configs(&mut self) -> &mut Self {
        let root = &self.workspace_root;

        self.inherited_tasks = self.config_loader.load_tasks_manager(root).unwrap();

        self.toolchain_config = self
            .config_loader
            .load_toolchain_config(root, &ProtoConfig::default())
            .unwrap();

        self.workspace_config = self.config_loader.load_workspace_config(root).unwrap();

        self
    }

    pub fn with_default_projects(&mut self) -> &mut Self {
        if !self.workspace_root.join(".moon/workspace.yml").exists() {
            // Use folders as project names
            let mut projects = WorkspaceProjectsConfig {
                globs: vec![
                    "*".into(),
                    "!.home".into(),
                    "!.moon".into(),
                    "!.proto".into(),
                ],
                ..WorkspaceProjectsConfig::default()
            };

            // Include a root project conditionally
            if self.workspace_root.join("moon.yml").exists() {
                projects
                    .sources
                    .insert("root".try_into().unwrap(), ".".into());
            }

            self.workspace_config.projects = WorkspaceProjects::Both(projects);
        }

        self
    }

    pub fn with_default_toolchain(&mut self) -> &mut Self {
        if self.toolchain_config.node.is_none() {
            self.toolchain_config.node = Some(NodeConfig::default());
        }

        self
    }

    pub fn with_global_tasks(&mut self) -> &mut Self {
        self.inherited_tasks.configs.insert(
            "*".into(),
            InheritedTasksEntry {
                input: ".moon/tasks.yml".into(),
                config: PartialInheritedTasksConfig {
                    tasks: Some(BTreeMap::from_iter([(
                        "global".try_into().unwrap(),
                        PartialTaskConfig::default(),
                    )])),
                    ..PartialInheritedTasksConfig::default()
                },
            },
        );

        self
    }

    pub fn with_vcs(&mut self) -> &mut Self {
        self.vcs = Some(Arc::new(Box::new(
            Git::load(&self.workspace_root, "master", &[]).unwrap(),
        )));

        self
    }

    pub fn create_context(&self) -> WorkspaceBuilderContext {
        WorkspaceBuilderContext {
            config_loader: &self.config_loader,
            enabled_toolchains: self.toolchain_config.get_enabled(),
            extend_project: Emitter::<ExtendProjectEvent>::new(),
            extend_project_graph: Emitter::<ExtendProjectGraphEvent>::new(),
            inherited_tasks: &self.inherited_tasks,
            toolchain_config: &self.toolchain_config,
            toolchain_registry: Arc::new(ToolchainRegistry::default()),
            vcs: self.vcs.clone(),
            working_dir: &self.workspace_root,
            workspace_config: &self.workspace_config,
            workspace_root: &self.workspace_root,
        }
    }

    pub async fn build_workspace_graph(&self) -> WorkspaceGraph {
        self.build_workspace_graph_with_options(WorkspaceMockOptions::default())
            .await
    }

    pub async fn build_workspace_graph_for(&self, ids: &[&str]) -> WorkspaceGraph {
        self.build_workspace_graph_with_options(WorkspaceMockOptions {
            ids: Vec::from_iter(ids.iter().map(|id| id.to_string())),
            ..Default::default()
        })
        .await
    }

    pub async fn build_workspace_graph_with_options(
        &self,
        mut options: WorkspaceMockOptions<'_>,
    ) -> WorkspaceGraph {
        let context = options
            .context
            .take()
            .unwrap_or_else(|| self.create_context());

        let mut builder = match &options.cache {
            Some(engine) => WorkspaceBuilder::new_with_cache(context, engine)
                .await
                .unwrap(),
            None => WorkspaceBuilder::new(context).await.unwrap(),
        };

        if options.ids.is_empty() {
            builder.load_projects().await.unwrap();
        } else {
            for id in &options.ids {
                builder.load_project(id).await.unwrap();
            }
        }

        builder.load_tasks().await.unwrap();

        let workspace_graph = builder.build().await.unwrap();

        if options.ids.is_empty() {
            workspace_graph.projects.get_all().unwrap();
        } else {
            for id in &options.ids {
                workspace_graph.projects.get(id).unwrap();
            }
        }

        workspace_graph
    }
}

#[derive(Default)]
pub struct WorkspaceMockOptions<'l> {
    pub cache: Option<CacheEngine>,
    pub context: Option<WorkspaceBuilderContext<'l>>,
    pub ids: Vec<String>,
}
