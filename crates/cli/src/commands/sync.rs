use moon_action_runner::{ActionRunner, DepGraph};
use moon_workspace::Workspace;

pub async fn sync() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;
    let mut graph = DepGraph::default();

    for project_id in workspace.projects.ids() {
        graph.sync_project(&project_id, &workspace.projects)?;
    }

    let results = ActionRunner::new_run(workspace, graph, None).await?;

    Ok(())
}
