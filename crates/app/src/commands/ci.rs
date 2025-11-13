use super::exec::*;
use crate::session::MoonSession;
use clap::Args;
use moon_affected::{DownstreamScope, UpstreamScope};
use moon_app_macros::with_shared_exec_props;
use moon_task::TargetLocator;
use starbase::AppResult;
use tracing::instrument;

#[with_shared_exec_props]
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
        let mut args = args.into_exec_args();
        args.targets = targets;
        args.affected = true;
        args.on_failure = OnFailure::Continue;
        args.only_ci_tasks = true;

        // Include direct dependents for regression checks
        if args.downstream.is_none() {
            args.downstream = Some(DownstreamScope::Direct);
        }

        args
    })
    .await
}
