use crate::commands::run::{run_target, RunOptions};
use crate::helpers::{generate_project_graph, load_workspace, AnyError};
use moon_logger::trace;
use moon_project::Project;
use std::env;

pub struct CheckOptions {
    pub report: bool,
    pub all: bool,
}

const LOG_TARGET: &str = "moon:check";

pub async fn check(project_ids: &Vec<String>, options: CheckOptions) -> Result<(), AnyError> {
    let mut workspace = load_workspace().await?;
    let project_graph = generate_project_graph(&mut workspace).await?;
    let mut projects: Vec<&Project> = vec![];

    // Load projects
    if options.all {
        trace!(target: LOG_TARGET, "Running check on all projects");

        projects.extend(project_graph.get_all()?);
    } else if project_ids.is_empty() {
        trace!(target: LOG_TARGET, "Loading from path");

        projects.push(project_graph.load_from_path(env::current_dir()?)?);
    } else {
        trace!(
            target: LOG_TARGET,
            "Running for specific projects: {}",
            project_ids.join(", ")
        );

        for id in project_ids {
            projects.push(project_graph.get(id)?);
        }
    };

    // Find all applicable targets
    let mut targets = vec![];

    for project in projects {
        for task in project.tasks.values() {
            if task.is_build_type() || task.is_test_type() {
                targets.push(task.target.id.clone());
            }
        }
    }

    // Run targets using our run command
    run_target(
        &targets,
        RunOptions {
            report: options.report,
            ..RunOptions::default()
        },
        workspace,
        project_graph,
    )
    .await?;

    Ok(())
}
