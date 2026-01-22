use super::exec::*;
use crate::app_options::SummaryOption;
use crate::session::MoonSession;
use clap::Args;
use moon_affected::{DownstreamScope, UpstreamScope};
use moon_app_macros::{with_affected_args, with_shared_exec_args};
use moon_task::TargetLocator;
use starbase::AppResult;
use tracing::instrument;

#[with_affected_args(always_affected)]
#[with_shared_exec_args]
#[derive(Args, Clone, Debug)]
pub struct CiArgs {
    #[arg(help = "List of explicit task targets to run")]
    targets: Vec<TargetLocator>,
}

#[instrument(skip(session))]
pub async fn ci(session: MoonSession, args: CiArgs) -> AppResult {
    let mut targets = args.targets.clone();

    if targets.is_empty() {
        let workspace_graph = session.get_workspace_graph().await?;

        for task in workspace_graph.get_tasks()? {
            targets.push(TargetLocator::Qualified(task.target.clone()));
        }
    }

    exec(session, {
        let mut exec = args.to_exec_args();
        args.apply_affected_to_exec_args(&mut exec);

        exec.targets = targets;
        exec.on_failure = OnFailure::Continue;
        exec.only_ci_tasks = true;
        exec.ci = Some(true);

        // Show full output in CI
        if exec.summary.is_none() {
            exec.summary = Some(Some(SummaryOption::Detailed));
        }

        // Include direct dependents for regression checks
        if exec.downstream.is_none() {
            exec.downstream = Some(DownstreamScope::Direct);
        }

        exec
    })
    .await
}
