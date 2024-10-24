use crate::workspace_mocker::*;
use starbase_sandbox::create_sandbox;
use std::path::Path;

pub use moon_project_graph::ProjectGraph;

pub fn create_project_graph_mocker(root: &Path) -> WorkspaceMocker {
    let mut mock = WorkspaceMocker::new(root);

    mock.with_default_configs()
        .with_default_projects()
        .with_default_toolchain()
        .with_global_tasks();

    mock
}

pub async fn generate_project_graph(fixture: &str) -> ProjectGraph {
    generate_project_graph_from_sandbox(create_sandbox(fixture).path()).await
}

pub async fn generate_project_graph_from_sandbox(root: &Path) -> ProjectGraph {
    create_project_graph_mocker(root)
        .build_project_graph()
        .await
}
