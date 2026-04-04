pub mod add;
pub mod download;
pub mod info;

use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum ExtensionCommands {
    #[command(name = "add", about = "Add and configure an extension plugin.")]
    Add(add::ExtensionAddArgs),

    #[command(
        name = "download",
        about = "Download all configured extension plugins."
    )]
    Download(download::ExtensionDownloadArgs),

    #[command(
        name = "info",
        about = "Show detailed information about an extension plugin."
    )]
    Info(info::ExtensionInfoArgs),
}
