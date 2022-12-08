use moon::{generate_project_graph, load_workspace_from};
use moon_config::{NodeConfig, NodeProjectAliasFormat};
use moon_project_graph::ProjectGraph;
use moon_test_utils::{
    assert_snapshot, create_sandbox_with_config, get_project_graph_aliases_fixture_configs, Sandbox,
};
use rustc_hash::FxHashMap;

async fn get_aliases_graph<C>(callback: C) -> (ProjectGraph, Sandbox)
where
    C: FnOnce(&mut NodeConfig),
{
    let (workspace_config, mut toolchain_config, projects_config) =
        get_project_graph_aliases_fixture_configs();

    if let Some(node_config) = &mut toolchain_config.node {
        callback(node_config);
    }

    let sandbox = create_sandbox_with_config(
        "project-graph/aliases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let graph = generate_project_graph(&mut workspace).unwrap();

    (graph, sandbox)
}

#[tokio::test]
async fn loads_node_aliases_name_only() {
    let (graph, _sandbox) = get_aliases_graph(|cfg| {
        cfg.alias_package_names = Some(NodeProjectAliasFormat::NameOnly);
    })
    .await;

    assert_eq!(
        graph.aliases,
        FxHashMap::from_iter([
            (
                "project-graph-aliases-explicit".to_owned(),
                "explicit".to_owned()
            ),
            (
                "project-graph-aliases-explicit-and-implicit".to_owned(),
                "explicitAndImplicit".to_owned()
            ),
            (
                "project-graph-aliases-implicit".to_owned(),
                "implicit".to_owned()
            ),
            ("project-graph-aliases-node".to_owned(), "node".to_owned()),
            ("pkg-bar".to_owned(), "nodeNameOnly".to_owned()),
            ("pkg-foo".to_owned(), "nodeNameScope".to_owned()),
            ("@scope/pkg-foo".to_owned(), "nodeNameScope".to_owned())
        ])
    );
}

#[tokio::test]
async fn loads_node_aliases_name_scopes() {
    let (graph, _sandbox) = get_aliases_graph(|cfg| {
        cfg.alias_package_names = Some(NodeProjectAliasFormat::NameAndScope);
    })
    .await;

    assert_eq!(
        graph.aliases,
        FxHashMap::from_iter([
            (
                "project-graph-aliases-explicit".to_owned(),
                "explicit".to_owned()
            ),
            (
                "project-graph-aliases-explicit-and-implicit".to_owned(),
                "explicitAndImplicit".to_owned()
            ),
            (
                "project-graph-aliases-implicit".to_owned(),
                "implicit".to_owned()
            ),
            ("project-graph-aliases-node".to_owned(), "node".to_owned()),
            ("pkg-bar".to_owned(), "nodeNameOnly".to_owned()),
            ("@scope/pkg-foo".to_owned(), "nodeNameScope".to_owned())
        ])
    );
}

#[tokio::test]
async fn returns_project_using_alias() {
    let (graph, _sandbox) = get_aliases_graph(|cfg| {
        cfg.alias_package_names = Some(NodeProjectAliasFormat::NameAndScope);
    })
    .await;

    assert_eq!(
        graph.get("@scope/pkg-foo").unwrap().id,
        "nodeNameScope".to_owned()
    );
}

#[tokio::test]
async fn graph_uses_id_for_nodes() {
    let (graph, _sandbox) = get_aliases_graph(|cfg| {
        cfg.alias_package_names = Some(NodeProjectAliasFormat::NameAndScope);
    })
    .await;

    graph.get("pkg-bar").unwrap();
    graph.get("@scope/pkg-foo").unwrap();

    assert_snapshot!(graph.to_dot());
}
