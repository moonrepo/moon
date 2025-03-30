pub mod vcs;

use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum DebugCommands {
    #[command(name = "vcs", about = "Debug the VCS.")]
    Vcs,
}
