use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_logger::{color, debug};
use pathdiff::diff_paths;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn sync_project(
    workspace: Arc<RwLock<Workspace>>,
    project_id: &str,
) -> Result<(), WorkspaceError> {
    let workspace = workspace.read().await;
    let project = workspace.projects.get(project_id)?;

    // Sync a project reference to the root `tsconfig.json`
    let node_config = workspace.config.node.as_ref().unwrap();

    if node_config
        .sync_typescript_project_references
        .unwrap_or(true)
    {
        if let Some(mut tsconfig) = workspace.load_tsconfig_json().await? {
            if tsconfig.add_project_ref(project.source.to_owned()) {
                debug!(
                    target: "moon:task-runner:sync-project",
                    "Syncing {} as a project reference to the root {}",
                    color::id(project_id),
                    color::path("tsconfig.json")
                );

                tsconfig.save().await?;
            }
        }
    }

    // Sync each dependency to `tsconfig.json` and `package.json`
    let manager = workspace.toolchain.get_node_package_manager();

    for dep_id in project.get_dependencies() {
        let dep_project = workspace.projects.get(&dep_id)?;

        // Update `dependencies` within `tsconfig.json`
        if node_config
            .sync_project_workspace_dependencies
            .unwrap_or(true)
        {
            if let Some(mut package) = project.load_package_json().await? {
                let dep_package_name = dep_project.get_package_name().await?.unwrap_or_default();

                // Only add if the dependent project has a `package.json`,
                // and this `package.json` has not already declared the dep.
                if !dep_package_name.is_empty()
                    && package.add_dependency(
                        dep_package_name,
                        manager.get_workspace_dependency_range(),
                        true,
                    )
                {
                    debug!(
                        target: "moon:task-runner:sync-project",
                        "Syncing {} as a dependency to {}'s {}",
                        color::id(&dep_id),
                        color::id(project_id),
                        color::path("package.json")
                    );

                    package.save().await?;
                }
            }
        }

        // Update `references` within `tsconfig.json`
        if node_config
            .sync_typescript_project_references
            .unwrap_or(true)
        {
            if let Some(mut tsconfig) = project.load_tsconfig_json().await? {
                let dep_ref_path = String::from(
                    diff_paths(&project.root, &dep_project.root)
                        .unwrap_or_else(|| PathBuf::from("."))
                        .to_string_lossy(),
                );

                if tsconfig.add_project_ref(dep_ref_path) {
                    debug!(
                        target: "moon:task-runner:sync-project",
                        "Syncing {} as a project reference to {}'s {}",
                        color::id(&dep_id),
                        color::id(project_id),
                        color::path("tsconfig.json")
                    );

                    tsconfig.save().await?;
                }
            }
        }
    }

    Ok(())
}
