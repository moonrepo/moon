use crate::action::ActionStatus;
use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_logger::{color, debug};
use moon_utils::is_ci;
use pathdiff::diff_paths;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

const TARGET: &str = "moon:task-runner:sync-project";

pub async fn sync_project(
    workspace: Arc<RwLock<Workspace>>,
    project_id: &str,
) -> Result<ActionStatus, WorkspaceError> {
    let workspace = workspace.read().await;
    let project = workspace.projects.load(project_id)?;
    let mut mutated_files = false;

    // Sync a project reference to the root `tsconfig.json`
    let node_config = &workspace.config.node;
    let typescript_config = &workspace.config.typescript;
    let tsconfig_root_name = &typescript_config.root_config_file_name;
    let tsconfig_branch_name = &typescript_config.project_config_file_name;

    if typescript_config.sync_project_references {
        if let Some(mut tsconfig) = workspace.load_tsconfig_json(tsconfig_root_name).await? {
            if tsconfig.add_project_ref(&project.source, tsconfig_branch_name) {
                debug!(
                    target: TARGET,
                    "Syncing {} as a project reference to the root {}",
                    color::id(project_id),
                    color::path(tsconfig_root_name)
                );

                tsconfig.save().await?;
                mutated_files = true;
            }
        }
    }

    // Sync each dependency to `tsconfig.json` and `package.json`
    let manager = workspace.toolchain.get_node_package_manager();

    for dep_id in project.get_dependencies() {
        let dep_project = workspace.projects.load(&dep_id)?;

        // Update `dependencies` within `tsconfig.json`
        if node_config.sync_project_workspace_dependencies {
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
                        target: TARGET,
                        "Syncing {} as a dependency to {}'s {}",
                        color::id(&dep_id),
                        color::id(project_id),
                        color::path("package.json")
                    );

                    package.save().await?;
                    mutated_files = true;
                }
            }
        }

        // Update `references` within `tsconfig.json`
        if typescript_config.sync_project_references {
            if let Some(mut tsconfig) = project.load_tsconfig_json(tsconfig_branch_name).await? {
                let dep_ref_path = String::from(
                    diff_paths(&project.root, &dep_project.root)
                        .unwrap_or_else(|| PathBuf::from("."))
                        .to_string_lossy(),
                );

                if tsconfig.add_project_ref(&dep_ref_path, tsconfig_branch_name) {
                    debug!(
                        target: TARGET,
                        "Syncing {} as a project reference to {}'s {}",
                        color::id(&dep_id),
                        color::id(project_id),
                        color::path(tsconfig_branch_name)
                    );

                    tsconfig.save().await?;
                    mutated_files = true;
                }
            }
        }
    }

    if mutated_files {
        // If files have been modified in CI, we should update the status to warning,
        // as these modifications should be committed to the repo.
        if is_ci() {
            return Ok(ActionStatus::Invalid);
        } else {
            return Ok(ActionStatus::Passed);
        }
    }

    Ok(ActionStatus::Skipped)
}
