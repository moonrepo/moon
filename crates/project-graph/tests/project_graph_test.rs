use insta::assert_snapshot;
use moon_cache::CacheEngine;
use moon_config::{
    GlobalProjectConfig, NodeConfig, NodeProjectAliasFormat, WorkspaceConfig, WorkspaceProjects,
};
use moon_project_graph::ProjectGraph;
use moon_utils::string_vec;
use moon_utils::test::{create_sandbox, create_sandbox_with_git, get_fixtures_dir};
use std::collections::HashMap;

async fn get_aliases_graph() -> ProjectGraph {
    let workspace_root = get_fixtures_dir("project-graph/aliases");
    let workspace_config = WorkspaceConfig {
        node: Some(NodeConfig {
            alias_package_names: Some(NodeProjectAliasFormat::NameAndScope),
            ..NodeConfig::default()
        }),
        projects: WorkspaceProjects::Map(HashMap::from([
            ("explicit".to_owned(), "explicit".to_owned()),
            (
                "explicitAndImplicit".to_owned(),
                "explicit-and-implicit".to_owned(),
            ),
            ("implicit".to_owned(), "implicit".to_owned()),
            ("noLang".to_owned(), "no-lang".to_owned()),
            ("node".to_owned(), "node".to_owned()),
            ("nodeNameOnly".to_owned(), "node-name-only".to_owned()),
            ("nodeNameScope".to_owned(), "node-name-scope".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };

    ProjectGraph::create(
        &workspace_root,
        &workspace_config,
        GlobalProjectConfig::default(),
        &CacheEngine::create(&workspace_root).await.unwrap(),
    )
    .await
    .unwrap()
}

async fn get_dependencies_graph() -> ProjectGraph {
    let workspace_root = get_fixtures_dir("project-graph/dependencies");
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Map(HashMap::from([
            ("a".to_owned(), "a".to_owned()),
            ("b".to_owned(), "b".to_owned()),
            ("c".to_owned(), "c".to_owned()),
            ("d".to_owned(), "d".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };

    ProjectGraph::create(
        &workspace_root,
        &workspace_config,
        GlobalProjectConfig::default(),
        &CacheEngine::create(&workspace_root).await.unwrap(),
    )
    .await
    .unwrap()
}

async fn get_dependents_graph() -> ProjectGraph {
    let workspace_root = get_fixtures_dir("project-graph/dependents");
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Map(HashMap::from([
            ("a".to_owned(), "a".to_owned()),
            ("b".to_owned(), "b".to_owned()),
            ("c".to_owned(), "c".to_owned()),
            ("d".to_owned(), "d".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };

    ProjectGraph::create(
        &workspace_root,
        &workspace_config,
        GlobalProjectConfig::default(),
        &CacheEngine::create(&workspace_root).await.unwrap(),
    )
    .await
    .unwrap()
}

mod globs {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn ignores_dot_folders() {
        // Use git so we can test against the .git folder
        let fixture = create_sandbox_with_git("projects");

        // Create fake node modules
        fs::create_dir_all(fixture.path().join("node_modules/moon")).unwrap();
        fs::write(fixture.path().join("node_modules/moon/package.json"), "{}").unwrap();

        let workspace_config = WorkspaceConfig {
            projects: WorkspaceProjects::List(string_vec!["**"]),
            ..WorkspaceConfig::default()
        };

        let graph = ProjectGraph::create(
            fixture.path(),
            &workspace_config,
            GlobalProjectConfig::default(),
            &CacheEngine::create(fixture.path()).await.unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(
            graph.projects_map,
            HashMap::from([
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
                ("no-config".to_owned(), "no-config".to_owned()),
                ("package-json".to_owned(), "package-json".to_owned()),
                ("tasks".to_owned(), "tasks".to_owned()),
                ("ts".to_owned(), "langs/ts".to_owned()),
            ])
        );
    }

    #[tokio::test]
    async fn supports_all_id_formats() {
        let fixture = create_sandbox("project-graph/ids");

        let workspace_config = WorkspaceConfig {
            projects: WorkspaceProjects::List(string_vec!["*"]),
            ..WorkspaceConfig::default()
        };

        let graph = ProjectGraph::create(
            fixture.path(),
            &workspace_config,
            GlobalProjectConfig::default(),
            &CacheEngine::create(fixture.path()).await.unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(
            graph.projects_map,
            HashMap::from([
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
        let graph = get_dependencies_graph().await;

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
        let graph = get_dependents_graph().await;

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
        let graph = get_dependencies_graph().await;

        graph.load("a").unwrap();
        graph.load("b").unwrap();
        graph.load("c").unwrap();
        graph.load("d").unwrap();

        assert_snapshot!(graph.to_dot());
    }
}

mod implicit_explicit_deps {
    use super::*;
    use moon_platform::Platformable;
    use moon_platform_node::NodePlatform;
    use moon_project::{ProjectDependency, ProjectDependencySource};

    #[tokio::test]
    async fn loads_implicit() {
        let mut graph = get_aliases_graph().await;

        graph
            .register_platform(Box::new(NodePlatform::default()))
            .unwrap();

        let project = graph.load("implicit").unwrap();

        assert_eq!(
            project.dependencies,
            HashMap::from([
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
        let mut graph = get_aliases_graph().await;

        graph
            .register_platform(Box::new(NodePlatform::default()))
            .unwrap();

        let project = graph.load("explicit").unwrap();

        assert_eq!(
            project.dependencies,
            HashMap::from([
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
        let mut graph = get_aliases_graph().await;

        graph
            .register_platform(Box::new(NodePlatform::default()))
            .unwrap();

        let project = graph.load("explicitAndImplicit").unwrap();

        assert_eq!(
            project.dependencies,
            HashMap::from([
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
