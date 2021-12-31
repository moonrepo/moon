use crate::vcs::{TouchedFiles, Vcs, VcsResult};
use async_trait::async_trait;
use moon_utils::exec_command_with_output;
use regex::Regex;
use std::collections::HashSet;

// TODO: This code hasn't been tested yet and may not be accurate!

pub struct Svn {
    origin_branch: String,
}

impl Svn {
    pub fn new() -> Self {
        Svn {
            origin_branch: String::from("trunk"),
        }
    }

    fn extract_line_from_info(&self, label: &str, info: &str) -> String {
        for line in info.split('\n') {
            if line.starts_with(label) {
                return String::from(line).replace(label, "").trim().to_owned();
            }
        }

        String::new()
    }

    async fn get_hash_for_rev(&self, rev: &str) -> VcsResult<String> {
        let output = self.run_command(vec!["info", "-r", rev]).await?;
        let id = self.extract_line_from_info("Revision:", &output);

        Ok(self.extract_line_from_info("Revision:", &id))
    }
}

// https://edoras.sdsu.edu/doc/svn-book-html-chunk/svn.ref.svn.c.info.html
#[async_trait]
impl Vcs for Svn {
    async fn get_local_branch(&self) -> VcsResult<String> {
        let output = self.run_command(vec!["info"]).await?;
        let url = self.extract_line_from_info("URL:", &output);
        let pattern = Regex::new("branches/([^/]+)").unwrap();

        if pattern.is_match(&url) {
            let caps = pattern.captures(&url).unwrap();

            return Ok(String::from(
                caps.get(1)
                    .map_or(self.origin_branch.as_str(), |m| m.as_str()),
            ));
        }

        Ok(self.origin_branch.clone())
    }

    async fn get_local_hash(&self) -> VcsResult<String> {
        Ok(self.get_hash_for_rev("BASE").await?)
    }

    async fn get_origin_branch(&self) -> VcsResult<String> {
        Ok(self.origin_branch.clone())
    }

    async fn get_origin_hash(&self) -> VcsResult<String> {
        Ok(self.get_hash_for_rev("HEAD").await?)
    }

    // https://svnbook.red-bean.com/en/1.8/svn.ref.svn.c.status.html
    async fn get_touched_files(&self) -> VcsResult<TouchedFiles> {
        let output = self.run_command(vec!["status", "wc"]).await?;
        let mut added = HashSet::new();
        let mut deleted = HashSet::new();
        let mut modified = HashSet::new();
        let mut untracked = HashSet::new();
        let mut staged = HashSet::new();
        let unstaged = HashSet::new();

        for line in output.split('\n') {
            let mut chars = line.chars();
            let c1 = chars.next().unwrap_or_default();
            let c2 = chars.next().unwrap_or_default();
            let file = String::from(&line[8..]);

            match c1 {
                'A' | 'C' => {
                    added.insert(file.clone());
                }
                'D' => {
                    deleted.insert(file.clone());
                }
                'M' | 'R' => {
                    modified.insert(file.clone());
                    staged.insert(file.clone());
                }
                '?' => {
                    untracked.insert(file.clone());
                }
                _ => {}
            }

            if c2 == 'M' {
                modified.insert(file.clone());
            }

            // svn files are always staged by default
            staged.insert(file.clone());
        }

        Ok(TouchedFiles {
            added,
            deleted,
            modified,
            staged,
            unstaged, // svn has no concept for this
            untracked,
        })
    }

    async fn run_command(&self, args: Vec<&str>) -> VcsResult<String> {
        Ok(exec_command_with_output("svn", args).await?)
    }
}
