mod app;
mod commands;
mod helpers;
mod terminal;

use app::{App, Commands, LogLevel};
use clap::Parser;
use commands::bin::bin;
use commands::project::project;
use commands::project_graph::project_graph;
use commands::setup::setup;
use commands::teardown::teardown;
use log::LevelFilter;
use moon_logger::Logger;
use moon_workspace::Workspace;
use terminal::*;

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
    Workspace::load().unwrap() // TODO
}

#[tokio::main]
async fn main() {
    // Create app and parse arguments
    let args = App::parse();

    // Instantiate the logger
    Logger::init(map_log_level(args.log_level.unwrap_or_default()));

    // Match and run subcommand
    let result;

    match &args.command {
        Commands::Bin { tool } => {
            result = bin(load_workspace(), tool).await;
        }
        Commands::Project { id, json } => {
            result = project(load_workspace(), id, json).await;
        }
        Commands::ProjectGraph { id } => {
            result = project_graph(load_workspace(), id).await;
        }
        Commands::Setup => {
            result = setup(load_workspace()).await;
        }
        Commands::Teardown => {
            result = teardown(load_workspace()).await;
        }
    }

    if let Err(error) = result {
        Terminal::render_error(error);
    }
}
