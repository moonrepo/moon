pub mod add;
pub mod info;

use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum ToolchainCommands {
    #[command(
        name = "add",
        about = "Add and configure a toolchain plugin in .moon/toolchain.yml."
    )]
    Add(add::ToolchainAddArgs),

    #[command(
        name = "info",
        about = "Show detailed information about a toolchain plugin."
    )]
    Info(info::ToolchainInfoArgs),
}
