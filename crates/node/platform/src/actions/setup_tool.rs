use moon_config::{NodeConfig, NodePackageManager, NodeVersionManager};
use moon_logger::debug;
use moon_node_lang::PackageJson;
use moon_node_tool::NodeTool;
use starbase_styles::color;
use starbase_utils::fs;
use std::path::Path;

const LOG_TARGET: &str = "moon:node-platform:setup-tool";

/// Add `packageManager` to `package.json`.
fn add_package_manager(node_config: &NodeConfig, package_json: &mut PackageJson) -> bool {
    let manager_version = match node_config.package_manager {
        // Not supported by corepack, so remove field
        NodePackageManager::Bun => Some(String::new()),
        NodePackageManager::Npm => node_config.npm.version.as_ref().map(|v| format!("npm@{v}")),
        NodePackageManager::Pnpm => node_config.pnpm.as_ref().map(|cfg| {
            cfg.version
                .as_ref()
                .map(|v| format!("pnpm@{v}"))
                .unwrap_or_default()
        }),
        NodePackageManager::Yarn => node_config.yarn.as_ref().map(|cfg| {
            cfg.version
                .as_ref()
                .map(|v| format!("yarn@{v}"))
                .unwrap_or_default()
        }),
    };

    if let Some(version) = manager_version {
        if !version.is_empty() && package_json.set_package_manager(&version) {
            debug!(
                target: LOG_TARGET,
                "Adding package manager version to {}",
                color::file("package.json")
            );

            return true;
        }
    }

    false
}

/// Add `engines` constraint to `package.json`.
fn add_engines_constraint(node_config: &NodeConfig, package_json: &mut PackageJson) -> bool {
    if let Some(node_version) = &node_config.version {
        if node_config.add_engines_constraint
            && package_json.add_engine("node", node_version.to_string())
        {
            debug!(
                target: LOG_TARGET,
                "Adding engines version constraint to {}",
                color::file("package.json")
            );

            return true;
        }
    }

    false
}

pub async fn setup_tool(node: &NodeTool, workspace_root: &Path) -> miette::Result<()> {
    // Find the `package.json` workspaces root
    let lockfile = match node.config.package_manager {
        NodePackageManager::Bun => "bun.lockb",
        NodePackageManager::Npm => "package-lock.json",
        NodePackageManager::Pnpm => "pnpm-lock.yaml",
        NodePackageManager::Yarn => "yarn.lock",
    };

    let packages_root = workspace_root.join(&node.config.packages_root);
    let packages_root = fs::find_upwards_root_until(lockfile, &packages_root, workspace_root)
        .unwrap_or(packages_root);

    // Sync values to root `package.json`
    PackageJson::sync(&packages_root, |package_json| {
        let added_manager = add_package_manager(&node.config, package_json);
        let added_constraint = add_engines_constraint(&node.config, package_json);

        Ok(added_manager || added_constraint)
    })?;

    // Create nvm/nodenv version file
    if let Some(version_manager) = &node.config.sync_version_manager_config {
        if let Some(node_version) = &node.config.version {
            let rc_name = match version_manager {
                NodeVersionManager::Nodenv => ".node-version".to_string(),
                NodeVersionManager::Nvm => ".nvmrc".to_string(),
            };
            let rc_path = packages_root.join(rc_name);

            fs::write_file(&rc_path, node_version.to_string())?;

            debug!(
                target: LOG_TARGET,
                "Syncing Node.js version to {}",
                color::path(&rc_path)
            );
        }
    }

    Ok(())
}
