use moon_logger::warn;
use moon_workspace::Workspace;
use starbase::AppResult;
use starbase_styles::color;

pub async fn sync(workspace: Workspace) -> AppResult {
    warn!(
        "This command is deprecated. Use {} instead.",
        color::shell("moon sync projects")
    );

    crate::commands::syncs::projects::sync(workspace).await?;

    Ok(())
}
