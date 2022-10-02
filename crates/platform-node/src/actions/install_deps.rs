use crate::deps::install_node_modules;
use moon_action::{Action, ActionContext, ActionStatus};
use moon_config::NodePackageManager;
use moon_contract::SupportedPlatform;
use moon_lang_node::{package::PackageJson, NPM};
use moon_logger::{color, debug};
use moon_toolchain::tools::node::NodeTool;
use moon_utils::fs;
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
    platform: &SupportedPlatform,
) -> Result<ActionStatus, WorkspaceError> {
    let workspace = workspace.read().await;
    let node = workspace.toolchain.node.get()?;
    let pm = node.get_package_manager();

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

    // Check if the lockfile or manifest has been modified anywhere in the workspace
    let lock_filename = pm.get_lock_filename();
    let manifest_filename = pm.get_manifest_filename();
    let has_modified_files = context
        .touched_files
        .iter()
        .any(|f| f.ends_with(&manifest_filename) || f.ends_with(&lock_filename));

    install_node_modules(
        &workspace,
        platform,
        node,
        &workspace.root,
        None,
        LOG_TARGET,
        has_modified_files,
    )
    .await
}
