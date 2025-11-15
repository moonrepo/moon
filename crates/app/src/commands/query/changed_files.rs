use crate::queries::changed_files::*;
use crate::session::MoonSession;
use clap::{ArgAction, Args};
use moon_common::is_ci;
use moon_vcs::ChangedStatus;
use starbase::AppResult;
use starbase_utils::json;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct QueryChangedFilesArgs {
    #[arg(long, help = "Base branch, commit, or revision to compare against")]
    base: Option<String>,

    #[arg(
        long,
        help = "When on the default branch, compare against the previous revision",
        action = ArgAction::SetTrue
    )]
    default_branch: Option<bool>,

    #[arg(long, help = "Current branch, commit, or revision to compare with")]
    head: Option<String>,

    #[arg(
        long,
        help = "Gather files from your local state instead of the remote",
        action = ArgAction::SetTrue,
        group = "local-remote",
    )]
    local: Option<bool>,

    #[arg(
        long,
        help = "Gather files from the remote state instead of your local",
        action = ArgAction::SetTrue,
        group = "local-remote",
    )]
    remote: Option<bool>,

    #[arg(long, help = "Filter files based on a changed status")]
    status: Vec<ChangedStatus>,
}

#[instrument(skip(session))]
pub async fn changed_files(session: MoonSession, args: QueryChangedFilesArgs) -> AppResult {
    let vcs = session.get_vcs_adapter()?;
    let ci = is_ci();

    let result = query_changed_files(
        &vcs,
        QueryChangedFilesOptions {
            base: args.base,
            default_branch: args.default_branch.unwrap_or(ci),
            head: args.head,
            local: match (args.local, args.remote) {
                (Some(local), None) => local,
                (None, Some(remote)) => !remote,
                _ => !ci,
            },
            status: args.status,
            stdin: false,
        },
    )
    .await?;

    session
        .console
        .out
        .write_line(json::format(&result, true)?)?;

    Ok(None)
}
