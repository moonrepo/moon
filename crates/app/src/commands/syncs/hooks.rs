use crate::helpers::create_progress_bar;
use crate::session::CliSession;
use clap::Args;
use moon_actions::operations::{sync_vcs_hooks, unsync_vcs_hooks};
use starbase::AppResult;
use starbase_styles::color;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct SyncHooksArgs {
    #[arg(long, help = "Clean and remove previously generated hooks")]
    clean: bool,

    #[arg(long, help = "Bypass cache and force create hooks")]
    force: bool,
}

#[instrument(skip_all)]
pub async fn sync(session: CliSession, args: SyncHooksArgs) -> AppResult {
    if session.workspace_config.vcs.hooks.is_empty() {
        println!(
            "No hooks available to sync. Configure them with the {} setting.",
            color::property("vcs.hooks")
        );
        println!(
            "Learn more: {}",
            color::url("https://moonrepo.dev/docs/guides/vcs-hooks")
        );

        return Ok(None);
    }

    let done = create_progress_bar(format!(
        "Syncing {} hooks...",
        session.workspace_config.vcs.manager
    ));
    let hook_names = session
        .workspace_config
        .vcs
        .hooks
        .keys()
        .map(color::id)
        .collect::<Vec<_>>()
        .join(", ");
    let context = session.get_app_context().await?;

    if args.clean {
        unsync_vcs_hooks(&context).await?;

        done(format!("Successfully removed {} hooks", hook_names), true);
    } else if sync_vcs_hooks(&context, args.force).await? {
        done(format!("Successfully created {} hooks", hook_names), true);
    } else {
        done("Did not sync hooks".into(), true);
    }

    Ok(None)
}
