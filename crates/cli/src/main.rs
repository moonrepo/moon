mod app;
mod commands;
mod helpers;

use app::{App, Commands, LogLevel};
use clap::Parser;
use commands::bin::bin;
use commands::project::project;
use commands::project_graph::project_graph;
use commands::setup::setup;
use commands::teardown::teardown;
use log::LevelFilter;
use monolith_logger::Logger;
use monolith_workspace::Workspace;

// This is annoying, but clap requires applying the `ArgEnum`
// trait onto the enum, which we can't apply to the log package.
fn map_log_level(level: LogLevel) -> LevelFilter {
    match level {
        LogLevel::Off => LevelFilter::Off,
        LogLevel::Error => LevelFilter::Error,
        LogLevel::Warn => LevelFilter::Warn,
        LogLevel::Info => LevelFilter::Info,
        LogLevel::Debug => LevelFilter::Debug,
        LogLevel::Trace => LevelFilter::Trace,
    }
}

fn load_workspace() -> Workspace {
    Workspace::load().unwrap() // TODO error
}

#[tokio::main]
async fn main() {
    // Create app and parse arguments
    let args = App::parse();

    // Instantiate the logger
    Logger::init(map_log_level(args.log_level.unwrap_or_default()));

    // Match and run subcommand
    match &args.command {
        Commands::Bin { tool } => {
            bin(load_workspace(), tool).await.unwrap(); // TODO error
        }
        Commands::Project { id, json } => {
            project(load_workspace(), id, json).await.unwrap(); // TODO error
        }
        Commands::ProjectGraph { id } => {
            project_graph(load_workspace(), id).await.unwrap(); // TODO error
        }
        Commands::Setup => {
            setup(load_workspace()).await.unwrap(); // TODO error
        }
        Commands::Teardown => {
            teardown(load_workspace()).await.unwrap(); // TODO error
        }
    }
}
