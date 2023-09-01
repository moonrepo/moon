use moon_common::path::{to_virtual_string, WorkspaceRelativePathBuf};
use moon_common::{color, consts, Id};
use moon_vcs::BoxedVcs;
use rustc_hash::FxHashMap;
use starbase_utils::{fs, glob};
use std::path::Path;
use tracing::warn;

/// Infer a project name from a source path, by using the name of
/// the project folder.
pub fn infer_project_id_and_source(path: &str) -> miette::Result<(Id, WorkspaceRelativePathBuf)> {
    let (id, source) = if path.contains('/') {
        (path.split('/').last().unwrap().to_owned(), path)
    } else {
        (path.to_owned(), path)
    };

    Ok((Id::clean(id)?, WorkspaceRelativePathBuf::from(source)))
}

/// For each pattern in the globs list, glob the file system
/// for potential projects, and infer their name and source.
pub fn locate_projects_with_globs<I, V>(
    workspace_root: &Path,
    globs: I,
    sources: &mut FxHashMap<Id, WorkspaceRelativePathBuf>,
    vcs: Option<&BoxedVcs>,
) -> miette::Result<()>
where
    I: IntoIterator<Item = V>,
    V: AsRef<str>,
{
    let root_source = ".".to_owned();
    let globs = globs
        .into_iter()
        .map(|glob| glob.as_ref().to_owned())
        .collect::<Vec<_>>();

    // Root-level project has special handling
    if globs.contains(&root_source) {
        let root_id = fs::file_name(workspace_root);

        sources.insert(
            Id::clean(if root_id.is_empty() {
                "root"
            } else {
                root_id.as_str()
            })?,
            WorkspaceRelativePathBuf::from(root_source),
        );
    }

    // Glob for all other projects
    let mut potential_projects = glob::walk(workspace_root, &globs)?;
    potential_projects.sort();

    for project_root in potential_projects {
        if project_root.is_dir() {
            let project_source =
                to_virtual_string(project_root.strip_prefix(workspace_root).unwrap())?;

            if project_source == consts::CONFIG_DIRNAME
                || project_source.starts_with(consts::CONFIG_DIRNAME)
            {
                continue;
            }

            if let Some(vcs) = vcs {
                if vcs.is_ignored(&project_root) {
                    warn!(
                        source = project_source,
                        "Found a project with source {}, but this path has been ignored by your VCS. Skipping ignored source.",
                        color::file(&project_source)
                    );

                    continue;
                }
            }

            let (id, source) = infer_project_id_and_source(&project_source)?;

            if let Some(existing_source) = sources.get(&id) {
                warn!(
                    source = project_source,
                    existing_source = existing_source.as_str(),
                    "A project already exists at source {}, skipping conflicting source {}. Try renaming the project folder to make it unique.",
                    color::file(existing_source),
                    color::file(&source)
                );
            } else {
                sources.insert(id, source);
            }
        }
    }

    Ok(())
}
