use crate::commands::run::{run_target, RunArgs};
use crate::session::CliSession;
use clap::Args;
use moon_common::Id;
use moon_project::Project;
use moon_task::TargetLocator;
use starbase::AppResult;
use std::sync::Arc;
use tracing::{instrument, trace};

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

#[instrument(skip_all)]
pub async fn check(session: CliSession, args: CheckArgs) -> AppResult {
    let project_graph = session.get_project_graph().await?;
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
            ids = ?args.ids,
            "Running for specific projects",
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
        &session,
        &RunArgs {
            summary: args.summary,
            update_cache: args.update_cache,
            ..RunArgs::default()
        },
        &targets,
    )
    .await?;

    Ok(())
}
