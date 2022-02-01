use crate::vcs::{TouchedFiles, Vcs, VcsResult};
use async_trait::async_trait;
use moon_utils::process::exec_command_with_output;
use std::collections::HashSet;

pub struct Git {
    default_branch: String,
}

impl Git {
    pub fn new(default_branch: &str) -> Self {
        Git {
            default_branch: String::from(default_branch),
        }
    }
}

#[async_trait]
impl Vcs for Git {
    async fn get_local_branch(&self) -> VcsResult<String> {
        self.run_command(vec!["branch", "--show-current"]).await
    }

    async fn get_local_branch_hash(&self) -> VcsResult<String> {
        self.run_command(vec!["rev-parse", "HEAD"]).await
    }

    fn get_default_branch(&self) -> &str {
        &self.default_branch
    }

    async fn get_default_branch_hash(&self) -> VcsResult<String> {
        self.run_command(vec!["rev-parse", &self.default_branch])
            .await
    }

    // https://git-scm.com/docs/git-status#_short_format
    async fn get_touched_files(&self) -> VcsResult<TouchedFiles> {
        let output = self.run_command(vec!["status", "-s", "-u"]).await?;

        if output.is_empty() {
            return Ok(TouchedFiles::default());
        }

        let mut added = HashSet::new();
        let mut deleted = HashSet::new();
        let mut modified = HashSet::new();
        let mut untracked = HashSet::new();
        let mut staged = HashSet::new();
        let mut unstaged = HashSet::new();
        let mut all = HashSet::new();

        for line in output.split('\n') {
            if line.is_empty() {
                continue;
            }

            let mut chars = line.chars();
            let x = chars.next().unwrap_or_default();
            let y = chars.next().unwrap_or_default();
            let mut file = &line[3..];

            // Copied files contain 2 file paths: ORIG -> NEW
            if let Some(index) = file.find("->") {
                file = &file[index + 1..];
            }

            // Convert to a normal string
            let file = file.to_owned();

            match x {
                'A' | 'C' => {
                    added.insert(file.clone());
                    staged.insert(file.clone());
                }
                'D' => {
                    deleted.insert(file.clone());
                    staged.insert(file.clone());
                }
                'M' | 'R' => {
                    modified.insert(file.clone());
                    staged.insert(file.clone());
                }
                _ => {}
            }

            match y {
                'A' | 'C' => {
                    added.insert(file.clone());
                    unstaged.insert(file.clone());
                }
                'D' => {
                    deleted.insert(file.clone());
                    unstaged.insert(file.clone());
                }
                'M' | 'R' => {
                    modified.insert(file.clone());
                    unstaged.insert(file.clone());
                }
                '?' => {
                    untracked.insert(file.clone());
                }
                _ => {}
            }

            all.insert(file.clone());
        }

        Ok(TouchedFiles {
            added,
            all,
            deleted,
            modified,
            staged,
            unstaged,
            untracked,
        })
    }

    async fn run_command(&self, args: Vec<&str>) -> VcsResult<String> {
        Ok(exec_command_with_output("git", args).await?)
    }
}
