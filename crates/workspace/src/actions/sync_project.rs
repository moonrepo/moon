use crate::action::ActionStatus;
use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_config::{tsconfig::TsConfigJson, TypeScriptConfig};
use moon_logger::{color, debug};
use moon_project::Project;
use moon_utils::is_ci;
use pathdiff::diff_paths;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

const TARGET: &str = "moon:action:sync-project";

fn sync_root_tsconfig(
    tsconfig: &mut TsConfigJson,
    typescript_config: &TypeScriptConfig,
    project: &Project,
) -> bool {
    if project
        .root
        .join(&typescript_config.project_config_file_name)
        .exists()
        && tsconfig.add_project_ref(&project.source, &typescript_config.project_config_file_name)
    {
        debug!(
            target: TARGET,
            "Syncing {} as a project reference to the root {}",
            color::id(&project.id),
            color::file(&typescript_config.root_config_file_name)
        );

        return true;
    }

    false
}

pub async fn sync_project(
    workspace: Arc<RwLock<Workspace>>,
    project_id: &str,
) -> Result<ActionStatus, WorkspaceError> {
    let mut mutated_files = false;
    let mut typescript_config;

    // Read only
    {
        let workspace = workspace.read().await;
        let project = workspace.projects.load(project_id)?;
        let node_config = &workspace.config.node;

        // Copy values outside of this block
        typescript_config = workspace.config.typescript.clone();

        // Sync each dependency to `tsconfig.json` and `package.json`
        let package_manager = workspace.toolchain.get_node().get_package_manager();
        let mut project_package_json = project.load_package_json().await?;
        let mut project_tsconfig_json = project
            .load_tsconfig_json(&typescript_config.project_config_file_name)
            .await?;

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
                    let tsconfig_branch_name = &typescript_config.project_config_file_name;
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
                    typescript_config.sync_project_references = false;
                }
            }
        }
    }

    // Writes root `tsconfig.json`
    {
        // Sync a project reference
        if typescript_config.sync_project_references {
            let mut workspace = workspace.write().await;
            let project = workspace.projects.load(project_id)?;

            if let Some(tsconfig) = &mut workspace.tsconfig_json {
                if sync_root_tsconfig(tsconfig, &typescript_config, &project) {
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
