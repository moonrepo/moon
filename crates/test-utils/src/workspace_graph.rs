use crate::workspace_mocker::*;
pub use moon_project_graph::ProjectGraph;
pub use moon_task_graph::TaskGraph;
pub use moon_workspace_graph::WorkspaceGraph;
use starbase_sandbox::create_sandbox;
use std::path::Path;

#[deprecated]
pub fn create_workspace_graph_mocker(root: &Path) -> WorkspaceMocker {
    WorkspaceMocker::new(root)
        .load_default_configs()
        .with_default_projects()
        .with_default_toolchains()
        .with_inherited_tasks()
}

#[deprecated]
pub async fn generate_workspace_graph(fixture: &str) -> WorkspaceGraph {
    generate_workspace_graph_from_sandbox(create_sandbox(fixture).path()).await
}

#[deprecated]
pub async fn generate_workspace_graph_from_sandbox(root: &Path) -> WorkspaceGraph {
    create_workspace_graph_mocker(root)
        .mock_workspace_graph()
        .await
}
