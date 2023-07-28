use moon_common::Id;
use moon_config::PartialTaskConfig;
use moon_config::{
    DependencyConfig, DependencyScope, DependencySource, InheritedTasksManager, NodeConfig,
    PartialInheritedTasksConfig, ToolchainConfig, WorkspaceConfig, WorkspaceProjects,
    WorkspaceProjectsConfig,
};
use moon_project::Project;
use moon_project_builder::DetectLanguageEvent;
use moon_project_graph2::{
    ExtendProjectData, ExtendProjectEvent, ExtendProjectGraphData, ExtendProjectGraphEvent,
    ProjectGraph, ProjectGraphBuilder, ProjectGraphBuilderContext,
};
use moon_query::build_query;
use moon_target::Target;
use moon_task_builder::DetectPlatformEvent;
use moon_vcs::{BoxedVcs, Git};
use rustc_hash::FxHashMap;
use starbase_events::{Emitter, EventState};
use starbase_sandbox::{assert_snapshot, create_sandbox, Sandbox};
use starbase_utils::fs;
use starbase_utils::string_vec;
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Default)]
struct GraphContainer {
    pub inherited_tasks: InheritedTasksManager,
    pub toolchain_config: ToolchainConfig,
    pub workspace_config: WorkspaceConfig,
    pub workspace_root: PathBuf,
    pub vcs: Option<BoxedVcs>,
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

    pub fn new_with_vcs(root: &Path) -> Self {
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

pub fn append_file<P: AsRef<Path>>(path: P, data: &str) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(path.as_ref())
        .unwrap();

    writeln!(file, "\n\n{data}").unwrap();
}

fn map_ids(ids: Vec<&Id>) -> Vec<String> {
    ids.iter().map(|id| id.to_string()).collect()
}

fn get_ids_from_projects(projects: Vec<Arc<Project>>) -> Vec<String> {
    let mut ids = projects
        .iter()
        .map(|p| p.id.to_string())
        .collect::<Vec<_>>();
    ids.sort();
    ids
}

mod project_graph {
    use super::*;

    async fn generate_project_graph(fixture: &str) -> ProjectGraph {
        let sandbox = create_sandbox(fixture);

        generate_project_graph_from_sandbox(sandbox.path()).await
    }

    async fn generate_project_graph_from_sandbox(path: &Path) -> ProjectGraph {
        let container = GraphContainer::new(path);
        let context = container.create_context();

        container.build_graph(context).await
    }

    // TODO
    // get by path

    mod sources {
        use super::*;

        #[tokio::test]
        async fn globs() {
            let graph = generate_project_graph("dependencies").await;

            assert_eq!(
                get_ids_from_projects(graph.get_all().unwrap()),
                ["a", "b", "c", "d"]
            );
        }

        #[tokio::test]
        async fn globs_with_root() {
            let sandbox = create_sandbox("dependencies");
            let root = sandbox.path().join("dir");

            // Move files so that we can infer a compatible root project name
            fs::copy_dir_all(sandbox.path(), sandbox.path(), &root).unwrap();

            let mut container = GraphContainer::new(&root);

            container.workspace_config.projects = WorkspaceProjects::Globs(string_vec!["*", "."]);

            let context = container.create_context();
            let graph = container.build_graph(context).await;

            assert_eq!(
                get_ids_from_projects(graph.get_all().unwrap()),
                ["a", "b", "c", "d", "dir"]
            );
        }

        #[tokio::test]
        async fn paths() {
            let sandbox = create_sandbox("dependencies");
            let mut container = GraphContainer::new(sandbox.path());

            container.workspace_config.projects =
                WorkspaceProjects::Sources(FxHashMap::from_iter([
                    ("c".into(), "c".into()),
                    ("b".into(), "b".into()),
                ]));

            let context = container.create_context();
            let graph = container.build_graph(context).await;

            assert_eq!(get_ids_from_projects(graph.get_all().unwrap()), ["b", "c"]);
        }

        #[tokio::test]
        async fn paths_and_globs() {
            let sandbox = create_sandbox("dependencies");
            let mut container = GraphContainer::new(sandbox.path());

            container.workspace_config.projects =
                WorkspaceProjects::Both(WorkspaceProjectsConfig {
                    globs: string_vec!["{a,c}"],
                    sources: FxHashMap::from_iter([
                        ("b".into(), "b".into()),
                        ("root".into(), ".".into()),
                    ]),
                });

            let context = container.create_context();
            let graph = container.build_graph(context).await;

            assert_eq!(
                get_ids_from_projects(graph.get_all().unwrap()),
                ["a", "b", "c", "root"]
            );
        }

