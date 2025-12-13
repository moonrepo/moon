use super::exec::*;
use crate::prompts::select_identifiers;
use crate::session::MoonSession;
use clap::Args;
use moon_affected::{DownstreamScope, UpstreamScope};
use moon_app_macros::{with_affected_args, with_shared_exec_args};
use moon_common::Id;
use moon_console::ui::{SelectOption, SelectProps};
use moon_project::Project;
use moon_task::TargetLocator;
use starbase::AppResult;
use std::sync::Arc;
use tracing::instrument;

#[with_affected_args]
#[with_shared_exec_args]
#[derive(Args, Clone, Debug)]
pub struct CheckArgs {
    #[arg(help = "List of explicit project IDs to check")]
    #[clap(group = "projects")]
    ids: Vec<Id>,

    #[arg(long, help = "Check all projects")]
    #[clap(group = "projects")]
    all: bool,

    #[arg(long, help = "Check the closest project")]
    #[clap(group = "projects")]
    closest: bool,
}

#[instrument(skip(session))]
pub async fn check(session: MoonSession, args: CheckArgs) -> AppResult {
    let workspace_graph = session.get_workspace_graph().await?;
    let mut projects: Vec<Arc<Project>> = vec![];

    // Check all projects
    if args.all {
        projects.extend(workspace_graph.get_projects()?);
    }
    // Check the closest project
    else if args.closest {
        projects.push(workspace_graph.get_project_from_path(Some(&session.working_dir))?);
    }
    // Check selected projects
    else {
        let ids = select_identifiers(&session.console, &args.ids, || {
            Ok(SelectProps {
                label: "Which project(s) to check?".into(),
                options: workspace_graph
                    .get_projects()?
                    .into_iter()
                    .map(|project| {
                        SelectOption::new(&project.id).description_opt(
                            project
                                .config
                                .project
                                .as_ref()
                                .and_then(|cfg| cfg.description.clone()),
                        )
                    })
                    .collect(),
                multiple: true,
                ..Default::default()
            })
        })
        .await?;

        for id in ids {
            projects.push(workspace_graph.get_project(id)?);
        }
    };

    // Find all applicable targets
    let mut targets = vec![];

    for project in projects {
        for task in workspace_graph.get_tasks_from_project(&project.id)? {
            if task.is_build_type() || task.is_test_type() {
                targets.push(TargetLocator::Qualified(task.target.clone()));
            }
        }
    }

    exec(session, {
        let mut exec = args.to_exec_args();
        args.apply_affected_to_exec_args(&mut exec);

        exec.targets = targets;
        exec.on_failure = OnFailure::Bail;
        exec
    })
    .await
}
