use crate::helpers::create_progress_bar;
use crate::session::CliSession;
use clap::Args;
use moon_actions::{sync_codeowners, unsync_codeowners};
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
    let workspace = session.get_workspace_legacy()?;

    if args.clean {
        let codeowners_path = unsync_codeowners(&workspace).await?;

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
        let project_graph = session.get_project_graph().await?;
        let codeowners_path = sync_codeowners(&workspace, &project_graph, args.force).await?;

        done(
            format!(
                "Successfully created {}",
                if let Some(path) = codeowners_path {
                    color::path(path.strip_prefix(&workspace.root).unwrap())
                } else {
                    "code owners".into()
                }
            ),
            true,
        );
    }

    Ok(())
}
