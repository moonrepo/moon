use moon_utils::{
    string_vec,
    test::{create_sandbox_with_git, run_git_command},
};
use moon_vcs::{Git, Vcs};
use std::collections::BTreeMap;
use std::fs;

#[tokio::test]
async fn returns_local_branch() {
    let fixture = create_sandbox_with_git("vcs");
    let git = Git::load("default", fixture.path()).unwrap();

    assert_eq!(git.get_local_branch().await.unwrap(), "master");
    assert_ne!(git.get_local_branch_revision().await.unwrap(), "");
}

mod file_hashing {
    use super::*;

    #[tokio::test]
    async fn hashes_a_list_of_files() {
        let fixture = create_sandbox_with_git("vcs");
        let git = Git::load("default", fixture.path()).unwrap();

        assert_eq!(
            git.get_file_hashes(&string_vec!["existing.txt", "rename-me.txt"])
                .await
                .unwrap(),
            BTreeMap::from([
                (
                    "existing.txt".to_owned(),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                ),
                (
                    "rename-me.txt".to_owned(),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                )
            ])
        );
    }

    #[tokio::test]
    async fn ignores_files_when_hashing() {
        let fixture = create_sandbox_with_git("vcs");

        fs::write(fixture.path().join(".gitignore"), "existing.txt").unwrap();

        let git = Git::load("default", fixture.path()).unwrap();

        assert_eq!(
            git.get_file_hashes(&string_vec!["existing.txt", "rename-me.txt"])
                .await
                .unwrap(),
            BTreeMap::from([(
                "rename-me.txt".to_owned(),
                "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
            )])
        );
    }

    #[tokio::test]
    async fn hashes_an_entire_folder() {
        let fixture = create_sandbox_with_git("vcs");
        let git = Git::load("default", fixture.path()).unwrap();

        assert_eq!(
            git.get_file_tree_hashes(".").await.unwrap(),
            BTreeMap::from([
                (
                    ".gitignore".to_owned(),
                    "b512c09d476623ff4bf8d0d63c29b784925dbdf8".to_owned()
                ),
                (
                    "delete-me.txt".to_owned(),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                ),
                (
                    "rename-me.txt".to_owned(),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                ),
                (
                    "existing.txt".to_owned(),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
                ),
                (
                    "shared-workspace.yml".to_owned(),
                    "b4be93368a88e7038c02969b78d024a23ebe97a5".to_owned()
                ),
            ])
        );
    }

    #[tokio::test]
    async fn filters_ignored_files() {
        let fixture = create_sandbox_with_git("ignore");
        let git = Git::load("master", fixture.path()).unwrap();

        assert_eq!(
            git.get_file_hashes(&string_vec!["foo", "bar", "dir/baz", "dir/qux"])
                .await
                .unwrap(),
            BTreeMap::from([
                (
                    "dir/qux".to_owned(),
                    "100b0dec8c53a40e4de7714b2c612dad5fad9985".to_owned()
                ),
                (
                    "foo".to_owned(),
                    "257cc5642cb1a054f08cc83f2d943e56fd3ebe99".to_owned()
                )
            ])
        );
    }

    #[tokio::test]
    async fn filters_ignored_files_tree() {
        let fixture = create_sandbox_with_git("ignore");
        let git = Git::load("master", fixture.path()).unwrap();

        assert_eq!(
            git.get_file_tree_hashes(".").await.unwrap(),
            BTreeMap::from([
                (
                    ".gitignore".to_owned(),
                    "589c59be54beff591804a008c972e76dea31d2d1".to_owned()
                ),
                (
                    "dir/qux".to_owned(),
                    "100b0dec8c53a40e4de7714b2c612dad5fad9985".to_owned()
                ),
                (
                    "foo".to_owned(),
                    "257cc5642cb1a054f08cc83f2d943e56fd3ebe99".to_owned()
                ),
                (
                    "shared-workspace.yml".to_owned(),
                    "b4be93368a88e7038c02969b78d024a23ebe97a5".to_owned()
                )
            ])
        );
    }
}

