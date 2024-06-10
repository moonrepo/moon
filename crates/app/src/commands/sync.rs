use crate::session::CliSession;
use starbase::AppResult;
use starbase_styles::color;
use tracing::warn;

pub async fn sync(session: CliSession) -> AppResult {
    warn!(
        "This command is deprecated. Use {} instead.",
        color::shell("moon sync projects")
    );

    crate::commands::syncs::projects::sync(session).await
}
