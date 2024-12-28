use crate::git::GitError;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, PartialEq)]
pub struct GitWorktree {
    /// Absolute path to where the worktree is checked out to within the repository.
    pub checkout_dir: PathBuf,

    /// Absolute path to the worktree's `.git` directory, which is housed in the
    /// parent's `.git/worktrees`.
    pub git_dir: PathBuf,
}

pub fn extract_gitdir_from_worktree(path: &Path) -> miette::Result<PathBuf> {
    let contents = fs::read_to_string(path).map_err(|error| GitError::LoadWorktreeFailed {
        path: path.to_owned(),
        error: Box::new(error),
    })?;

    for line in contents.lines() {
        if let Some(suffix) = line.strip_prefix("gitdir:") {
            let git_dir = PathBuf::from(suffix.trim());

            return Ok(git_dir
                .canonicalize()
                .map_err(|error| GitError::LoadWorktreeFailed {
                    path: git_dir,
                    error: Box::new(error),
                })?);
        }
    }

    Err(GitError::ParseWorktreeFailed.into())
}
