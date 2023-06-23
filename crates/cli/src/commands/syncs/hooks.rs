use crate::helpers::create_progress_bar;
use moon::load_workspace;
use moon_actions::{sync_vcs_hooks, unsync_vcs_hooks};
use starbase::AppResult;
use starbase_styles::color;

pub struct SyncHooksOptions {
    pub clean: bool,
    pub force: bool,
}

pub async fn sync(options: SyncHooksOptions) -> AppResult {
    let workspace = load_workspace().await?;

    if workspace.config.vcs.hooks.is_empty() {
        println!(
            "No hooks available to sync. Configure them with the {} setting.",
            color::id("vcs.hooks")
        );
        println!(
            "Learn more: {}",
            color::url("https://moonrepo.dev/docs/guides/vcs-hooks")
        );

        return Ok(());
    }

    let done = create_progress_bar(format!("Syncing {} hooks...", workspace.config.vcs.manager));
    let hook_names = workspace
        .config
        .vcs
        .hooks
        .keys()
        .map(color::id)
        .collect::<Vec<_>>()
        .join(", ");

    if options.clean {
        unsync_vcs_hooks(&workspace).await?;

        done(format!("Successfully removed {} hooks", hook_names), true);
    } else {
        sync_vcs_hooks(&workspace, options.force).await?;

        done(format!("Successfully created {} hooks", hook_names), true);
    }

    Ok(())
}
