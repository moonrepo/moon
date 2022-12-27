use moon_config::{NodePackageManager, NodeVersionManager};
use moon_lang::has_vendor_installed_dependencies;
use moon_logger::{color, debug, warn};
use moon_node_lang::{PackageJson, NODE, NODENV, NPM, NVM};
use moon_node_tool::NodeTool;
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_tool::ToolError;
use moon_utils::{fs, is_ci, is_test_env};
use std::path::Path;

const LOG_TARGET: &str = "moon:node-platform:install-deps";

/// Add `packageManager` to `package.json`.
fn add_package_manager(node: &NodeTool, package_json: &mut PackageJson) -> bool {
    let manager_version = match node.config.package_manager {
        NodePackageManager::Npm => format!("npm@{}", node.config.npm.version),
        NodePackageManager::Pnpm => format!(
            "pnpm@{}",
            match &node.config.pnpm {
                Some(pnpm) => &pnpm.version,
                None => {
                    return false;
                }
            }
        ),
        NodePackageManager::Yarn => format!(
            "yarn@{}",
            match &node.config.yarn {
                Some(yarn) => &yarn.version,
                None => {
                    return false;
                }
            }
        ),
    };

    if package_json.set_package_manager(&manager_version) {
        debug!(
            target: LOG_TARGET,
            "Adding package manager version to {}",
            color::file(NPM.manifest)
        );

        return true;
    }

    false
}

/// Add `engines` constraint to `package.json`.
fn add_engines_constraint(node: &NodeTool, package_json: &mut PackageJson) -> bool {
    if let Some(node_version) = &node.config.version {
        if node.config.add_engines_constraint && package_json.add_engine("node", node_version) {
            debug!(
                target: LOG_TARGET,
                "Adding engines version constraint to {}",
                color::file(NPM.manifest)
            );

            return true;
        }
    }

    false
}

pub async fn install_deps(node: &NodeTool, working_dir: &Path) -> Result<(), ToolError> {
    // When in CI, we can avoid installing dependencies because
    // we can assume they've already been installed before moon runs!
    if is_ci() && has_vendor_installed_dependencies(working_dir, &NODE) {
        warn!(
            target: LOG_TARGET,
            "In a CI environment and dependencies already exist, skipping install"
        );

        return Ok(());
    }

    // Sync values to `package.json`
    PackageJson::sync(working_dir, |package_json| {
        add_package_manager(node, package_json);
        add_engines_constraint(node, package_json);

        Ok(())
    })?;

    // Create nvm/nodenv version file
    if let Some(version_manager) = &node.config.sync_version_manager_config {
        let rc_name = match version_manager {
            NodeVersionManager::Nodenv => NODENV.version_file.to_string(),
            NodeVersionManager::Nvm => NVM.version_file.to_string(),
        };

        if let Some(node_version) = &node.config.version {
            let rc_path = working_dir.join(rc_name);

            fs::write(&rc_path, node_version)?;

            debug!(
                target: LOG_TARGET,
                "Syncing Node.js version to {}",
                color::path(&rc_path)
            );
        }
    }

    let package_manager = node.get_package_manager();

    // Install dependencies
    {
        debug!(target: LOG_TARGET, "Installing dependencies");

        print_checkpoint(
            match node.config.package_manager {
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
    if !is_ci() && node.config.dedupe_on_lockfile_change {
        debug!(target: LOG_TARGET, "Deduping dependencies");

        print_checkpoint(
            match node.config.package_manager {
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
