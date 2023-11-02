use moon_config::NodePackageManager;
use moon_lang::has_vendor_installed_dependencies;
use moon_logger::{debug, info};
use moon_node_lang::NODE;
use moon_node_tool::NodeTool;
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_utils::{is_ci, is_test_env};
use std::path::Path;

const LOG_TARGET: &str = "moon:node-platform:install-deps";

pub async fn install_deps(node: &NodeTool, working_dir: &Path) -> miette::Result<()> {
    // When in CI, we can avoid installing dependencies because
    // we can assume they've already been installed before moon runs!
    if is_ci() && has_vendor_installed_dependencies(working_dir, &NODE) {
        info!(
            target: LOG_TARGET,
            "In a CI environment and dependencies already exist, skipping install"
        );

        return Ok(());
    }

    let package_manager = node.get_package_manager();

    // Install dependencies
    {
        debug!(target: LOG_TARGET, "Installing dependencies");

        print_checkpoint(
            match node.config.package_manager {
                NodePackageManager::Bun => "bun install",
                NodePackageManager::Npm => "npm install",
                NodePackageManager::Pnpm => "pnpm install",
                NodePackageManager::Yarn => "yarn install",
            },
            Checkpoint::Setup,
        );

        package_manager
            .install_dependencies(node, working_dir, !is_test_env())
            .await?;
    }

    // Dedupe dependencies
    if !is_ci()
        && node.config.dedupe_on_lockfile_change
        && !matches!(node.config.package_manager, NodePackageManager::Bun)
    {
        debug!(target: LOG_TARGET, "Deduping dependencies");

        print_checkpoint(
            match node.config.package_manager {
                NodePackageManager::Bun => "bun dedupe",
                NodePackageManager::Npm => "npm dedupe",
                NodePackageManager::Pnpm => "pnpm dedupe",
                NodePackageManager::Yarn => "yarn dedupe",
            },
            Checkpoint::Setup,
        );

        package_manager
            .dedupe_dependencies(node, working_dir, !is_test_env())
            .await?;
    }

    Ok(())
}
