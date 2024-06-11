use super::syncs::codeowners::SyncCodeownersArgs;
use super::syncs::hooks::SyncHooksArgs;
use crate::session::CliSession;
use clap::Subcommand;
use starbase::AppResult;
use starbase_styles::color;
use tracing::warn;

#[derive(Clone, Debug, Subcommand)]
pub enum SyncCommands {
    #[command(
        name = "codeowners",
        about = "Aggregate and sync code owners to a `CODEOWNERS` file."
    )]
    Codeowners(SyncCodeownersArgs),

    #[command(
        name = "hooks",
        about = "Generate and sync hook scripts for the workspace configured VCS."
    )]
    Hooks(SyncHooksArgs),

    #[command(
        name = "projects",
        about = "Sync all projects and configs in the workspace."
    )]
    Projects,
}

pub async fn sync(session: CliSession) -> AppResult {
    warn!(
        "This command is deprecated. Use {} instead.",
        color::shell("moon sync projects")
    );

    crate::commands::syncs::projects::sync(session).await
}
