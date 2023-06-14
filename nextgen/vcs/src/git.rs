use crate::vcs::{Vcs, VcsResult};
use crate::vcs_error::VcsError;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use once_map::OnceMap;
use relative_path::RelativePathBuf;
use std::path::{Path, PathBuf};
use tracing::debug;

pub struct Git {
    /// Output cache of all executed git commands.
    cache: OnceMap<String, String>,

    /// Default git branch name.
    default_branch: String,

    /// Path between the git and workspace root.
    file_prefix: RelativePathBuf,

    /// Ignore rules derived from a root `.gitignore` file.
    ignore: Option<Gitignore>,

    /// Root of the git repository (where `.git` is located).
    git_root: PathBuf,

    /// Root of the moon workspace.
    workspace_root: PathBuf,
}

impl Git {
    pub fn load<R: AsRef<Path>, B: AsRef<str>>(
        workspace_root: R,
        default_branch: B,
    ) -> VcsResult<Git> {
        debug!("Using git as a VCS");

        let workspace_root = workspace_root.as_ref();

        debug!(
            starting_dir = %workspace_root.display(),
            "Attempting to find a .git directory"
        );

        // Find the .git dir
        let mut git_root = workspace_root;

        loop {
            if git_root.join(".git").exists() {
                debug!(
                    git_root = %git_root.display(),
                    "Found a .git directory"
                );

                break;
            }

            match git_root.parent() {
                Some(parent) => git_root = parent,
                None => {
                    debug!("Unable to find .git, falling back to workspace root");

                    git_root = workspace_root;
                    break;
                }
            };
        }

        // Load .gitignore
        let mut ignore: Option<Gitignore> = None;
        let ignore_path = git_root.join(".gitignore");

        if ignore_path.exists() {
            debug!(
                ignore_path = %ignore_path.display(),
                "Loading ignore rules from .gitignore",
            );

            let mut builder = GitignoreBuilder::new(&git_root);

            if let Some(error) = builder.add(ignore_path) {
                return Err(VcsError::LoadGitignoreFailed { error });
            }

            ignore = Some(
                builder
                    .build()
                    .map_err(|error| VcsError::LoadGitignoreFailed { error })?,
            );
        }

        Ok(Git {
            cache: OnceMap::new(),
            default_branch: default_branch.as_ref().to_owned(),
            ignore,
            file_prefix: RelativePathBuf::from_path(workspace_root.strip_prefix(git_root).unwrap())
                .unwrap(),
            git_root: git_root.to_owned(),
            workspace_root: workspace_root.to_owned(),
        })
    }
}

impl Vcs for Git {}
