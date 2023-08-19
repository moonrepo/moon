use moon_common::path::WorkspaceRelativePathBuf;
use moon_vcs::{clean_git_version, Git, TouchedFiles, Vcs};
use rustc_hash::FxHashSet;
use starbase_sandbox::{create_sandbox, Sandbox};
use std::collections::BTreeMap;
use std::fs;

fn create_git_sandbox(fixture: &str) -> (Sandbox, Git) {
    let sandbox = create_sandbox(fixture);
    sandbox.enable_git();

    let git = Git::load(sandbox.path(), "master", &["origin".into()]).unwrap();

    (sandbox, git)
}

fn create_git_sandbox_with_ignored(fixture: &str) -> (Sandbox, Git) {
    let sandbox = create_sandbox(fixture);
    sandbox.enable_git();
    sandbox.create_file(".gitignore", "foo/*.txt");

    let git = Git::load(sandbox.path(), "master", &["origin".into()]).unwrap();

    (sandbox, git)
}

fn create_touched_set<I: IntoIterator<Item = V>, V: AsRef<str>>(
    files: I,
) -> FxHashSet<WorkspaceRelativePathBuf> {
    FxHashSet::from_iter(
        files
            .into_iter()
            .map(|v| WorkspaceRelativePathBuf::from(v.as_ref())),
    )
}

mod root_detection {
    use super::*;

    #[tokio::test]
    async fn same_dir() {
        let (sandbox, git) = create_git_sandbox("vcs");

        assert_eq!(git.repository_root, sandbox.path());
        assert_eq!(git.worktree_root, None);
        assert_eq!(git.process.root, sandbox.path());
    }

    #[tokio::test]
    async fn same_dir_if_no_git_dir() {
        let sandbox = create_sandbox("vcs");

        let git = Git::load(sandbox.path(), "master", &["origin".into()]).unwrap();

        assert_eq!(git.repository_root, sandbox.path());
        assert_eq!(git.worktree_root, None);
        assert_eq!(git.process.root, sandbox.path());
    }

    #[tokio::test]
    async fn different_dirs() {
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::load(sandbox.path().join("bar/sub"), "master", &["origin".into()]).unwrap();

        assert_eq!(git.repository_root, sandbox.path());
        assert_eq!(git.worktree_root, None);
        assert_eq!(git.process.root, sandbox.path().join("bar/sub"));
    }
}

mod local {
    use super::*;

    #[tokio::test]
    async fn bin_version() {
        let (_sandbox, git) = create_git_sandbox("vcs");

        assert_eq!(git.get_version().await.unwrap().major, 2);
    }

    #[tokio::test]
    async fn local_branch() {
        let (_sandbox, git) = create_git_sandbox("vcs");

        assert_eq!(git.get_local_branch().await.unwrap(), "master");
    }

    #[tokio::test]
    async fn local_branch_after_switching() {
        let (sandbox, git) = create_git_sandbox("vcs");

        sandbox.run_git(|cmd| {
            cmd.args(["checkout", "-b", "feature"]);
        });

        assert_eq!(git.get_local_branch().await.unwrap(), "feature");
    }

    #[tokio::test]
    async fn local_revision() {
        let (_sandbox, git) = create_git_sandbox("vcs");

        // Hash changes every time, so check that it's not empty
        assert_ne!(git.get_local_branch_revision().await.unwrap(), "");
    }

    #[tokio::test]
    async fn default_branch() {
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::load(sandbox.path(), "main", &[]).unwrap();

        assert_eq!(git.get_default_branch().await.unwrap(), "main");
    }

    #[tokio::test]
    async fn default_revision() {
        let (_sandbox, git) = create_git_sandbox("vcs");

        // Hash changes every time, so check that it's not empty
        assert_ne!(git.get_default_branch_revision().await.unwrap(), "");
    }
}

mod file_hashing {
    use super::*;

    #[tokio::test]
    async fn hashes_a_list_of_files() {
        let (_sandbox, git) = create_git_sandbox("vcs");

        assert_eq!(
            git.get_file_hashes(
                &["foo/file2.txt".into(), "baz/file5.txt".into()],
                false,
                100
            )
            .await
            .unwrap(),
            BTreeMap::from([
                (
                    WorkspaceRelativePathBuf::from("baz/file5.txt"),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                ),
                (
                    WorkspaceRelativePathBuf::from("foo/file2.txt"),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                )
            ])
        );
    }

