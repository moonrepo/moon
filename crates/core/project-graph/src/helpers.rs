use crate::errors::ProjectGraphError;
use moon_config::ProjectsSourcesMap;
use moon_logger::{color, warn};
use moon_utils::{fs, glob, path, regex};
use std::path::Path;

/// Infer a project name from a source path, by using the name of
/// the project folder.
pub fn infer_project_name_and_source(source: &str) -> (String, String) {
    let source = path::standardize_separators(source);

    if source.contains('/') {
        (source.split('/').last().unwrap().to_owned(), source)
    } else {
        (source.clone(), source)
    }
}

/// For each pattern in the globs list, glob the file system
/// for potential projects, and infer their name and source.
#[track_caller]
pub fn detect_projects_with_globs(
    workspace_root: &Path,
    globs: &[String],
    projects: &mut ProjectsSourcesMap,
) -> Result<(), ProjectGraphError> {
    let root_source = ".".to_owned();

    // Root-level project has special handling
    if globs.contains(&root_source) {
        let root_id = fs::file_name(workspace_root);

        projects.insert(
            regex::clean_id(if root_id.is_empty() {
                "root"
            } else {
                root_id.as_ref()
            }),
            root_source,
        );
    }

    // Glob for all other projects
    for project_root in glob::walk(workspace_root, globs)? {
        if project_root.is_dir() {
            let project_source =
                path::to_virtual_string(project_root.strip_prefix(workspace_root).unwrap())?;

            let (id, source) = infer_project_name_and_source(&project_source);
            let id = regex::clean_id(&id);

            if let Some(existing_source) = projects.get(&id) {
                warn!(
                    target: "moon:project",
                    "A project already exists for {} at source {}. Skipping conflicting source {}. Try renaming the project folder to make it unique.",
                    color::id(&id),
                    color::file(existing_source),
                    color::file(&source)
                );
            } else {
                projects.insert(id, source);
            }
        }
    }

    Ok(())
}
