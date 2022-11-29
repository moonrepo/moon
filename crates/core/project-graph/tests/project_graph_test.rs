use moon_cache::CacheEngine;
use moon_config::{GlobalProjectConfig, ToolchainConfig, WorkspaceConfig, WorkspaceProjects};
use moon_node_platform::NodePlatform;
use moon_platform::Platformable;
use moon_project::{ProjectDependency, ProjectDependencySource};
use moon_project_graph::ProjectGraph;
use moon_test_utils::{
    assert_snapshot, create_sandbox_with_config, get_project_graph_aliases_fixture_configs, Sandbox,
};
use moon_utils::string_vec;
use rustc_hash::FxHashMap;
use std::fs;

async fn get_aliases_graph() -> (ProjectGraph, Sandbox) {
    let (workspace_config, toolchain_config, projects_config) =
        get_project_graph_aliases_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "project-graph/aliases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let graph = ProjectGraph::generate(
        sandbox.path(),
        &workspace_config,
        &toolchain_config,
        GlobalProjectConfig::default(),
        &CacheEngine::load(sandbox.path()).await.unwrap(),
    )
    .await
    .unwrap();

    (graph, sandbox)
}

async fn get_dependencies_graph() -> (ProjectGraph, Sandbox) {
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

    let graph = ProjectGraph::generate(
        sandbox.path(),
        &workspace_config,
        &ToolchainConfig::default(),
        GlobalProjectConfig::default(),
        &CacheEngine::load(sandbox.path()).await.unwrap(),
    )
    .await
    .unwrap();

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

    let graph = ProjectGraph::generate(
        sandbox.path(),
        &workspace_config,
        &ToolchainConfig::default(),
        GlobalProjectConfig::default(),
        &CacheEngine::load(sandbox.path()).await.unwrap(),
    )
    .await
    .unwrap();

    (graph, sandbox)
}

#[tokio::test]
async fn can_use_map_and_globs_setting() {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Both {
            globs: string_vec!["deps/*"],
            sources: FxHashMap::from_iter([
                ("basic".to_owned(), "basic".to_owned()),
                ("noConfig".to_owned(), "noConfig".to_owned()),
            ]),
        },
        ..WorkspaceConfig::default()
    };

    let sandbox = create_sandbox_with_config("projects", Some(&workspace_config), None, None);

    let graph = ProjectGraph::generate(
        sandbox.path(),
        &workspace_config,
        &ToolchainConfig::default(),
        GlobalProjectConfig::default(),
        &CacheEngine::load(sandbox.path()).await.unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(
        graph.projects_map,
        FxHashMap::from_iter([
            ("noConfig".to_owned(), "noConfig".to_owned()),
            ("bar".to_owned(), "deps/bar".to_owned()),
            ("basic".to_owned(), "basic".to_owned()),
            ("baz".to_owned(), "deps/baz".to_owned()),
            ("foo".to_owned(), "deps/foo".to_owned()),
        ])
    );
}

mod globs {
    use super::*;

