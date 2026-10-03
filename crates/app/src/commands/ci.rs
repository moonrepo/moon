use super::exec::*;
use crate::app_options::SummaryOption;
use crate::session::{MoonSession, SessionResult};
use clap::Args;
use moon_affected::{DownstreamScope, UpstreamScope};
use moon_app_macros::{with_affected_args, with_shared_exec_args};
use moon_task::TargetLocator;
use tracing::instrument;

#[with_affected_args(always_affected)]
#[with_shared_exec_args]
#[derive(Args, Clone, Debug)]
pub struct CiArgs {
    #[arg(help = "List of explicit task targets to run")]
    targets: Vec<TargetLocator>,
}

#[instrument(skip(session))]
pub async fn ci(session: MoonSession, args: CiArgs) -> SessionResult {
    let mut targets = args.targets.clone();
    let mut skip_persistent = false;

    if targets.is_empty() && args.plan.is_none() {
        let workspace_graph = session.get_workspace_graph().await?;

        for task in workspace_graph.get_tasks_unexpanded()? {
            targets.push(TargetLocator::Qualified(task.target.clone()));
        }

        // Persistent tasks never complete, so only run them when another
        // task depends on them, as none of these have been requested
        skip_persistent = true;
    }

    exec(session, {
        let mut exec = args.to_exec_args();
        args.apply_affected_to_exec_args(&mut exec);

        exec.targets = targets;
        exec.on_failure = OnFailure::Continue;
        exec.ignore_ci_checks = false;
        exec.ci = Some(true);
        exec.skip_persistent = skip_persistent;

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
