use crate::app::GlobalArgs;
use crate::commands::run::{run_target, RunArgs};
use clap::Args;
use moon::{generate_project_graph, load_workspace};
use moon_common::Id;
use moon_logger::trace;
use moon_project::Project;
use starbase::{system, ExecuteArgs};
use std::env;
use std::sync::Arc;

#[derive(Args, Clone, Debug)]
pub struct CheckArgs {
    #[arg(help = "List of project IDs to explicitly check")]
    #[clap(group = "projects")]
    ids: Vec<Id>,

    #[arg(long, help = "Run check for all projects in the workspace")]
    #[clap(group = "projects")]
    all: bool,

    #[arg(
        short = 'u',
        long = "updateCache",
        help = "Bypass cache and force update any existing items"
    )]
    update_cache: bool,
}

const LOG_TARGET: &str = "moon:check";

#[system]
pub async fn check(args: StateRef<ExecuteArgs, CheckArgs>, global_args: StateRef<GlobalArgs>) {
    let mut workspace = load_workspace().await?;
    let project_graph = generate_project_graph(&mut workspace).await?;
    let mut projects: Vec<Arc<Project>> = vec![];

    // Load projects
    if args.all {
        trace!(target: LOG_TARGET, "Running check on all projects");

        projects.extend(project_graph.get_all()?);
    } else if args.ids.is_empty() {
        trace!(target: LOG_TARGET, "Loading from path");

        projects.push(project_graph.get_from_path(env::current_dir().unwrap())?);
    } else {
        trace!(
            target: LOG_TARGET,
            "Running for specific projects: {}",
            args.ids
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        for id in &args.ids {
            projects.push(project_graph.get(id)?);
        }
    };

    // Find all applicable targets
    let mut targets = vec![];

    for project in projects {
        for task in project.get_tasks()? {
            if task.is_build_type() || task.is_test_type() {
                targets.push(task.target.id.clone());
            }
        }
    }

    // Run targets using our run command
    run_target(
        &targets,
        &RunArgs {
            update_cache: args.update_cache,
            ..RunArgs::default()
        },
        global_args.concurrency,
        workspace,
        project_graph,
    )
    .await?;
}
