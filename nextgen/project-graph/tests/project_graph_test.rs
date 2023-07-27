use moon_common::Id;
use moon_config::PartialTaskConfig;
use moon_config::{
    DependencyConfig, DependencyScope, DependencySource, InheritedTasksManager, NodeConfig,
    PartialInheritedTasksConfig, ToolchainConfig, WorkspaceConfig, WorkspaceProjects,
};
use moon_project_builder::DetectLanguageEvent;
use moon_project_graph2::{
    ExtendProjectData, ExtendProjectEvent, ExtendProjectGraphData, ExtendProjectGraphEvent,
    ProjectGraph, ProjectGraphBuilder, ProjectGraphBuilderContext,
};
use moon_target::Target;
use moon_task_builder::DetectPlatformEvent;
use rustc_hash::FxHashMap;
use starbase_events::{Emitter, EventState};
use starbase_sandbox::{assert_snapshot, create_sandbox};
use starbase_utils::fs;
use starbase_utils::string_vec;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Default)]
struct GraphContainer {
    pub inherited_tasks: InheritedTasksManager,
    pub toolchain_config: ToolchainConfig,
    pub workspace_config: WorkspaceConfig,
    pub workspace_root: PathBuf,
}

impl GraphContainer {
    pub fn new(root: &Path) -> Self {
        let mut graph = Self {
            workspace_root: root.to_path_buf(),
            ..Default::default()
        };

        // Add a noop tasks to all projects
        graph.inherited_tasks.configs.insert(
            "*".into(),
            PartialInheritedTasksConfig {
                tasks: Some(BTreeMap::from_iter([(
                    "noop".into(),
                    PartialTaskConfig::default(),
                )])),
                ..PartialInheritedTasksConfig::default()
            },
        );

        // Always use the node platform
        graph.toolchain_config.node = Some(NodeConfig::default());

        // Use folders as project names
        graph.workspace_config.projects = WorkspaceProjects::Globs(string_vec!["*"]);

        graph
    }

