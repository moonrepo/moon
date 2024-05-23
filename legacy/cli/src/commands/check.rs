use crate::app::GlobalArgs;
use crate::commands::run::{run_target, RunArgs};
use clap::Args;
use moon::generate_project_graph;
use moon_app_components::Console;
use moon_common::Id;
use moon_project::Project;
use moon_target::TargetLocator;
use moon_workspace::Workspace;
use starbase::system;
use std::sync::Arc;
use tracing::trace;

#[derive(Args, Clone, Debug)]
pub struct CheckArgs {
    #[arg(help = "List of project IDs to explicitly check")]
    #[clap(group = "projects")]
    ids: Vec<Id>,

    #[arg(long, help = "Run check for all projects in the workspace")]
    #[clap(group = "projects")]
    all: bool,

    #[arg(
        long,
        short = 's',
        help = "Include a summary of all actions that were processed in the pipeline"
    )]
    pub summary: bool,

    #[arg(
        short = 'u',
        long = "updateCache",
        help = "Bypass cache and force update any existing items"
    )]
    update_cache: bool,
}

#[system]
pub async fn check(
    args: ArgsRef<CheckArgs>,
    global_args: StateRef<GlobalArgs>,
    resources: Resources,
) {
    let mut workspace = resources.get_async::<Workspace>().await;
    let console = resources.get_async::<Console>().await;

    let project_graph = generate_project_graph(&mut workspace).await?;
    let mut projects: Vec<Arc<Project>> = vec![];

    // Load projects
    if args.all {
        trace!("Running check on all projects");

        projects.extend(project_graph.get_all()?);
    } else if args.ids.is_empty() {
        trace!("Loading from path");

        projects.push(project_graph.get_from_path(None)?);
    } else {
        trace!(
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
            if !task.is_internal() && (task.is_build_type() || task.is_test_type()) {
                targets.push(TargetLocator::Qualified(task.target.clone()));
            }
        }
    }

    // Run targets using our run command
    run_target(
        &targets,
        &RunArgs {
            summary: args.summary,
            update_cache: args.update_cache,
            ..RunArgs::default()
        },
        global_args.concurrency,
        &workspace,
        &console,
        project_graph,
    )
    .await?;
}
