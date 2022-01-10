use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_config::json::JsonValue;
use moon_project::Project;
use pathdiff::diff_paths;
use std::collections::HashSet;
use std::path::PathBuf;

fn get_ref_path(item: &JsonValue) -> String {
    match item {
        JsonValue::Object(data) => {
            if let JsonValue::String(path) = &data["path"] {
                path.to_owned()
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

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

        // if let Some(package) = &project.package_json {}
        // Update `dependencies` within `tsconfig.json`

        // Update `references` within `tsconfig.json`
        if project.tsconfig_json.is_some() {
            let tsconfig_json = project.tsconfig_json.as_mut().unwrap();
            let tsconfig = &mut tsconfig_json.value;

            // Convert to a vec so we can sort it
            let mut references: Vec<JsonValue> = if tsconfig["references"].is_null() {
                vec![]
            } else {
                tsconfig["references"].members().cloned().collect()
            };

            // Check if the reference already exists
            let dep_reference_path = String::from(
                diff_paths(&project.root, &dep_project.root)
                    .unwrap_or_else(|| PathBuf::from("."))
                    .to_string_lossy(),
            );

            let has_reference = references
                .iter()
                .find(|item| get_ref_path(item) == dep_reference_path);

            // Add if it does not exist
            if has_reference.is_none() {
                let mut reference = JsonValue::new_object();
                reference["path"] = JsonValue::String(dep_reference_path);

                references.push(reference);
            }

            // Sort the references
            references.sort_by_key(get_ref_path);

            // Save and write to the file
            tsconfig["references"] = JsonValue::Array(references);
            tsconfig_json.save().unwrap();
        }
    }

    synced.insert(project.id.to_owned());

    Ok(())
}

pub async fn sync_project_config_deps(
    workspace: &Workspace,
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

    // Then sync the current project last
    sync_project(workspace, project, &mut synced).await?;

    Ok(())
}
