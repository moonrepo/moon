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
use crate::commands::teardown::teardown;
use crate::commands::upgrade::upgrade;
use crate::helpers::setup_colors;
use app::{App, Commands, DockerCommands, MigrateCommands, NodeCommands, QueryCommands};
use clap::Parser;
use console::Term;
use enums::{CacheMode, LogLevel};
use moon_launchpad::check_version;
use moon_logger::{color, debug, LevelFilter, Logger};
use moon_terminal::ExtendedTerm;
use std::env;
use std::path::PathBuf;

pub use app::BIN_NAME;

fn setup_logging(level: &LogLevel, log_file: Option<PathBuf>) {
    if env::var("MOON_LOG").is_err() {
        env::set_var("MOON_LOG", level.to_string());
    }

    // This is annoying, but clap requires applying the `ValueEnum`
    // trait onto the enum, which we can't apply to the log package.
    Logger::init(
        match level {
            LogLevel::Off => LevelFilter::Off,
            LogLevel::Error => LevelFilter::Error,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Trace => LevelFilter::Trace,
        },
        log_file,
    );

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

pub async fn run_cli() {
    // Create app and parse arguments
    let args = App::parse();

    setup_colors(args.color);
    setup_logging(&args.log, args.log_file);
    setup_caching(&args.cache);

    let version_check = tokio::spawn(check_version(env!("CARGO_PKG_VERSION")));

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
        Commands::Sync => sync().await,
        Commands::Query { command } => match command {
            QueryCommands::Projects {
                alias,
                affected,
                id,
                json,
                language,
                source,
                tasks,
                type_of,
            } => {
                query::projects(&QueryProjectsOptions {
                    alias,
                    affected,
                    id,
                    json,
                    language,
                    source,
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
                    source,
                    tasks,
                    type_of,
                })
                .await
            }
        },
        Commands::Run {
            targets,
            affected,
            dependents,
            force,
            interactive,
            update_cache,
            status,
            passthrough,
            profile,
            remote,
        } => {
            run(
                &targets,
                RunOptions {
                    affected,
                    concurrency: args.concurrency,
                    dependents,
                    force,
                    interactive,
                    status,
                    passthrough,
                    profile,
                    remote,
                    update_cache,
                },
            )
            .await
        }
        Commands::Upgrade => upgrade().await,
        Commands::Setup => setup().await,
        Commands::Teardown => teardown().await,
    };

    // Defer checking for a new version as it requires the workspace root
    // to exist. Otherwise, the `init` command would panic while checking!
    match version_check.await {
        Ok(Ok((newer_version, true))) => {
            println!(
                "There's a new version of moon! {newer_version}\n\
                Run `moon upgrade` or install from https://moonrepo.dev/docs/install",
            );
        }
        Ok(Err(error)) => {
            debug!(target: "moon:cli", "Failed to check for current version: {}", error);
        }
        _ => {}
    }

    if let Err(error) = result {
        let error_message = error.to_string();

        // Rust crashes with a broken pipe error by default,
        // so we unfortunately need to work around it with this hack!
        // https://github.com/rust-lang/rust/issues/46016
        if error_message.to_lowercase().contains("broken pipe") {
            std::process::exit(0);
        } else {
            Term::buffered_stderr().render_error(error);
        }
    }
}
