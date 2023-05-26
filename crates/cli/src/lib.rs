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
use crate::commands::init::{init, InitOptions};
use crate::commands::migrate;
use crate::commands::node;
use crate::commands::project::project;
use crate::commands::query::{self, QueryProjectsOptions, QueryTouchedFilesOptions};
use crate::commands::run::{run, RunOptions};
use crate::commands::setup::setup;
use crate::commands::sync::sync;
use crate::commands::task::task;
use crate::commands::teardown::teardown;
use crate::commands::upgrade::upgrade;
use crate::helpers::{check_for_new_version, setup_colors};
use app::{App as CLI, Commands, DockerCommands, MigrateCommands, NodeCommands, QueryCommands};
use clap::Parser;
use enums::{CacheMode, LogLevel};
use moon_logger::debug;
use query::QueryHashDiffOptions;
use starbase::{tracing::TracingOptions, App, AppResult};
use starbase_styles::color;
use starbase_utils::string_vec;
use std::env;

pub use app::BIN_NAME;

fn setup_logging(level: &LogLevel) {
    if env::var("MOON_LOG").is_err() {
        env::set_var("MOON_LOG", level.to_string());
    }

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

fn setup_caching(mode: &CacheMode) {
    if env::var("MOON_CACHE").is_err() {
        env::set_var("MOON_CACHE", mode.to_string());
    }
}

pub async fn run_cli() -> AppResult {
    App::setup_diagnostics();

    // Create app and parse arguments
    let args = CLI::parse();

    setup_colors(args.color);
    setup_logging(&args.log);
    setup_caching(&args.cache);

    App::setup_tracing_with_options(TracingOptions {
        filter_modules: string_vec!["moon", "proto", "starbase"],
        log_env: "MOON_LOG".into(),
        log_file: args.log_file,
        test_env: "MOON_TEST".into(),
        ..TracingOptions::default()
    });

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
        Commands::DepGraph { target, dot, json } => dep_graph(target, dot, json).await,
        Commands::Docker { command } => match command {
            DockerCommands::Prune => docker::prune().await,
            DockerCommands::Scaffold { ids, include } => docker::scaffold(&ids, &include).await,
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
        Commands::Init {
            dest,
            force,
            minimal,
            tool,
            yes,
        } => {
            init(
                dest,
                tool,
                InitOptions {
                    force,
                    minimal,
                    yes,
                },
            )
            .await
        }
        Commands::Migrate {
            command,
            skip_touched_files_check,
        } => match command {
            MigrateCommands::FromPackageJson { id } => {
                migrate::from_package_json(id, skip_touched_files_check).await
            }
            MigrateCommands::FromTurborepo => {
                migrate::from_turborepo(skip_touched_files_check).await
            }
        },
        Commands::Node { command } => match command {
            NodeCommands::RunScript { name, project } => node::run_script(name, project).await,
        },
        Commands::Project { id, json } => project(id, json).await,
        Commands::ProjectGraph { id, dot, json } => project_graph(id, dot, json).await,
        Commands::Query { command } => match command {
            QueryCommands::Hash { hash, json } => query::hash(&hash, json).await,
            QueryCommands::HashDiff { left, right, json } => {
                query::hash_diff(&QueryHashDiffOptions { json, left, right }).await
            }
            QueryCommands::Projects {
                alias,
                affected,
                id,
                json,
                language,
                query,
                source,
                tags,
                tasks,
                type_of,
            } => {
                query::projects(&QueryProjectsOptions {
                    alias,
                    affected,
                    id,
                    json,
                    language,
                    query,
                    source,
                    tags,
                    tasks,
                    type_of,
                })
                .await
            }
            QueryCommands::TouchedFiles {
                base,
                default_branch,
                head,
                json,
                local,
                status,
            } => {
                query::touched_files(&mut QueryTouchedFilesOptions {
                    base: base.unwrap_or_default(),
                    default_branch,
                    head: head.unwrap_or_default(),
                    json,
                    local,
                    log: false,
                    status,
                })
                .await
            }
            QueryCommands::Tasks {
                alias,
                affected,
                id,
                json,
                language,
                query,
                source,
                tasks,
                type_of,
            } => {
                query::tasks(&QueryProjectsOptions {
                    alias,
                    affected,
                    id,
                    json,
                    language,
                    query,
                    source,
                    tasks,
                    type_of,
                    ..QueryProjectsOptions::default()
                })
                .await
            }
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
        Commands::Sync => sync().await,
        Commands::Task { target, json } => task(target, json).await,
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
