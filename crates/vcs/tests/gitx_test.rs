use moon_common::path::{RelativePath, RelativePathBuf, WorkspaceRelativePathBuf};
use moon_vcs::TouchedFiles;
use moon_vcs::Vcs;
use moon_vcs::gitx::*;
use rustc_hash::FxHashSet;
use starbase_sandbox::{Sandbox, create_empty_sandbox, create_sandbox};
use std::collections::BTreeMap;
use std::fs;

fn create_root_sandbox() -> (Sandbox, Gitx) {
    let sandbox = create_empty_sandbox();

    sandbox.run_git(|cmd| {
        cmd.args([
            "clone",
            "https://github.com/moonrepo/git-test.git",
            ".",
            "--recurse-submodules",
        ]);
    });

    let git = Gitx::load(sandbox.path(), "master", &["origin".into()]).unwrap();

    (sandbox, git)
}

fn create_worktree_sandbox() -> (Sandbox, Gitx) {
    let sandbox = create_empty_sandbox();

    sandbox.run_git(|cmd| {
        cmd.args([
            "clone",
            "https://github.com/moonrepo/git-test.git",
            ".",
            "--recurse-submodules",
        ]);
    });

    sandbox.run_git(|cmd| {
        cmd.args(["worktree", "add", "worktrees/one", "-b", "one"]);
    });

    sandbox.run_git(|cmd| {
        cmd.args(["submodule", "update"])
            .current_dir(sandbox.path().join("worktrees/one"));
    });

    let git = Gitx::load(
        sandbox.path().join("worktrees/one"),
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

    mod root {
        use super::*;

        #[test]
        fn loads_trees() {
            let (sandbox, git) = create_root_sandbox();

            assert_eq!(git.repository_root, sandbox.path());
            assert_eq!(git.workspace_root, sandbox.path());
            assert_eq!(git.worktree_root, None);
            assert_eq!(git.worktree.git_dir, sandbox.path().join(".git"));
            assert_eq!(git.worktree.work_dir, sandbox.path());
            assert_eq!(git.worktree.path.as_str(), "");
            assert_eq!(git.worktree.type_of, GitTreeType::Root);
            assert_eq!(
                git.submodules,
                vec![
                    GitTree {
                        git_dir: sandbox.path().join(".git/modules/submodules/mono"),
                        path: "submodules/mono".into(),
                        type_of: GitTreeType::Submodule,
                        work_dir: sandbox.path().join("submodules/mono"),
                        ..Default::default()
                    },
                    GitTree {
                        git_dir: sandbox.path().join(".git/modules/submodules/poly"),
                        path: "submodules/poly".into(),
                        type_of: GitTreeType::Submodule,
                        work_dir: sandbox.path().join("submodules/poly"),
                        ..Default::default()
                    }
                ]
            )
        }

        #[tokio::test]
        async fn returns_correct_values() {
            let (sandbox, git) = create_root_sandbox();

            assert_eq!(git.get_local_branch().await.unwrap().as_str(), "master");
            assert_eq!(
                git.get_local_branch_revision().await.unwrap().as_str(),
                "d12c8a437ff760f9d1e323ff77ebc593e87ff30d"
            );
            assert_eq!(git.get_default_branch().await.unwrap().as_str(), "master");
            assert_eq!(
                git.get_default_branch_revision().await.unwrap().as_str(),
                "d12c8a437ff760f9d1e323ff77ebc593e87ff30d"
            );
            assert_eq!(
                git.get_repository_slug().await.unwrap().as_str(),
                "moonrepo/git-test"
            );
            assert_eq!(git.get_repository_root().await.unwrap(), sandbox.path());
            assert_eq!(
                git.get_hooks_dir().await.unwrap(),
                sandbox.path().join(".git/hooks")
            );
        }

        #[tokio::test]
        async fn get_file_tree() {
            let (_sandbox, git) = create_root_sandbox();

            // Returns all
            let list = git.get_file_tree(&RelativePath::new(".")).await.unwrap();

            assert_eq!(list.len(), 37);
            assert!(list.contains(&RelativePathBuf::from("projects/a/moon.yml"))); // In root
            assert!(list.contains(&RelativePathBuf::from(
                "submodules/mono/packages/c/moon.yml"
            ))); // In submodule

            // Returns from root dir
            let list = git
                .get_file_tree(&RelativePath::new("projects/a"))
                .await
                .unwrap();

            assert_eq!(list.len(), 3);
            assert!(list.contains(&RelativePathBuf::from("projects/a/moon.yml")));

            // Returns from submodule dir
            let list = git
                .get_file_tree(&RelativePath::new("submodules/mono/packages/c"))
                .await
                .unwrap();

            assert_eq!(list.len(), 3);
            assert!(list.contains(&RelativePathBuf::from(
                "submodules/mono/packages/c/src/index.cjs"
            )));
        }

        #[tokio::test]
        async fn get_touched_files() {
            let (sandbox, git) = create_root_sandbox();

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
            let (sandbox, git) = create_worktree_sandbox();

            assert_eq!(git.repository_root, sandbox.path());
            assert_eq!(git.workspace_root, sandbox.path().join("worktrees/one"));
            assert_eq!(
                git.worktree_root,
                Some(sandbox.path().join("worktrees/one"))
            );
            assert!(git.worktree.git_dir.ends_with(".git/worktrees/one"),);
            assert_eq!(git.worktree.work_dir, sandbox.path().join("worktrees/one"));
            assert_eq!(git.worktree.path.as_str(), "");
            assert_eq!(git.worktree.type_of, GitTreeType::Worktree);
            assert_eq!(
                git.submodules,
                vec![
                    GitTree {
                        git_dir: sandbox.path().join(".git/modules/submodules/mono"),
                        path: "submodules/mono".into(),
                        type_of: GitTreeType::Submodule,
                        work_dir: sandbox.path().join("worktrees/one/submodules/mono"),
                        ..Default::default()
                    },
                    GitTree {
                        git_dir: sandbox.path().join(".git/modules/submodules/poly"),
                        path: "submodules/poly".into(),
                        type_of: GitTreeType::Submodule,
                        work_dir: sandbox.path().join("worktrees/one/submodules/poly"),
                        ..Default::default()
                    }
                ]
            )
        }

        #[tokio::test]
        async fn returns_correct_values() {
            let (sandbox, git) = create_worktree_sandbox();

            assert_eq!(git.get_local_branch().await.unwrap().as_str(), "one");
            assert_eq!(
                git.get_local_branch_revision().await.unwrap().as_str(),
                "d12c8a437ff760f9d1e323ff77ebc593e87ff30d"
            );
            assert_eq!(git.get_default_branch().await.unwrap().as_str(), "master");
            assert_eq!(
                git.get_default_branch_revision().await.unwrap().as_str(),
                "d12c8a437ff760f9d1e323ff77ebc593e87ff30d"
            );
            assert_eq!(
                git.get_repository_slug().await.unwrap().as_str(),
                "moonrepo/git-test"
            );
            assert_eq!(git.get_repository_root().await.unwrap(), sandbox.path());
            assert_eq!(
                git.get_hooks_dir().await.unwrap(),
                sandbox.path().join(".git/hooks")
            );
        }

        #[tokio::test]
        async fn get_file_tree() {
            let (_sandbox, git) = create_worktree_sandbox();

            // Returns all
            let list = git.get_file_tree(&RelativePath::new(".")).await.unwrap();

            assert_eq!(list.len(), 37);
            assert!(list.contains(&RelativePathBuf::from("projects/a/moon.yml"))); // In root
            assert!(list.contains(&RelativePathBuf::from(
                "submodules/mono/packages/c/moon.yml"
            ))); // In submodule

            // Returns from root dir
            let list = git
                .get_file_tree(&RelativePath::new("projects/a"))
                .await
                .unwrap();

            assert_eq!(list.len(), 3);
            assert!(list.contains(&RelativePathBuf::from("projects/a/moon.yml")));

            // Returns from submodule dir
            let list = git
                .get_file_tree(&RelativePath::new("submodules/mono/packages/c"))
                .await
                .unwrap();

            assert_eq!(list.len(), 3);
            assert!(list.contains(&RelativePathBuf::from(
                "submodules/mono/packages/c/src/index.cjs"
            )));
        }

        #[tokio::test]
        async fn get_touched_files() {
            let (sandbox, git) = create_worktree_sandbox();

            // Returns nothing
            let files = git.get_touched_files().await.unwrap();

            assert_eq!(files, TouchedFiles::default());

            // Returns all
            sandbox.create_file("root.txt", "");
            sandbox.create_file("worktrees/one/tree.txt", "");
            sandbox.create_file("worktrees/one/submodules/mono/packages/a/sub.txt", "");

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
}
