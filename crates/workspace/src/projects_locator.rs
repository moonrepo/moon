use crate::workspace_builder::WorkspaceBuilderContext;
use moon_common::path::{WorkspaceRelativePathBuf, is_root_level_source, to_virtual_string};
use moon_common::{Id, color, consts};
use moon_config::{ProjectSourceEntry, ProjectsSourcesList};
use moon_feature_flags::glob_walk_with_options;
use starbase_utils::fs;
use starbase_utils::glob::GlobWalkOptions;
use tracing::{debug, instrument, warn};

fn is_hidden(path: &str) -> bool {
    let last = match path.rfind('/') {
        Some(index) => &path[index + 1..],
        None => path,
    };

    last.starts_with('.')
}

/// Infer a project name from a source path, by using the name of
/// the project folder.
fn infer_project_id_and_source(path: &str) -> miette::Result<ProjectSourceEntry> {
    let (id, source) = if let Some(index) = path.rfind('/') {
        (&path[index + 1..], path)
    } else {
        (path, path)
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
    let mut add_root_level = false;
    let has_root_level = sources
        .iter()
        .any(|(_, source)| is_root_level_source(source));

    // Root-level project has special handling
    for glob in globs.into_iter() {
        let glob = glob.as_ref();

        if glob == "." {
            add_root_level = true;
        } else {
            locate_globs.push(glob);
        }
    }

    // Glob for all other projects
    let config_names = context.config_loader.get_project_file_names();
    let mut potential_projects = glob_walk_with_options(
        context.workspace_root,
        locate_globs,
        GlobWalkOptions::default().log_results(),
    )?;
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
                    .map(|name| !is_hidden(name))
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
            if project_root == context.workspace_root {
                add_root_level = true;
                continue;
            }

            let project_source =
                to_virtual_string(project_root.strip_prefix(context.workspace_root).unwrap())?;

            if project_source == consts::CONFIG_DIRNAME
                || project_source.starts_with(consts::CONFIG_DIRNAME)
            {
                continue;
            }

            if let Some(vcs) = &context.vcs
                && vcs.is_ignored(&project_root)
            {
                warn!(
                    source = project_source,
                    "Found a project with source {}, but this path has been ignored by your VCS, skipping",
                    color::file(&project_source)
                );

                continue;
            }

            if is_hidden(&project_source) {
                debug!(
                    source = project_source,
                    "Received a project for a hidden folder. These are not supported through globs, but can be mapped explicitly with project sources!"
                );
            } else {
                sources.push(infer_project_id_and_source(&project_source)?);
            }
        }
    }

    if add_root_level && !has_root_level {
        sources.push((
            Id::clean(fs::file_name(context.workspace_root))?,
            WorkspaceRelativePathBuf::from("."),
        ));
    }

    Ok(())
}
