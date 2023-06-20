use crate::helpers::create_progress_bar;
use moon::load_workspace;
use moon_actions::sync_vcs_hooks;
use starbase::AppResult;
use starbase_styles::color;

pub async fn sync() -> AppResult {
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

    sync_vcs_hooks(&workspace).await?;

    done(
        format!(
            "Successfully synced {} hooks",
            workspace
                .config
                .vcs
                .hooks
                .keys()
                .map(color::id)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        true,
    );

    Ok(())
}
