mod commands;
mod config;
mod helpers;

use clap::{Parser, Subcommand};
use proto::{color, ToolType};
use std::process::exit;

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
    command: Commands,
}

// TODO: alias, unalias, shell completions
#[derive(Debug, Subcommand)]
enum Commands {
    #[command(name = "bin", about = "Display the absolute path to a tools binary")]
    Bin {
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,

        #[arg(help = "Version of tool")]
        semver: Option<String>,

        #[arg(long, help = "Display shim path when available")]
        shim: bool,
    },

    #[command(name = "install", about = "Download and install a tool")]
    Install {
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,

        #[arg(default_value = "latest", help = "Version of tool")]
        semver: Option<String>,
    },

    #[command(name = "global", about = "Set the global default version of a tool")]
    Global {
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,

        #[arg(required = true, help = "Version of tool")]
        semver: String,
    },

    #[command(name = "list", alias = "ls", about = "List installed versions")]
    List {
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,
    },

    #[command(name = "list-remote", alias = "lsr", about = "List available versions")]
    ListRemote {
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,
    },

    #[command(
        name = "local",
        about = "Set the local version of a tool",
        long_about = "This will create a .prototools file (if it does not exist) in the current working directory with the appropriate tool and version."
    )]
    Local {
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,

        #[arg(required = true, help = "Version of tool")]
        semver: String,
    },

    #[command(
        name = "run",
        about = "Run a tool after detecting a version from the environment"
    )]
    Run {
        #[arg(required = true, value_enum, help = "Type of tool")]
        tool: ToolType,

        #[arg(help = "Version of tool")]
        semver: Option<String>,

        // Passthrough args (after --)
        #[arg(
            last = true,
            help = "Arguments to pass through to the underlying command"
        )]
        passthrough: Vec<String>,
    },

    #[command(name = "uninstall", about = "Uninstall a tool")]
    Uninstall {
        #[arg(required = true, value_enum, help = "Type of tool to uninstall")]
        tool: ToolType,

        #[arg(required = true, help = "Version of tool to uninstall")]
        semver: String,
    },
}

#[tokio::main]
async fn main() {
    env_logger::builder().format_timestamp(None).init();

    let app = App::parse();

    let result = match app.command {
        Commands::Bin { tool, semver, shim } => commands::bin(tool, semver, shim).await,
        Commands::Install { tool, semver } => commands::install(tool, semver).await,
        Commands::Global { tool, semver } => commands::global(tool, semver).await,
        Commands::List { tool } => commands::list(tool).await,
        Commands::ListRemote { tool } => commands::list_remote(tool).await,
        Commands::Local { tool, semver } => commands::local(tool, semver).await,
        Commands::Run {
            tool,
            semver,
            passthrough,
        } => commands::run(tool, semver, passthrough).await,
        Commands::Uninstall { tool, semver } => commands::uninstall(tool, semver).await,
    };

    if let Err(error) = result {
        eprintln!("{}", color::failure(error.to_string()));
        exit(1);
    }
}
