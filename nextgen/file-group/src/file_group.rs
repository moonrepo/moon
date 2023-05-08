use crate::errors::FileGroupError;
use moon_common::Id;
use moon_path::{expand_to_workspace_relative, ProjectRelativePathBuf, WorkspaceRelativePathBuf};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use starbase_utils::glob;
use std::path::PathBuf;
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
}
