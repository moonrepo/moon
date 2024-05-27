pub mod analyze;
pub mod execute;
pub mod shutdown;
pub mod startup;

use crate::app::{App as CLI, Commands};

pub fn requires_workspace(cli: &CLI) -> bool {
    !matches!(
        cli.command,
        Commands::Completions(_) | Commands::Init(_) | Commands::Setup | Commands::Upgrade
    )
}

pub fn requires_toolchain(cli: &CLI) -> bool {
    matches!(
        cli.command,
        Commands::Bin(_) | Commands::Docker { .. } | Commands::Node { .. } | Commands::Teardown
    )
}
