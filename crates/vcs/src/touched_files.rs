use miette::IntoDiagnostic;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf};
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::Hash;
use std::path::PathBuf;
use std::str::FromStr;

pub fn map_absolute_to_workspace_relative_paths<I>(
    paths: I,
    workspace_root: &PathBuf,
) -> miette::Result<Vec<WorkspaceRelativePathBuf>>
where
    I: IntoIterator<Item = PathBuf>,
{
    let mut new_paths = vec![];

    for path in paths {
        new_paths.push(path.relative_to(workspace_root).into_diagnostic()?);
    }

    Ok(new_paths)
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct TouchedFiles<T: Hash + Eq + PartialEq = WorkspaceRelativePathBuf> {
    pub added: FxHashSet<T>,
    pub deleted: FxHashSet<T>,
    pub modified: FxHashSet<T>,
    pub untracked: FxHashSet<T>,

    // Will contain files from the previous fields
    pub staged: FxHashSet<T>,
    pub unstaged: FxHashSet<T>,
}

impl<T: Hash + Eq + PartialEq> TouchedFiles<T> {
    pub fn all(&self) -> FxHashSet<&T> {
        let mut files = FxHashSet::default();
        files.extend(&self.added);
        files.extend(&self.deleted);
        files.extend(&self.modified);
        files.extend(&self.untracked);
        files.extend(&self.staged);
        files.extend(&self.unstaged);
        files
    }

    pub fn merge(&mut self, other: TouchedFiles<T>) {
        self.added.extend(other.added);
        self.deleted.extend(other.deleted);
        self.modified.extend(other.modified);
        self.untracked.extend(other.untracked);
        self.staged.extend(other.staged);
        self.unstaged.extend(other.unstaged);
    }
}

impl TouchedFiles<PathBuf> {
    pub fn into_workspace_relative(
        self,
        workspace_root: &PathBuf,
    ) -> miette::Result<TouchedFiles<WorkspaceRelativePathBuf>> {
        let mut files = TouchedFiles::default();

        files.added.extend(map_absolute_to_workspace_relative_paths(
            self.added,
            workspace_root,
        )?);
        files
            .deleted
            .extend(map_absolute_to_workspace_relative_paths(
                self.deleted,
                workspace_root,
            )?);
        files
            .modified
            .extend(map_absolute_to_workspace_relative_paths(
                self.modified,
                workspace_root,
            )?);
        files
            .untracked
            .extend(map_absolute_to_workspace_relative_paths(
                self.untracked,
                workspace_root,
            )?);
        files
            .staged
            .extend(map_absolute_to_workspace_relative_paths(
                self.staged,
                workspace_root,
            )?);
        files
            .unstaged
            .extend(map_absolute_to_workspace_relative_paths(
                self.unstaged,
                workspace_root,
            )?);

        Ok(files)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TouchedStatus {
    Added,
    #[default]
    All,
    Deleted,
    Modified,
    Staged,
    Unstaged,
    Untracked,
}

impl fmt::Display for TouchedStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                TouchedStatus::Added => "added",
                TouchedStatus::All => "all",
                TouchedStatus::Deleted => "deleted",
                TouchedStatus::Modified => "modified",
                TouchedStatus::Staged => "staged",
                TouchedStatus::Unstaged => "unstaged",
                TouchedStatus::Untracked => "untracked",
            }
        )?;

        Ok(())
    }
}

impl FromStr for TouchedStatus {
    type Err = miette::Report;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value.to_lowercase().as_str() {
            "added" => Self::Added,
            "all" => Self::All,
            "deleted" => Self::Deleted,
            "modified" => Self::Modified,
            "staged" => Self::Staged,
            "unstaged" => Self::Unstaged,
            "untracked" => Self::Untracked,
            other => return Err(miette::miette!("Unknown touched status {}", other)),
        })
    }
}
