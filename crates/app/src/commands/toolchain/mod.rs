pub mod info;

use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum ToolchainCommands {
    #[command(
        name = "info",
        about = "Show detailed information about a toolchain plugin."
    )]
    Info(info::ToolchainInfoArgs),
}
