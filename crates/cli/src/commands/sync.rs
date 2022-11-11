use crate::helpers::{create_progress_bar, load_workspace};
use moon_runner::{DepGraph, Runner};

pub async fn sync() -> Result<(), Box<dyn std::error::Error>> {
    let done = create_progress_bar("Syncing projects...");

    let workspace = load_workspace().await?;
    let mut project_count = 0;
    let mut graph = DepGraph::default();

    for project_id in workspace.projects.ids() {
        let project = workspace.projects.load(&project_id)?;

        graph.sync_project(&project, &workspace.projects)?;
        project_count += 1;
    }

    let mut runner = Runner::new(workspace);
    let results = runner.run(graph, None).await?;

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
