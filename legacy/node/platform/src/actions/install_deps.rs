use moon_action::Operation;
use moon_config::NodePackageManager;
use moon_console::{Checkpoint, Console};
use moon_lang::has_vendor_installed_dependencies;
use moon_logger::{debug, error, info};
use moon_node_tool::NodeTool;
use moon_utils::{is_ci, is_test_env};
use std::path::Path;

const LOG_TARGET: &str = "moon:node-platform:install-deps";

pub async fn install_deps(
    node: &NodeTool,
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

    let package_manager = node.get_package_manager();

    // Install dependencies
    {
        debug!(target: LOG_TARGET, "Installing dependencies");

        let command = match node.config.package_manager {
            NodePackageManager::Bun => "bun install",
            NodePackageManager::Npm => "npm install",
            NodePackageManager::Pnpm => "pnpm install",
            NodePackageManager::Yarn => "yarn install",
        };

        for attempt in 1..=3 {
            if attempt == 1 {
                console.print_checkpoint(Checkpoint::Setup, command)?;
            } else {
                console.print_checkpoint_with_comments(
                    Checkpoint::Setup,
                    command,
                    [format!("attempt {attempt} of 3")],
                )?;
            }

            let mut op = Operation::task_execution(command);
            let result = Operation::do_track_async(&mut op, || {
                package_manager.install_dependencies(node, working_dir, !is_test_env())
            })
            .await;

            operations.push(op);

            if let Err(error) = result {
                if attempt == 3 {
                    return Err(error);
                } else {
                    error!(
                        "Failed to install {} dependencies, retrying...",
                        node.config.package_manager
                    );
                }
            } else {
                break;
            }
        }
    }

    // Dedupe dependencies
    if !is_ci()
        && node.config.dedupe_on_lockfile_change
        && !matches!(node.config.package_manager, NodePackageManager::Bun)
    {
        debug!(target: LOG_TARGET, "Deduping dependencies");

        let command = match node.config.package_manager {
            NodePackageManager::Bun => "bun dedupe",
            NodePackageManager::Npm => "npm dedupe",
            NodePackageManager::Pnpm => "pnpm dedupe",
            NodePackageManager::Yarn => "yarn dedupe",
        };

        console.print_checkpoint(Checkpoint::Setup, command)?;

        operations.push(
            Operation::task_execution(command)
                .track_async(|| {
                    package_manager.dedupe_dependencies(node, working_dir, !is_test_env())
                })
                .await?,
        );
    }

    Ok(operations)
}
