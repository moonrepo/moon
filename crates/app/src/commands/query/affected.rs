use crate::app_options::AffectedOption;
use crate::queries::changed_files::*;
use crate::session::{MoonSession, SessionResult};
use clap::Args;
use moon_affected::{AffectedTracker, Depth};
use moon_common::is_ci;
use starbase_utils::json;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct QueryAffectedArgs {
    #[arg(help = "Conditions in which to track affected")]
    by: Option<AffectedOption>,

    #[arg(
        long,
        default_value_t,
        alias = "downstream",
        help = "Include dependents of affected projects and tasks, up to this depth (none, direct, deep, or a number of hops)"
    )]
    dependents: Depth,
}

#[instrument(skip(session))]
pub async fn affected(session: MoonSession, args: QueryAffectedArgs) -> SessionResult {
    let vcs = session.get_vcs_adapter()?;

    let mut affected_tracker = AffectedTracker::new(
        session.get_workspace_graph().await?,
        query_changed_files_for_affected(&vcs, args.by.as_ref()).await?,
    );
    affected_tracker.set_ci_check(is_ci());
    affected_tracker.set_depth(args.dependents);
    affected_tracker.track_projects().await?;
    affected_tracker.track_tasks().await?;

    let affected = affected_tracker.build();

    session
        .console
        .out
        .write_line(json::format(&affected, true)?)?;

    Ok(None)
}