    #[tokio::test]
    async fn ignores_dot_folders() {
        let workspace_config = WorkspaceConfig {
            projects: WorkspaceProjects::Globs(string_vec!["**"]),
            ..WorkspaceConfig::default()
        };

        // Use git so we can test against the .git folder
        let sandbox = create_sandbox_with_config("projects", Some(&workspace_config), None, None);
        sandbox.enable_git();

        // Create fake node modules
        sandbox.create_file("node_modules/moon/package.json", "{}");

        let graph = ProjectGraph::generate(
            sandbox.path(),
            &workspace_config,
            &ToolchainConfig::default(),
            GlobalProjectConfig::default(),
            &CacheEngine::load(sandbox.path()).await.unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(
            graph.projects_map,
            FxHashMap::from_iter([
                ("advanced".to_owned(), "advanced".to_owned()),
                ("bar".to_owned(), "deps/bar".to_owned()),
                ("bash".to_owned(), "langs/bash".to_owned()),
                ("basic".to_owned(), "basic".to_owned()),
                ("baz".to_owned(), "deps/baz".to_owned()),
                ("deps".to_owned(), "deps".to_owned()),
                ("empty-config".to_owned(), "empty-config".to_owned()),
                ("foo".to_owned(), "deps/foo".to_owned()),
                ("js".to_owned(), "langs/js".to_owned()),
                ("langs".to_owned(), "langs".to_owned()),
                ("platforms".to_owned(), "platforms".to_owned()),
                ("no-config".to_owned(), "no-config".to_owned()),
                ("package-json".to_owned(), "package-json".to_owned()),
                ("tasks".to_owned(), "tasks".to_owned()),
                ("ts".to_owned(), "langs/ts".to_owned()),
            ])
        );
    }

    #[tokio::test]
    async fn supports_all_id_formats() {
        let workspace_config = WorkspaceConfig {
            projects: WorkspaceProjects::Globs(string_vec!["*"]),
            ..WorkspaceConfig::default()
        };

        let sandbox =
            create_sandbox_with_config("project-graph/ids", Some(&workspace_config), None, None);

        let graph = ProjectGraph::generate(
            sandbox.path(),
            &workspace_config,
            &ToolchainConfig::default(),
            GlobalProjectConfig::default(),
            &CacheEngine::load(sandbox.path()).await.unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(
            graph.projects_map,
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
        let (graph, _sandbox) = get_dependencies_graph().await;

        let a = graph.load("a").unwrap();
        let b = graph.load("b").unwrap();
        let c = graph.load("c").unwrap();
        let d = graph.load("d").unwrap();

        assert_eq!(graph.get_dependencies_of(&a).unwrap(), string_vec!["b"]);
        assert_eq!(graph.get_dependencies_of(&b).unwrap(), string_vec!["c"]);
        assert_eq!(graph.get_dependencies_of(&c).unwrap(), string_vec![]);
        assert_eq!(
            graph.get_dependencies_of(&d).unwrap(),
            string_vec!["c", "b", "a"]
        );
    }
}

mod get_dependents_of {
    use super::*;

    #[tokio::test]
    async fn returns_dep_list() {
        let (graph, _sandbox) = get_dependents_graph().await;

        let a = graph.load("a").unwrap();
        let b = graph.load("b").unwrap();
        let c = graph.load("c").unwrap();
        let d = graph.load("d").unwrap();

        assert_eq!(graph.get_dependents_of(&a).unwrap(), string_vec![]);
        assert_eq!(graph.get_dependents_of(&b).unwrap(), string_vec!["a"]);
        assert_eq!(graph.get_dependents_of(&c).unwrap(), string_vec!["b"]);
        assert_eq!(
            graph.get_dependents_of(&d).unwrap(),
            string_vec!["a", "b", "c"]
        );
    }
}

mod to_dot {
    use super::*;

    #[tokio::test]
    async fn renders_tree() {
        let (graph, _sandbox) = get_dependencies_graph().await;

        graph.load("a").unwrap();
        graph.load("b").unwrap();
        graph.load("c").unwrap();
        graph.load("d").unwrap();

        assert_snapshot!(graph.to_dot());
    }
}

mod implicit_explicit_deps {
    use super::*;

    #[tokio::test]
    async fn loads_implicit() {
        let (mut graph, _sandbox) = get_aliases_graph().await;

        graph
            .register_platform(Box::new(NodePlatform::default()))
            .unwrap();

        let project = graph.load("implicit").unwrap();

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
        let (mut graph, _sandbox) = get_aliases_graph().await;

        graph
            .register_platform(Box::new(NodePlatform::default()))
            .unwrap();

        let project = graph.load("explicit").unwrap();

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
        let (mut graph, _sandbox) = get_aliases_graph().await;

        graph
            .register_platform(Box::new(NodePlatform::default()))
            .unwrap();

        let project = graph.load("explicitAndImplicit").unwrap();

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
