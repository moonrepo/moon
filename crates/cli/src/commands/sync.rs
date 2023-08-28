use moon_logger::warn;
use starbase::system;
use starbase_styles::color;

#[system]
pub async fn sync() {
    warn!(
        "This command is deprecated. Use {} instead.",
        color::shell("moon sync projects")
    );

    crate::commands::syncs::projects::internal_sync().await?;
}
