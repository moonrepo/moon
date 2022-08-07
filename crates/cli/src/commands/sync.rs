use indicatif::{ProgressBar, ProgressStyle};
use moon_action_runner::{ActionRunner, DepGraph};
use moon_terminal::create_theme;
use moon_workspace::Workspace;
use std::time::Duration;

pub async fn sync() -> Result<(), Box<dyn std::error::Error>> {
    let theme = create_theme();

    let pb = ProgressBar::new_spinner();
    pb.set_message("Syncing projects...");
    pb.enable_steady_tick(Duration::from_millis(50));

    let workspace = Workspace::load().await?;
    let mut project_count = 0;
    let mut graph = DepGraph::default();

    for project_id in workspace.projects.ids() {
        graph.sync_project(&project_id, &workspace.projects)?;
        project_count += 1;
    }

    let mut runner = ActionRunner::new(workspace);
    let results = runner.run(graph, None).await?;

    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{prefix} {msg}")
            .unwrap(),
    );

    if runner.has_failed() {
        pb.set_prefix(theme.error_prefix.to_string());
        pb.finish_with_message("Failed to sync projects");
    } else {
        pb.set_prefix(theme.success_prefix.to_string());
        pb.finish_with_message(format!("Successfully synced {} projects", project_count));
    }

    runner.render_results(&results)?;

    Ok(())
}
