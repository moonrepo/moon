use crate::workspace_builder::WorkspaceBuilderContext;
use moon_common::path::{WorkspaceRelativePathBuf, is_root_level_source, to_virtual_string};
use moon_common::{Id, color, consts};
use moon_config::{ProjectSourceEntry, ProjectsSourcesList};
use moon_feature_flags::glob_walk;
use starbase_utils::fs;
use tracing::{debug, instrument, warn};

/// Infer a project name from a source path, by using the name of
/// the project folder.
fn infer_project_id_and_source(path: &str) -> miette::Result<ProjectSourceEntry> {
    let (id, source) = if path.contains('/') {
        (path.split('/').next_back().unwrap().to_owned(), path)
    } else {
        (path.to_owned(), path)
    };

    Ok((Id::clean(id)?, WorkspaceRelativePathBuf::from(source)))
}

/// For each pattern in the globs list, glob the file system
/// for potential projects, and infer their name and source.
#[instrument(skip_all)]
pub fn locate_projects_with_globs<'glob, I, V>(
    context: &WorkspaceBuilderContext,
    globs: I,
    sources: &mut ProjectsSourcesList,
) -> miette::Result<()>
where
    I: IntoIterator<Item = &'glob V>,
    V: AsRef<str> + 'glob,
{
    let mut locate_globs = vec![];
    let mut has_root_level = sources
        .iter()
        .any(|(_, source)| is_root_level_source(source));

    // Root-level project has special handling
    for glob in globs.into_iter() {
        let glob = glob.as_ref();

        if glob == "." {
            if has_root_level {
                continue;
            }

            has_root_level = true;
            sources.push((
                Id::clean(fs::file_name(context.workspace_root))?,
                WorkspaceRelativePathBuf::from("."),
            ));
        } else {
            locate_globs.push(glob);
        }
    }

    // Glob for all other projects
    let config_names = context.config_loader.get_project_file_names();
    let mut potential_projects = glob_walk(context.workspace_root, locate_globs)?;
    potential_projects.sort();

    for mut project_root in potential_projects {
        // Remove trailing moon filename
        if project_root.is_file() {
            if config_names.iter().any(|name| project_root.ends_with(name)) {
                project_root = project_root.parent().unwrap().to_owned();

                // Avoid overwriting an existing root project
                if project_root == context.workspace_root && has_root_level {
                    continue;
                }
            } else {
                // Don't warn on dotfiles
                if project_root
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| !name.starts_with('.'))
                    .unwrap_or_default()
                {
                    warn!(
                        source = ?project_root,
                        "Received a file path for a project root, must be a directory",
                    );
                }

                continue;
            }
        }

        if project_root.is_dir() {
            let project_source =
                to_virtual_string(project_root.strip_prefix(context.workspace_root).unwrap())?;

            if project_source == consts::CONFIG_DIRNAME
                || project_source.starts_with(consts::CONFIG_DIRNAME)
            {
                continue;
            }

            if let Some(vcs) = &context.vcs {
                if vcs.is_ignored(&project_root) {
                    warn!(
                        source = project_source,
                        "Found a project with source {}, but this path has been ignored by your VCS, skipping",
                        color::file(&project_source)
                    );

                    continue;
                }
            }

            let (id, source) = infer_project_id_and_source(&project_source)?;

            if id.starts_with(".") {
                debug!(
                    project_id = id.as_str(),
                    source = source.as_str(),
                    "Received a project for a hidden folder. These are not supported through globs, but can be mapped explicitly with project sources!"
                );
            } else {
                sources.push((id, source));
            }
        }
    }

    Ok(())
}
