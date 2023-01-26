use moon::{generate_project_graph, load_workspace_from};
use moon_config::{WorkspaceConfig, WorkspaceProjects};
use moon_project::{ProjectDependency, ProjectDependencySource};
use moon_project_graph::ProjectGraph;
use moon_test_utils::{
    assert_snapshot, create_sandbox_with_config, get_project_graph_aliases_fixture_configs, Sandbox,
};
use moon_utils::string_vec;
use rustc_hash::FxHashMap;

async fn get_aliases_graph() -> (ProjectGraph, Sandbox) {
    let (workspace_config, toolchain_config, projects_config) =
        get_project_graph_aliases_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "project-graph/aliases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let graph = generate_project_graph(&mut workspace).await.unwrap();

    (graph, sandbox)
}

async fn get_dependencies_graph(enable_git: bool) -> (ProjectGraph, Sandbox) {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("a".to_owned(), "a".to_owned()),
            ("b".to_owned(), "b".to_owned()),
            ("c".to_owned(), "c".to_owned()),
            ("d".to_owned(), "d".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };

    let sandbox = create_sandbox_with_config(
        "project-graph/dependencies",
        Some(&workspace_config),
        None,
        None,
    );

    if enable_git {
        sandbox.enable_git();
    }

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let graph = generate_project_graph(&mut workspace).await.unwrap();

    (graph, sandbox)
}

async fn get_dependents_graph() -> (ProjectGraph, Sandbox) {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("a".to_owned(), "a".to_owned()),
            ("b".to_owned(), "b".to_owned()),
            ("c".to_owned(), "c".to_owned()),
            ("d".to_owned(), "d".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };

    let sandbox = create_sandbox_with_config(
        "project-graph/dependents",
        Some(&workspace_config),
        None,
        None,
    );

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let graph = generate_project_graph(&mut workspace).await.unwrap();

    (graph, sandbox)
}

#[tokio::test]
async fn can_use_map_and_globs_setting() {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Both {
            globs: string_vec!["deps/*"],
            sources: FxHashMap::from_iter([
                ("basic".to_owned(), "basic".to_owned()),
                ("noConfig".to_owned(), "no-config".to_owned()),
            ]),
        },
        ..WorkspaceConfig::default()
    };

    let sandbox = create_sandbox_with_config("projects", Some(&workspace_config), None, None);

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let graph = generate_project_graph(&mut workspace).await.unwrap();

    assert_eq!(
        graph.sources,
        FxHashMap::from_iter([
            ("noConfig".to_owned(), "no-config".to_owned()),
            ("bar".to_owned(), "deps/bar".to_owned()),
            ("basic".to_owned(), "basic".to_owned()),
            ("baz".to_owned(), "deps/baz".to_owned()),
            ("foo".to_owned(), "deps/foo".to_owned()),
        ])
    );
}

mod caching {
    use super::*;
    use moon_cache::ProjectsState;

    #[tokio::test]
    async fn caches_and_hashes_projects_state() {
        let (_, sandbox) = get_dependencies_graph(true).await;
        let state_path = sandbox.path().join(".moon/cache/states/projects.json");
        let graph_path = sandbox.path().join(".moon/cache/states/projectGraph.json");

        assert!(state_path.exists());
        assert!(graph_path.exists());

        let state = ProjectsState::load(state_path).unwrap();

        assert_eq!(state.globs, string_vec![]);
        assert_eq!(state.last_glob_time, 0);
        assert_eq!(
            state.last_hash,
            "f142a8f9dc18b06a35a5c01a78b8e62ca36d23fd306530631ebab648e481fe35"
        );
        assert_eq!(
            state.projects,
            FxHashMap::from_iter([
                ("a".to_string(), "a".to_string()),
                ("b".to_string(), "b".to_string()),
                ("c".to_string(), "c".to_string()),
                ("d".to_string(), "d".to_string()),
            ])
        );

        assert!(sandbox
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{}.json", state.last_hash))
            .exists());
    }

