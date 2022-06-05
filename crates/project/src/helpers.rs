use crate::errors::ProjectError;
use crate::types::ProjectsSourceMap;
use moon_error::MoonError;
use moon_logger::{color, warn};
use moon_utils::{glob, path, regex};
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
pub fn detect_projects_with_globs(
    workspace_root: &Path,
    globs: &[String],
    projects: &mut ProjectsSourceMap,
) -> Result<(), ProjectError> {
    for project_root in glob::walk(workspace_root, globs)? {
        if project_root.is_dir() {
            let project_source = project_root
                .strip_prefix(workspace_root)
                .unwrap()
                .to_str()
                .ok_or_else(|| MoonError::PathInvalidUTF8(project_root.clone()))?;

            let (id, source) = infer_project_name_and_source(project_source);
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
