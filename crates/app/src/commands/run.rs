use super::exec::*;
use crate::session::MoonSession;
use clap::Args;
use moon_affected::{DownstreamScope, UpstreamScope};
use moon_app_macros::{with_affected_args, with_shared_exec_args};
use moon_task::TargetLocator;
use starbase::AppResult;
use std::mem;
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
pub async fn run(session: MoonSession, mut args: RunArgs) -> AppResult {
    let targets = mem::take(&mut args.targets);
    let passthrough = mem::take(&mut args.passthrough);
    let query = args.query.take();

    exec(session, {
        let mut args = args.into_exec_args();
        args.targets = targets;
        args.on_failure = OnFailure::Bail;
        args.passthrough = passthrough;
        args.query = query;
        args
    })
    .await
}