    #[tokio::test]
    async fn doesnt_cache_if_no_vcs() {
        let (_, sandbox) = get_dependencies_graph(false).await;
        sandbox.debug_files();
        let state_path = sandbox.path().join(".moon/cache/states/projects.json");
        let graph_path = sandbox.path().join(".moon/cache/states/projectGraph.json");

        assert!(state_path.exists());
        assert!(!graph_path.exists());

        let state = ProjectsState::load(state_path).unwrap();

        assert_eq!(state.last_hash, "");
    }
}

mod globs {
    use super::*;

    #[tokio::test]
    async fn ignores_dot_folders() {
        let workspace_config = WorkspaceConfig {
            projects: WorkspaceProjects::Globs(string_vec!["*"]),
            ..WorkspaceConfig::default()
        };

        // Use git so we can test against the .git folder
        let sandbox =
            create_sandbox_with_config("project-graph/langs", Some(&workspace_config), None, None);
        sandbox.enable_git();
        sandbox.create_file(".foo/moon.yml", "{}");
        sandbox.create_file("node_modules/moon/package.json", "{}");

        let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
        let graph = generate_project_graph(&mut workspace).await.unwrap();

        assert!(!graph.sources.contains_key(".foo"));
        assert!(!graph.sources.contains_key(".git"));
        assert!(!graph.sources.contains_key("node_modules"));
    }

    #[tokio::test]
    async fn supports_all_id_formats() {
        let workspace_config = WorkspaceConfig {
            projects: WorkspaceProjects::Globs(string_vec!["*"]),
            ..WorkspaceConfig::default()
        };

        let sandbox =
            create_sandbox_with_config("project-graph/ids", Some(&workspace_config), None, None);

        let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
        let graph = generate_project_graph(&mut workspace).await.unwrap();

        assert_eq!(
            graph.sources,
            FxHashMap::from_iter([
                ("camelCase".to_owned(), "camelCase".to_owned()),
                ("Capital".to_owned(), "Capital".to_owned()),
                ("kebab-case".to_owned(), "kebab-case".to_owned()),
                ("PascalCase".to_owned(), "PascalCase".to_owned()),
                ("snake_case".to_owned(), "snake_case".to_owned()),
                ("With_nums-123".to_owned(), "With_nums-123".to_owned())
            ])
        );
    }
}

mod get_dependencies_of {
    use super::*;

    #[tokio::test]
    async fn returns_dep_list() {
        let (graph, _sandbox) = get_dependencies_graph(false).await;

        let a = graph.get("a").unwrap();
        let b = graph.get("b").unwrap();
        let c = graph.get("c").unwrap();
        let d = graph.get("d").unwrap();

        assert_eq!(graph.get_dependencies_of(a).unwrap(), string_vec!["b"]);
        assert_eq!(graph.get_dependencies_of(b).unwrap(), string_vec!["c"]);
        assert_eq!(graph.get_dependencies_of(c).unwrap(), string_vec![]);
        assert_eq!(
            graph.get_dependencies_of(d).unwrap(),
            string_vec!["a", "b", "c"]
        );
    }
}

mod get_dependents_of {
    use super::*;

    #[tokio::test]
    async fn returns_dep_list() {
        let (graph, _sandbox) = get_dependents_graph().await;

        let a = graph.get("a").unwrap();
        let b = graph.get("b").unwrap();
        let c = graph.get("c").unwrap();
        let d = graph.get("d").unwrap();

        assert_eq!(graph.get_dependents_of(a).unwrap(), string_vec![]);
        assert_eq!(graph.get_dependents_of(b).unwrap(), string_vec!["a"]);
        assert_eq!(graph.get_dependents_of(c).unwrap(), string_vec!["b"]);
        assert_eq!(
            graph.get_dependents_of(d).unwrap(),
            string_vec!["a", "b", "c"]
        );
    }
}

