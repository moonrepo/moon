use moon_common::path::{RelativePath, RelativePathBuf, WorkspaceRelativePathBuf};
use moon_vcs::{TouchedFiles, Vcs, gitx::*};
use rustc_hash::FxHashSet;
use starbase_sandbox::{Sandbox, create_empty_sandbox, create_sandbox};
use std::collections::BTreeMap;
use std::fs;

fn create_root_sandbox(bare: bool) -> (Sandbox, Gitx) {
    let sandbox = create_empty_sandbox();

    sandbox.run_git(|cmd| {
        cmd.args([
            "clone",
            "https://github.com/moonrepo/git-test.git",
            ".",
            "--recurse-submodules",
        ]);

        if bare {
            cmd.arg("--bare");
        }
    });

    let git = Gitx::load(sandbox.path(), "master", &["origin".into()]).unwrap();

    (sandbox, git)
}

fn create_worktree_sandbox(bare: bool) -> (Sandbox, Gitx) {
    let sandbox = create_empty_sandbox();

    sandbox.run_git(|cmd| {
        cmd.args([
            "clone",
            "https://github.com/moonrepo/git-test.git",
            ".",
            "--recurse-submodules",
        ]);

        if bare {
            cmd.arg("--bare");
        }
    });

    sandbox.run_git(|cmd| {
        cmd.args(["worktree", "add", "trees/one", "-b", "one"]);
    });

    sandbox.run_git(|cmd| {
        cmd.args(["submodule", "update"])
            .current_dir(sandbox.path().join("trees/one"));
    });

    let git = Gitx::load(
        sandbox.path().join("trees/one"),
        "master",
        &["origin".into()],
    )
    .unwrap();

    (sandbox, git)
}

fn create_git_sandbox(fixture: &str) -> (Sandbox, Gitx) {
    let sandbox = create_sandbox(fixture);
    sandbox.enable_git();

    let git = Gitx::load(sandbox.path(), "master", &["origin".into()]).unwrap();

    (sandbox, git)
}

fn create_git_sandbox_with_ignored(fixture: &str) -> (Sandbox, Gitx) {
    let sandbox = create_sandbox(fixture);
    sandbox.enable_git();
    sandbox.create_file(".gitignore", "foo/*.txt");

    let git = Gitx::load(sandbox.path(), "master", &["origin".into()]).unwrap();

    (sandbox, git)
}

