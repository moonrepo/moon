use crate::commands::run::{run, RunOptions};
use crate::helpers::load_workspace;
use std::env;

pub async fn check(project_id: &Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;
    let project = if let Some(id) = project_id {
        workspace.projects.load(id)?
    } else {
        workspace.projects.load_from_path(env::current_dir()?)?
    };

    // Find all applicable targets
    let mut targets = vec![];

    for task in project.tasks.values() {
        if task.should_run_in_ci() {
            targets.push(task.target.clone());
        }
    }

    // Run all targets using our runner
    run(&targets, RunOptions::default()).await?;

    Ok(())
}
