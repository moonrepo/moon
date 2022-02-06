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
    let mut workspace = workspace.write().await;
    let manager = workspace.toolchain.get_package_manager();
    let mut project = workspace.projects.get(project_id)?;

    // Sync a project reference to the root `tsconfig.json`
    let node_config = workspace.config.node.as_ref().unwrap();

    // if let Some(tsconfig) = &mut workspace.tsconfig_json {
    //     if node_config.sync_typescript_project_references.unwrap()
    //         && tsconfig.add_project_ref(project.source.to_owned())
    //     {
    //         debug!(
    //             target: "moon:task-runner:sync-project",
    //             "Syncing {} as a project reference to the root {}",
    //             color::id(project_id),
    //             color::path("tsconfig.json")
    //         );
    //         tsconfig.save()?;
    //     }
    // }

    // Sync each dependency to `tsconfig.json` and `package.json`
    for dep_id in project.get_dependencies() {
        let dep_project = workspace.projects.get(&dep_id)?;

        // Update `dependencies` within `tsconfig.json`
        if let Some(package) = &mut project.package_json {
            if let Some(package_deps) = &mut package.dependencies {
                let dep_package_name = dep_project.get_package_name().unwrap_or_default();

                // Only add if the dependent project has a `package.json`,
                // and this `package.json` has not already declared the dep.
                if node_config.sync_project_workspace_dependencies.unwrap()
                    && !dep_package_name.is_empty()
                    && !package_deps.contains_key(&dep_package_name)
                {
                    debug!(
                        target: "moon:task-runner:sync-project",
                        "Syncing {} as a dependency to {}'s {}",
                        color::id(&dep_id),
                        color::id(project_id),
                        color::path("package.json")
                    );

                    package_deps.insert(dep_package_name, manager.get_workspace_dependency_range());
                    package.save()?;
                }
            }
        }

        // Update `references` within `tsconfig.json`
        if let Some(tsconfig) = &mut project.tsconfig_json {
            let dep_ref_path = String::from(
                diff_paths(&project.root, &dep_project.root)
                    .unwrap_or_else(|| PathBuf::from("."))
                    .to_string_lossy(),
            );

            if node_config.sync_typescript_project_references.unwrap()
                && tsconfig.add_project_ref(dep_ref_path)
            {
                debug!(
                    target: "moon:task-runner:sync-project",
                    "Syncing {} as a project reference to {}'s {}",
                    color::id(&dep_id),
                    color::id(project_id),
                    color::path("tsconfig.json")
                );

                tsconfig.save()?;
            }
        }
    }

    Ok(())
}
