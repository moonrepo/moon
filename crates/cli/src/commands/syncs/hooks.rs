use crate::helpers::create_progress_bar;
use moon::load_workspace;
use moon_actions::sync_vcs_hooks;
use starbase::AppResult;

pub async fn sync() -> AppResult {
    let workspace = load_workspace().await?;

    let done = create_progress_bar(format!("Syncing {} hooks...", workspace.config.vcs.manager));

    sync_vcs_hooks(&workspace).await?;

    done(
        format!(
            "Successfully synced {} hooks",
            workspace.config.vcs.hooks.len()
        ),
        true,
    );

    Ok(())
}
