use crate::vcs::{TouchedFiles, Vcs, VcsResult};
use async_trait::async_trait;
use moon_utils::exec_command_with_output;
use std::collections::HashSet;

pub struct Git {
    origin_branch: String,
}

impl Git {
    pub fn new(origin_branch: &str) -> Self {
        Git {
            origin_branch: origin_branch.to_owned(),
        }
    }
}

#[async_trait]
impl Vcs for Git {
    async fn get_local_branch(&self) -> VcsResult<String> {
        self.run_command(vec!["branch", "--show-current"]).await
    }

    async fn get_local_hash(&self) -> VcsResult<String> {
        self.run_command(vec!["rev-parse", "HEAD"]).await
    }

    async fn get_origin_branch(&self) -> VcsResult<String> {
        Ok(self.origin_branch.clone())
    }

    async fn get_origin_hash(&self) -> VcsResult<String> {
        self.run_command(vec!["rev-parse", &self.origin_branch])
            .await
    }

    async fn get_touched_files(&self) -> VcsResult<TouchedFiles> {
        let output = self.run_command(vec!["status", "-s", "-u", "-z"]).await?;
        let mut added = HashSet::new();
        let mut deleted = HashSet::new();
        let mut modified = HashSet::new();
        let mut untracked = HashSet::new();
        let mut staged = HashSet::new();
        let mut unstaged = HashSet::new();

        // -z uses null for breaks instead of new lines
        for line in output.split('\0') {
            let mut chars = line.chars();
            let x = chars.next().unwrap_or_default();
            let y = chars.next().unwrap_or_default();
            let mut file = &line[3..];

            // Copied files contain 2 file paths: ORIG -> NEW
            if let Some(index) = file.find(' ') {
                file = &file[index..];
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
        }

        Ok(TouchedFiles {
            added,
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
