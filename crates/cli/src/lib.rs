mod app;
pub mod commands;
pub mod enums;
mod helpers;
pub mod queries;
mod states;
mod systems;

use crate::helpers::setup_colors;
use app::App as CLI;
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
    let global_args = CLI::parse();

    setup_colors(global_args.color);
    setup_logging(&global_args.log);
    setup_caching(&global_args.cache);

    App::setup_tracing_with_options(TracingOptions {
        filter_modules: string_vec!["moon", "proto", "schematic", "starbase"],
        log_env: "STARBASE_LOG".into(),
        log_file: global_args.log_file,
        // test_env: "MOON_TEST".into(),
        ..TracingOptions::default()
    });

    detect_running_version();

    let mut app = App::new();
    app.startup(systems::load_workspace);
    app.execute(systems::check_for_new_version);
    app.execute(systems::run_command);
    app.run().await?;

    Ok(())
}
