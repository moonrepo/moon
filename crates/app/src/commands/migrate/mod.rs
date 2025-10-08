mod v2;

pub use v2::*;

use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum MigrateCommands {
    #[command(
        name = "v2",
        about = "Migrate an existing moon v1 workspace to moon v2."
    )]
    V2(MigrateV2Args),
}
