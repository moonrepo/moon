use crate::commands::run::{run, RunOptions};
use crate::helpers::load_workspace;
use moon_project::Project;
use moon_project_graph::project_graph::ProjectGraph;
use std::env;

pub struct CheckOptions {
    pub report: bool,
}

pub async fn check(
    project_ids: &Vec<String>,
    options: CheckOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;
    let project_graph = ProjectGraph::generate(&workspace).await?;
    let mut projects: Vec<Project> = vec![];

    // Load projects
    if project_ids.is_empty() {
        projects.push(project_graph.load_from_path(env::current_dir()?)?);
    } else {
        for id in project_ids {
            projects.push(project_graph.load(id)?);
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
    run(
        &targets,
        RunOptions {
            report: options.report,
            ..RunOptions::default()
        },
        Some(workspace),
    )
    .await?;

    Ok(())
}