mod touched_files {
    use super::*;
    use moon_vcs::TouchedFiles;
    use rustc_hash::FxHashSet;

    #[tokio::test]
    async fn returns_defaults_when_nothing() {
        let fixture = create_sandbox_with_git("vcs");
        let git = Git::load("default", fixture.path()).unwrap();

        assert_eq!(
            git.get_touched_files().await.unwrap(),
            TouchedFiles::default()
        );
    }

    #[tokio::test]
    async fn handles_untracked() {
        let fixture = create_sandbox_with_git("vcs");
        let git = Git::load("default", fixture.path()).unwrap();

        fs::write(fixture.path().join("added.txt"), "").unwrap();

        assert_eq!(
            git.get_touched_files().await.unwrap(),
            TouchedFiles {
                all: FxHashSet::from_iter(string_vec!["added.txt"]),
                untracked: FxHashSet::from_iter(string_vec!["added.txt"]),
                ..TouchedFiles::default()
            }
        );
    }

    #[tokio::test]
    async fn handles_added() {
        let fixture = create_sandbox_with_git("vcs");
        let git = Git::load("default", fixture.path()).unwrap();

        fs::write(fixture.path().join("added.txt"), "").unwrap();

        run_git_command(fixture.path(), |cmd| {
            cmd.args(["add", "added.txt"]);
        });

        assert_eq!(
            git.get_touched_files().await.unwrap(),
            TouchedFiles {
                all: FxHashSet::from_iter(string_vec!["added.txt"]),
                added: FxHashSet::from_iter(string_vec!["added.txt"]),
                staged: FxHashSet::from_iter(string_vec!["added.txt"]),
                ..TouchedFiles::default()
            }
        );
    }

    #[tokio::test]
    async fn handles_deleted() {
        let fixture = create_sandbox_with_git("vcs");
        let git = Git::load("default", fixture.path()).unwrap();

        fs::remove_file(fixture.path().join("delete-me.txt")).unwrap();

        assert_eq!(
            git.get_touched_files().await.unwrap(),
            TouchedFiles {
                all: FxHashSet::from_iter(string_vec!["delete-me.txt"]),
                deleted: FxHashSet::from_iter(string_vec!["delete-me.txt"]),
                unstaged: FxHashSet::from_iter(string_vec!["delete-me.txt"]),
                ..TouchedFiles::default()
            }
        );
    }

    #[tokio::test]
    async fn handles_modified() {
        let fixture = create_sandbox_with_git("vcs");
        let git = Git::load("default", fixture.path()).unwrap();

        fs::write(fixture.path().join("existing.txt"), "modified").unwrap();

        assert_eq!(
            git.get_touched_files().await.unwrap(),
            TouchedFiles {
                all: FxHashSet::from_iter(string_vec!["existing.txt"]),
                modified: FxHashSet::from_iter(string_vec!["existing.txt"]),
                unstaged: FxHashSet::from_iter(string_vec!["existing.txt"]),
                ..TouchedFiles::default()
            }
        );
    }

    #[tokio::test]
    async fn handles_renamed() {
        let fixture = create_sandbox_with_git("vcs");
        let git = Git::load("default", fixture.path()).unwrap();

        fs::rename(
            fixture.path().join("rename-me.txt"),
            fixture.path().join("renamed.txt"),
        )
        .unwrap();

        assert_eq!(
            git.get_touched_files().await.unwrap(),
            TouchedFiles {
                all: FxHashSet::from_iter(string_vec!["rename-me.txt", "renamed.txt"]),
                deleted: FxHashSet::from_iter(string_vec!["rename-me.txt"]),
                unstaged: FxHashSet::from_iter(string_vec!["rename-me.txt"]),
                untracked: FxHashSet::from_iter(string_vec!["renamed.txt"]),
                ..TouchedFiles::default()
            }
        );
    }
}

