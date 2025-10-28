use crate::queries::changed_files::*;
use crate::session::MoonSession;
use clap::Args;
use moon_affected::{AffectedTracker, DownstreamScope, UpstreamScope};
use starbase::AppResult;
use starbase_utils::json;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct QueryAffectedArgs {
    #[arg(long, default_value_t, help = "Include downstream dependents")]
    downstream: DownstreamScope,

    #[arg(long, default_value_t, help = "Include upstream dependencies")]
    upstream: UpstreamScope,
}

#[instrument(skip(session))]
pub async fn affected(session: MoonSession, args: QueryAffectedArgs) -> AppResult {
    let vcs = session.get_vcs_adapter()?;

    let mut affected_tracker = AffectedTracker::new(
        session.get_workspace_graph().await?,
        query_changed_files_for_affected(&vcs).await?,
    );
    affected_tracker.with_scopes(args.upstream, args.downstream);
    affected_tracker.track_projects()?;
    affected_tracker.track_tasks()?;

    let affected = affected_tracker.build();

    session
        .console
        .out
        .write_line(json::format(&affected, true)?)?;

    Ok(None)
}
