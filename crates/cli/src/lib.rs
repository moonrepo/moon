mod app;
pub mod commands;
pub mod enums;
mod helpers;
pub mod queries;

use crate::commands::bin::bin;
use crate::commands::check::{check, CheckOptions};
use crate::commands::ci::{ci, CiOptions};
use crate::commands::clean::{clean, CleanOptions};
use crate::commands::completions;
use crate::commands::docker;
use crate::commands::generate::{generate, GenerateOptions};
use crate::commands::graph::{dep::dep_graph, project::project_graph};
use crate::commands::init::init;
use crate::commands::migrate;
use crate::commands::node;
use crate::commands::project::project;
use crate::commands::query;
use crate::commands::run::{run, RunOptions};
use crate::commands::setup::setup;
use crate::commands::sync::sync;
use crate::commands::syncs;
use crate::commands::task::task;
use crate::commands::teardown::teardown;
use crate::commands::upgrade::upgrade;
use crate::helpers::{check_for_new_version, setup_colors};
use app::{
    App as CLI, Commands, DockerCommands, MigrateCommands, NodeCommands, QueryCommands,
    SyncCommands,
};
use clap::Parser;
use enums::{CacheMode, LogLevel};
use moon_logger::debug;
use starbase::{tracing::TracingOptions, App, AppResult};
use starbase_styles::color;
use starbase_utils::string_vec;
use std::env;

pub use app::BIN_NAME;

fn setup_logging(level: &LogLevel) {
    env::set_var("STARBASE_LOG", level.to_string());

    if env::var("MOON_LOG").is_err() {
        env::set_var("MOON_LOG", level.to_string());
    }
}

fn setup_caching(mode: &CacheMode) {
    if env::var("MOON_CACHE").is_err() {
        env::set_var("MOON_CACHE", mode.to_string());
    }

    if matches!(mode, CacheMode::Off | CacheMode::Write) {
        env::set_var("PROTO_CACHE", "off");
    }
}

fn detect_running_version() {
    let version = env!("CARGO_PKG_VERSION");

    if let Ok(exe_with) = env::var("MOON_EXECUTED_WITH") {
        debug!(
            target: "moon",
            "Running moon v{} (with {})",
            version,
            color::file(exe_with),
        );
    } else {
        debug!(target: "moon", "Running moon v{}", version);
    }

    env::set_var("MOON_VERSION", version);
}

pub async fn run_cli() -> AppResult {
    App::setup_diagnostics();

    // Create app and parse arguments
    let args = CLI::parse();

    setup_colors(args.color);
    setup_logging(&args.log);
    setup_caching(&args.cache);

    App::setup_tracing_with_options(TracingOptions {
        filter_modules: string_vec!["moon", "proto", "schematic", "starbase"],
        log_env: "STARBASE_LOG".into(),
        log_file: args.log_file,
        // test_env: "MOON_TEST".into(),
        ..TracingOptions::default()
    });

    detect_running_version();

    // Check for new version
    let version_handle = if matches!(
        &args.command,
        Commands::Check { .. } | Commands::Ci { .. } | Commands::Run { .. } | Commands::Sync { .. }
    ) {
        Some(tokio::spawn(check_for_new_version()))
    } else {
        None
    };

    // Match and run subcommand
    let result = match args.command {
        Commands::Bin { tool } => bin(tool).await,
        Commands::Ci {
            base,
            head,
            job,
            job_total,
        } => {
            ci(CiOptions {
                base,
                concurrency: args.concurrency,
                head,
                job,
                job_total,
            })
            .await
        }
        Commands::Check {
            ids,
            all,
            update_cache,
        } => {
            check(
                &ids,
                CheckOptions {
                    all,
                    concurrency: args.concurrency,
                    update_cache,
                },
            )
            .await
        }
        Commands::Clean { lifetime } => {
            clean(CleanOptions {
                cache_lifetime: lifetime.to_owned(),
            })
            .await
        }
        Commands::Completions { shell } => completions::completions(shell).await,
        Commands::DepGraph(args) => dep_graph(args).await,
        Commands::Docker { command } => match command {
            DockerCommands::Prune => docker::prune().await,
            DockerCommands::Scaffold(args) => docker::scaffold(args).await,
            DockerCommands::Setup => docker::setup().await,
        },
        Commands::Generate {
            name,
            dest,
            defaults,
            dry_run,
            force,
            template,
            vars,
        } => {
            generate(
                name,
                GenerateOptions {
                    defaults,
                    dest,
                    dry_run,
                    force,
                    template,
                    vars,
                },
            )
            .await
        }
        Commands::Init(args) => init(args).await,
        Commands::Migrate {
            command,
            skip_touched_files_check,
        } => match command {
            MigrateCommands::FromPackageJson(args) => {
                migrate::from_package_json(args, skip_touched_files_check).await
            }
            MigrateCommands::FromTurborepo => {
                migrate::from_turborepo(skip_touched_files_check).await
            }
        },
        Commands::Node { command } => match command {
            NodeCommands::RunScript(args) => node::run_script(args).await,
        },
        Commands::Project(args) => project(args).await,
        Commands::ProjectGraph(args) => project_graph(args).await,
        Commands::Query { command } => match command {
            QueryCommands::Hash(args) => query::hash(args).await,
            QueryCommands::HashDiff(args) => query::hash_diff(args).await,
            QueryCommands::Projects(args) => query::projects(args).await,
            QueryCommands::Tasks(args) => query::tasks(args).await,
            QueryCommands::TouchedFiles(args) => query::touched_files(args).await,
        },
        Commands::Run {
            affected,
            dependents,
            force,
            interactive,
            passthrough,
            profile,
            query,
            remote,
            status,
            targets,
            update_cache,
        } => {
            run(
                &targets,
                RunOptions {
                    affected,
                    concurrency: args.concurrency,
                    dependents,
                    force,
                    interactive,
                    passthrough,
                    profile,
                    query,
                    remote,
                    status,
                    update_cache,
                },
            )
            .await
        }
        Commands::Setup => setup().await,
        Commands::Sync { command } => match command {
            Some(SyncCommands::Codeowners(args)) => syncs::codeowners::sync(args).await,
            Some(SyncCommands::Hooks(args)) => syncs::hooks::sync(args).await,
            Some(SyncCommands::Projects) => syncs::projects::sync().await,
            None => sync().await,
        },
        Commands::Task(args) => task(args).await,
        Commands::Teardown => teardown().await,
        Commands::Upgrade => upgrade().await,
    };

    if let Some(version_check) = version_handle {
        let _ = version_check.await;
    }

    if let Err(error) = result {
        // Rust crashes with a broken pipe error by default,
        // so we unfortunately need to work around it with this hack!
        // https://github.com/rust-lang/rust/issues/46016
        if error.to_string().to_lowercase().contains("broken pipe") {
            std::process::exit(0);
        } else {
            return Err(error);
        }
    }

    Ok(())
}