    #[tokio::test]
    async fn ignores_files_when_hashing() {
        let (_sandbox, git) = create_git_sandbox_with_ignored("vcs");

        assert_eq!(
            git.get_file_hashes(
                &[
                    "foo/file1.txt".into(),
                    "foo/file2.txt".into(),
                    "baz/file5.txt".into()
                ],
                false,
                100
            )
            .await
            .unwrap(),
            BTreeMap::from([(
                WorkspaceRelativePathBuf::from("baz/file5.txt"),
                "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
            )])
        );
    }

    #[tokio::test]
    async fn can_allow_ignored_files_when_hashing() {
        let (_sandbox, git) = create_git_sandbox_with_ignored("vcs");

        assert_eq!(
            git.get_file_hashes(
                &[
                    "foo/file1.txt".into(),
                    "foo/file2.txt".into(),
                    "baz/file5.txt".into()
                ],
                true,
                100
            )
            .await
            .unwrap(),
            BTreeMap::from([
                (
                    WorkspaceRelativePathBuf::from("baz/file5.txt"),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                ),
                (
                    WorkspaceRelativePathBuf::from("foo/file1.txt"),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                ),
                (
                    WorkspaceRelativePathBuf::from("foo/file2.txt"),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                )
            ])
        );
    }

    #[tokio::test]
    async fn hashes_an_entire_folder() {
        let (_sandbox, git) = create_git_sandbox("vcs");

        let tree = git
            .get_file_tree(".")
            .await
            .unwrap()
            .into_iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>();

        let hashes = git.get_file_hashes(&tree, false, 100).await.unwrap();

        assert_eq!(
            hashes,
            BTreeMap::from([
                (
                    WorkspaceRelativePathBuf::from(".gitignore"),
                    "2c085d1d2fb7e1d865a5c1161f0fbbcb682af240".to_owned()
                ),
                (
                    WorkspaceRelativePathBuf::from("bar/sub/dir/file4.txt"),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                ),
                (
                    WorkspaceRelativePathBuf::from("baz/dir/file6.txt"),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                ),
                (
                    WorkspaceRelativePathBuf::from("baz/file5.txt"),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                ),
                (
                    WorkspaceRelativePathBuf::from("foo/file1.txt"),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                ),
                (
                    WorkspaceRelativePathBuf::from("foo/file2.txt"),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                ),
                (
                    WorkspaceRelativePathBuf::from("foo/file3.txt"),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                ),
            ])
        );
    }

    #[tokio::test]
    async fn hashes_and_ignores_an_entire_folder() {
        let (_sandbox, git) = create_git_sandbox_with_ignored("vcs");

        let tree = git
            .get_file_tree(".")
            .await
            .unwrap()
            .into_iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>();

        let hashes = git.get_file_hashes(&tree, false, 100).await.unwrap();

        assert_eq!(
            hashes,
            BTreeMap::from([
                (
                    WorkspaceRelativePathBuf::from(".gitignore"),
                    "666918819a0845b940d6022bd47a8adf85a094aa".to_owned()
                ),
                (
                    WorkspaceRelativePathBuf::from("bar/sub/dir/file4.txt"),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                ),
                (
                    WorkspaceRelativePathBuf::from("baz/dir/file6.txt"),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                ),
                (
                    WorkspaceRelativePathBuf::from("baz/file5.txt"),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                ),
            ])
        );
    }

    #[tokio::test]
    async fn hashes_a_massive_number_of_files() {
        let (sandbox, git) = create_git_sandbox("vcs");

        for i in 0..10000 {
            fs::write(sandbox.path().join(format!("file{}", i)), i.to_string()).unwrap();
        }

        let tree = git
            .get_file_tree(".")
            .await
            .unwrap()
            .into_iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>();

        let hashes = git.get_file_hashes(&tree, false, 100).await.unwrap();

        assert!(hashes.len() >= 10000);
    }

    #[tokio::test]
    async fn cannot_hash_dirs() {
        let (_sandbox, git) = create_git_sandbox("vcs");

        assert_eq!(
            git.get_file_hashes(&["foo".into()], false, 100)
                .await
                .unwrap(),
            BTreeMap::new()
        );
    }
}

mod file_tree {
    use super::*;

    #[tokio::test]
    async fn returns_from_dir() {
        let (_sandbox, git) = create_git_sandbox_with_ignored("vcs");

        let tree = git.get_file_tree("foo").await.unwrap();

        assert_eq!(
            tree,
            vec![
                WorkspaceRelativePathBuf::from("foo/file1.txt"),
                WorkspaceRelativePathBuf::from("foo/file2.txt"),
                WorkspaceRelativePathBuf::from("foo/file3.txt"),
            ]
        );
    }

