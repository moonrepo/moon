use moon_config::VcsConfig;
use moon_test_utils::WorkspaceMocker;
use moon_vcs_hooks::HooksGenerator;
use rustc_hash::FxHashMap;
use starbase_sandbox::{assert_snapshot, create_empty_sandbox};
use std::fs;
use std::path::Path;

fn create_config() -> VcsConfig {
    VcsConfig {
        hooks: FxHashMap::from_iter([
            (
                "pre-commit".into(),
                vec!["moon run :lint".into(), "some-command $ARG1".into()],
            ),
            ("post-push".into(), vec!["moon check --all".into()]),
        ]),
        ..VcsConfig::default()
    }
}

async fn run_generator(root: &Path) {
    let mock = WorkspaceMocker::new(root);

    HooksGenerator::new(&mock.mock_app_context(), &create_config())
        .generate()
        .await
        .unwrap();
}

async fn clean_generator(root: &Path) {
    let mock = WorkspaceMocker::new(root);

    HooksGenerator::new(&mock.mock_app_context(), &create_config())
        .cleanup()
        .await
        .unwrap();
}

mod vcs_hooks {
    use super::*;

    #[tokio::test]
    async fn doesnt_generate_when_no_hooks() {
        let sandbox = create_empty_sandbox();
        sandbox.enable_git();

        let mock = WorkspaceMocker::new(sandbox.path());

        HooksGenerator::new(&mock.mock_app_context(), &VcsConfig::default())
            .generate()
            .await
            .unwrap();

        assert!(!sandbox.path().join(".moon/hooks").exists());
        assert!(
            !fs::read_to_string(sandbox.path().join(".git/config"))
                .unwrap()
                .contains("hooksPath =")
        );
    }

    #[tokio::test]
    async fn doesnt_generate_when_no_commands() {
        let sandbox = create_empty_sandbox();
        sandbox.enable_git();

        let mock = WorkspaceMocker::new(sandbox.path());

        HooksGenerator::new(
            &mock.mock_app_context(),
            &VcsConfig {
                hooks: FxHashMap::from_iter([
                    ("pre-commit".into(), vec![]),
                    ("post-push".into(), vec![]),
                ]),
                ..VcsConfig::default()
            },
        )
        .generate()
        .await
        .unwrap();

        assert!(!sandbox.path().join(".moon/hooks").exists());
        assert!(
            !fs::read_to_string(sandbox.path().join(".git/config"))
                .unwrap()
                .contains("hooksPath =")
        );
    }

    #[tokio::test]
    async fn cleans_up_hooks() {
        let sandbox = create_empty_sandbox();
        sandbox.enable_git();

        run_generator(sandbox.path()).await;

        let pre_commit = sandbox.path().join(".moon/hooks/pre-commit");
        let post_push = sandbox.path().join(".moon/hooks/post-push");

        assert!(pre_commit.exists());
        assert!(post_push.exists());

        clean_generator(sandbox.path()).await;

        assert!(!pre_commit.exists());
        assert!(!post_push.exists());
        assert!(
            !fs::read_to_string(sandbox.path().join(".git/config"))
                .unwrap()
                .contains("hooksPath =")
        );
    }

    #[tokio::test]
    async fn doesnt_adopt_a_foreign_hooks_dir() {
        let sandbox = create_empty_sandbox();
        sandbox.enable_git();

        // Simulate another tool managing hooks (husky, lefthook, etc)
        sandbox.create_file(".husky/pre-commit", "echo husky");

        sandbox.run_git(|cmd| {
            cmd.args([
                "config",
                "core.hooksPath",
                sandbox.path().join(".husky").to_str().unwrap(),
            ]);
        });

        run_generator(sandbox.path()).await;

        // Hooks are written to the moon-owned directory,
        // and the foreign directory is left untouched
        assert!(sandbox.path().join(".moon/hooks/pre-commit").exists());
        assert_eq!(
            fs::read_to_string(sandbox.path().join(".husky/pre-commit")).unwrap(),
            "echo husky"
        );

        clean_generator(sandbox.path()).await;

        assert!(!sandbox.path().join(".moon/hooks").exists());
        assert!(sandbox.path().join(".husky/pre-commit").exists());
    }