        #[tokio::test]
        async fn ignores_git_moon_folders() {
            let sandbox = create_sandbox("dependencies");

            sandbox.enable_git();
            sandbox.create_file(".moon/workspace.yml", "");

            let graph = generate_project_graph_from_sandbox(sandbox.path()).await;

            assert_eq!(
                get_ids_from_projects(graph.get_all().unwrap()),
                ["a", "b", "c", "d"]
            );
        }

        #[tokio::test]
        #[should_panic(expected = "Invalid format for .foo")]
        async fn errors_for_dot_folders() {
            let sandbox = create_sandbox("dependencies");
            sandbox.create_file(".foo/moon.yml", "");

            let graph = generate_project_graph_from_sandbox(sandbox.path()).await;

            assert_eq!(
                get_ids_from_projects(graph.get_all().unwrap()),
                ["a", "b", "c", "d"]
            );
        }

        #[tokio::test]
        async fn filters_using_gitignore() {
            let sandbox = create_sandbox("type-constraints");

            sandbox.enable_git();
            sandbox.create_file(".gitignore", "*-other");

            let container = GraphContainer::new_with_vcs(sandbox.path());
            let context = container.create_context();
            let graph = container.build_graph(context).await;

            assert_eq!(
                get_ids_from_projects(graph.get_all().unwrap()),
                ["app", "library", "tool", "unknown"]
            );
        }

        #[tokio::test]
        async fn supports_id_formats() {
            let graph = generate_project_graph("ids").await;

            assert_eq!(
                get_ids_from_projects(graph.get_all().unwrap()),
                [
                    "Capital",
                    "PascalCase",
                    "With_nums-123",
                    "camelCase",
                    "kebab-case",
                    "snake_case"
                ]
            );
        }
    }

    mod cache {
        use super::*;

        // TODO
    }

    mod cycles {
        use super::*;

        // TODO
    }

    mod inheritance {
        use super::*;

        // TODO
    }

    mod expansion {
        use super::*;

        // TODO
    }

    mod dependencies {
        use super::*;

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

    mod type_constraints {
        use super::*;

        async fn generate_type_constraints_project_graph(
            func: impl FnOnce(&Sandbox),
        ) -> ProjectGraph {
            let sandbox = create_sandbox("type-constraints");

            func(&sandbox);

            let mut container = GraphContainer::new(sandbox.path());

            container
                .workspace_config
                .constraints
                .enforce_project_type_relationships = true;

            let context = container.create_context();

            container.build_graph(context).await
        }

        #[tokio::test]
        async fn app_can_use_unknown() {
            generate_type_constraints_project_graph(|sandbox| {
                append_file(sandbox.path().join("app/moon.yml"), "dependsOn: [unknown]");
            })
            .await;
        }

        #[tokio::test]
        async fn app_can_use_library() {
            generate_type_constraints_project_graph(|sandbox| {
                append_file(sandbox.path().join("app/moon.yml"), "dependsOn: [library]");
            })
            .await;
        }

        #[tokio::test]
        async fn app_can_use_tool() {
            generate_type_constraints_project_graph(|sandbox| {
                append_file(sandbox.path().join("app/moon.yml"), "dependsOn: [tool]");
            })
            .await;
        }

        #[tokio::test]
        #[should_panic(
            expected = "Invalid project relationship. Project app of type application cannot"
        )]
        async fn app_cannot_use_app() {
            generate_type_constraints_project_graph(|sandbox| {
                append_file(
                    sandbox.path().join("app/moon.yml"),
                    "dependsOn: [app-other]",
                );
            })
            .await;
        }

        #[tokio::test]
        async fn library_can_use_unknown() {
            generate_type_constraints_project_graph(|sandbox| {
                append_file(
                    sandbox.path().join("library/moon.yml"),
                    "dependsOn: [unknown]",
                );
            })
            .await;
        }

        #[tokio::test]
        async fn library_can_use_library() {
            generate_type_constraints_project_graph(|sandbox| {
                append_file(
                    sandbox.path().join("library/moon.yml"),
                    "dependsOn: [library-other]",
                );
            })
            .await;
        }

