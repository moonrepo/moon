use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_project::Project;
use pathdiff::diff_paths;
use std::collections::HashSet;
use std::path::PathBuf;

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

    for dep in depends_on {
        let dep_project = workspace.projects.get(&dep)?;

        // Update `dependencies` within `tsconfig.json`

        // Update `references` within `tsconfig.json`
        if let Some(tsconfig) = &mut project.tsconfig_json {
            let dep_ref_path = String::from(
                diff_paths(&project.root, &dep_project.root)
                    .unwrap_or_else(|| PathBuf::from("."))
                    .to_string_lossy(),
            );

            if tsconfig.add_project_ref(dep_ref_path) {
                tsconfig.save()?;
            }
        }
    }

    synced.insert(project.id.to_owned());

    Ok(())
}

pub async fn sync_project_config_deps(
    workspace: &mut Workspace,
    project: &mut Project,
) -> Result<(), WorkspaceError> {
    let mut synced = HashSet::<String>::new();

    // Sync all dependent projects first to ensure their configs are correct
    for dep in workspace.projects.get_dependencies_of(project)? {
        if dep != project.id {
            let mut dep_project = workspace.projects.get(&dep)?;

            sync_project(workspace, &mut dep_project, &mut synced).await?;
        }
    }

    // Sync a project reference to the root `tsconfig.json`
    if let Some(tsconfig) = &mut workspace.tsconfig_json {
        if tsconfig.add_project_ref(project.source.to_owned()) {
            tsconfig.save()?;
        }
    }

    // Then sync the current project last
    sync_project(workspace, project, &mut synced).await?;

    Ok(())
}
