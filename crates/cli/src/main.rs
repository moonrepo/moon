mod app;
mod commands;
mod helpers;

use app::{App, Commands, LogLevel};
use clap::Parser;
use commands::bin::bin;
use commands::init::init;
use commands::project::project;
use commands::project_graph::project_graph;
use commands::run::{run, RunOptions};
use commands::setup::setup;
use commands::teardown::teardown;
use console::Term;
use moon_logger::{LevelFilter, Logger};
use moon_terminal::ExtendedTerm;

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
        Commands::Init { dest, force } => {
            result = init(dest, *force).await;
        }
        Commands::Project { id, json } => {
            result = project(id, *json).await;
        }
        Commands::ProjectGraph { id } => {
            result = project_graph(id).await;
        }
        Commands::Run {
            target,
            affected,
            status,
        } => {
            result = run(
                target,
                RunOptions {
                    affected: *affected,
                    status: status.clone().unwrap_or_default(),
                },
            )
            .await;
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
