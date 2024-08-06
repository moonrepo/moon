mod lookup;

use clap::Parser;
use lookup::*;
use mimalloc::MiMalloc;
use moon_app::commands::docker::DockerCommands;
use moon_app::commands::migrate::MigrateCommands;
use moon_app::commands::node::NodeCommands;
use moon_app::commands::query::QueryCommands;
use moon_app::commands::sync::SyncCommands;
use moon_app::{commands, systems::bootstrap, Cli, CliSession, Commands, ExitCode};
use starbase::diagnostics::IntoDiagnostic;
use starbase::tracing::TracingOptions;
use starbase::{App, MainResult};
use starbase_styles::color;
use starbase_utils::{dirs, string_vec};
use std::env;
use std::process::{exit, Command};
use tracing::debug;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn get_version() -> String {
    let version = env!("CARGO_PKG_VERSION");

    env::set_var("MOON_VERSION", version);

    version.to_owned()
}

fn get_tracing_modules() -> Vec<String> {
    let mut modules = string_vec!["moon", "proto", "schematic", "starbase", "warpgate"];

    if env::var("MOON_DEBUG_WASM").is_ok() {
        modules.push("extism".into());
    } else {
        modules.push("extism::pdk".into());
    }

    modules
}

#[cfg(unix)]
fn exec_local_bin(mut command: Command) -> std::io::Result<()> {
    use std::os::unix::process::CommandExt;

    Err(command.exec())
}

#[cfg(windows)]
fn exec_local_bin(mut command: Command) -> std::io::Result<()> {
    let result = command.spawn()?.wait()?;

    if !result.success() {
        exit(result.code().unwrap_or(1));
    }

    Ok(())
}

#[tokio::main]
async fn main() -> MainResult {
    sigpipe::reset();
    // console_subscriber::init();

    // Detect info about the current process
    let version = get_version();
    let (args, has_executable) = bootstrap::gather_args();

    let cli = Cli::parse_from(&args);
    cli.setup_env_vars();

    // Setup diagnostics and tracing
    let app = App::default();
    app.setup_diagnostics();

    let _guard = app.setup_tracing(TracingOptions {
        dump_trace: cli.dump,
        filter_modules: get_tracing_modules(),
        intercept_log: true,
        log_env: "MOON_APP_LOG".into(), // Don't conflict with proto
        log_file: cli.log_file.clone(),
        ..TracingOptions::default()
    });

    if let Ok(exe) = env::current_exe() {
        debug!(
            args = ?args,
            "Running moon v{} (with {})",
            version,
            color::path(exe),
        );
    } else {
        debug!(args = ?args, "Running moon v{}", version);
    }

    // Detect if we've been installed globally
    if let (Some(home_dir), Ok(current_dir)) = (dirs::home_dir(), env::current_dir()) {
        if is_globally_installed(&home_dir) {
            if let Some(local_bin) = has_locally_installed(&home_dir, &current_dir) {
                debug!("Binary is running from a global path, but we found a local binary to use instead");
                debug!("Will now execute the local binary and replace this running process");

                let start_index = if has_executable { 1 } else { 0 };

                let mut command = Command::new(local_bin);
                command.args(&args[start_index..]);
                command.current_dir(current_dir);

                return exec_local_bin(command).into_diagnostic();
            }
        }
    }

    // Otherwise just run the CLI
    let result = app
        .run(CliSession::new(cli, version), |session| async {
            match session.cli.command.clone() {
                Commands::ActionGraph(args) => {
                    commands::graph::action::action_graph(session, args).await
                }
                Commands::Bin(args) => commands::bin::bin(session, args).await,
                Commands::Ci(args) => commands::ci::ci(session, args).await,
                Commands::Check(args) => commands::check::check(session, args).await,
                Commands::Clean(args) => commands::clean::clean(session, args).await,
                Commands::Completions(args) => {
                    commands::completions::completions(session, args).await
                }
                Commands::DepGraph(args) => commands::graph::dep::dep_graph(session, args).await,
                Commands::Docker { command } => match command {
                    DockerCommands::File(args) => commands::docker::file(session, args).await,
                    DockerCommands::Prune => commands::docker::prune(session).await,
                    DockerCommands::Scaffold(args) => {
                        commands::docker::scaffold(session, args).await
                    }
                    DockerCommands::Setup => commands::docker::setup(session).await,
                },
                Commands::Ext(args) => commands::ext::ext(session, args).await,
                Commands::Generate(args) => commands::generate::generate(session, args).await,
                Commands::Init(args) => commands::init::init(session, args).await,
                Commands::Migrate {
                    command,
                    skip_touched_files_check,
                } => match command {
                    MigrateCommands::FromPackageJson(mut args) => {
                        args.skip_touched_files_check = skip_touched_files_check;
                        commands::migrate::from_package_json(session, args).await
                    }
                    MigrateCommands::FromTurborepo => commands::migrate::from_turborepo().await,
                },
                Commands::Node { command } => match command {
                    NodeCommands::RunScript(args) => {
                        commands::node::run_script(session, args).await
                    }
                },
                Commands::Project(args) => commands::project::project(session, args).await,
                Commands::ProjectGraph(args) => {
                    commands::graph::project::project_graph(session, args).await
                }
                Commands::Query { command } => match command {
                    QueryCommands::Hash(args) => commands::query::hash(session, args).await,
                    QueryCommands::HashDiff(args) => {
                        commands::query::hash_diff(session, args).await
                    }
                    QueryCommands::Projects(args) => commands::query::projects(session, args).await,
                    QueryCommands::Tasks(args) => commands::query::tasks(session, args).await,
                    QueryCommands::TouchedFiles(args) => {
                        commands::query::touched_files(session, args).await
                    }
                },
                Commands::Run(args) => commands::run::run(session, args).await,
                Commands::Setup => commands::setup::setup(session).await,
                Commands::Sync { command } => match command {
                    Some(SyncCommands::Codeowners(args)) => {
                        commands::syncs::codeowners::sync(session, args).await
                    }
                    Some(SyncCommands::ConfigSchemas(args)) => {
                        commands::syncs::config_schemas::sync(session, args).await
                    }
                    Some(SyncCommands::Hooks(args)) => {
                        commands::syncs::hooks::sync(session, args).await
                    }
                    Some(SyncCommands::Projects) => commands::syncs::projects::sync(session).await,
                    None => commands::sync::sync(session).await,
                },
                Commands::Task(args) => commands::task::task(session, args).await,
                Commands::Teardown => commands::teardown::teardown().await,
                Commands::Templates => commands::templates::templates(session).await,
                Commands::Upgrade => commands::upgrade::upgrade(session).await,
            }
        })
        .await;

    if let Err(error) = result {
        // Rust crashes with a broken pipe error by default,
        // so we unfortunately need to work around it with this hack!
        // https://github.com/rust-lang/rust/issues/46016
        // if error.to_string().to_lowercase().contains("broken pipe") {
        //     exit(0);
        // }

        // If we received a custom exit code, use it
        if let Some(ExitCode(code)) = error.downcast_ref::<ExitCode>() {
            exit(*code);
        }

        return Err(error);
    }

    Ok(())
}
