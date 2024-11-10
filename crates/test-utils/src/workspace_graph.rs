use crate::workspace_mocker::*;
pub use moon_project_graph::ProjectGraph;
pub use moon_task_graph::TaskGraph;
pub use moon_workspace_graph::WorkspaceGraph;
use starbase_sandbox::create_sandbox;
use std::path::Path;

pub fn create_workspace_graph_mocker(root: &Path) -> WorkspaceMocker {
    let mut mock = WorkspaceMocker::new(root);

    mock.with_default_configs()
        .with_default_projects()
        .with_default_toolchain()
        .with_global_tasks();

    mock
}

pub async fn generate_workspace_graph(fixture: &str) -> WorkspaceGraph {
    generate_workspace_graph_from_sandbox(create_sandbox(fixture).path()).await
}

pub async fn generate_workspace_graph_from_sandbox(root: &Path) -> WorkspaceGraph {
    create_workspace_graph_mocker(root)
        .build_workspace_graph()
        .await
}
