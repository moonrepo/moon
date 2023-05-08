use crate::file_group_error::FileGroupError;
use common_path::common_path_all;
use moon_common::Id;
use moon_path::{expand_to_workspace_relative, WorkspaceRelativePathBuf};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use starbase_utils::glob;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct FileGroup {
    pub files: Vec<WorkspaceRelativePathBuf>,

    pub globs: Vec<WorkspaceRelativePathBuf>,

    pub id: Id,

    #[serde(skip)]
    walk_cache: OnceCell<Vec<PathBuf>>,
}

impl FileGroup {
    pub fn new<T>(id: T) -> Result<FileGroup, FileGroupError>
    where
        T: AsRef<str>,
    {
        let id = Id::new(id)?;

        debug!(id = %id, "Creating file group");

        Ok(FileGroup {
            files: vec![],
            globs: vec![],
            id,
            walk_cache: OnceCell::new(),
        })
    }

    pub fn new_with_source<T, I, V>(
        id: T,
        project_source: &str,
        patterns: I,
    ) -> Result<FileGroup, FileGroupError>
    where
        T: AsRef<str>,
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        let mut file_group = FileGroup::new(id)?;
        file_group.set_patterns(project_source, patterns);

        Ok(file_group)
    }

    /// Add patterns (file paths or globs) to the file group, while expanding to a
    /// workspace relative path based on the provided project source.
    /// This will overwrite any existing patterns!
    pub fn set_patterns<I, V>(&mut self, project_source: &str, patterns: I) -> &mut Self
    where
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        let mut log_patterns = vec![];

        for pattern in patterns {
            let pattern = pattern.as_ref();
            let path = expand_to_workspace_relative(pattern, project_source);

            log_patterns.push(path.as_str().to_owned());

            if glob::is_glob(pattern) {
                self.globs.push(path);
            } else {
                self.files.push(path);
            }
        }

        debug!(
            id = %self.id,
            patterns = ?log_patterns,
            "Setting patterns to file group"
        );

        self
    }

    /// Return the file group as an expanded list of directory paths.
    /// If a glob is detected, it will aggregate all directories found.
    pub fn dirs(
        &self,
        workspace_root: &Path,
    ) -> Result<Vec<WorkspaceRelativePathBuf>, FileGroupError> {
        self.walk(true, workspace_root)
    }

    /// Return the file group as an expanded list of file paths.
    /// If a glob is detected, it will aggregate all files found.
    pub fn files(
        &self,
        workspace_root: &Path,
    ) -> Result<Vec<WorkspaceRelativePathBuf>, FileGroupError> {
        self.walk(false, workspace_root)
    }

    /// Return the file group as a list of file globs (as-is),
    /// relative to the project root.
    pub fn globs(&self) -> Result<&Vec<WorkspaceRelativePathBuf>, FileGroupError> {
        if self.globs.is_empty() {
            return Err(FileGroupError::NoGlobs(self.id.to_string()));
        }

        Ok(&self.globs)
    }

    /// Return the file group reduced down to the lowest common directory.
    /// If the reduced directories is not = 1, the project root "." will be returned.
    pub fn root(
        &self,
        workspace_root: &Path,
        project_source: &str,
    ) -> Result<WorkspaceRelativePathBuf, FileGroupError> {
        let dirs = self.dirs(workspace_root)?;

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
    ) -> Result<Vec<WorkspaceRelativePathBuf>, FileGroupError> {
        let mut list = vec![];

        for path in &self.files {
            let allowed = if is_dir {
                path.to_path(workspace_root).is_dir()
            } else {
                path.to_path(workspace_root).is_file()
            };

            if allowed {
                list.push(path.to_owned());
            }
        }

        if !self.globs.is_empty() {
            let globs = &self.globs;
            let walk_paths = self
                .walk_cache
                .get_or_try_init(|| glob::walk(workspace_root, globs))?;

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

impl PartialEq for FileGroup {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.files == other.files && self.globs == other.globs
    }
}
