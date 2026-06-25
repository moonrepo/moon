use crate::task_runner_error::TaskRunnerError;
use moon_cache::CasStore;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf};
use moon_hash::Digest;
use starbase_utils::fs::FsError;
use starbase_utils::glob::{self, GlobWalkOptions};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct OutputTree {
    pub files: BTreeMap<WorkspaceRelativePathBuf, Digest>,
    pub symlinks: BTreeMap<WorkspaceRelativePathBuf, WorkspaceRelativePathBuf>,
    pub workspace_root: PathBuf,
}

impl OutputTree {
    pub fn new(workspace_root: &Path) -> Self {
        Self {
            files: BTreeMap::new(),
            symlinks: BTreeMap::new(),
            workspace_root: workspace_root.to_owned(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.symlinks.is_empty()
    }

    /// Hash the file/dir at `abs_path` into the tree. Bytes are streamed into
    /// the provided CAS in a single pass — they are never materialized in
    /// memory, and only the resulting digest is retained.
    pub fn insert(&mut self, abs_path: PathBuf, cas: &CasStore) -> miette::Result<()> {
        if !abs_path.starts_with(&self.workspace_root) {
            return Err(TaskRunnerError::OutputFileOutsideOfWorkspace { output: abs_path }.into());
        }

        if abs_path.is_symlink() {
            self.insert_symlink(abs_path)?;
        } else if abs_path.is_file() {
            self.insert_file(abs_path, cas)?;
        } else if abs_path.is_dir() {
            self.insert_dir(abs_path, cas)?;
        }

        Ok(())
    }

    fn insert_dir(&mut self, abs_path: PathBuf, cas: &CasStore) -> miette::Result<()> {
        for abs_file in
            glob::walk_fast_with_options(abs_path, ["**/*"], GlobWalkOptions::default().files())?
        {
            self.insert_file(abs_file, cas)?;
        }

        Ok(())
    }

    fn insert_file(&mut self, abs_path: PathBuf, cas: &CasStore) -> miette::Result<()> {
        // Stream the file directly into CAS. The file is hashed up front, so an
        // object already in the store short-circuits without creating a temp
        // file; only a cache miss copies bytes in.
        self.files
            .insert(self.convert_path(&abs_path)?, cas.store_file(&abs_path)?);

        Ok(())
    }

    fn insert_symlink(&mut self, abs_path: PathBuf) -> miette::Result<()> {
        let link = fs::read_link(&abs_path).map_err(|error| FsError::Read {
            path: abs_path.clone(),
            error: Box::new(error),
        })?;

        if !link.starts_with(&self.workspace_root) {
            return Err(TaskRunnerError::OutputSymlinkOutsideOfWorkspace {
                output: abs_path,
                target: link,
            }
            .into());
        }

        self.symlinks
            .insert(self.convert_path(&abs_path)?, self.convert_path(&link)?);

        Ok(())
    }

    fn convert_path(&self, abs_path: &Path) -> miette::Result<WorkspaceRelativePathBuf> {
        let rel_path = abs_path.relative_to(&self.workspace_root).map_err(|_| {
            TaskRunnerError::OutputFileOutsideOfWorkspace {
                output: abs_path.to_owned(),
            }
        })?;

        Ok(rel_path)
    }
}
