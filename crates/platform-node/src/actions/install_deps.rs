use moon_action::{Action, ActionContext, ActionStatus};
use moon_config::NodePackageManager;
use moon_error::map_io_to_fs_error;
use moon_lang::has_vendor_installed_dependencies;
use moon_lang_node::{package::PackageJson, NODE, NPM};
use moon_logger::{color, debug, warn};
use moon_terminal::{label_checkpoint, Checkpoint};
use moon_toolchain::tools::node::NodeTool;
use moon_utils::{fs, is_ci, is_offline};
use moon_workspace::{Workspace, WorkspaceError};
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:platform-node:install-deps";

/// Add `packageManager` to root `package.json`.
fn add_package_manager(node: &NodeTool, package_json: &mut PackageJson) -> bool {
    let manager_version = match node.config.package_manager {
        NodePackageManager::Npm => format!("npm@{}", node.config.npm.version),
        NodePackageManager::Pnpm => format!(
            "pnpm@{}",
            match &node.config.pnpm {
                Some(pnpm) => pnpm.version.clone(),
                None => {
                    return false;
                }
            }
        ),
        NodePackageManager::Yarn => format!(
            "yarn@{}",
            match &node.config.yarn {
                Some(yarn) => yarn.version.clone(),
                None => {
                    return false;
                }
            }
        ),
    };

    if manager_version != "npm@inherit"
        && node.is_corepack_aware()
        && package_json.set_package_manager(&manager_version)
    {
        debug!(
            target: LOG_TARGET,
            "Adding package manager version to root {}",
            color::file(&NPM.manifest_filename)
        );

        return true;
    }

    false
}

/// Add `engines` constraint to root `package.json`.
fn add_engines_constraint(node: &NodeTool, package_json: &mut PackageJson) -> bool {
    if node.config.add_engines_constraint && package_json.add_engine("node", &node.config.version) {
        debug!(
            target: LOG_TARGET,
            "Adding engines version constraint to root {}",
            color::file(&NPM.manifest_filename)
        );

        return true;
    }

    false
}

pub async fn install_deps(
    _action: &mut Action,
    context: &ActionContext,
    workspace: Arc<RwLock<Workspace>>,
) -> Result<ActionStatus, WorkspaceError> {
    let workspace = workspace.read().await;
    let node = workspace.toolchain.get_node()?;
    let mut cache = workspace.cache.cache_workspace_state().await?;

    // Sync values to root `package.json`
    PackageJson::sync(&workspace.root, |package_json| {
        add_package_manager(node, package_json);
        add_engines_constraint(node, package_json);

        Ok(())
    })?;

    // Create nvm/nodenv version file
    if let Some(version_manager) = &node.config.sync_version_manager_config {
        let rc_name = version_manager.get_config_filename();
        let rc_path = workspace.root.join(&rc_name);

        fs::write(&rc_path, &node.config.version).await?;

        debug!(
            target: LOG_TARGET,
            "Syncing Node.js version to root {}",
            color::file(&rc_name)
        );
    }

    // Get the last modified time of the root lockfile
    let manager = node.get_package_manager();
    let lockfile_name = manager.get_lock_filename();
    let lockfile = workspace.root.join(&lockfile_name);
    let mut last_modified = 0;

    if lockfile.exists() {
        let lockfile_metadata = fs::metadata(&lockfile).await?;

        last_modified = cache.to_millis(
            lockfile_metadata
                .modified()
                .map_err(|e| map_io_to_fs_error(e, lockfile.clone()))?,
        );
    }

    // If a `package.json` has been modified manually, we should account for that
    let has_modified_manifests = context
        .touched_files
        .iter()
        .any(|f| f.ends_with(&NPM.manifest_filename) || f.ends_with(&lockfile_name));

    // Install deps if the lockfile has been modified
    // since the last time dependencies were installed!
    if has_modified_manifests
        || last_modified == 0
        || last_modified > cache.item.last_node_install_time
    {
        debug!(target: LOG_TARGET, "Installing Node.js dependencies");

        // When in CI, we can avoid installing dependencies because
        // we can assume they've already been installed before moon runs!
        if is_ci() && has_vendor_installed_dependencies(&workspace.root, &NODE) {
            warn!(
                target: LOG_TARGET,
                "In a CI environment and dependencies already exist, skipping install"
            );

            return Ok(ActionStatus::Skipped);
        }

        if is_offline() {
            warn!(
                target: LOG_TARGET,
                "No internet connection, assuming offline and skipping install"
            );

            return Ok(ActionStatus::Skipped);
        }

        let install_command = match node.config.package_manager {
            NodePackageManager::Npm => "npm install",
            NodePackageManager::Pnpm => "pnpm install",
            NodePackageManager::Yarn => "yarn install",
        };

        println!("{}", label_checkpoint(install_command, Checkpoint::Pass));

        manager.install_dependencies(&workspace.toolchain).await?;

        if !is_ci() && node.config.dedupe_on_lockfile_change {
            debug!(target: LOG_TARGET, "Dedupeing dependencies");

            manager.dedupe_dependencies(&workspace.toolchain).await?;
        }

        // Update the cache with the timestamp
        cache.item.last_node_install_time = cache.now_millis();
        cache.save().await?;

        return Ok(ActionStatus::Passed);
    }

    debug!(
        target: LOG_TARGET,
        "Lockfile has not changed since last install, skipping Node.js dependencies",
    );

    Ok(ActionStatus::Skipped)
}