    pub fn create_context(&self) -> ProjectGraphBuilderContext {
        ProjectGraphBuilderContext {
            extend_project: Emitter::<ExtendProjectEvent>::new(),
            extend_project_graph: Emitter::<ExtendProjectGraphEvent>::new(),
            detect_language: Emitter::<DetectLanguageEvent>::new(),
            detect_platform: Emitter::<DetectPlatformEvent>::new(),
            inherited_tasks: &self.inherited_tasks,
            toolchain_config: &self.toolchain_config,
            vcs: None,
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
}

async fn generate_project_graph(fixture: &str) -> ProjectGraph {
    let sandbox = create_sandbox(fixture);
    let container = GraphContainer::new(sandbox.path());
    let context = container.create_context();

    container.build_graph(context).await
}

fn map_ids(ids: Vec<&Id>) -> Vec<String> {
    ids.iter().map(|id| id.to_string()).collect()
}

mod project_graph {
    use super::*;

    mod dependencies {
        use super::*;

        #[tokio::test]
        async fn explicit_depends_on() {
            let graph = generate_project_graph("dependencies").await;

            assert_snapshot!(graph.to_dot());
        }

        #[tokio::test]
        async fn lists_ids_of_dependencies() {
            let graph = generate_project_graph("dependencies").await;

            assert_eq!(
                map_ids(graph.dependencies_of(&graph.get("a").unwrap()).unwrap()),
                ["b"]
            );
            assert_eq!(
                map_ids(graph.dependencies_of(&graph.get("b").unwrap()).unwrap()),
                ["c"]
            );
            assert_eq!(
                map_ids(graph.dependencies_of(&graph.get("c").unwrap()).unwrap()),
                string_vec![]
            );
            assert_eq!(
                map_ids(graph.dependencies_of(&graph.get("d").unwrap()).unwrap()),
                ["c", "b", "a"]
            );
        }

        #[tokio::test]
        async fn lists_ids_of_dependents() {
            let graph = generate_project_graph("dependencies").await;

            assert_eq!(
                map_ids(graph.dependents_of(&graph.get("a").unwrap()).unwrap()),
                ["d"]
            );
            assert_eq!(
                map_ids(graph.dependents_of(&graph.get("b").unwrap()).unwrap()),
                ["d", "a"]
            );
            assert_eq!(
                map_ids(graph.dependents_of(&graph.get("c").unwrap()).unwrap()),
                ["d", "b"]
            );
            assert_eq!(
                map_ids(graph.dependents_of(&graph.get("d").unwrap()).unwrap()),
                string_vec![]
            );
        }
    }

    mod aliases {
        use super::*;

        async fn generate_aliases_project_graph() -> ProjectGraph {
            let sandbox = create_sandbox("aliases");
            let container = GraphContainer::new(sandbox.path());
            let context = container.create_context();

            // Set aliases for projects
            context
                .extend_project_graph
                .on(
                    |event: Arc<ExtendProjectGraphEvent>,
                     data: Arc<RwLock<ExtendProjectGraphData>>| async move {
                        let mut data = data.write().await;

                        for (id, source) in &event.sources {
                            let alias_path = source.join("alias").to_path(&event.workspace_root);

                            if alias_path.exists() {
                                data.aliases.insert(
                                    fs::read_file(alias_path).unwrap().trim().to_owned(),
                                    id.to_owned(),
                                );
                            }
                        }

                        Ok(EventState::Continue)
                    },
                )
                .await;

            // Set implicit deps for projects
            context
                .extend_project
                .on(
                    |event: Arc<ExtendProjectEvent>,
                     data: Arc<RwLock<ExtendProjectData>>| async move {
                        let mut data = data.write().await;

                        if event.project_id == "explicit-and-implicit" || event.project_id == "implicit" {
                            data.dependencies.push(DependencyConfig {
                                id: "@three".into(),
                                scope: DependencyScope::Build,
                                ..Default::default()
                            });
                        }

                        if event.project_id == "implicit" {
                            data.dependencies.push(DependencyConfig {
                                id: "@one".into(),
                                scope: DependencyScope::Peer,
                                ..Default::default()
                            });
                        }

                        Ok(EventState::Continue)
                    },
                )
                .await;

            container.build_graph(context).await
        }

        #[tokio::test]
        async fn loads_aliases() {
            let graph = generate_aliases_project_graph().await;

            assert_snapshot!(graph.to_dot());

            assert_eq!(
                graph
                    .nodes
                    .into_iter()
                    .map(|(id, node)| (id, node.alias))
                    .collect::<FxHashMap<_, _>>(),
                FxHashMap::from_iter([
                    ("alias-one".into(), Some("@one".to_owned())),
                    ("alias-two".into(), Some("@two".to_owned())),
                    ("alias-three".into(), Some("@three".to_owned())),
                    ("dupes-depends-on".into(), None),
                    ("dupes-task-deps".into(), None),
                    ("explicit".into(), None),
                    ("explicit-and-implicit".into(), None),
                    ("implicit".into(), None),
                    ("tasks".into(), None),
                ])
            );
        }

        #[tokio::test]
        async fn can_get_projects_by_alias() {
            let graph = generate_aliases_project_graph().await;

            assert!(graph.get("@one").is_ok());
            assert!(graph.get("@two").is_ok());
            assert!(graph.get("@three").is_ok());

            assert_eq!(graph.get("@one").unwrap(), graph.get("alias-one").unwrap());
            assert_eq!(graph.get("@two").unwrap(), graph.get("alias-two").unwrap());
            assert_eq!(
                graph.get("@three").unwrap(),
                graph.get("alias-three").unwrap()
            );
        }

        #[tokio::test]
        async fn can_depends_on_by_alias() {
            let graph = generate_aliases_project_graph().await;

            assert_eq!(
                map_ids(
                    graph
                        .dependencies_of(&graph.get("explicit").unwrap())
                        .unwrap()
                ),
                ["alias-one", "alias-two"]
            );

            assert_eq!(
                map_ids(
                    graph
                        .dependencies_of(&graph.get("explicit-and-implicit").unwrap())
                        .unwrap()
                ),
                ["alias-three", "alias-two"]
            );

            assert_eq!(
                map_ids(
                    graph
                        .dependencies_of(&graph.get("implicit").unwrap())
                        .unwrap()
                ),
                ["alias-three", "alias-one"]
            );
        }

        #[tokio::test]
        async fn removes_or_flattens_dupes() {
            let graph = generate_aliases_project_graph().await;

            assert_eq!(
                graph
                    .get("dupes-depends-on")
                    .unwrap()
                    .dependencies
                    .values()
                    .map(|c| c.to_owned())
                    .collect::<Vec<_>>(),
                [DependencyConfig {
                    id: "alias-two".into(),
                    scope: DependencyScope::Build,
                    source: Some(DependencySource::Explicit),
                    ..DependencyConfig::default()
                }]
            );

            assert_eq!(
                graph
                    .get("dupes-task-deps")
                    .unwrap()
                    .get_task("no-dupes")
                    .unwrap()
                    .deps,
                [Target::parse("alias-one:noop").unwrap()]
            );
        }

        #[tokio::test]
        async fn can_use_aliases_as_task_deps() {
            let graph = generate_aliases_project_graph().await;

            assert_eq!(
                graph
                    .get("tasks")
                    .unwrap()
                    .get_task("with-aliases")
                    .unwrap()
                    .deps,
                [
                    Target::parse("alias-one:noop").unwrap(),
                    Target::parse("alias-three:noop").unwrap(),
                    Target::parse("implicit:noop").unwrap(),
                ]
            );
        }
    }
}
