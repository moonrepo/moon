use crate::file_group_error::FileGroupError;
use common_path::common_path_all;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use starbase_utils::glob;
use std::path::{Path, PathBuf};
use tracing::trace;

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
    pub fn new<T>(id: T) -> miette::Result<FileGroup>
    where
        T: AsRef<str>,
    {
        let id = Id::new(id)?;

        Ok(FileGroup {
            files: vec![],
            globs: vec![],
            id,
            walk_cache: OnceCell::new(),
        })
    }

    pub fn new_with_source<T, I>(id: T, patterns: I) -> miette::Result<FileGroup>
    where
        T: AsRef<str>,
        I: IntoIterator<Item = WorkspaceRelativePathBuf>,
    {
        let mut file_group = FileGroup::new(id)?;
        file_group.set_patterns(patterns);

        Ok(file_group)
    }

    /// Add patterns (file paths or globs) to the file group, while expanding to a
    /// workspace relative path based on the provided project source.
    /// This will overwrite any existing patterns!
    pub fn set_patterns<I>(&mut self, patterns: I) -> &mut Self
    where
        I: IntoIterator<Item = WorkspaceRelativePathBuf>,
    {
        self.files = vec![];
        self.globs = vec![];

        let mut log_patterns = vec![];

        for path in patterns {
            log_patterns.push(path.as_str().to_owned());

            if glob::is_glob(&path) {
                self.globs.push(path);
            } else {
                self.files.push(path);
            }
        }

        trace!(
            id = self.id.as_str(),
            patterns = ?log_patterns,
            "Creating file group"
        );

        self
    }

    /// Return the file group as an expanded list of directory paths.
    /// If a glob is detected, it will aggregate all directories found.
    pub fn dirs(
        &self,
        workspace_root: &Path,
        loose_check: bool,
    ) -> miette::Result<Vec<WorkspaceRelativePathBuf>> {
        self.walk(true, workspace_root, loose_check)
    }

    /// Return the file group as an expanded list of file paths.
    /// If a glob is detected, it will aggregate all files found.
    pub fn files(
        &self,
        workspace_root: &Path,
        loose_check: bool,
    ) -> miette::Result<Vec<WorkspaceRelativePathBuf>> {
        self.walk(false, workspace_root, loose_check)
    }

    /// Return the file group as a list of file globs (as-is),
    /// relative to the project root.
    pub fn globs(&self) -> miette::Result<&Vec<WorkspaceRelativePathBuf>> {
        if self.globs.is_empty() {
            return Err(FileGroupError::NoGlobs(self.id.clone()).into());
        }

        Ok(&self.globs)
    }

    /// Return the file group reduced down to the lowest common directory.
    /// If the reduced directories is not = 1, the project root "." will be returned.
    pub fn root<P: AsRef<Path>, S: AsRef<str>>(
        &self,
        workspace_root: P,
        project_source: S,
    ) -> miette::Result<WorkspaceRelativePathBuf> {
        let dirs = self.dirs(workspace_root.as_ref(), false)?;
        let project_source = project_source.as_ref();

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
        loose_check: bool,
    ) -> miette::Result<Vec<WorkspaceRelativePathBuf>> {
        let mut list = vec![];

        for path in &self.files {
            let file = path.to_path(workspace_root);
            let mut allowed = false;

            if is_dir && (file.is_dir() || loose_check && file.extension().is_none()) {
                allowed = true;
            } else if !is_dir && (file.is_file() || loose_check && file.extension().is_some()) {
                allowed = true;
            }

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

        list.sort();

        Ok(list)
    }
}

impl PartialEq for FileGroup {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.files == other.files && self.globs == other.globs
    }
}
