use moon::{generate_project_graph, load_workspace_from};
use moon_project_graph::ProjectGraph;
use moon_test_utils::{
    assert_snapshot, create_sandbox_with_config, get_project_graph_aliases_fixture_configs, Sandbox,
};
use rustc_hash::FxHashMap;

async fn get_aliases_graph() -> (ProjectGraph, Sandbox) {
    let (workspace_config, toolchain_config, tasks_config) =
        get_project_graph_aliases_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "project-graph/aliases",
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    );

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let graph = generate_project_graph(&mut workspace).await.unwrap();

    (graph, sandbox)
}

#[tokio::test]
async fn loads_node_aliases_name_scopes() {
    let (graph, _sandbox) = get_aliases_graph().await;

    assert_eq!(
        graph.aliases,
        FxHashMap::from_iter([
            ("project-graph-aliases-explicit".into(), "explicit".into()),
            (
                "project-graph-aliases-explicit-and-implicit".into(),
                "explicitAndImplicit".into()
            ),
            ("project-graph-aliases-implicit".into(), "implicit".into()),
            ("project-graph-aliases-node".into(), "node".into()),
            ("pkg-bar".into(), "nodeNameOnly".into()),
            ("@scope/pkg-foo".into(), "nodeNameScope".into())
        ])
    );
}

#[tokio::test]
async fn returns_project_using_alias() {
    let (graph, _sandbox) = get_aliases_graph().await;

    assert_eq!(
        graph.get("@scope/pkg-foo").unwrap().id,
        "nodeNameScope".to_owned()
    );
}

#[tokio::test]
async fn graph_uses_id_for_nodes() {
    let (graph, _sandbox) = get_aliases_graph().await;

    graph.get("pkg-bar").unwrap();
    graph.get("@scope/pkg-foo").unwrap();

    assert_snapshot!(graph.to_dot());
}
