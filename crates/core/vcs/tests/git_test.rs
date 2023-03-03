use moon_config::{VcsConfig, VcsManager};
use moon_test_utils::create_sandbox;
use moon_utils::string_vec;
use moon_vcs::{Git, Vcs};
use std::collections::BTreeMap;
use std::fs;

fn create_config(branch: &str) -> VcsConfig {
    VcsConfig {
        default_branch: branch.to_owned(),
        manager: VcsManager::Git,
        ..VcsConfig::default()
    }
}

#[tokio::test]
async fn returns_local_branch() {
    let sandbox = create_sandbox("vcs");
    sandbox.enable_git();

    let git = Git::load(&create_config("default"), sandbox.path()).unwrap();

    assert_eq!(git.get_local_branch().await.unwrap(), "master");
    assert_ne!(git.get_local_branch_revision().await.unwrap(), "");
}

mod file_hashing {
    use super::*;

    #[tokio::test]
    async fn hashes_a_list_of_files() {
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::load(&create_config("default"), sandbox.path()).unwrap();

        assert_eq!(
            git.get_file_hashes(&string_vec!["existing.txt", "rename-me.txt"], false)
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
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        sandbox.create_file(".gitignore", "existing.txt");

        let git = Git::load(&create_config("default"), sandbox.path()).unwrap();

        assert_eq!(
            git.get_file_hashes(&string_vec!["existing.txt", "rename-me.txt"], false)
                .await
                .unwrap(),
            BTreeMap::from([(
                "rename-me.txt".to_owned(),
                "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned()
            )])
        );
    }

    #[tokio::test]
    async fn can_allow_ignored_files_when_hashing() {
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        sandbox.create_file(".gitignore", "existing.txt");

        let git = Git::load(&create_config("default"), sandbox.path()).unwrap();

        assert_eq!(
            git.get_file_hashes(&string_vec!["existing.txt", "rename-me.txt"], true)
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
    async fn hashes_an_entire_folder() {
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::load(&create_config("default"), sandbox.path()).unwrap();

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
            ])
        );
    }

    #[tokio::test]
    async fn filters_ignored_files() {
        let sandbox = create_sandbox("ignore");
        sandbox.enable_git();

        let git = Git::load(&create_config("master"), sandbox.path()).unwrap();

        assert_eq!(
            git.get_file_hashes(&string_vec!["foo", "bar", "dir/baz", "dir/qux"], false)
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
        let sandbox = create_sandbox("ignore");
        sandbox.enable_git();

        let git = Git::load(&create_config("master"), sandbox.path()).unwrap();

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
                )
            ])
        );
    }

    #[tokio::test]
    async fn hashes_a_massive_number_of_files() {
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::load(&create_config("default"), sandbox.path()).unwrap();

        for i in 0..10000 {
            fs::write(sandbox.path().join(format!("file{}", i)), i.to_string()).unwrap();
        }

        assert!(git.get_file_tree_hashes(".").await.unwrap().len() >= 10000);
    }
}

mod touched_files {
    use super::*;
    use moon_vcs::TouchedFiles;
    use rustc_hash::FxHashSet;

    #[tokio::test]
    async fn returns_defaults_when_nothing() {
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::load(&create_config("default"), sandbox.path()).unwrap();

        assert_eq!(
            git.get_touched_files().await.unwrap(),
            TouchedFiles::default()
        );
    }

    #[tokio::test]
    async fn handles_untracked() {
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::load(&create_config("default"), sandbox.path()).unwrap();

        sandbox.create_file("added.txt", "");

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
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::load(&create_config("default"), sandbox.path()).unwrap();

        sandbox.create_file("added.txt", "");

        sandbox.run_git(|cmd| {
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
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::load(&create_config("default"), sandbox.path()).unwrap();

        fs::remove_file(sandbox.path().join("delete-me.txt")).unwrap();

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
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::load(&create_config("default"), sandbox.path()).unwrap();

        sandbox.create_file("existing.txt", "modified");

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
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::load(&create_config("default"), sandbox.path()).unwrap();

        fs::rename(
            sandbox.path().join("rename-me.txt"),
            sandbox.path().join("renamed.txt"),
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
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::load(&create_config("default"), sandbox.path()).unwrap();

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
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::load(&create_config("default"), sandbox.path()).unwrap();

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
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::load(&create_config("default"), sandbox.path()).unwrap();

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
                all: FxHashSet::from_iter(string_vec!["added.txt"]),
                added: FxHashSet::from_iter(string_vec!["added.txt"]),
                staged: FxHashSet::from_iter(string_vec!["added.txt"]),
                ..TouchedFiles::default()
            }
        );
    }

    #[tokio::test]
    async fn handles_deleted() {
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::load(&create_config("default"), sandbox.path()).unwrap();

        sandbox.run_git(|cmd| {
            cmd.args(["checkout", "-b", "current"]);
        });

        fs::remove_file(sandbox.path().join("delete-me.txt")).unwrap();

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
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::load(&create_config("default"), sandbox.path()).unwrap();

        sandbox.run_git(|cmd| {
            cmd.args(["checkout", "-b", "current"]);
        });

        sandbox.create_file("existing.txt", "modified");

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
        let sandbox = create_sandbox("vcs");
        sandbox.enable_git();

        let git = Git::load(&create_config("default"), sandbox.path()).unwrap();

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
                all: FxHashSet::from_iter(string_vec!["rename-me.txt"]),
                deleted: FxHashSet::from_iter(string_vec!["rename-me.txt"]),
                staged: FxHashSet::from_iter(string_vec!["rename-me.txt"]),
                ..TouchedFiles::default()
            }
        );
    }
}

mod slug_parsing {
    use super::*;

    #[test]
    fn supports_http() {
        assert_eq!(
            Git::extract_slug_from_remote("http://github.com/moonrepo/moon".into()).unwrap(),
            "moonrepo/moon"
        );
        assert_eq!(
            Git::extract_slug_from_remote("http://github.com/moonrepo/moon.git".into()).unwrap(),
            "moonrepo/moon"
        );
        assert_eq!(
            Git::extract_slug_from_remote("https://github.com/moonrepo/moon".into()).unwrap(),
            "moonrepo/moon"
        );
        assert_eq!(
            Git::extract_slug_from_remote("https://github.com/moonrepo/moon.git".into()).unwrap(),
            "moonrepo/moon"
        );
    }

    #[test]
    fn supports_git() {
        assert_eq!(
            Git::extract_slug_from_remote("git@github.com:moonrepo/moon".into()).unwrap(),
            "moonrepo/moon"
        );
        assert_eq!(
            Git::extract_slug_from_remote("git@github.com:moonrepo/moon.git".into()).unwrap(),
            "moonrepo/moon"
        );
    }
}
