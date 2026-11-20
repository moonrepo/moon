mod utils;
mod v2;
mod v3;

pub use v2::*;
pub use v3::*;

use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum MigrateCommands {
    #[command(
        name = "v2",
        about = "Migrate an existing moon v1 workspace to moon v2."
    )]
    V2(MigrateV2Args),

    #[command(
        name = "v3",
        about = "Migrate an existing moon v2 workspace to moon v3."
    )]
    V3(MigrateV3Args),
}