fn create_nested_git_sandbox() -> (Sandbox, Gitx) {
    let sandbox = create_sandbox("nested");
    sandbox.enable_git();

    let git = Gitx::load(
        sandbox.path().join("frontend"),
        "master",
        &["origin".into()],
    )
    .unwrap();

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

mod gitx {
    use super::*;

    #[tokio::test]
    async fn bin_version() {
        let (_sandbox, git) = create_git_sandbox("vcs");

        assert_eq!(git.get_version().await.unwrap().major, 2);
    }

    mod root {
        use super::*;

        #[test]
        fn loads_trees() {
            let (sandbox, git) = create_root_sandbox(false);

            assert_eq!(git.repository_root, sandbox.path());
            assert_eq!(git.workspace_root, sandbox.path());
            assert_eq!(git.worktree.git_dir, sandbox.path().join(".git"));
            assert_eq!(git.worktree.work_dir, sandbox.path());
            assert_eq!(git.worktree.path.as_str(), "");
            assert_eq!(git.worktree.type_of, GitTreeType::Root);
            assert_eq!(
                git.submodules,
                vec![
                    GitTree {
                        git_dir: sandbox
                            .path()
                            .join(".git/modules/submodules/mono")
                            .canonicalize()
                            .unwrap(),
                        path: "submodules/mono".into(),
                        type_of: GitTreeType::Submodule,
                        work_dir: sandbox.path().join("submodules/mono"),
                        ..Default::default()
                    },
                    GitTree {
                        git_dir: sandbox
                            .path()
                            .join(".git/modules/submodules/poly")
                            .canonicalize()
                            .unwrap(),
                        path: "submodules/poly".into(),
                        type_of: GitTreeType::Submodule,
                        work_dir: sandbox.path().join("submodules/poly"),
                        ..Default::default()
                    }
                ]
            )
        }

        #[test]
        fn loads_trees_when_bare() {
            let (sandbox, git) = create_root_sandbox(true);

            assert_eq!(git.repository_root, sandbox.path());
            assert_eq!(git.workspace_root, sandbox.path());
            assert_eq!(git.worktree.git_dir, sandbox.path());
            assert_eq!(git.worktree.work_dir, sandbox.path());
            assert_eq!(git.worktree.path.as_str(), "");
            assert_eq!(git.worktree.type_of, GitTreeType::Root);
            assert_eq!(git.submodules, vec![])
        }

        #[tokio::test]
        async fn returns_correct_values() {
            let (sandbox, git) = create_root_sandbox(false);

            assert_eq!(git.get_local_branch().await.unwrap().as_str(), "master");
            assert_eq!(
                git.get_local_branch_revision().await.unwrap().as_str(),
                "89df0bd49ccdf58d166ba27944baaa42b494516e"
            );
            assert_eq!(git.get_default_branch().await.unwrap().as_str(), "master");
            assert_eq!(
                git.get_default_branch_revision().await.unwrap().as_str(),
                "89df0bd49ccdf58d166ba27944baaa42b494516e"
            );
            assert_eq!(
                git.get_repository_slug().await.unwrap().as_str(),
                "moonrepo/git-test"
            );
            assert_eq!(git.get_repository_root().await.unwrap(), sandbox.path());
            assert_eq!(git.get_working_root().await.unwrap(), sandbox.path());
            assert_eq!(
                git.get_hooks_dir().await.unwrap(),
                sandbox.path().join(".git/hooks")
            );

            // Change branches
            sandbox.run_git(|cmd| {
                cmd.args(["checkout", "-b", "feature"]);
            });

            assert_eq!(git.get_local_branch().await.unwrap().as_str(), "feature");
        }

        #[tokio::test]
        async fn returns_correct_values_when_bare() {
            let (sandbox, git) = create_root_sandbox(true);

            assert_eq!(git.get_repository_root().await.unwrap(), sandbox.path());
            assert_eq!(git.get_working_root().await.unwrap(), sandbox.path());
            assert_eq!(
                git.get_hooks_dir().await.unwrap(),
                sandbox.path().join("hooks")
            );
        }

        #[tokio::test]
        async fn get_file_hashes() {
            let (_sandbox, git) = create_root_sandbox(false);

            let map = git
                .get_file_hashes(
                    &[
                        // In root
                        WorkspaceRelativePathBuf::from("projects/a/moon.yml"),
                        // In submodule
                        WorkspaceRelativePathBuf::from("submodules/mono/packages/b/moon.yml"),
                        // In subtree
                        WorkspaceRelativePathBuf::from("subtrees/one/moon.yml"),
                    ],
                    false,
                )
                .await
                .unwrap();

            assert_eq!(
                map,
                BTreeMap::from_iter([
                    (
                        "projects/a/moon.yml".into(),
                        "40273776247e4e2e36de5c005d9ab68b1ce185c8".into()
                    ),
                    (
                        "submodules/mono/packages/b/moon.yml".into(),
                        "de782e7483e43d345f671c872d0968f5993fc276".into()
                    ),
                    (
                        "subtrees/one/moon.yml".into(),
                        "6881b3074a0c606ed2af036c4ae33d9ae3320ae7".into()
                    ),
                ])
            );
        }

        #[tokio::test]
        async fn get_file_tree() {
            let (_sandbox, git) = create_root_sandbox(false);

            // Returns all
            let list = git.get_file_tree(RelativePath::new(".")).await.unwrap();

            assert_eq!(list.len(), 37);
            assert!(list.contains(&RelativePathBuf::from("projects/a/moon.yml"))); // In root
            assert!(list.contains(&RelativePathBuf::from(
                "submodules/mono/packages/c/moon.yml"
            ))); // In submodule

            // Returns from root dir
            let list = git
                .get_file_tree(RelativePath::new("projects/a"))
                .await
                .unwrap();

            assert_eq!(list.len(), 3);
            assert!(list.contains(&RelativePathBuf::from("projects/a/moon.yml")));

            // Returns from submodule dir
            let list = git
                .get_file_tree(RelativePath::new("submodules/mono/packages/c"))
                .await
                .unwrap();

            assert_eq!(list.len(), 3);
            assert!(list.contains(&RelativePathBuf::from(
                "submodules/mono/packages/c/src/index.cjs"
            )));
        }

        #[tokio::test]
        async fn get_touched_files() {
            let (sandbox, git) = create_root_sandbox(false);

            // Returns nothing
            let files = git.get_touched_files().await.unwrap();

            assert_eq!(files, TouchedFiles::default());

            // Returns all
            sandbox.create_file("root.txt", "");
            sandbox.create_file("submodules/mono/packages/a/sub.txt", "");

            let files = git.get_touched_files().await.unwrap();

            assert_eq!(
                files,
                TouchedFiles {
                    untracked: create_touched_set([
                        "root.txt",
                        "submodules/mono/packages/a/sub.txt"
                    ]),
                    ..TouchedFiles::default()
                }
            );
        }
    }

    mod worktree {
        use super::*;

        #[test]
        fn loads_trees() {
            let (sandbox, git) = create_worktree_sandbox(false);

            assert_eq!(git.repository_root, sandbox.path());
            assert_eq!(git.workspace_root, sandbox.path().join("trees/one"));
            assert_eq!(
                git.worktree.git_dir,
                sandbox
                    .path()
                    .join(".git/worktrees/one")
                    .canonicalize()
                    .unwrap()
            );
            assert_eq!(git.worktree.work_dir, sandbox.path().join("trees/one"));
            assert_eq!(git.worktree.path.as_str(), "");
            assert_eq!(git.worktree.type_of, GitTreeType::Worktree);
            assert_eq!(
                git.submodules,
                vec![
                    GitTree {
                        git_dir: sandbox
                            .path()
                            .join(".git/worktrees/one/modules/submodules/mono")
                            .canonicalize()
                            .unwrap(),
                        path: "submodules/mono".into(),
                        type_of: GitTreeType::Submodule,
                        work_dir: sandbox.path().join("trees/one/submodules/mono"),
                        ..Default::default()
                    },
                    GitTree {
                        git_dir: sandbox
                            .path()
                            .join(".git/worktrees/one/modules/submodules/poly")
                            .canonicalize()
                            .unwrap(),
                        path: "submodules/poly".into(),
                        type_of: GitTreeType::Submodule,
                        work_dir: sandbox.path().join("trees/one/submodules/poly"),
                        ..Default::default()
                    }
                ]
            )
        }

        #[test]
        fn loads_trees_when_bare() {
            let (sandbox, git) = create_worktree_sandbox(true);

            assert_eq!(git.repository_root, sandbox.path());
            assert_eq!(git.workspace_root, sandbox.path().join("trees/one"));
            assert_eq!(
                git.worktree.git_dir,
                sandbox.path().join("worktrees/one").canonicalize().unwrap()
            );
            assert_eq!(git.worktree.work_dir, sandbox.path().join("trees/one"));
            assert_eq!(git.worktree.path.as_str(), "");
            assert_eq!(git.worktree.type_of, GitTreeType::Worktree);
            assert_eq!(
                git.submodules,
                vec![
                    GitTree {
                        git_dir: sandbox
                            .path()
                            .join("worktrees/one/modules/submodules/mono")
                            .canonicalize()
                            .unwrap(),
                        path: "submodules/mono".into(),
                        type_of: GitTreeType::Submodule,
                        work_dir: sandbox.path().join("trees/one/submodules/mono"),
                        ..Default::default()
                    },
                    GitTree {
                        git_dir: sandbox
                            .path()
                            .join("worktrees/one/modules/submodules/poly")
                            .canonicalize()
                            .unwrap(),
                        path: "submodules/poly".into(),
                        type_of: GitTreeType::Submodule,
                        work_dir: sandbox.path().join("trees/one/submodules/poly"),
                        ..Default::default()
                    }
                ]
            )
        }

        #[tokio::test]
        async fn returns_correct_values() {
            let (sandbox, git) = create_worktree_sandbox(false);

            assert_eq!(git.get_local_branch().await.unwrap().as_str(), "one");
            assert_eq!(
                git.get_local_branch_revision().await.unwrap().as_str(),
                "89df0bd49ccdf58d166ba27944baaa42b494516e"
            );
            assert_eq!(git.get_default_branch().await.unwrap().as_str(), "master");
            assert_eq!(
                git.get_default_branch_revision().await.unwrap().as_str(),
                "89df0bd49ccdf58d166ba27944baaa42b494516e"
            );
            assert_eq!(
                git.get_repository_slug().await.unwrap().as_str(),
                "moonrepo/git-test"
            );
            assert_eq!(git.get_repository_root().await.unwrap(), sandbox.path());
            assert_eq!(
                git.get_working_root().await.unwrap(),
                sandbox.path().join("trees/one")
            );
            assert_eq!(
                git.get_hooks_dir().await.unwrap(),
                sandbox.path().join(".git/hooks")
            );

            // Change branches
            sandbox.run_git(|cmd| {
                cmd.args(["checkout", "-b", "feature"])
                    .current_dir(sandbox.path().join("trees/one"));
            });

            assert_eq!(git.get_local_branch().await.unwrap().as_str(), "feature");
        }

        #[tokio::test]
        async fn get_file_hashes() {
            let (_sandbox, git) = create_worktree_sandbox(false);

            let map = git
                .get_file_hashes(
                    &[
                        // In root
                        WorkspaceRelativePathBuf::from("projects/a/moon.yml"),
                        // In submodule
                        WorkspaceRelativePathBuf::from("submodules/mono/packages/b/moon.yml"),
                        // In subtree
                        WorkspaceRelativePathBuf::from("subtrees/one/moon.yml"),
                    ],
                    false,
                )
                .await
                .unwrap();

            assert_eq!(
                map,
                BTreeMap::from_iter([
                    (
                        "projects/a/moon.yml".into(),
                        "40273776247e4e2e36de5c005d9ab68b1ce185c8".into()
                    ),
                    (
                        "submodules/mono/packages/b/moon.yml".into(),
                        "de782e7483e43d345f671c872d0968f5993fc276".into()
                    ),
                    (
                        "subtrees/one/moon.yml".into(),
                        "6881b3074a0c606ed2af036c4ae33d9ae3320ae7".into()
                    ),
                ])
            );
        }

        #[tokio::test]
        async fn get_file_tree() {
            let (_sandbox, git) = create_worktree_sandbox(false);

            // Returns all
            let list = git.get_file_tree(RelativePath::new(".")).await.unwrap();

            assert_eq!(list.len(), 37);
            assert!(list.contains(&RelativePathBuf::from("projects/a/moon.yml"))); // In root
            assert!(list.contains(&RelativePathBuf::from(
                "submodules/mono/packages/c/moon.yml"
            ))); // In submodule

            // Returns from root dir
            let list = git
                .get_file_tree(RelativePath::new("projects/a"))
                .await
                .unwrap();

            assert_eq!(list.len(), 3);
            assert!(list.contains(&RelativePathBuf::from("projects/a/moon.yml")));

            // Returns from submodule dir
            let list = git
                .get_file_tree(RelativePath::new("submodules/mono/packages/c"))
                .await
                .unwrap();

            assert_eq!(list.len(), 3);
            assert!(list.contains(&RelativePathBuf::from(
                "submodules/mono/packages/c/src/index.cjs"
            )));
        }

        #[tokio::test]
        async fn get_touched_files() {
            let (sandbox, git) = create_worktree_sandbox(false);

            // Returns nothing
            let files = git.get_touched_files().await.unwrap();

            assert_eq!(files, TouchedFiles::default());

            // Returns all
            sandbox.create_file("root.txt", "");
            sandbox.create_file("trees/one/tree.txt", "");
            sandbox.create_file("trees/one/submodules/mono/packages/a/sub.txt", "");

            let files = git.get_touched_files().await.unwrap();

            assert_eq!(
                files,
                TouchedFiles {
                    untracked: create_touched_set([
                        "tree.txt",
                        "submodules/mono/packages/a/sub.txt"
                    ]),
                    ..TouchedFiles::default()
                }
            );
        }
    }

    mod submodules {
        use super::*;

        #[tokio::test]
        async fn doesnt_error_if_submodules_arent_checked_out() {
            let sandbox = create_empty_sandbox();

            sandbox.run_git(|cmd| {
                cmd.args([
                    "clone",
                    "https://github.com/moonrepo/git-test.git",
                    ".",
                    // No recurse submodules
                ]);
            });

            let git = Gitx::load(sandbox.path(), "master", &["origin".into()]).unwrap();

            assert!(git.submodules.is_empty());
        }
    }

    mod root_detection {
        use super::*;

        #[tokio::test]
        async fn same_dir() {
            let (sandbox, git) = create_git_sandbox("vcs");

            assert_eq!(git.worktree.git_dir, sandbox.path().join(".git"));
            assert_eq!(git.worktree.work_dir, sandbox.path());
            assert_eq!(git.workspace_root, sandbox.path());
            assert_eq!(git.repository_root, sandbox.path());
        }

        #[tokio::test]
        async fn same_dir_if_no_git_dir() {
            let sandbox = create_sandbox("vcs");

            let git = Gitx::load(sandbox.path(), "master", &["origin".into()]).unwrap();

            assert_eq!(git.worktree.git_dir, sandbox.path().join(".git"));
            assert_eq!(git.worktree.work_dir, sandbox.path());
            assert_eq!(git.workspace_root, sandbox.path());
            assert_eq!(git.repository_root, sandbox.path());
        }

        #[tokio::test]
        async fn different_dirs() {
            let sandbox = create_sandbox("vcs");
            sandbox.enable_git();

            let git = Gitx::load(
                sandbox.path().join("nested/moon"),
                "master",
                &["origin".into()],
            )
            .unwrap();

            assert_eq!(git.worktree.git_dir, sandbox.path().join(".git"));
            assert_eq!(git.worktree.work_dir, sandbox.path());
            assert_eq!(git.workspace_root, sandbox.path().join("nested/moon"));
            assert_eq!(git.repository_root, sandbox.path());
        }
    }

    mod file_hashing {
        use super::*;

        #[tokio::test]
        async fn hashes_a_list_of_files() {
            let (_sandbox, git) = create_git_sandbox("vcs");

            assert_eq!(
                git.get_file_hashes(&["foo/file2.txt".into(), "baz/file5.txt".into()], false)
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
                .get_file_tree(RelativePath::new("."))
                .await
                .unwrap()
                .into_iter()
                .collect::<Vec<_>>();

            let hashes = git.get_file_hashes(&tree, false).await.unwrap();

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
                .get_file_tree(RelativePath::new("."))
                .await
                .unwrap()
                .into_iter()
                .collect::<Vec<_>>();

            let hashes = git.get_file_hashes(&tree, false).await.unwrap();

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
                fs::write(sandbox.path().join(format!("file{i}")), i.to_string()).unwrap();
            }

            let tree = git
                .get_file_tree(RelativePath::new("."))
                .await
                .unwrap()
                .into_iter()
                .collect::<Vec<_>>();

            let hashes = git.get_file_hashes(&tree, false).await.unwrap();

            assert!(hashes.len() >= 10000);
        }

        #[tokio::test]
        async fn cannot_hash_dirs() {
            let (_sandbox, git) = create_git_sandbox("vcs");

            assert_eq!(
                git.get_file_hashes(&["foo".into()], false).await.unwrap(),
                BTreeMap::new()
            );
        }

        #[tokio::test]
        async fn removes_nested_workspace_prefix() {
            let (_sandbox, git) = create_nested_git_sandbox();

            assert_eq!(
                git.get_file_hashes(
                    &[
                        // valid
                        "file.js".into(),
                        // invalid
                        "frontend/file.js".into()
                    ],
                    false,
                )
                .await
                .unwrap(),
                BTreeMap::from([(
                    WorkspaceRelativePathBuf::from("file.js"),
                    "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".into()
                )])
            );
        }
    }

    mod file_tree {
        use super::*;

        #[tokio::test]
        async fn returns_from_dir() {
            let (_sandbox, git) = create_git_sandbox_with_ignored("vcs");

            let tree = git.get_file_tree(RelativePath::new("foo")).await.unwrap();

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

            let tree = git
                .get_file_tree(RelativePath::new("bar/sub/dir"))
                .await
                .unwrap();

            assert_eq!(
                tree,
                vec![WorkspaceRelativePathBuf::from("bar/sub/dir/file4.txt")]
            );
        }

        #[tokio::test]
        async fn includes_untracked() {
            let (sandbox, git) = create_git_sandbox_with_ignored("vcs");

            sandbox.create_file("baz/extra.txt", "");

            let tree = git.get_file_tree(RelativePath::new("baz")).await.unwrap();

            assert_eq!(
                tree,
                vec![
                    WorkspaceRelativePathBuf::from("baz/extra.txt"),
                    WorkspaceRelativePathBuf::from("baz/dir/file6.txt"),
                    WorkspaceRelativePathBuf::from("baz/file5.txt"),
                ]
            );
        }

        #[tokio::test]
        async fn removes_nested_workspace_prefix() {
            let (_sandbox, git) = create_nested_git_sandbox();

            assert_eq!(
                git.get_file_tree(RelativePath::new(".")).await.unwrap(),
                vec![WorkspaceRelativePathBuf::from("file.js")]
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

        #[tokio::test]
        async fn removes_nested_workspace_prefix() {
            let (sandbox, git) = create_nested_git_sandbox();

            sandbox.create_file("frontend/file.js", "modified");

            assert_eq!(
                git.get_touched_files().await.unwrap(),
                TouchedFiles {
                    modified: create_touched_set(["file.js"]),
                    unstaged: create_touched_set(["file.js"]),
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
                git.get_touched_files_between_revisions("master", "")
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
                git.get_touched_files_between_revisions("master", "")
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
                git.get_touched_files_between_revisions("master", "")
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
                git.get_touched_files_between_revisions("master", "")
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
                git.get_touched_files_between_revisions("master", "")
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
                git.get_touched_files_between_revisions("master", "")
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
}
