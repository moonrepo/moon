use clap::{Parser, Subcommand};
use proto::ToolType;

#[derive(Debug, Parser)]
#[command(
    name = "proto",
    version,
    about,
    long_about = None,
    disable_colored_help = true,
    disable_help_subcommand = true,
    propagate_version = true,
    next_line_help = false,
    rename_all = "camelCase")]
struct App {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(name = "install", about = "Install a tool")]
    Install {
        #[arg(required = true, value_enum, help = "Name of tool to install")]
        tool: ToolType,

        #[arg(default_value = "latest", help = "Version of tool to install")]
        semver: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let app = App::parse();

    dbg!(&app);
}
