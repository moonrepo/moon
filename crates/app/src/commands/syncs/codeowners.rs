use crate::helpers::create_progress_bar;
use crate::session::CliSession;
use clap::Args;
use moon_actions::operations::{sync_codeowners, unsync_codeowners};
use starbase::AppResult;
use starbase_styles::color;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct SyncCodeownersArgs {
    #[arg(long, help = "Clean and remove previously generated file")]
    clean: bool,

    #[arg(long, help = "Bypass cache and force create file")]
    force: bool,
}

#[instrument(skip_all)]
pub async fn sync(session: CliSession, args: SyncCodeownersArgs) -> AppResult {
    let done = create_progress_bar("Syncing code owners...");
    let context = session.get_app_context().await?;

    if args.clean {
        let codeowners_path = unsync_codeowners(&context).await?;

        done(
            format!(
                "Successfully removed {}",
                color::path(
                    codeowners_path
                        .strip_prefix(&session.workspace_root)
                        .unwrap()
                )
            ),
            true,
        );
    } else {
        let workspace_graph = session.get_workspace_graph().await?;
        let codeowners_path = sync_codeowners(&context, &workspace_graph, args.force).await?;

        done(
            format!(
                "Successfully created {}",
                if let Some(path) = codeowners_path {
                    color::path(path.strip_prefix(&session.workspace_root).unwrap())
                } else {
                    "code owners".into()
                }
            ),
            true,
        );
    }

    Ok(None)
}
