use starbase::system;
use starbase_styles::color;
use tracing::warn;

#[system]
pub async fn sync(resources: ResourcesMut) {
    warn!(
        "This command is deprecated. Use {} instead.",
        color::shell("moon sync projects")
    );

    crate::commands::syncs::projects::internal_sync(resources).await?;
}
