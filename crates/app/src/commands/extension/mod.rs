pub mod add;
pub mod info;

use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum ExtensionCommands {
    #[command(name = "add", about = "Add and configure an extension plugin.")]
    Add(add::ExtensionAddArgs),

    #[command(
        name = "info",
        about = "Show detailed information about an extension plugin."
    )]
    Info(info::ExtensionInfoArgs),
}
