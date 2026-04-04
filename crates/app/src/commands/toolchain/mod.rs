pub mod add;
pub mod download;
pub mod info;

use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum ToolchainCommands {
    #[command(name = "add", about = "Add and configure a toolchain plugin.")]
    Add(add::ToolchainAddArgs),

    #[command(
        name = "download",
        about = "Download all configured toolchain plugins."
    )]
    Download(download::ToolchainDownloadArgs),

    #[command(
        name = "info",
        about = "Show detailed information about a toolchain plugin."
    )]
    Info(info::ToolchainInfoArgs),
}
