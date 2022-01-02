mod app;
mod commands;
mod helpers;
mod output;
mod terminal;

use app::{App, Commands, LogLevel};
use clap::Parser;
use commands::bin::bin;
use commands::project::project;
use commands::project_graph::project_graph;
use commands::run::run;
use commands::setup::setup;
use commands::teardown::teardown;
use console::Term;
use log::LevelFilter;
use moon_logger::Logger;
use terminal::ExtendedTerm;

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
            result = bin(tool).await;
        }
        Commands::Project { id, json } => {
            result = project(id, json).await;
        }
        Commands::ProjectGraph { id } => {
            result = project_graph(id).await;
        }
        Commands::Run { target, status } => {
            result = run(target, status).await;
        }
        Commands::Setup => {
            result = setup().await;
        }
        Commands::Teardown => {
            result = teardown().await;
        }
    }

    if let Err(error) = result {
        Term::buffered_stderr().render_error(error);
    }
}
