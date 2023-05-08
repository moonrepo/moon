use crate::errors::FileGroupError;
use common_path::common_path_all;
use moon_common::Id;
use moon_path::{expand_to_workspace_relative, ProjectRelativePathBuf, WorkspaceRelativePathBuf};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use starbase_utils::glob;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct FileGroup {
    pub files: Vec<ProjectRelativePathBuf>,

    pub globs: Vec<ProjectRelativePathBuf>,

    pub id: Id,

    #[serde(skip)]
    walk_cache: OnceCell<Vec<PathBuf>>,
}

impl FileGroup {
    pub fn new<T, I, V>(id: T, patterns: I) -> Result<FileGroup, FileGroupError>
    where
        T: AsRef<str>,
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        let id = id.as_ref();

        debug!(id, "Creating file group");

        let mut group = FileGroup {
            files: vec![],
            globs: vec![],
            id: Id::new(id)?,
            walk_cache: OnceCell::new(),
        };

        group.merge(patterns);

        Ok(group)
    }

    pub fn merge<I, V>(&mut self, patterns: I)
    where
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        // Local files should always override global
        self.files = vec![];
        self.globs = vec![];

        for pattern in patterns {
            let pattern = pattern.as_ref();

            if glob::is_glob(pattern) {
                self.globs.push(ProjectRelativePathBuf::from(pattern));
            } else {
                self.files.push(ProjectRelativePathBuf::from(pattern));
            }
        }
    }

    /// Returns the file group as-is, with each file converted to a workspace relative path.
    /// File paths and globs will be separated as they have different semantics.
    pub fn all(
        &self,
        project_source: &str,
    ) -> Result<(Vec<WorkspaceRelativePathBuf>, Vec<WorkspaceRelativePathBuf>), FileGroupError>
    {
        let mut files = vec![];
        let mut globs = vec![];

        for file in &self.files {
            files.push(expand_to_workspace_relative(file, project_source));
        }

        for file in &self.globs {
            globs.push(expand_to_workspace_relative(file, project_source));
        }

        Ok((files, globs))
    }

    /// Return the file group as an expanded list of directory paths.
    /// If a glob is detected, it will aggregate all directories found.
    pub fn dirs(
        &self,
        workspace_root: &Path,
        project_source: &str,
    ) -> Result<Vec<WorkspaceRelativePathBuf>, FileGroupError> {
        self.walk(true, workspace_root, project_source)
    }

    /// Return the file group as an expanded list of file paths.
    /// If a glob is detected, it will aggregate all files found.
    pub fn files(
        &self,
        workspace_root: &Path,
        project_source: &str,
    ) -> Result<Vec<WorkspaceRelativePathBuf>, FileGroupError> {
        self.walk(false, workspace_root, project_source)
    }

    /// Return the file group as a list of file globs (as-is),
    /// relative to the project root.
    pub fn globs(
        &self,
        project_source: &str,
    ) -> Result<Vec<WorkspaceRelativePathBuf>, FileGroupError> {
        if self.globs.is_empty() {
            return Err(FileGroupError::NoGlobs(self.id.to_string()));
        }

        let mut globs = vec![];

        for file in &self.globs {
            globs.push(expand_to_workspace_relative(file, project_source));
        }

        Ok(globs)
    }

    /// Return the file group reduced down to the lowest common directory.
    /// If the reduced directories is not = 1, the project root "." will be returned.
    pub fn root(
        &self,
        workspace_root: &Path,
        project_source: &str,
    ) -> Result<WorkspaceRelativePathBuf, FileGroupError> {
        let dirs = self.dirs(workspace_root, project_source)?;

        if !dirs.is_empty() {
            let paths = dirs
                .iter()
                .filter_map(|d| d.strip_prefix(project_source).ok())
                .map(|d| Path::new(d.as_str()))
                .collect::<Vec<&Path>>();
            let common_dir = common_path_all(paths);

            if let Some(dir) = common_dir {
                return Ok(
                    WorkspaceRelativePathBuf::from(project_source).join(dir.to_str().unwrap())
                );
            }
        }

        Ok(".".into())
    }

    fn walk(
        &self,
        is_dir: bool,
        workspace_root: &Path,
        project_source: &str,
    ) -> Result<Vec<WorkspaceRelativePathBuf>, FileGroupError> {
        let (paths, globs) = self.all(project_source)?;
        let mut list = vec![];

        for path in paths {
            let allowed = if is_dir {
                path.to_path(workspace_root).is_dir()
            } else {
                path.to_path(workspace_root).is_file()
            };

            if allowed {
                list.push(path);
            }
        }

        if !globs.is_empty() {
            let walk_paths = self
                .walk_cache
                .get_or_try_init(|| glob::walk(workspace_root, &globs))?;

            // Glob results are absolute paths!
            for path in walk_paths {
                let allowed = if is_dir {
                    path.is_dir()
                } else {
                    path.is_file()
                };

                if allowed {
                    list.push(
                        WorkspaceRelativePathBuf::from_path(
                            path.strip_prefix(workspace_root).unwrap(),
                        )
                        .unwrap(),
                    );
                }
            }
        }

        Ok(list)
    }
}
