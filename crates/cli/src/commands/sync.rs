use moon_workspace::Workspace;
use starbase::system;
use starbase_styles::color;
use tracing::warn;

#[system]
pub async fn sync(workspace: ResourceMut<Workspace>) {
    warn!(
        "This command is deprecated. Use {} instead.",
        color::shell("moon sync projects")
    );

    crate::commands::syncs::projects::internal_sync(workspace).await?;
}
