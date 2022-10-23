use crate::helpers::{create_progress_bar, load_workspace};
use moon_project_graph::project_graph::ProjectGraph;
use moon_runner::{DepGraph, Runner};

pub async fn sync() -> Result<(), Box<dyn std::error::Error>> {
    let done = create_progress_bar("Syncing projects...");

    let workspace = load_workspace().await?;
    let project_graph = ProjectGraph::generate(&workspace).await?;
    let mut project_count = 0;
    let mut graph = DepGraph::generate(&project_graph);

    for project_id in project_graph.ids() {
        let project = project_graph.load(&project_id)?;
        let runtime = graph.get_runtime_from_project(&project);

        graph.sync_project(&runtime, &project)?;
        project_count += 1;
    }

    let mut runner = Runner::new();
    let results = runner.run(workspace, graph, None).await?;

    if runner.has_failed() {
        done("Failed to sync projects", false);
    } else {
        done(
            format!("Successfully synced {} projects", project_count).as_ref(),
            true,
        );
    }

    runner.render_results(&results)?;

    Ok(())
}
