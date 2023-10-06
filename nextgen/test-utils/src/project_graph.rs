use moon_config::{
    InheritedTasksEntry, InheritedTasksManager, NodeConfig, PartialInheritedTasksConfig,
    PartialTaskConfig, ToolchainConfig, ToolsConfig, WorkspaceConfig, WorkspaceProjects,
};
use moon_project_graph::{
    DetectLanguageEvent, DetectPlatformEvent, ExtendProjectEvent, ExtendProjectGraphEvent,
    ProjectGraph, ProjectGraphBuilder, ProjectGraphBuilderContext,
};
use moon_vcs::{BoxedVcs, Git};
use starbase_events::Emitter;
use starbase_sandbox::create_sandbox;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct ProjectGraphContainer {
    pub inherited_tasks: InheritedTasksManager,
    pub toolchain_config: ToolchainConfig,
    pub workspace_config: WorkspaceConfig,
    pub workspace_root: PathBuf,
    pub vcs: Option<BoxedVcs>,
}

impl ProjectGraphContainer {
    pub fn new(root: &Path) -> Self {
        let proto = ToolsConfig::default();
        let mut graph = Self {
            inherited_tasks: InheritedTasksManager::load_from(root).unwrap(),
            toolchain_config: ToolchainConfig::load_from(root, &proto).unwrap(),
            workspace_root: root.to_path_buf(),
            ..Default::default()
        };

        // Add a global task to all projects
        graph.inherited_tasks.configs.insert(
            "*".into(),
            InheritedTasksEntry {
                input: ".moon/tasks.yml".into(),
                config: PartialInheritedTasksConfig {
                    tasks: Some(BTreeMap::from_iter([(
                        "global".into(),
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
        graph.workspace_config.projects = WorkspaceProjects::Globs(vec!["*".into()]);

        graph
    }

    pub fn with_vcs(root: &Path) -> Self {
        let mut container = Self::new(root);
        container.vcs = Some(Box::new(Git::load(root, "master", &[]).unwrap()));
        container
    }

    pub fn create_context(&self) -> ProjectGraphBuilderContext {
        ProjectGraphBuilderContext {
            extend_project: Emitter::<ExtendProjectEvent>::new(),
            extend_project_graph: Emitter::<ExtendProjectGraphEvent>::new(),
            detect_language: Emitter::<DetectLanguageEvent>::new(),
            detect_platform: Emitter::<DetectPlatformEvent>::new(),
            inherited_tasks: &self.inherited_tasks,
            toolchain_config: &self.toolchain_config,
            vcs: self.vcs.as_ref(),
            working_dir: &self.workspace_root,
            workspace_config: &self.workspace_config,
            workspace_root: &self.workspace_root,
        }
    }

    pub async fn build_graph<'l>(&self, context: ProjectGraphBuilderContext<'l>) -> ProjectGraph {
        let mut builder = ProjectGraphBuilder::new(context).await.unwrap();
        builder.load_all().await.unwrap();

        let mut graph = builder.build().await.unwrap();
        graph.check_boundaries = true;
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

        let mut graph = builder.build().await.unwrap();
        graph.check_boundaries = true;

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
