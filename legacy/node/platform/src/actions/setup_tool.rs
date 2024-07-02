use moon_config::{NodeConfig, NodePackageManager, NodeVersionManager};
use moon_logger::debug;
use moon_node_lang::PackageJsonCache;
use moon_node_tool::NodeTool;
use proto_core::UnresolvedVersionSpec;
use starbase_styles::color;
use starbase_utils::fs;
use std::path::Path;

const LOG_TARGET: &str = "moon:node-platform:setup-tool";

/// Add `packageManager` to `package.json`.
fn add_package_manager(node_config: &NodeConfig, package_json: &mut PackageJsonCache) -> bool {
    if !node_config.sync_package_manager_field {
        return false;
    }

    let format_version_value = |key: &str, config: Option<&UnresolvedVersionSpec>| -> String {
        // Only full versions are allowed
        if let Some(UnresolvedVersionSpec::Semantic(version)) = config {
            return format!("{key}@{version}");
        }

        String::new()
    };

    let version = match node_config.package_manager {
        // Not supported by corepack, so remove field
        NodePackageManager::Bun => String::new(),
        NodePackageManager::Npm => format_version_value("npm", node_config.npm.version.as_ref()),
        NodePackageManager::Pnpm => node_config
            .pnpm
            .as_ref()
            .map(|cfg| format_version_value("pnpm", cfg.version.as_ref()))
            .unwrap_or_default(),
        NodePackageManager::Yarn => node_config
            .yarn
            .as_ref()
            .map(|cfg| format_version_value("yarn", cfg.version.as_ref()))
            .unwrap_or_default(),
    };

    if package_json.set_package_manager(&version) {
        if version.is_empty() {
            debug!(
                target: LOG_TARGET,
                "Removing package manager version from {}",
                color::file("package.json")
            );
        } else {
            debug!(
                target: LOG_TARGET,
                "Adding package manager version to {}",
                color::file("package.json")
            );
        }

        return true;
    }

    false
}

/// Add `engines` constraint to `package.json`.
fn add_engines_constraint(node_config: &NodeConfig, package_json: &mut PackageJsonCache) -> bool {
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
    PackageJsonCache::sync(&packages_root, |package_json| {
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
