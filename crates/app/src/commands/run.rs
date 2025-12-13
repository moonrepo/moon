use super::exec::*;
use crate::session::MoonSession;
use clap::Args;
use moon_affected::{DownstreamScope, UpstreamScope};
use moon_app_macros::{with_affected_args, with_shared_exec_args};
use moon_task::TargetLocator;
use starbase::AppResult;
use tracing::instrument;

#[with_affected_args]
#[with_shared_exec_args(passthrough)]
#[derive(Args, Clone, Debug, Default)]
pub struct RunArgs {
    #[arg(help = "List of explicit task targets to run")]
    targets: Vec<TargetLocator>,

    #[arg(
        long,
        help = "Filter tasks based on the result of a query",
        help_heading = super::HEADING_WORKFLOW,
    )]
    query: Option<String>,
}

#[instrument(skip(session))]
pub async fn run(session: MoonSession, args: RunArgs) -> AppResult {
    exec(session, {
        let mut exec = args.to_exec_args();
        args.apply_affected_to_exec_args(&mut exec);

        exec.targets = args.targets;
        exec.on_failure = OnFailure::Bail;
        exec.query = args.query;
        exec
    })
    .await
}