    #[tokio::test]
    async fn returns_from_deeply_nested_dir() {
        let (_sandbox, git) = create_git_sandbox_with_ignored("vcs");

        let tree = git.get_file_tree("bar/sub/dir").await.unwrap();

        assert_eq!(
            tree,
            vec![WorkspaceRelativePathBuf::from("bar/sub/dir/file4.txt")]
        );
    }

    #[tokio::test]
    async fn includes_untracked() {
        let (sandbox, git) = create_git_sandbox_with_ignored("vcs");

        sandbox.create_file("baz/extra.txt", "");

        let tree = git.get_file_tree("baz").await.unwrap();

        assert_eq!(
            tree,
            vec![
                WorkspaceRelativePathBuf::from("baz/extra.txt"),
                WorkspaceRelativePathBuf::from("baz/dir/file6.txt"),
                WorkspaceRelativePathBuf::from("baz/file5.txt"),
            ]
        );
    }
}

mod touched_files {
    use super::*;

    #[tokio::test]
    async fn returns_defaults_when_nothing() {
        let (_sandbox, git) = create_git_sandbox("touched");

        assert_eq!(
            git.get_touched_files().await.unwrap(),
            TouchedFiles::default()
        );
    }

    #[tokio::test]
    async fn handles_untracked() {
        let (sandbox, git) = create_git_sandbox("touched");

        sandbox.create_file("added.txt", "");

        assert_eq!(
            git.get_touched_files().await.unwrap(),
            TouchedFiles {
                untracked: create_touched_set(["added.txt"]),
                ..TouchedFiles::default()
            }
        );
    }

    #[tokio::test]
    async fn handles_added() {
        let (sandbox, git) = create_git_sandbox("touched");

        sandbox.create_file("added.txt", "");

        sandbox.run_git(|cmd| {
            cmd.args(["add", "added.txt"]);
        });

        assert_eq!(
            git.get_touched_files().await.unwrap(),
            TouchedFiles {
                added: create_touched_set(["added.txt"]),
                staged: create_touched_set(["added.txt"]),
                ..TouchedFiles::default()
            }
        );
    }

    #[tokio::test]
    async fn handles_deleted() {
        let (sandbox, git) = create_git_sandbox("touched");

        fs::remove_file(sandbox.path().join("delete-me.txt")).unwrap();

        assert_eq!(
            git.get_touched_files().await.unwrap(),
            TouchedFiles {
                deleted: create_touched_set(["delete-me.txt"]),
                unstaged: create_touched_set(["delete-me.txt"]),
                ..TouchedFiles::default()
            }
        );
    }

    #[tokio::test]
    async fn handles_modified() {
        let (sandbox, git) = create_git_sandbox("touched");

        sandbox.create_file("existing.txt", "modified");

        assert_eq!(
            git.get_touched_files().await.unwrap(),
            TouchedFiles {
                modified: create_touched_set(["existing.txt"]),
                unstaged: create_touched_set(["existing.txt"]),
                ..TouchedFiles::default()
            }
        );
    }

    #[tokio::test]
    async fn handles_renamed() {
        let (sandbox, git) = create_git_sandbox("touched");

        fs::rename(
            sandbox.path().join("rename-me.txt"),
            sandbox.path().join("renamed.txt"),
        )
        .unwrap();

        assert_eq!(
            git.get_touched_files().await.unwrap(),
            TouchedFiles {
                deleted: create_touched_set(["rename-me.txt"]),
                unstaged: create_touched_set(["rename-me.txt"]),
                untracked: create_touched_set(["renamed.txt"]),
                ..TouchedFiles::default()
            }
        );
    }
}

mod touched_files_via_diff {
    use super::*;

    #[tokio::test]
    async fn returns_defaults_when_nothing() {
        let (sandbox, git) = create_git_sandbox("touched");

        sandbox.run_git(|cmd| {
            cmd.args(["checkout", "-b", "current"]);
        });

        assert_eq!(
            git.get_touched_files_between_revisions("master", "current")
                .await
                .unwrap(),
            TouchedFiles::default()
        );
    }

    #[tokio::test]
    async fn handles_untracked() {
        let (sandbox, git) = create_git_sandbox("touched");

        sandbox.run_git(|cmd| {
            cmd.args(["checkout", "-b", "current"]);
        });

        sandbox.create_file("added.txt", "");

        assert_eq!(
            git.get_touched_files_between_revisions("master", "current")
                .await
                .unwrap(),
            // Untracked isn't captured between branches
            TouchedFiles::default()
        );
    }

