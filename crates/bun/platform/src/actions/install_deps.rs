use moon_action::Operation;
use moon_bun_tool::BunTool;
use moon_console::{Checkpoint, Console};
use moon_lang::has_vendor_installed_dependencies;
use moon_logger::{debug, info};
use moon_tool::DependencyManager;
use moon_utils::{is_ci, is_test_env};
use std::path::Path;

const LOG_TARGET: &str = "moon:bun-platform:install-deps";

pub async fn install_deps(
    bun: &BunTool,
    working_dir: &Path,
    console: &Console,
) -> miette::Result<Vec<Operation>> {
    let mut operations = vec![];

    // When in CI, we can avoid installing dependencies because
    // we can assume they've already been installed before moon runs!
    if is_ci() && has_vendor_installed_dependencies(working_dir, "node_modules") {
        info!(
            target: LOG_TARGET,
            "In a CI environment and dependencies already exist, skipping install"
        );

        return Ok(operations);
    }

    debug!(target: LOG_TARGET, "Installing dependencies");

    console
        .out
        .print_checkpoint(Checkpoint::Setup, "bun install")?;

    operations.push(
        Operation::task_execution("bun install")
            .track_async(|| bun.install_dependencies(&(), working_dir, !is_test_env()))
            .await?,
    );

    Ok(operations)
}
