use crate::commands::run::{run, RunOptions};
use crate::helpers::load_workspace;
use moon_project::Project;
use std::env;

pub async fn check(project_ids: &Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;
    let mut projects: Vec<Project> = vec![];

    // Load projects
    if project_ids.is_empty() {
        projects.push(workspace.projects.load_from_path(env::current_dir()?)?);
    } else {
        for id in project_ids {
            projects.push(workspace.projects.load(id)?);
        }
    };

    // Find all applicable targets
    let mut targets = vec![];

    for project in projects {
        for task in project.tasks.values() {
            if task.is_build_type() || task.is_test_type() {
                targets.push(task.target.clone());
            }
        }
    }

    // Run targets using our run command
    run(&targets, RunOptions::default(), Some(workspace)).await?;

    Ok(())
}