    #[tokio::test]
    async fn cleanup_doesnt_delete_a_foreign_hooks_dir_from_state() {
        let sandbox = create_empty_sandbox();
        sandbox.enable_git();

        sandbox.create_file(".husky/pre-commit", "echo husky");
        sandbox.create_file(".husky/other-file", "keep me");

        // Simulate state from an older moon version that adopted a foreign dir
        sandbox.create_file(
            ".moon/cache/states/vcsHooks.json",
            r#"{"hookNames":["pre-commit"],"relativeHooksDir":".husky"}"#,
        );

        clean_generator(sandbox.path()).await;

        // moon's named hook is removed, but the directory
        // and all other files remain untouched
        assert!(!sandbox.path().join(".husky/pre-commit").exists());
        assert!(sandbox.path().join(".husky/other-file").exists());
    }

    #[tokio::test]
    async fn removes_stale_hooks_on_subsequent_runs() {
        let sandbox = create_empty_sandbox();
        sandbox.enable_git();

        let pre_commit = sandbox.path().join(".moon/hooks/pre-commit");
        let post_push = sandbox.path().join(".moon/hooks/post-push");

        let mock = WorkspaceMocker::new(sandbox.path());
        let mut config = create_config();

        // First
        HooksGenerator::new(&mock.mock_app_context(), &config)
            .generate()
            .await
            .unwrap();

        assert!(pre_commit.exists());
        assert!(post_push.exists());

        // Second
        config.hooks.remove("pre-commit");

        HooksGenerator::new(&mock.mock_app_context(), &config)
            .generate()
            .await
            .unwrap();

        assert!(!pre_commit.exists());
        assert!(post_push.exists());
    }

    #[tokio::test]
    async fn sets_git_hooks_path_config() {
        let sandbox = create_empty_sandbox();
        sandbox.enable_git();

        run_generator(sandbox.path()).await;

        let config = fs::read_to_string(sandbox.path().join(".git/config")).unwrap();

        assert!(config.contains("hooksPath ="));
    }

    #[tokio::test]
    async fn places_hooks_alongside_config_moon_dir() {
        let sandbox = create_empty_sandbox();
        sandbox.enable_git();

        // Workspace config lives at `.config/moon` instead of `.moon`
        sandbox.create_file(".config/moon/workspace.yml", "");

        run_generator(sandbox.path()).await;

        // Hooks are placed alongside the config, not in `.moon`
        assert!(
            sandbox
                .path()
                .join(".config/moon/hooks/pre-commit")
                .exists()
        );
        assert!(sandbox.path().join(".config/moon/hooks/post-push").exists());
        assert!(!sandbox.path().join(".moon/hooks").exists());

        // And the git config points to the correct directory. Normalize the
        // separators, as Git stores Windows paths with escaped backslashes.
        let config = fs::read_to_string(sandbox.path().join(".git/config"))
            .unwrap()
            .replace(r"\\", "/")
            .replace('\\', "/");

        assert!(config.contains(".config/moon/hooks"));

        clean_generator(sandbox.path()).await;

        assert!(!sandbox.path().join(".config/moon/hooks").exists());
        assert!(
            !fs::read_to_string(sandbox.path().join(".git/config"))
                .unwrap()
                .contains("hooksPath =")
        );
    }

    #[cfg(unix)]
    mod unix {
        use super::*;

        #[tokio::test]
        async fn creates_local_hook_files() {
            let sandbox = create_empty_sandbox();
            sandbox.enable_git();

            run_generator(sandbox.path()).await;

            let pre_commit = sandbox.path().join(".moon/hooks/pre-commit");
            let post_push = sandbox.path().join(".moon/hooks/post-push");

            assert!(pre_commit.exists());
            assert!(post_push.exists());

            assert_snapshot!(fs::read_to_string(pre_commit).unwrap());
            assert_snapshot!(fs::read_to_string(post_push).unwrap());
        }

