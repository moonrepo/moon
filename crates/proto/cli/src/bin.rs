mod commands;

use clap::{Parser, Subcommand};
use proto::ToolType;
use std::{env, process::exit};

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
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "proto=debug");
    }

    env_logger::builder().format_timestamp(None).init();

    let app = App::parse();

    let result = match app.command {
        Commands::Install { tool, semver } => commands::install::install(tool, semver).await,
    };

    if let Err(error) = result {
        eprintln!("{}", error);
        exit(1);
    }
}
