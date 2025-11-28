pub mod codeowners;
pub mod config_schemas;
pub mod projects;
pub mod vcs_hooks;

use clap::Subcommand;
use codeowners::SyncCodeownersArgs;
use config_schemas::SyncConfigSchemasArgs;
use vcs_hooks::SyncVcsHooksArgs;

#[derive(Clone, Debug, Subcommand)]
pub enum SyncCommands {
    #[command(
        name = "code-owners",
        alias = "codeowners",
        about = "Sync aggregated code owners to a `CODEOWNERS` file."
    )]
    Codeowners(SyncCodeownersArgs),

    #[command(
        name = "config-schemas",
        alias = "schemas",
        about = "Sync and generate configuration JSON schemas for use within editors."
    )]
    ConfigSchemas(SyncConfigSchemasArgs),

    #[command(
        name = "projects",
        about = "Sync all projects and configs in the workspace."
    )]
    Projects,

    #[command(
        name = "vcs-hooks",
        alias = "hooks",
        about = "Sync and generate hook scripts for the workspace configured VCS."
    )]
    VcsHooks(SyncVcsHooksArgs),
}
