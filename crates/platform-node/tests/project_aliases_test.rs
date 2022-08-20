use insta::assert_snapshot;
use moon_cache::CacheEngine;
use moon_config::{
    GlobalProjectConfig, NodeConfig, NodeProjectAliasFormat, WorkspaceConfig, WorkspaceProjects,
};
use moon_contract::Platformable;
use moon_platform_node::NodePlatform;
use moon_project_graph::ProjectGraph;
use moon_utils::test::get_fixtures_dir;
use std::collections::HashMap;

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

    let mut graph = ProjectGraph::create(
        &workspace_root,
        &workspace_config,
        GlobalProjectConfig::default(),
        &CacheEngine::create(&workspace_root).await.unwrap(),
    )
    .await
    .unwrap();

    graph
        .register_platform(Box::new(NodePlatform::default()))
        .unwrap();

    graph
}

#[tokio::test]
async fn loads_node_aliases_name_only() {
    let graph = get_aliases_graph(NodeConfig {
        alias_package_names: Some(NodeProjectAliasFormat::NameOnly),
        ..NodeConfig::default()
    })
    .await;

    assert_eq!(
        graph.aliases_map,
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
        graph.aliases_map,
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