        #[tokio::test]
        #[should_panic(
            expected = "Invalid project relationship. Project library of type library cannot"
        )]
        async fn library_cannot_use_app() {
            generate_type_constraints_project_graph(|sandbox| {
                append_file(sandbox.path().join("library/moon.yml"), "dependsOn: [app]");
            })
            .await;
        }

        #[tokio::test]
        #[should_panic(
            expected = "Invalid project relationship. Project library of type library cannot"
        )]
        async fn library_cannot_use_tool() {
            generate_type_constraints_project_graph(|sandbox| {
                append_file(sandbox.path().join("library/moon.yml"), "dependsOn: [tool]");
            })
            .await;
        }

        #[tokio::test]
        async fn tool_can_use_unknown() {
            generate_type_constraints_project_graph(|sandbox| {
                append_file(sandbox.path().join("tool/moon.yml"), "dependsOn: [unknown]");
            })
            .await;
        }

        #[tokio::test]
        async fn tool_can_use_library() {
            generate_type_constraints_project_graph(|sandbox| {
                append_file(sandbox.path().join("tool/moon.yml"), "dependsOn: [library]");
            })
            .await;
        }

        #[tokio::test]
        #[should_panic(expected = "Invalid project relationship. Project tool of type tool cannot")]
        async fn tool_cannot_use_app() {
            generate_type_constraints_project_graph(|sandbox| {
                append_file(sandbox.path().join("tool/moon.yml"), "dependsOn: [app]");
            })
            .await;
        }

        #[tokio::test]
        #[should_panic(expected = "Invalid project relationship. Project tool of type tool cannot")]
        async fn tool_cannot_use_tool() {
            generate_type_constraints_project_graph(|sandbox| {
                append_file(
                    sandbox.path().join("tool/moon.yml"),
                    "dependsOn: [tool-other]",
                );
            })
            .await;
        }
    }

    mod tag_constraints {
        use super::*;

        async fn generate_tag_constraints_project_graph(
            func: impl FnOnce(&Sandbox),
        ) -> ProjectGraph {
            let sandbox = create_sandbox("tag-constraints");

            func(&sandbox);

            let mut container = GraphContainer::new(sandbox.path());

            container
                .workspace_config
                .constraints
                .tag_relationships
                .insert(
                    "warrior".into(),
                    vec![Id::raw("barbarian"), Id::raw("paladin"), Id::raw("druid")],
                );

            container
                .workspace_config
                .constraints
                .tag_relationships
                .insert(
                    "mage".into(),
                    vec![Id::raw("wizard"), Id::raw("sorcerer"), Id::raw("druid")],
                );

            let context = container.create_context();

            container.build_graph(context).await
        }

        #[tokio::test]
        async fn can_depon_tags_but_self_empty() {
            generate_tag_constraints_project_graph(|sandbox| {
                append_file(sandbox.path().join("a/moon.yml"), "dependsOn: [b, c]");
                append_file(sandbox.path().join("b/moon.yml"), "tags: [barbarian]");
                append_file(sandbox.path().join("c/moon.yml"), "tags: [druid]");
            })
            .await;
        }

        #[tokio::test]
        async fn ignores_unconfigured_relationships() {
            generate_tag_constraints_project_graph(|sandbox| {
                append_file(sandbox.path().join("a/moon.yml"), "dependsOn: [b, c]");
                append_file(sandbox.path().join("b/moon.yml"), "tags: [some]");
                append_file(sandbox.path().join("c/moon.yml"), "tags: [value]");
            })
            .await;
        }

        #[tokio::test]
        async fn matches_with_source_tag() {
            generate_tag_constraints_project_graph(|sandbox| {
                append_file(
                    sandbox.path().join("a/moon.yml"),
                    "dependsOn: [b]\ntags: [warrior]",
                );
                append_file(sandbox.path().join("b/moon.yml"), "tags: [warrior]");
            })
            .await;
        }

        #[tokio::test]
        #[should_panic(expected = "Invalid tag relationship. Project a with tag #warrior cannot")]
        async fn errors_for_no_source_tag_match() {
            generate_tag_constraints_project_graph(|sandbox| {
                append_file(
                    sandbox.path().join("a/moon.yml"),
                    "dependsOn: [b]\ntags: [warrior]",
                );
                append_file(sandbox.path().join("b/moon.yml"), "tags: [other]");
            })
            .await;
        }

        #[tokio::test]
        async fn matches_with_allowed_tag() {
            generate_tag_constraints_project_graph(|sandbox| {
                append_file(
                    sandbox.path().join("a/moon.yml"),
                    "dependsOn: [b]\ntags: [warrior]",
                );
                append_file(sandbox.path().join("b/moon.yml"), "tags: [barbarian]");
            })
            .await;
        }

        #[tokio::test]
        #[should_panic(expected = "Invalid tag relationship. Project a with tag #warrior cannot")]
        async fn errors_for_no_allowed_tag_match() {
            generate_tag_constraints_project_graph(|sandbox| {
                append_file(
                    sandbox.path().join("a/moon.yml"),
                    "dependsOn: [b]\ntags: [warrior]",
                );
                append_file(sandbox.path().join("b/moon.yml"), "tags: [other]");
            })
            .await;
        }

        #[tokio::test]
        #[should_panic(expected = "Invalid tag relationship. Project a with tag #mage cannot")]
        async fn errors_for_depon_empty_tags() {
            generate_tag_constraints_project_graph(|sandbox| {
                append_file(
                    sandbox.path().join("a/moon.yml"),
                    "dependsOn: [b]\ntags: [mage]",
                );
            })
            .await;
        }

        #[tokio::test]
        async fn matches_multiple_source_tags_to_a_single_allowed_tag() {
            generate_tag_constraints_project_graph(|sandbox| {
                append_file(
                    sandbox.path().join("a/moon.yml"),
                    "dependsOn: [b]\ntags: [warrior, mage]",
                );
                append_file(sandbox.path().join("b/moon.yml"), "tags: [druid]");
            })
            .await;
        }

        #[tokio::test]
        async fn matches_single_source_tag_to_a_multiple_allowed_tags() {
            generate_tag_constraints_project_graph(|sandbox| {
                append_file(
                    sandbox.path().join("a/moon.yml"),
                    "dependsOn: [b, c]\ntags: [mage]",
                );
                append_file(sandbox.path().join("b/moon.yml"), "tags: [druid, wizard]");
                append_file(
                    sandbox.path().join("c/moon.yml"),
                    "tags: [wizard, sorcerer, barbarian]",
                );
            })
            .await;
        }
    }

    mod query {
        use super::*;

        #[tokio::test]
        async fn by_language() {
            let graph = generate_project_graph("query").await;

            let projects = graph
                .query(build_query("language!=[typescript,python]").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["a", "d"]);
        }

        #[tokio::test]
        async fn by_project() {
            let graph = generate_project_graph("query").await;

            let projects = graph.query(build_query("project~{b,d}").unwrap()).unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["b", "d"]);
        }

        #[tokio::test]
        async fn by_project_type() {
            let graph = generate_project_graph("query").await;

            let projects = graph
                .query(build_query("projectType!=[library]").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["a", "c"]);
        }

        #[tokio::test]
        async fn by_project_source() {
            let graph = generate_project_graph("query").await;

            let projects = graph
                .query(build_query("projectSource~a").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["a"]);
        }

        #[tokio::test]
        async fn by_tag() {
            let graph = generate_project_graph("query").await;

            let projects = graph
                .query(build_query("tag=[three,five]").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["b", "c"]);
        }

        #[tokio::test]
        async fn by_task() {
            let graph = generate_project_graph("query").await;

            let projects = graph
                .query(build_query("task=[test,build]").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["a", "c", "d"]);
        }

        #[tokio::test]
        async fn by_task_platform() {
            let graph = generate_project_graph("query").await;

            let projects = graph
                .query(build_query("taskPlatform=[node]").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["a", "b"]);

            let projects = graph
                .query(build_query("taskPlatform=system").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["c", "d"]);
        }

        #[tokio::test]
        async fn by_task_type() {
            let graph = generate_project_graph("query").await;

            let projects = graph.query(build_query("taskType=run").unwrap()).unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["a"]);
        }

        #[tokio::test]
        async fn with_and_conditions() {
            let graph = generate_project_graph("query").await;

            let projects = graph
                .query(build_query("task=build && taskPlatform=deno").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["d"]);
        }

        #[tokio::test]
        async fn with_or_conditions() {
            let graph = generate_project_graph("query").await;

            let projects = graph
                .query(build_query("language=javascript || language=typescript").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["a", "b"]);
        }

        #[tokio::test]
        async fn with_nested_conditions() {
            let graph = generate_project_graph("query").await;

            let projects = graph
                .query(build_query("projectType=library && (taskType=build || tag=three)").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["b", "d"]);
        }
    }

    mod to_dot {
        use super::*;

        #[tokio::test]
        async fn renders_full() {
            let graph = generate_project_graph("dependencies").await;

            assert_snapshot!(graph.to_dot());
        }

        #[tokio::test]
        async fn renders_partial() {
            let sandbox = create_sandbox("dependencies");
            let container = GraphContainer::new(sandbox.path());
            let context = container.create_context();
            let graph = container.build_graph_for(context, &["b"]).await;

            assert_snapshot!(graph.to_dot());
        }
    }
}