        #[tokio::test]
        async fn creates_hook_files_with_trailing_newline() {
            let sandbox = create_empty_sandbox();
            sandbox.enable_git();

            run_generator(sandbox.path()).await;

            let pre_commit = sandbox.path().join(".moon/hooks/pre-commit");

            assert!(pre_commit.exists());
            assert_eq!(
                fs::read_to_string(pre_commit).unwrap().chars().last(),
                Some('\n')
            )
        }

        #[tokio::test]
        async fn supports_git_worktrees() {
            let sandbox = create_empty_sandbox();
            sandbox.enable_git();

            sandbox.run_git(|cmd| {
                cmd.args(["worktree", "add", "tree"]);
            });

            run_generator(&sandbox.path().join("tree")).await;

            let pre_commit = sandbox.path().join("tree/.moon/hooks/pre-commit");
            let post_push = sandbox.path().join("tree/.moon/hooks/post-push");

            assert!(pre_commit.exists());
            assert!(post_push.exists());
        }
    }

    #[cfg(windows)]
    mod windows {
        use super::*;
        use moon_config::VcsHookFormat;

        // Standardize snapshots across machines with different powershell versions
        fn clean_powershell(content: String) -> String {
            content.replace("powershell", "pwsh")
        }

        #[tokio::test]
        async fn creates_local_hook_files() {
            let sandbox = create_empty_sandbox();
            sandbox.enable_git();

            run_generator(sandbox.path()).await;

            let pre_commit = sandbox.path().join(".moon/hooks/pre-commit.ps1");
            let post_push = sandbox.path().join(".moon/hooks/post-push.ps1");

            assert!(pre_commit.exists());
            assert!(post_push.exists());

            assert_snapshot!(clean_powershell(fs::read_to_string(pre_commit).unwrap()));
            assert_snapshot!(clean_powershell(fs::read_to_string(post_push).unwrap()));

            let pre_commit = sandbox.path().join(".moon/hooks/pre-commit");
            let post_push = sandbox.path().join(".moon/hooks/post-push");

            assert!(pre_commit.exists());
            assert!(post_push.exists());

            assert!(
                fs::read_to_string(pre_commit)
                    .unwrap()
                    .contains(".moon/hooks/pre-commit.ps1")
            );
            assert!(
                fs::read_to_string(post_push)
                    .unwrap()
                    .contains(".moon/hooks/post-push.ps1")
            );
        }

        #[tokio::test]
        async fn creates_local_hook_files_as_bash() {
            let sandbox = create_empty_sandbox();
            sandbox.enable_git();

            let mock = WorkspaceMocker::new(sandbox.path());
            let mut config = create_config();
            config.hook_format = VcsHookFormat::Bash;

            HooksGenerator::new(&mock.mock_app_context(), &config)
                .generate()
                .await
                .unwrap();

            let pre_commit = sandbox.path().join(".moon/hooks/pre-commit");
            let post_push = sandbox.path().join(".moon/hooks/post-push");

            assert!(pre_commit.exists());
            assert!(post_push.exists());

            assert_snapshot!(clean_powershell(fs::read_to_string(pre_commit).unwrap()));
            assert_snapshot!(clean_powershell(fs::read_to_string(post_push).unwrap()));
        }

        #[tokio::test]
        async fn supports_git_worktrees() {
            let sandbox = create_empty_sandbox();
            sandbox.enable_git();

            sandbox.run_git(|cmd| {
                cmd.args(["worktree", "add", "tree"]);
            });

            run_generator(&sandbox.path().join("tree")).await;

            let pre_commit = sandbox.path().join("tree/.moon/hooks/pre-commit.ps1");
            let post_push = sandbox.path().join("tree/.moon/hooks/post-push.ps1");

            assert!(pre_commit.exists());
            assert!(post_push.exists());
        }
    }
}
