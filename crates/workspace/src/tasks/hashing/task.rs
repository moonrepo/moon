use crate::{Workspace, WorkspaceError};
use moon_cache::Hasher;
use moon_project::{Project, Task};

pub async fn hash_task(
    workspace: &Workspace,
    project: &Project,
    task: &Task,
) -> Result<String, WorkspaceError> {
    let mut hasher = Hasher::new(workspace.config.node.version.clone());

    hasher.hash_project(project);
    hasher.hash_task(task);

    // Hash root configs first
    hasher.hash_package_json(&workspace.load_package_json().await?);

    if let Some(root_tsconfig) = workspace
        .load_tsconfig_json(&workspace.config.typescript.root_config_file_name)
        .await?
    {
        hasher.hash_tsconfig_json(&root_tsconfig);
    }

    // Hash project configs second so they can override
    if let Some(package) = project.load_package_json().await? {
        hasher.hash_package_json(&package);
    }

    if let Some(tsconfig) = project
        .load_tsconfig_json(&workspace.config.typescript.project_config_file_name)
        .await?
    {
        hasher.hash_tsconfig_json(&tsconfig);
    }

    Ok(hasher.to_hash())
}