mod touched_files_via_diff {
    use super::*;
    use moon_vcs::TouchedFiles;
    use rustc_hash::FxHashSet;

    #[tokio::test]
    async fn returns_defaults_when_nothing() {
        let fixture = create_sandbox_with_git("vcs");
        let git = Git::load("default", fixture.path()).unwrap();

        run_git_command(fixture.path(), |cmd| {
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
        let fixture = create_sandbox_with_git("vcs");
        let git = Git::load("default", fixture.path()).unwrap();

        run_git_command(fixture.path(), |cmd| {
            cmd.args(["checkout", "-b", "current"]);
        });

        fs::write(fixture.path().join("added.txt"), "").unwrap();

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
        let fixture = create_sandbox_with_git("vcs");
        let git = Git::load("default", fixture.path()).unwrap();

        run_git_command(fixture.path(), |cmd| {
            cmd.args(["checkout", "-b", "current"]);
        });

        fs::write(fixture.path().join("added.txt"), "").unwrap();

        run_git_command(fixture.path(), |cmd| {
            cmd.args(["add", "added.txt"]);
        });

        assert_eq!(
            git.get_touched_files_between_revisions("master", "current")
                .await
                .unwrap(),
            TouchedFiles {
                all: FxHashSet::from_iter(string_vec!["added.txt"]),
                added: FxHashSet::from_iter(string_vec!["added.txt"]),
                staged: FxHashSet::from_iter(string_vec!["added.txt"]),
                ..TouchedFiles::default()
            }
        );
    }

    #[tokio::test]
    async fn handles_deleted() {
        let fixture = create_sandbox_with_git("vcs");
        let git = Git::load("default", fixture.path()).unwrap();

        run_git_command(fixture.path(), |cmd| {
            cmd.args(["checkout", "-b", "current"]);
        });

        fs::remove_file(fixture.path().join("delete-me.txt")).unwrap();

        assert_eq!(
            git.get_touched_files_between_revisions("master", "current")
                .await
                .unwrap(),
            TouchedFiles {
                all: FxHashSet::from_iter(string_vec!["delete-me.txt"]),
                deleted: FxHashSet::from_iter(string_vec!["delete-me.txt"]),
                staged: FxHashSet::from_iter(string_vec!["delete-me.txt"]),
                ..TouchedFiles::default()
            }
        );
    }

    #[tokio::test]
    async fn handles_modified() {
        let fixture = create_sandbox_with_git("vcs");
        let git = Git::load("default", fixture.path()).unwrap();

        run_git_command(fixture.path(), |cmd| {
            cmd.args(["checkout", "-b", "current"]);
        });

        fs::write(fixture.path().join("existing.txt"), "modified").unwrap();

        assert_eq!(
            git.get_touched_files_between_revisions("master", "current")
                .await
                .unwrap(),
            TouchedFiles {
                all: FxHashSet::from_iter(string_vec!["existing.txt"]),
                modified: FxHashSet::from_iter(string_vec!["existing.txt"]),
                staged: FxHashSet::from_iter(string_vec!["existing.txt"]),
                ..TouchedFiles::default()
            }
        );
    }

    #[tokio::test]
    async fn handles_renamed() {
        let fixture = create_sandbox_with_git("vcs");
        let git = Git::load("default", fixture.path()).unwrap();

        run_git_command(fixture.path(), |cmd| {
            cmd.args(["checkout", "-b", "current"]);
        });

        fs::rename(
            fixture.path().join("rename-me.txt"),
            fixture.path().join("renamed.txt"),
        )
        .unwrap();

        assert_eq!(
            git.get_touched_files_between_revisions("master", "current")
                .await
                .unwrap(),
            TouchedFiles {
                all: FxHashSet::from_iter(string_vec!["rename-me.txt"]),
                deleted: FxHashSet::from_iter(string_vec!["rename-me.txt"]),
                staged: FxHashSet::from_iter(string_vec!["rename-me.txt"]),
                ..TouchedFiles::default()
            }
        );
    }
}