    #[tokio::test]
    async fn handles_added() {
        let (sandbox, git) = create_git_sandbox("touched");

        sandbox.run_git(|cmd| {
            cmd.args(["checkout", "-b", "current"]);
        });

        sandbox.create_file("added.txt", "");

        sandbox.run_git(|cmd| {
            cmd.args(["add", "added.txt"]);
        });

        assert_eq!(
            git.get_touched_files_between_revisions("master", "current")
                .await
                .unwrap(),
            TouchedFiles {
                added: create_touched_set(["added.txt"]),
                staged: create_touched_set(["added.txt"]),
                ..TouchedFiles::default()
            }
        );
    }

    #[tokio::test]
    async fn handles_deleted() {
        let (sandbox, git) = create_git_sandbox("touched");

        sandbox.run_git(|cmd| {
            cmd.args(["checkout", "-b", "current"]);
        });

        fs::remove_file(sandbox.path().join("delete-me.txt")).unwrap();

        assert_eq!(
            git.get_touched_files_between_revisions("master", "current")
                .await
                .unwrap(),
            TouchedFiles {
                deleted: create_touched_set(["delete-me.txt"]),
                staged: create_touched_set(["delete-me.txt"]),
                ..TouchedFiles::default()
            }
        );
    }

    #[tokio::test]
    async fn handles_modified() {
        let (sandbox, git) = create_git_sandbox("touched");

        sandbox.run_git(|cmd| {
            cmd.args(["checkout", "-b", "current"]);
        });

        sandbox.create_file("existing.txt", "modified");

        assert_eq!(
            git.get_touched_files_between_revisions("master", "current")
                .await
                .unwrap(),
            TouchedFiles {
                modified: create_touched_set(["existing.txt"]),
                staged: create_touched_set(["existing.txt"]),
                ..TouchedFiles::default()
            }
        );
    }

    #[tokio::test]
    async fn handles_renamed() {
        let (sandbox, git) = create_git_sandbox("touched");

        sandbox.run_git(|cmd| {
            cmd.args(["checkout", "-b", "current"]);
        });

        fs::rename(
            sandbox.path().join("rename-me.txt"),
            sandbox.path().join("renamed.txt"),
        )
        .unwrap();

        assert_eq!(
            git.get_touched_files_between_revisions("master", "current")
                .await
                .unwrap(),
            TouchedFiles {
                deleted: create_touched_set(["rename-me.txt"]),
                staged: create_touched_set(["rename-me.txt"]),
                ..TouchedFiles::default()
            }
        );
    }
}

mod version_cleaning {
    use super::*;

    #[test]
    fn unix() {
        assert_eq!(clean_git_version("git version 1.2.3".into()), "1.2.3");
        assert_eq!(clean_git_version(" git version 1.2.3".into()), "1.2.3");
        assert_eq!(clean_git_version("git version 1.2.3 ".into()), "1.2.3");
        assert_eq!(clean_git_version(" git version 1.2.3 ".into()), "1.2.3");
        assert_eq!(
            clean_git_version("git version 1.2.3 (64-bit)".into()),
            "1.2.3"
        );
        assert_eq!(
            clean_git_version("git version 1.2.3 (32bit)".into()),
            "1.2.3"
        );
    }

    #[test]
    fn macos() {
        assert_eq!(
            clean_git_version("git version 1.2.3 (Apple Git-55)".into()),
            "1.2.3"
        );
        assert_eq!(
            clean_git_version("git version 2.15.1 (Apple Git-101)".into()),
            "2.15.1"
        );
    }

    #[test]
    fn windows() {
        assert_eq!(
            clean_git_version("git version 1.2.3.windows.1".into()),
            "1.2.3"
        );
        assert_eq!(
            clean_git_version(" git for windows 1.2.3.windows.0".into()),
            "1.2.3"
        );
        assert_eq!(
            clean_git_version("git version 1.2.3.windows.10 (32-Bit)  ".into()),
            "1.2.3"
        );

        assert_eq!(
            clean_git_version("  git for windows 1.2.3.win.1".into()),
            "1.2.3"
        );
        assert_eq!(clean_git_version("git 1.2.3.msysgit.1".into()), "1.2.3");
        assert_eq!(
            clean_git_version(" git version 1.2.3.msysgit.11 ".into()),
            "1.2.3"
        );
        assert_eq!(
            clean_git_version("git for windows 1.2.3.msysgit.23  (64bit) ".into()),
            "1.2.3"
        );
    }
}
