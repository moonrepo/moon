use super::action::*;
use crate::session::CliSession;
use moon_common::color;
use starbase::AppResult;
use tracing::warn;

pub async fn dep_graph(session: CliSession, args: ActionGraphArgs) -> AppResult {
    warn!(
        "This command is deprecated. Use {} instead.",
        color::shell("moon action-graph")
    );

    action_graph(session, args).await
}
