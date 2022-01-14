use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_project::Project;
use pathdiff::diff_paths;
use std::collections::HashSet;
use std::path::PathBuf;

#[allow(dead_code)]
async fn sync_project(
    workspace: &Workspace,
    project: &mut Project,
    synced: &mut HashSet<String>,
) -> Result<(), WorkspaceError> {
    let manager = workspace.toolchain.get_package_manager();
    let depends_on = project.get_dependencies();

    if depends_on.is_empty() || synced.contains(&project.id) {
        return Ok(());
    }

    let node_config = workspace.config.node.as_ref().unwrap();

    for dep in depends_on {
        let dep_project = workspace.projects.get(&dep)?;

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
                tsconfig.save()?;
            }
        }
    }

    synced.insert(project.id.to_owned());

    Ok(())
}

#[allow(dead_code)]
pub async fn sync_project_config_deps(
    workspace: &mut Workspace,
    project: &mut Project,
) -> Result<(), WorkspaceError> {
    let mut synced = HashSet::<String>::new();
    let node_config = workspace.config.node.as_ref().unwrap();

    // Sync all dependent projects first to ensure their configs are correct
    for dep in workspace.projects.get_dependencies_of(project)? {
        if dep != project.id {
            let mut dep_project = workspace.projects.get(&dep)?;

            sync_project(workspace, &mut dep_project, &mut synced).await?;
        }
    }

    // Sync a project reference to the root `tsconfig.json`
    if let Some(tsconfig) = &mut workspace.tsconfig_json {
        if node_config.sync_typescript_project_references.unwrap()
            && tsconfig.add_project_ref(project.source.to_owned())
        {
            tsconfig.save()?;
        }
    }

    // Then sync the current project last
    sync_project(workspace, project, &mut synced).await?;

    Ok(())
}
