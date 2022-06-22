use crate::action::ActionStatus;
use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_config::PackageManager;
use moon_error::map_io_to_fs_error;
use moon_logger::{color, debug, warn};
use moon_terminal::output::{label_checkpoint, Checkpoint};
use moon_utils::{fs, is_offline};
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action:install-node-deps";

/// Add `packageManager` to root `package.json`.
#[track_caller]
fn add_package_manager(workspace: &mut Workspace) -> bool {
    let manager_version = match workspace.config.node.package_manager {
        PackageManager::Npm => format!("npm@{}", workspace.config.node.npm.version),
        PackageManager::Pnpm => format!(
            "pnpm@{}",
            workspace.config.node.pnpm.as_ref().unwrap().version
        ),
        PackageManager::Yarn => format!(
            "yarn@{}",
            workspace.config.node.yarn.as_ref().unwrap().version
        ),
    };

    if manager_version != "npm@inherit"
        && workspace.toolchain.get_node().is_corepack_aware()
        && workspace.package_json.set_package_manager(&manager_version)
    {
        debug!(
            target: LOG_TARGET,
            "Adding package manager version to root {}",
            color::file("package.json")
        );

        return true;
    }

    false
}

/// Add `engines` constraint to root `package.json`.
fn add_engines_constraint(workspace: &mut Workspace) -> bool {
    if workspace.config.node.add_engines_constraint
        && workspace
            .package_json
            .add_engine("node", &workspace.config.node.version)
    {
        debug!(
            target: LOG_TARGET,
            "Adding engines version constraint to root {}",
            color::file("package.json")
        );

        return true;
    }

    false
}

pub async fn install_node_deps(
    workspace: Arc<RwLock<Workspace>>,
) -> Result<ActionStatus, WorkspaceError> {
    // Writes root `package.json`
    {
        let mut workspace = workspace.write().await;
        let added_manager = add_package_manager(&mut workspace);
        let added_engines = add_engines_constraint(&mut workspace);

        if added_manager || added_engines {
            workspace.package_json.save().await?;
        }
    }

    // Read only
    {
        let workspace = workspace.read().await;
        let mut cache = workspace.cache.cache_workspace_state().await?;
        let manager = workspace.toolchain.get_node().get_package_manager();
        let node_config = &workspace.config.node;

        // Create nvm/nodenv config file
        if let Some(version_manager) = &node_config.sync_version_manager_config {
            let rc_name = version_manager.get_config_filename();
            let rc_path = workspace.root.join(&rc_name);

            fs::write(&rc_path, &node_config.version).await?;

            debug!(
                target: LOG_TARGET,
                "Syncing Node.js version to root {}",
                color::file(&rc_name)
            );
        }

        // Get the last modified time of the root lockfile
        let lockfile = workspace.root.join(manager.get_lock_filename());
        let mut last_modified = 0;

        if lockfile.exists() {
            let lockfile_metadata = fs::metadata(&lockfile).await?;

            last_modified = cache.to_millis(
                lockfile_metadata
                    .modified()
                    .map_err(|e| map_io_to_fs_error(e, lockfile.clone()))?,
            );
        }

        // Install deps if the lockfile has been modified
        // since the last time dependencies were installed!
        if last_modified == 0 || last_modified > cache.item.last_node_install_time {
            debug!(target: LOG_TARGET, "Installing Node.js dependencies");

            if is_offline() {
                warn!(
                    target: LOG_TARGET,
                    "No internet connection, assuming offline and skipping install"
                );

                return Ok(ActionStatus::Skipped);
            }

            let install_command = match workspace.config.node.package_manager {
                PackageManager::Npm => "npm install",
                PackageManager::Pnpm => "pnpm install",
                PackageManager::Yarn => "yarn install",
            };

            println!("{}", label_checkpoint(install_command, Checkpoint::Pass));

            manager.install_dependencies(&workspace.toolchain).await?;

            if node_config.dedupe_on_lockfile_change {
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
    }

    Ok(ActionStatus::Skipped)
}
