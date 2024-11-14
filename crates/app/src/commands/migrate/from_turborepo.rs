use starbase::AppResult;
use starbase_styles::color;
use tracing::warn;

pub async fn from_turborepo() -> AppResult {
    warn!(
        "This command is deprecated. Use {} instead.",
        color::shell("moon ext migrate-turborepo")
    );

    Ok(None)
}
