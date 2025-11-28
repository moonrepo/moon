use miette::IntoDiagnostic;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::Hash;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Default, Eq, PartialEq)]
pub struct ChangedFiles<T: Hash + Eq + PartialEq = WorkspaceRelativePathBuf> {
    pub files: FxHashMap<T, Vec<ChangedStatus>>,
}

impl<T: Hash + Eq + PartialEq> ChangedFiles<T> {
    pub fn all(&self) -> Vec<&T> {
        self.files.keys().collect()
    }

    pub fn added(&self) -> Vec<&T> {
        self.select(ChangedStatus::Added)
    }

    pub fn deleted(&self) -> Vec<&T> {
        self.select(ChangedStatus::Deleted)
    }

    pub fn modified(&self) -> Vec<&T> {
        self.select(ChangedStatus::Modified)
    }

    pub fn staged(&self) -> Vec<&T> {
        self.select(ChangedStatus::Staged)
    }

    pub fn unstaged(&self) -> Vec<&T> {
        self.select(ChangedStatus::Unstaged)
    }

    pub fn untracked(&self) -> Vec<&T> {
        self.select(ChangedStatus::Untracked)
    }

    pub fn merge(&mut self, other: ChangedFiles<T>) {
        for (file, statuses) in other.files {
            self.files.entry(file).or_default().extend(statuses);
        }
    }

    pub fn select(&self, status: ChangedStatus) -> Vec<&T> {
        self.files
            .iter()
            .filter_map(|(file, statuses)| {
                if statuses.contains(&status) {
                    Some(file)
                } else {
                    None
                }
            })
            .collect()
    }
}

impl ChangedFiles<PathBuf> {
    pub fn into_workspace_relative(
        self,
        workspace_root: &PathBuf,
    ) -> miette::Result<ChangedFiles<WorkspaceRelativePathBuf>> {
        let mut files = ChangedFiles::default();

        for (file, statuses) in self.files {
            files.files.insert(
                file.relative_to(workspace_root).into_diagnostic()?,
                statuses,
            );
        }

        Ok(files)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangedStatus {
    Added,
    #[default]
    All,
    Deleted,
    Modified,
    Staged,
    Unstaged,
    Untracked,
}

impl fmt::Display for ChangedStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                ChangedStatus::Added => "added",
                ChangedStatus::All => "all",
                ChangedStatus::Deleted => "deleted",
                ChangedStatus::Modified => "modified",
                ChangedStatus::Staged => "staged",
                ChangedStatus::Unstaged => "unstaged",
                ChangedStatus::Untracked => "untracked",
            }
        )?;

        Ok(())
    }
}

impl FromStr for ChangedStatus {
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
            other => return Err(miette::miette!("Unknown changed status {}", other)),
        })
    }
}
