use moon_bun_tool::BunTool;
use moon_lang::has_vendor_installed_dependencies;
use moon_logger::{debug, info};
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_tool::DependencyManager;
use moon_utils::{is_ci, is_test_env};
use std::path::Path;

const LOG_TARGET: &str = "moon:bun-platform:install-deps";

pub async fn install_deps(bun: &BunTool, working_dir: &Path) -> miette::Result<()> {
    // When in CI, we can avoid installing dependencies because
    // we can assume they've already been installed before moon runs!
    if is_ci() && has_vendor_installed_dependencies(working_dir, "node_modules") {
        info!(
            target: LOG_TARGET,
            "In a CI environment and dependencies already exist, skipping install"
        );

        return Ok(());
    }

    debug!(target: LOG_TARGET, "Installing dependencies");

    print_checkpoint("bun install", Checkpoint::Setup);

    bun.install_dependencies(&(), working_dir, !is_test_env())
        .await?;

    Ok(())
}
