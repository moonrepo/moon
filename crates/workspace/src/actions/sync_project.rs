use crate::action::ActionStatus;
use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_logger::{color, debug};
use moon_utils::is_ci;
use pathdiff::diff_paths;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

const TARGET: &str = "moon:action:sync-project";

pub async fn sync_project(
    workspace: Arc<RwLock<Workspace>>,
    project_id: &str,
) -> Result<ActionStatus, WorkspaceError> {
    let mut mutated_files = false;
    let mut sync_project_references;

    // Read only
    {
        let workspace = workspace.read().await;
        let node_config = &workspace.config.node;
        let typescript_config = &workspace.config.typescript;
        let tsconfig_branch_name = &typescript_config.project_config_file_name;
        let project = workspace.projects.load(project_id)?;

        // Copy values outside of this block
        sync_project_references = typescript_config.sync_project_references;

        // Sync each dependency to `tsconfig.json` and `package.json`
        let package_manager = workspace.toolchain.get_node_package_manager();
        let mut project_package_json = project.load_package_json().await?;
        let mut project_tsconfig_json = project.load_tsconfig_json(tsconfig_branch_name).await?;

        for dep_id in project.get_dependencies() {
            let dep_project = workspace.projects.load(&dep_id)?;

            // Update `dependencies` within this project's `package.json`
            if node_config.sync_project_workspace_dependencies {
                if let Some(package_json) = &mut project_package_json {
                    let dep_package_name =
                        dep_project.get_package_name().await?.unwrap_or_default();

                    // Only add if the dependent project has a `package.json`,
                    // and this `package.json` has not already declared the dep.
                    if !dep_package_name.is_empty()
                        && package_json.add_dependency(
                            dep_package_name,
                            package_manager.get_workspace_dependency_range(),
                            true,
                        )
                    {
                        debug!(
                            target: TARGET,
                            "Syncing {} as a dependency to {}'s {}",
                            color::id(&dep_id),
                            color::id(project_id),
                            color::file("package.json")
                        );

                        package_json.save().await?;
                        mutated_files = true;
                    }
                }
            }

            // Update `references` within this project's `tsconfig.json`
            if typescript_config.sync_project_references {
                if let Some(tsconfig_json) = &mut project_tsconfig_json {
                    let dep_ref_path = String::from(
                        diff_paths(&dep_project.root, &project.root)
                            .unwrap_or_else(|| PathBuf::from("."))
                            .to_string_lossy(),
                    );

                    // Only add if the dependent project has a `tsconfig.json`,
                    // and this `tsconfig.json` has not already declared the dep.
                    if dep_project.root.join(tsconfig_branch_name).exists()
                        && tsconfig_json.add_project_ref(&dep_ref_path, tsconfig_branch_name)
                    {
                        debug!(
                            target: TARGET,
                            "Syncing {} as a project reference to {}'s {}",
                            color::id(&dep_id),
                            color::id(project_id),
                            color::file(tsconfig_branch_name)
                        );

                        tsconfig_json.save().await?;
                        mutated_files = true;
                    }
                } else {
                    // Projects doesnt have a `tsconfig.json`
                    sync_project_references = false;
                }
            }
        }
    }

    // Writes root `tsconfig.json`
    {
        // Sync a project reference to the root `tsconfig.json`
        if sync_project_references {
            let workspace = workspace.write().await;
            let tsconfig_root_name = &workspace.config.typescript.root_config_file_name;
            let tsconfig_branch_name = &workspace.config.typescript.project_config_file_name;
            let project = workspace.projects.load(project_id)?;

            if let Some(mut tsconfig) = workspace.load_tsconfig_json(tsconfig_root_name).await? {
                if project.root.join(tsconfig_branch_name).exists()
                    && tsconfig.add_project_ref(&project.source, tsconfig_branch_name)
                {
                    debug!(
                        target: TARGET,
                        "Syncing {} as a project reference to the root {}",
                        color::id(project_id),
                        color::file(tsconfig_root_name)
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
