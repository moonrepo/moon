mod app;
mod commands;
mod config;
mod helpers;

use app::{App, Commands};
use clap::Parser;
use proto::color;
use std::process::exit;

#[tokio::main]
async fn main() {
    env_logger::builder().format_timestamp(None).init();

    let app = App::parse();

    let result = match app.command {
        Commands::Bin { tool, semver, shim } => commands::bin(tool, semver, shim).await,
        Commands::Completions { shell } => commands::completions(shell).await,
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
