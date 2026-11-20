use moon_app_context::AppContext;
use moon_common::Id;
use moon_config::{HasherOptimization, WorkspaceConfig};
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
    create_graphs_with(sandbox, |_| {}).await
}

async fn create_graphs_with(
    sandbox: &Sandbox,
    op: impl FnMut(&mut WorkspaceConfig),
) -> (Arc<AppContext>, Arc<WorkspaceGraph>) {
    let mocker = WorkspaceMocker::new(sandbox.path())
        .load_default_configs()
        .update_workspace_config(op)
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

// The dependencies (of every scope) hashed for `tc-tier2`, as the other test
// toolchains locate the workspace root regardless of the project
fn get_all_dependencies(hash: &str) -> Value {
    serde_json::from_str::<Vec<Value>>(hash)
        .unwrap()
        .into_iter()
        .find(|content| content["toolchain"] == "tc-tier2")
        .map(|content| content["dependencies"].clone())
        .unwrap_or_default()
}

// The production dependencies hashed for `tc-tier2`
fn get_dependencies(hash: &str) -> Value {
    get_all_dependencies(hash)["production"].clone()
}

// Like Cargo's `[workspace.dependencies]`, the root manifest defines the
// versions that packages inherit. The root is also a package, which inherits
// `dep-a` itself, and has its own `dep-b`, which members don't inherit
fn create_inherited_dependencies(sandbox: &Sandbox, dep_a_version: &str) {
    sandbox.create_file(
        "tc.cfg",
        format!(
            "inheritable-dep dep-a {dep_a_version}\n\
             inheritable-dep dep-b ^2.0.0\n\
             dep dep-a inherited\n\
             dep dep-b ^0.5.0\n"
        ),
    );
    sandbox.create_file(
        "packages/a/tc.cfg",
        "dep dep-a inherited\ndev-dep dep-b inherited\n",
    );
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

    #[tokio::test(flavor = "multi_thread")]
    async fn hashes_inherited_dependencies() {
        let sandbox = create_sandbox("layout-hash");

        create_inherited_dependencies(&sandbox, "^1.0.0");

        let (app_context, workspace_graph) = create_graphs(&sandbox).await;

        // The root package's own dependencies aren't inherited, and the
        // dependencies it can inherit are only hashed when inherited
        for (project_id, deps) in [
            (
                "a",
                serde_json::json!({
                    "development": { "dep-b": "2.0.0" },
                    "production": { "dep-a": "1.0.0" },
                }),
            ),
            (
                "root",
                serde_json::json!({
                    "production": { "dep-a": "1.0.0", "dep-b": "^0.5.0" },
                }),
            ),
        ] {
            let layout = place_project(&app_context, &workspace_graph, project_id).await;
            let hash = hash_toolchains(&app_context, &workspace_graph, &layout, project_id).await;

            assert_eq!(get_all_dependencies(&hash), deps, "project {project_id}");
        }
    }

    // Both versions are locked, so only the inherited requirement selects
    // which one is hashed
    #[tokio::test(flavor = "multi_thread")]
    async fn hashes_a_changed_inherited_version() {
        let sandbox = create_sandbox("layout-hash");

        sandbox.create_file("tc.lock", "dep-a@1.0.0\ndep-a@2.0.0\ndep-b@2.0.0\n");

        let mut hashes = vec![];

        for version in ["^1.0.0", "^2.0.0"] {
            create_inherited_dependencies(&sandbox, version);

            // Plugin calls are cached per process, so new graphs are needed
            let (app_context, workspace_graph) = create_graphs(&sandbox).await;
            let layout = place_project(&app_context, &workspace_graph, "a").await;

            hashes.push(get_dependencies(
                &hash_toolchains(&app_context, &workspace_graph, &layout, "a").await,
            ));
        }

        assert_eq!(
            hashes,
            [
                serde_json::json!({ "dep-a": "1.0.0" }),
                serde_json::json!({ "dep-a": "2.0.0" }),
            ]
        );
    }

    // Lockfiles aren't parsed, so the inherited requirements are hashed
    #[tokio::test(flavor = "multi_thread")]
    async fn hashes_inherited_requirements_when_optimizing_for_performance() {
        let sandbox = create_sandbox("layout-hash");

        create_inherited_dependencies(&sandbox, "^1.0.0");

        let (app_context, workspace_graph) = create_graphs_with(&sandbox, |config| {
            config.hasher.optimization = HasherOptimization::Performance;
        })
        .await;
        let layout = place_project(&app_context, &workspace_graph, "a").await;
        let hash = hash_toolchains(&app_context, &workspace_graph, &layout, "a").await;

        assert_eq!(
            get_all_dependencies(&hash),
            serde_json::json!({
                "development": { "dep-b": "^2.0.0" },
                "production": { "dep-a": "^1.0.0" },
            })
        );
    }
}
