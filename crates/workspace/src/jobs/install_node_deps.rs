use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use std::fs;
use std::time::SystemTime;

#[allow(dead_code)]
pub async fn install_node_deps(workspace: &Workspace) -> Result<(), WorkspaceError> {
    let mut cache = workspace.cache.workspace_state().await?;
    let toolchain = &workspace.toolchain;
    let manager = toolchain.get_package_manager();

    // Get the last modified time of the root lockfile
    let current_time = workspace.cache.to_millis(SystemTime::now());
    let last_modified = workspace
        .cache
        .to_millis(fs::metadata(&workspace.root.join(manager.get_lockfile_name()))?.modified()?);

    // Install deps if the lockfile has been modified
    // since the last time dependencies were installed!
    if last_modified > current_time {
        manager.install_deps(toolchain).await?;

        if let Some(node_config) = &workspace.config.node {
            if node_config.dedupe_on_install.unwrap_or(true) {
                manager.dedupe_deps(toolchain).await?;
            }
        }
    }

    // Update the cache with the timestamp
    cache.item.last_node_install = current_time;
    cache.save().await?;

    Ok(())
}
