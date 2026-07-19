use ignore::WalkBuilder;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use miette::IntoDiagnostic;
use moon_common::path::{WorkspaceRelativePath, WorkspaceRelativePathBuf};
use starbase_utils::hash;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub struct WorkspaceFiles {
    ignore: Option<Gitignore>,
    root: PathBuf,
}

impl WorkspaceFiles {
    pub fn new(root: impl Into<PathBuf>) -> miette::Result<Self> {
        let root = root.into();
        let ignore_file = root.join(".gitignore");
        let ignore = if ignore_file.exists() {
            let mut builder = GitignoreBuilder::new(&root);

            if let Some(error) = builder.add(&ignore_file) {
                return Err(miette::miette!(error));
            }

            Some(builder.build().into_diagnostic()?)
        } else {
            None
        };

        Ok(Self { ignore, root })
    }

    pub async fn hash_files(
        &self,
        files: &[WorkspaceRelativePathBuf],
        allow_ignored: bool,
    ) -> miette::Result<BTreeMap<WorkspaceRelativePathBuf, String>> {
        let mut hashes = BTreeMap::new();

        for file in files {
            let absolute = file.to_logical_path(&self.root);

            if absolute.is_file() && (allow_ignored || !self.is_ignored(&absolute)) {
                hashes.insert(file.clone(), hash::sha256::from_file(&absolute)?);
            }
        }

        Ok(hashes)
    }

    pub fn list(
        &self,
        dir: &WorkspaceRelativePath,
    ) -> miette::Result<Vec<WorkspaceRelativePathBuf>> {
        let mut files = BTreeSet::new();
        let start = dir.to_logical_path(&self.root);
        let root = self.root.clone();
        let mut builder = WalkBuilder::new(start);
        builder
            .follow_links(false)
            .git_exclude(true)
            .git_global(true)
            .git_ignore(true)
            .hidden(false)
            .parents(true)
            .filter_entry(move |entry| {
                let Ok(relative) = entry.path().strip_prefix(&root) else {
                    return true;
                };
                !relative.components().any(|part| {
                    matches!(
                        part.as_os_str().to_str(),
                        Some(".git" | ".hg" | ".jj" | ".pijul" | "_darcs")
                    )
                })
            });

        for entry in builder.build() {
            let entry = entry.into_diagnostic()?;

            if entry.file_type().is_some_and(|kind| kind.is_file()) {
                files.insert(WorkspaceRelativePathBuf::from(
                    entry
                        .path()
                        .strip_prefix(&self.root)
                        .into_diagnostic()?
                        .to_string_lossy()
                        .replace('\\', "/"),
                ));
            }
        }

        let tracked = Command::new("git")
            .args(["ls-files", "--cached", "-z", "--", dir.as_str()])
            .current_dir(&self.root)
            .output();

        if let Ok(output) = tracked
            && output.status.success()
        {
            for file in output.stdout.split(|byte| *byte == 0) {
                if !file.is_empty() {
                    files.insert(WorkspaceRelativePathBuf::from(
                        String::from_utf8_lossy(file).replace('\\', "/"),
                    ));
                }
            }
        }

        Ok(files.into_iter().collect())
    }

    pub fn is_ignored(&self, file: &Path) -> bool {
        self.ignore
            .as_ref()
            .is_some_and(|ignore| ignore.matched(file, file.is_dir()).is_ignore())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starbase_sandbox::create_empty_sandbox;

    #[tokio::test]
    async fn provides_files_without_a_source_control_repository() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("tracked.txt", "tracked");
        sandbox.create_file("ignored.txt", "ignored");
        sandbox.create_file(".gitignore", "ignored.txt\n");
        sandbox.create_file(".git/internal", "metadata");
        sandbox.create_file("nested/.git/internal", "metadata");
        let files = WorkspaceFiles::new(sandbox.path()).unwrap();

        assert_eq!(
            files.list(WorkspaceRelativePath::new(".")).unwrap(),
            vec![
                WorkspaceRelativePathBuf::from(".gitignore"),
                WorkspaceRelativePathBuf::from("tracked.txt"),
            ]
        );
        assert_eq!(
            files
                .hash_files(&["tracked.txt".into()], true)
                .await
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn includes_tracked_files_that_are_ignored_later() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("tracked.txt", "tracked");
        let run_git = |args: &[&str]| {
            assert!(
                Command::new("git")
                    .args(args)
                    .current_dir(sandbox.path())
                    .status()
                    .unwrap()
                    .success()
            );
        };
        run_git(&["init", "--initial-branch=master"]);
        run_git(&["add", "tracked.txt"]);
        sandbox.create_file(".gitignore", "tracked.txt\n");
        let files = WorkspaceFiles::new(sandbox.path()).unwrap();

        assert!(
            files
                .list(WorkspaceRelativePath::new("."))
                .unwrap()
                .contains(&WorkspaceRelativePathBuf::from("tracked.txt"))
        );
    }
}
