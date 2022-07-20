mod app;
pub mod commands;
pub mod enums;
mod helpers;
pub mod queries;

use crate::commands::bin::bin;
use crate::commands::ci::{ci, CiOptions};
use crate::commands::dep_graph::dep_graph;
use crate::commands::init::{init, InitOptions};
use crate::commands::migrate::from_package_json;
use crate::commands::project::project;
use crate::commands::project_graph::project_graph;
use crate::commands::query::{self, QueryProjectsOptions, QueryTouchedFilesOptions};
use crate::commands::run::{run, RunOptions};
use crate::commands::setup::setup;
use crate::commands::sync::sync;
use crate::commands::teardown::teardown;
use crate::helpers::setup_colors;
use app::{App, Commands, MigrateCommands, QueryCommands};
use clap::Parser;
use console::Term;
use enums::LogLevel;
use moon_logger::{LevelFilter, Logger};
use moon_terminal::ExtendedTerm;
use std::env;

pub use app::BIN_NAME;

// This is annoying, but clap requires applying the `ValueEnum`
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

    setup_colors(args.color);

    // Setup logging
    if env::var("MOON_LOG").is_err() {
        env::set_var("MOON_LOG", args.log.to_string().to_lowercase());
    }

    Logger::init(map_log_level(args.log), args.log_file);

    // Setup caching
    if env::var("MOON_CACHE").is_err() {
        env::set_var("MOON_CACHE", args.cache.to_string().to_lowercase());
    }

    // Match and run subcommand
    let result = match &args.command {
        Commands::Bin { tool } => bin(tool).await,
        Commands::Ci {
            base,
            head,
            job,
            job_total,
        } => {
            ci(CiOptions {
                base: base.clone(),
                head: head.clone(),
                job: *job,
                job_total: *job_total,
            })
            .await
        }
        Commands::DepGraph { target } => dep_graph(target).await,
        Commands::Init {
            dest,
            force,
            inherit_projects,
            package_manager,
            yes,
        } => {
            init(
                dest,
                InitOptions {
                    force: *force,
                    inherit_projects: inherit_projects.clone(),
                    package_manager: package_manager.clone(),
                    yes: *yes,
                },
            )
            .await
        }
        Commands::Migrate { command } => match command {
            MigrateCommands::FromPackageJson { id } => from_package_json(id).await,
        },
        Commands::Project { id, json } => project(id, *json).await,
        Commands::ProjectGraph { id } => project_graph(id).await,
        Commands::Sync => sync().await,
        Commands::Query { command } => match command {
            QueryCommands::Projects {
                id,
                language,
                source,
                tasks,
                type_of,
            } => {
                query::projects(&QueryProjectsOptions {
                    id: id.clone(),
                    language: language.clone(),
                    source: source.clone(),
                    tasks: tasks.clone(),
                    type_of: type_of.clone(),
                })
                .await
            }
            QueryCommands::TouchedFiles {
                base,
                default_branch,
                head,
                local,
                status,
            } => {
                query::touched_files(&mut QueryTouchedFilesOptions {
                    base: base.clone().unwrap_or_default(),
                    default_branch: *default_branch,
                    head: head.clone().unwrap_or_default(),
                    local: *local,
                    log: false,
                    status: *status,
                })
                .await
            }
        },
        Commands::Run {
            target,
            affected,
            dependents,
            status,
            passthrough,
            profile,
            upstream,
        } => {
            run(
                target,
                RunOptions {
                    affected: *affected,
                    dependents: *dependents,
                    status: *status,
                    passthrough: passthrough.clone(),
                    profile: profile.clone(),
                    upstream: *upstream,
                },
            )
            .await
        }
        Commands::Setup => setup().await,
        Commands::Teardown => teardown().await,
    };

    if let Err(error) = result {
        Term::buffered_stderr().render_error(error);
    }
}
