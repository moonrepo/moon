use moon_action::Operation;
use moon_bun_tool::BunTool;
use moon_console::{Checkpoint, Console};
use moon_lang::has_vendor_installed_dependencies;
use moon_logger::{debug, error, info};
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

    for attempt in 1..=3 {
        if attempt == 1 {
            console.print_checkpoint(Checkpoint::Setup, "bun install")?;
        } else {
            console.print_checkpoint_with_comments(
                Checkpoint::Setup,
                "bun install",
                [format!("attempt {attempt} of 3")],
            )?;
        }

        let mut op = Operation::task_execution("bun install");
        let result = Operation::do_track_async(&mut op, || {
            bun.install_dependencies(&(), working_dir, !is_test_env())
        })
        .await;

        operations.push(op);

        if let Err(error) = result {
            if attempt == 3 {
                return Err(error);
            } else {
                error!("Failed to install Bun dependencies, retrying...");
            }
        } else {
            break;
        }
    }

    Ok(operations)
}
