mod app;
pub mod commands;
pub mod enums;
mod helpers;
pub mod queries;
mod resources;
mod systems;

use crate::app::{
    Commands, DockerCommands, MigrateCommands, NodeCommands, QueryCommands, SyncCommands,
};
use crate::commands::bin::bin;
use crate::commands::check::check;
use crate::commands::ci::ci;
use crate::commands::clean::clean;
use crate::commands::completions;
use crate::commands::docker;
use crate::commands::generate::generate;
use crate::commands::graph::{action::action_graph, dep::dep_graph, project::project_graph};
use crate::commands::init::init;
use crate::commands::migrate;
use crate::commands::node;
use crate::commands::project::project;
use crate::commands::query;
use crate::commands::run::run;
use crate::commands::setup::setup;
use crate::commands::sync::sync;
use crate::commands::syncs;
use crate::commands::task::task;
use crate::commands::teardown::teardown;
use crate::commands::upgrade::upgrade;
use crate::helpers::setup_colors;
use app::App as CLI;
use clap::Parser;
use commands::migrate::FromTurborepoArgs;
use enums::{CacheMode, LogLevel};
use moon_common::consts::BIN_NAME;
use starbase::{tracing::TracingOptions, App, AppResult};
use starbase_styles::color;
use starbase_utils::string_vec;
use std::env;
use std::ffi::OsString;
use systems::{requires_toolchain, requires_workspace};
use tracing::debug;

fn setup_logging(level: &LogLevel) {
    env::set_var("MOON_APP_LOG", level.to_string());

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

fn detect_running_version(args: &[OsString]) {
    let version = env!("CARGO_PKG_VERSION");

    if let Ok(exe_with) = env::var("MOON_EXECUTED_WITH") {
        debug!(
            args = ?args,
            "Running moon v{} (with {})",
            version,
            color::file(exe_with)
        );
    } else {
        debug!(args = ?args, "Running moon v{}", version);
    }

    env::set_var("MOON_VERSION", version);
}

fn gather_args() -> Vec<OsString> {
    let mut args: Vec<OsString> = vec![];
    let mut leading_args: Vec<OsString> = vec![];
    let mut check_for_target = true;

    env::args_os().enumerate().for_each(|(index, arg)| {
        if let Some(a) = arg.to_str() {
            // Script being executed, so persist it
            if index == 0 && a.ends_with(BIN_NAME) {
                leading_args.push(arg);
                return;
            }

            // Find first non-option value
            if check_for_target && !a.starts_with('-') {
                check_for_target = false;

                // Looks like a target, but is not `run`, so prepend!
                if a.contains(':') {
                    leading_args.push(OsString::from("run"));
                }
            }
        }

        args.push(arg);
    });

    // We need a separate args list because options before the
    // target cannot be placed before "run"
    leading_args.extend(args);

    leading_args
}

pub async fn run_cli() -> AppResult {
    App::setup_diagnostics();

    // Create app and parse arguments
    let args = gather_args();
    let cli = CLI::parse_from(&args);

    setup_colors(cli.color);
    setup_logging(&cli.log);
    setup_caching(&cli.cache);

    App::setup_tracing_with_options(TracingOptions {
        filter_modules: string_vec!["moon", "proto", "schematic", "starbase", "warpgate"],
        log_env: "MOON_APP_LOG".into(),
        log_file: cli.log_file.clone(),
        // test_env: "MOON_TEST".into(),
        ..TracingOptions::default()
    });

    detect_running_version(&args);

    let mut app = App::new();
    app.set_state(cli.global_args());
    app.set_state(cli.clone());

    if requires_workspace(&cli) {
        app.startup(systems::load_workspace);
        app.startup(systems::install_proto);

        if requires_toolchain(&cli) {
            app.analyze(systems::load_toolchain);
        }
    }

    match cli.command {
        Commands::ActionGraph(args) => app.execute_with_args(action_graph, args),
        Commands::Bin(args) => app.execute_with_args(bin, args),
        Commands::Ci(args) => {
            app.execute(systems::check_for_new_version);
            app.execute_with_args(ci, args)
        }
        Commands::Check(args) => {
            app.execute(systems::check_for_new_version);
            app.execute_with_args(check, args)
        }
        Commands::Clean(args) => app.execute_with_args(clean, args),
        Commands::Completions(args) => app.execute_with_args(completions::completions, args),
        Commands::DepGraph(args) => app.execute_with_args(dep_graph, args),
        Commands::Docker { command } => match command {
            DockerCommands::Prune => app.execute(docker::prune),
            DockerCommands::Scaffold(args) => app.execute_with_args(docker::scaffold, args),
            DockerCommands::Setup => app.execute(docker::setup),
        },
        Commands::Generate(args) => app.execute_with_args(generate, args),
        Commands::Init(args) => app.execute_with_args(init, args),
        Commands::Migrate {
            command,
            skip_touched_files_check,
        } => match command {
            MigrateCommands::FromPackageJson(mut args) => {
                args.skip_touched_files_check = skip_touched_files_check;
                app.execute_with_args(migrate::from_package_json, args)
            }
            MigrateCommands::FromTurborepo => app.execute_with_args(
                migrate::from_turborepo,
                FromTurborepoArgs {
                    skip_touched_files_check,
                },
            ),
        },
        Commands::Node { command } => match command {
            NodeCommands::RunScript(args) => app.execute_with_args(node::run_script, args),
        },
        Commands::Project(args) => app.execute_with_args(project, args),
        Commands::ProjectGraph(args) => app.execute_with_args(project_graph, args),
        Commands::Query { command } => match command {
            QueryCommands::Hash(args) => app.execute_with_args(query::hash, args),
            QueryCommands::HashDiff(args) => app.execute_with_args(query::hash_diff, args),
            QueryCommands::Projects(args) => app.execute_with_args(query::projects, args),
            QueryCommands::Tasks(args) => app.execute_with_args(query::tasks, args),
            QueryCommands::TouchedFiles(args) => app.execute_with_args(query::touched_files, args),
        },
        Commands::Run(args) => {
            app.execute(systems::check_for_new_version);
            app.execute_with_args(run, args)
        }
        Commands::Setup => app.execute(setup),
        Commands::Sync { command } => {
            app.execute(systems::check_for_new_version);

            match command {
                Some(SyncCommands::Codeowners(args)) => {
                    app.execute_with_args(syncs::codeowners::sync, args)
                }
                Some(SyncCommands::Hooks(args)) => app.execute_with_args(syncs::hooks::sync, args),
                Some(SyncCommands::Projects) => app.execute(syncs::projects::sync),
                None => app.execute(sync),
            }
        }
        Commands::Task(args) => app.execute_with_args(task, args),
        Commands::Teardown => app.execute(teardown),
        Commands::Upgrade => app.execute(upgrade),
    };

    let result = app.run().await;

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
