use insta::assert_snapshot;
use moon_cache::CacheEngine;
use moon_config::{
    GlobalProjectConfig, NodeConfig, NodeProjectAliasFormat, WorkspaceConfig, WorkspaceProjects,
};
use moon_project_graph::ProjectGraph;
use moon_utils::string_vec;
use moon_utils::test::get_fixtures_dir;
use std::collections::HashMap;

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

async fn get_aliases_graph(node_config: NodeConfig) -> ProjectGraph {
    let workspace_root = get_fixtures_dir("project-graph/aliases");
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Map(HashMap::from([
            ("noLang".to_owned(), "no-lang".to_owned()),
            ("nodeNameOnly".to_owned(), "node-name-only".to_owned()),
            ("nodeNameScope".to_owned(), "node-name-scope".to_owned()),
        ])),
        node: node_config,
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

mod aliases {
    use super::*;

    #[tokio::test]
    async fn loads_node_aliases_name_only() {
        let graph = get_aliases_graph(NodeConfig {
            alias_package_names: Some(NodeProjectAliasFormat::NameOnly),
            ..NodeConfig::default()
        })
        .await;

        assert_eq!(
            graph.aliases,
            HashMap::from([
                ("pkg-bar".to_owned(), "nodeNameOnly".to_owned()),
                ("pkg-foo".to_owned(), "nodeNameScope".to_owned())
            ])
        );
    }

    #[tokio::test]
    async fn loads_node_aliases_name_scopes() {
        let graph = get_aliases_graph(NodeConfig {
            alias_package_names: Some(NodeProjectAliasFormat::NameAndScope),
            ..NodeConfig::default()
        })
        .await;

        assert_eq!(
            graph.aliases,
            HashMap::from([
                ("pkg-bar".to_owned(), "nodeNameOnly".to_owned()),
                ("@scope/pkg-foo".to_owned(), "nodeNameScope".to_owned())
            ])
        );
    }

    #[tokio::test]
    async fn returns_project_using_alias() {
        let graph = get_aliases_graph(NodeConfig {
            alias_package_names: Some(NodeProjectAliasFormat::NameAndScope),
            ..NodeConfig::default()
        })
        .await;

        assert_eq!(
            graph.load("@scope/pkg-foo").unwrap().id,
            "nodeNameScope".to_owned()
        );
    }

    #[tokio::test]
    async fn graph_uses_id_for_nodes() {
        let graph = get_aliases_graph(NodeConfig {
            alias_package_names: Some(NodeProjectAliasFormat::NameAndScope),
            ..NodeConfig::default()
        })
        .await;

        graph.load("pkg-bar").unwrap();
        graph.load("@scope/pkg-foo").unwrap();

        assert_snapshot!(graph.to_dot());
    }
}