mod to_dot {
    use super::*;
    use moon::build_project_graph;

    #[tokio::test]
    async fn renders_tree() {
        let (graph, _sandbox) = get_dependencies_graph(false).await;

        assert_snapshot!(graph.to_dot());
    }

    #[tokio::test]
    async fn renders_partial_tree() {
        let workspace_config = WorkspaceConfig {
            projects: WorkspaceProjects::Sources(FxHashMap::from_iter([
                ("a".to_owned(), "a".to_owned()),
                ("b".to_owned(), "b".to_owned()),
                ("c".to_owned(), "c".to_owned()),
                ("d".to_owned(), "d".to_owned()),
            ])),
            ..WorkspaceConfig::default()
        };

        let sandbox = create_sandbox_with_config(
            "project-graph/dependencies",
            Some(&workspace_config),
            None,
            None,
        );

        let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
        let mut graph = build_project_graph(&mut workspace).await.unwrap();

        graph.load("b").unwrap();

        let graph = graph.build();

        assert_snapshot!(graph.to_dot());
    }
}

mod implicit_explicit_deps {
    use super::*;

    #[tokio::test]
    async fn loads_implicit() {
        let (graph, _sandbox) = get_aliases_graph().await;

        let project = graph.get("implicit").unwrap();

        assert_eq!(
            project.dependencies,
            FxHashMap::from_iter([
                (
                    "nodeNameScope".to_string(),
                    ProjectDependency {
                        id: "nodeNameScope".into(),
                        scope: moon_config::DependencyScope::Development,
                        source: ProjectDependencySource::Implicit,
                        via: Some("@scope/pkg-foo".to_string())
                    }
                ),
                (
                    "node".to_string(),
                    ProjectDependency {
                        id: "node".into(),
                        scope: moon_config::DependencyScope::Production,
                        source: ProjectDependencySource::Implicit,
                        via: Some("project-graph-aliases-node".to_string())
                    }
                )
            ])
        );
    }

    #[tokio::test]
    async fn loads_explicit() {
        let (graph, _sandbox) = get_aliases_graph().await;

        let project = graph.get("explicit").unwrap();

        assert_eq!(
            project.dependencies,
            FxHashMap::from_iter([
                (
                    "nodeNameScope".to_string(),
                    ProjectDependency {
                        id: "nodeNameScope".into(),
                        scope: moon_config::DependencyScope::Production,
                        source: ProjectDependencySource::Explicit,
                        via: None
                    }
                ),
                (
                    "node".to_string(),
                    ProjectDependency {
                        id: "node".into(),
                        scope: moon_config::DependencyScope::Development,
                        source: ProjectDependencySource::Explicit,
                        via: None
                    }
                )
            ])
        );
    }

    #[tokio::test]
    async fn loads_explicit_and_implicit() {
        let (graph, _sandbox) = get_aliases_graph().await;

        let project = graph.get("explicitAndImplicit").unwrap();

        assert_eq!(
            project.dependencies,
            FxHashMap::from_iter([
                (
                    "nodeNameScope".to_string(),
                    ProjectDependency {
                        id: "nodeNameScope".into(),
                        scope: moon_config::DependencyScope::Production,
                        source: ProjectDependencySource::Explicit,
                        via: None
                    }
                ),
                (
                    "node".to_string(),
                    ProjectDependency {
                        id: "node".into(),
                        scope: moon_config::DependencyScope::Development,
                        source: ProjectDependencySource::Explicit,
                        via: None
                    }
                ),
                (
                    "nodeNameOnly".to_string(),
                    ProjectDependency {
                        id: "nodeNameOnly".into(),
                        scope: moon_config::DependencyScope::Peer,
                        source: ProjectDependencySource::Implicit,
                        via: Some("pkg-bar".to_string())
                    }
                )
            ])
        );
    }
}
