use moon_workspace::{DepGraph, TaskRunner, Workspace};

pub async fn run(target: &str) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;

    // Generate a dependency graph for all the targets that need to be ran
    let mut dep_graph = DepGraph::default();
    dep_graph.run_target(target, &workspace.projects)?;

    // Process all tasks in the graph
    TaskRunner::new(workspace)
        .set_primary_target(target)
        .run(dep_graph)
        .await?;

    Ok(())
}
