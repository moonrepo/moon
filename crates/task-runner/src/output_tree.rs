use crate::task_runner_error::TaskRunnerError;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf};
use moon_hash::{Blob, Digest};
use starbase_utils::fs::FsError;
use starbase_utils::glob::{self, GlobWalkOptions};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub type OutputDigestsMap = BTreeMap<WorkspaceRelativePathBuf, Digest>;

#[derive(Debug)]
pub struct OutputTree {
    pub files: BTreeMap<WorkspaceRelativePathBuf, Blob>,
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

    pub fn get_digests(&self) -> OutputDigestsMap {
        self.files
            .iter()
            .map(|(k, v)| (k.clone(), v.digest.clone()))
            .collect()
    }

    pub fn insert(&mut self, abs_path: PathBuf, source_blob: Option<Blob>) -> miette::Result<()> {
        if !abs_path.starts_with(&self.workspace_root) {
            return Err(TaskRunnerError::OutputFileOutsideOfWorkspace { output: abs_path }.into());
        }

        if abs_path.is_symlink() {
            self.insert_symlink(abs_path)?;
        } else if abs_path.is_file() {
            self.insert_file(abs_path, source_blob)?;
        } else if abs_path.is_dir() {
            self.insert_dir(abs_path)?;
        }

        Ok(())
    }

    fn insert_dir(&mut self, abs_path: PathBuf) -> miette::Result<()> {
        for abs_file in
            glob::walk_fast_with_options(abs_path, ["**/*"], GlobWalkOptions::default().files())?
        {
            self.insert_file(abs_file, None)?;
        }

        Ok(())
    }

    fn insert_file(&mut self, abs_path: PathBuf, source_blob: Option<Blob>) -> miette::Result<()> {
        self.files.insert(
            self.convert_path(&abs_path)?,
            match source_blob {
                Some(inner) => inner,
                None => Blob::from_file(&abs_path)?,
            },
        );

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
