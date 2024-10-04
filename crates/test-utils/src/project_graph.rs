use moon_config::{
    ConfigLoader, InheritedTasksEntry, InheritedTasksManager, NodeConfig,
    PartialInheritedTasksConfig, PartialTaskConfig, ToolchainConfig, WorkspaceConfig,
    WorkspaceProjects, WorkspaceProjectsConfig,
};
use moon_project_graph::{
    ExtendProjectEvent, ExtendProjectGraphEvent, ProjectGraphBuilder, ProjectGraphBuilderContext,
};
use moon_vcs::{BoxedVcs, Git};
use proto_core::ProtoConfig;
use starbase_events::Emitter;
use starbase_sandbox::create_sandbox;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use moon_project_graph::ProjectGraph;

#[derive(Default)]
pub struct ProjectGraphContainer {
    pub config_loader: ConfigLoader,
    pub inherited_tasks: InheritedTasksManager,
    pub toolchain_config: ToolchainConfig,
    pub workspace_config: WorkspaceConfig,
    pub workspace_root: PathBuf,
    pub vcs: Option<Arc<BoxedVcs>>,
}

impl ProjectGraphContainer {
    pub fn new(root: &Path) -> Self {
        let proto_config = ProtoConfig::default();
        let config_loader = ConfigLoader::default();
        let mut graph = Self {
            inherited_tasks: config_loader.load_tasks_manager(root).unwrap(),
            toolchain_config: config_loader
                .load_toolchain_config(root, &proto_config)
                .unwrap(),
            workspace_root: root.to_path_buf(),
            config_loader,
            ..Default::default()
        };

        // Add a global task to all projects
        graph.inherited_tasks.configs.insert(
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

        // Always use the node platform
        if graph.toolchain_config.node.is_none() {
            graph.toolchain_config.node = Some(NodeConfig::default());
        }

        // Use folders as project names
        if root.join(".moon/workspace.yml").exists() {
            graph.workspace_config = graph.config_loader.load_workspace_config(root).unwrap();
        } else {
            let mut projects = WorkspaceProjectsConfig {
                globs: vec![
                    "*".into(),
                    "!.home".into(),
                    "!.moon".into(),
                    "!.proto".into(),
                ],
                ..WorkspaceProjectsConfig::default()
            };

            if root.join("moon.yml").exists() {
                projects
                    .sources
                    .insert("root".try_into().unwrap(), ".".into());
            }

            graph.workspace_config.projects = WorkspaceProjects::Both(projects);
        }

        graph
    }

    pub fn with_vcs(root: &Path) -> Self {
        let mut container = Self::new(root);
        container.vcs = Some(Arc::new(Box::new(Git::load(root, "master", &[]).unwrap())));
        container
    }

    pub fn create_context(&self) -> ProjectGraphBuilderContext {
        ProjectGraphBuilderContext {
            config_loader: &self.config_loader,
            extend_project: Emitter::<ExtendProjectEvent>::new(),
            extend_project_graph: Emitter::<ExtendProjectGraphEvent>::new(),
            inherited_tasks: &self.inherited_tasks,
            toolchain_config: &self.toolchain_config,
            vcs: self.vcs.clone(),
            working_dir: &self.workspace_root,
            workspace_config: &self.workspace_config,
            workspace_root: &self.workspace_root,
        }
    }

    pub async fn build_graph<'l>(&self, context: ProjectGraphBuilderContext<'l>) -> ProjectGraph {
        let mut builder = ProjectGraphBuilder::new(context).await.unwrap();
        builder.load_all().await.unwrap();

        let graph = builder.build().await.unwrap();
        graph.get_all().unwrap();
        graph
    }

    pub async fn build_graph_for<'l>(
        &self,
        context: ProjectGraphBuilderContext<'l>,
        ids: &[&str],
    ) -> ProjectGraph {
        let mut builder = ProjectGraphBuilder::new(context).await.unwrap();

        for id in ids {
            builder.load(id).await.unwrap();
        }

        let graph = builder.build().await.unwrap();

        for id in ids {
            graph.get(id).unwrap();
        }

        graph
    }
}

pub async fn generate_project_graph(fixture: &str) -> ProjectGraph {
    generate_project_graph_from_sandbox(create_sandbox(fixture).path()).await
}

pub async fn generate_project_graph_from_sandbox(root: &Path) -> ProjectGraph {
    generate_project_graph_with_changes(root, |_| {}).await
}

pub async fn generate_project_graph_with_changes<F>(root: &Path, mut op: F) -> ProjectGraph
where
    F: FnMut(&mut ProjectGraphContainer),
{
    let mut container = ProjectGraphContainer::new(root);

    op(&mut container);

    let context = container.create_context();

    container.build_graph(context).await
}
