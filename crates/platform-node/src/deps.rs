use moon_action::ActionStatus;
use moon_config::NodePackageManager;
use moon_contract::SupportedPlatform;
use moon_error::map_io_to_fs_error;
use moon_lang::has_vendor_installed_dependencies;
use moon_lang_node::NODE;
use moon_logger::{color, debug, warn};
use moon_terminal::{label_checkpoint, Checkpoint};
use moon_toolchain::tools::node::NodeTool;
use moon_utils::{fs, is_ci, is_offline};
use moon_workspace::{Workspace, WorkspaceError};
use std::path::Path;

pub async fn install_node_modules(
    workspace: &Workspace,
    platform: &SupportedPlatform,
    node: &NodeTool,
    working_dir: &Path,
    project_id: Option<&str>,
    log_target: &str,
    has_modified_files: bool,
) -> Result<ActionStatus, WorkspaceError> {
    let mut cache = workspace
        .cache
        .cache_deps_state(platform, project_id)
        .await?;

    let pm = node.get_package_manager();
    let lock_filename = pm.get_lock_filename();
    let lock_filepath = working_dir.join(&lock_filename);
    let mut last_modified = 0;

    if lock_filepath.exists() {
        last_modified = cache.to_millis(
            fs::metadata(&lock_filepath)
                .await?
                .modified()
                .map_err(|e| map_io_to_fs_error(e, lock_filepath.clone()))?,
        );
    }

    // Install deps if the lockfile has been modified
    // since the last time dependencies were installed!
    if has_modified_files || last_modified == 0 || last_modified > cache.item.last_install_time {
        debug!(
            target: log_target,
            "Installing {} dependencies in {}",
            platform.label(),
            color::path(working_dir)
        );

        // When in CI, we can avoid installing dependencies because
        // we can assume they've already been installed before moon runs!
        if is_ci() && has_vendor_installed_dependencies(&working_dir, &NODE) {
            warn!(
                target: log_target,
                "In a CI environment and dependencies already exist, skipping install"
            );

            return Ok(ActionStatus::Skipped);
        }

        if is_offline() {
            warn!(
                target: log_target,
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

        pm.install_dependencies(node, working_dir).await?;

        if !is_ci() && node.config.dedupe_on_lockfile_change {
            debug!(target: log_target, "Dedupeing dependencies");

            pm.dedupe_dependencies(node, working_dir).await?;
        }

        // Update the cache with the timestamp
        cache.item.last_install_time = cache.now_millis();
        cache.save().await?;

        return Ok(ActionStatus::Passed);
    }

    debug!(
        target: log_target,
        "Lockfile has not changed since last install, skipping Node.js dependencies",
    );

    Ok(ActionStatus::Skipped)
}
