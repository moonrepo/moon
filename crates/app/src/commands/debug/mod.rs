pub mod config;
pub mod vcs;

use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum DebugCommands {
    #[command(name = "config", about = "Debug loaded configuration.")]
    Config,

    #[command(name = "vcs", about = "Debug the VCS.")]
    Vcs,
}
