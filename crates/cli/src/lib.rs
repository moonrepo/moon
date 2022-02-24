mod app;
mod commands;
mod helpers;

use crate::commands::bin::bin;
use crate::commands::init::init;
use crate::commands::project::project;
use crate::commands::project_graph::project_graph;
use crate::commands::run::{run, RunOptions};
use crate::commands::setup::setup;
use crate::commands::teardown::teardown;
use app::{App, Commands, LogLevel};
use clap::Parser;
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

pub async fn run_cli() {
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
            local,
            status,
            passthrough,
        } => {
            result = run(
                target,
                RunOptions {
                    affected: *affected,
                    local: *local,
                    status: status.clone().unwrap_or_default(),
                    passthrough: passthrough.clone(),
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
