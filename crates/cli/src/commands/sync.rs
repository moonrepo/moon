use moon_logger::warn;
use starbase::AppResult;
use starbase_styles::color;

pub async fn sync() -> AppResult {
    warn!(
        "This command is deprecated. Use {} instead.",
        color::shell("moon sync projects")
    );

    crate::commands::syncs::projects::sync().await?;

    Ok(())
}
