use moon_app_context::AppContext;
use moon_common::Id;
use moon_hash::ContentHasher;
use moon_layout_graph::{LayoutGraph, LayoutGraphBuilder};
use moon_task_hasher::hash_toolchain_task_contents;
use moon_test_utils::{WorkspaceGraph, WorkspaceMocker};
use starbase_sandbox::{Sandbox, create_sandbox};
use starbase_utils::json::serde_json::{self, Value};
use std::sync::Arc;

// The `layout-hash` fixture is a dependencies workspace at the root (locking
// `dep-a` and `dep-b`, with `packages/*` members), a `standalone` package root
// (locking `dep-c`), and `tools`, which isn't a member of the workspace that
// it locates, but depends on `dep-b`
async fn create_graphs(sandbox: &Sandbox) -> (Arc<AppContext>, Arc<WorkspaceGraph>) {
    let mocker = WorkspaceMocker::new(sandbox.path())
        .load_default_configs()
        .with_test_toolchains()
        .update_toolchains_config(|config| {
            // Locate roots from each project, instead of the working dir
            config
                .plugins
                .get_mut(&Id::raw("tc-tier2"))
                .unwrap()
                .config
                .insert(
                    "testLocateRoots".into(),
                    starbase_utils::json::JsonValue::Bool(true),
                );
        })
        .with_global_envs();

    (
        Arc::new(mocker.mock_app_context()),
        Arc::new(mocker.mock_workspace_graph().await),
    )
}

async fn place_project(
    app_context: &Arc<AppContext>,
    workspace_graph: &Arc<WorkspaceGraph>,
    project_id: &str,
) -> LayoutGraph {
    let mut builder = LayoutGraphBuilder::new(Arc::clone(app_context), Arc::clone(workspace_graph));

    builder
        .place_project(workspace_graph.get_project(project_id).unwrap())
        .await
        .unwrap();

    builder.build()
}

async fn hash_toolchains(
    app_context: &Arc<AppContext>,
    workspace_graph: &WorkspaceGraph,
    layout_graph: &LayoutGraph,
    project_id: &str,
) -> String {
    let project = workspace_graph.get_project(project_id).unwrap();
    let task = workspace_graph
        .get_task_from_project(project_id, "build")
        .unwrap();
    let mut hasher = ContentHasher::new("test");

    hash_toolchain_task_contents(app_context, layout_graph, &project, &task, &mut hasher)
        .await
        .unwrap();

    hasher.serialize().unwrap().to_owned()
}

// The production dependencies hashed for `tc-tier2`, as the other test
// toolchains locate the workspace root regardless of the project
fn get_dependencies(hash: &str) -> Value {
    serde_json::from_str::<Vec<Value>>(hash)
        .unwrap()
        .into_iter()
        .find(|content| content["toolchain"] == "tc-tier2")
        .map(|content| content["dependencies"]["production"].clone())
        .unwrap_or_default()
}

mod toolchain_hashing {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn hashes_locked_dependencies_of_the_root() {
        let sandbox = create_sandbox("layout-hash");
        let (app_context, workspace_graph) = create_graphs(&sandbox).await;

        // A member, a package root, and a project outside of the workspace,
        // which still hashes the lockfile of the root that it located
        for (project_id, dep, version) in [
            ("a", "dep-a", "1.0.0"),
            ("standalone", "dep-c", "3.0.0"),
            ("tools", "dep-b", "2.0.0"),
        ] {
            let layout = place_project(&app_context, &workspace_graph, project_id).await;
            let hash = hash_toolchains(&app_context, &workspace_graph, &layout, project_id).await;

            assert_eq!(
                get_dependencies(&hash),
                serde_json::json!({ dep: version }),
                "project {project_id}"
            );
        }
    }

    // The root that's hashed is the one that was placed, which isn't located
    // again, so a closer lockfile only applies once a project is placed again
    #[tokio::test(flavor = "multi_thread")]
    async fn hashes_the_placed_root() {
        let sandbox = create_sandbox("layout-hash");
        let (app_context, workspace_graph) = create_graphs(&sandbox).await;
        let layout = place_project(&app_context, &workspace_graph, "a").await;

        sandbox.create_file("packages/a/tc.lock", "dep-a@1.5.0\n");

        let before = hash_toolchains(&app_context, &workspace_graph, &layout, "a").await;

        // Plugin calls are cached per process, so placing again needs new
        // graphs (and plugins), like a later run
        let (app_context, workspace_graph) = create_graphs(&sandbox).await;
        let layout = place_project(&app_context, &workspace_graph, "a").await;
        let after = hash_toolchains(&app_context, &workspace_graph, &layout, "a").await;

        assert_eq!(
            get_dependencies(&before),
            serde_json::json!({ "dep-a": "1.0.0" })
        );
        assert_eq!(
            get_dependencies(&after),
            serde_json::json!({ "dep-a": "1.5.0" })
        );
    }

    // A toolchain that wasn't placed has no root, so no dependencies
    #[tokio::test(flavor = "multi_thread")]
    async fn hashes_no_dependencies_if_not_placed() {
        let sandbox = create_sandbox("layout-hash");
        let (app_context, workspace_graph) = create_graphs(&sandbox).await;
        let hash =
            hash_toolchains(&app_context, &workspace_graph, &LayoutGraph::default(), "a").await;

        assert_eq!(get_dependencies(&hash), Value::Null);
    }
}
