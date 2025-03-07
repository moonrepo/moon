use moon_config::VcsConfig;
use moon_vcs::{BoxedVcs, Git};
use moon_vcs_hooks::HooksGenerator;
use rustc_hash::FxHashMap;
use starbase_sandbox::{assert_snapshot, create_empty_sandbox};
use std::fs;
use std::path::Path;

fn load_git(root: &Path) -> BoxedVcs {
    Box::new(Git::load(root, "master", &[]).unwrap())
}

fn create_config() -> VcsConfig {
    VcsConfig {
        hooks: FxHashMap::from_iter([
            (
                "pre-commit".into(),
                vec!["moon run :lint".into(), "some-command".into()],
            ),
            ("post-push".into(), vec!["moon check --all".into()]),
        ]),
        ..VcsConfig::default()
    }
}

async fn run_generator(root: &Path) {
    HooksGenerator::new(&load_git(root), &create_config(), root)
        .generate()
        .await
        .unwrap();
}

async fn clean_generator(root: &Path) {
    HooksGenerator::new(&load_git(root), &create_config(), root)
        .cleanup()
        .await
        .unwrap();
}

#[tokio::test]
async fn doesnt_generate_when_no_hooks() {
    let sandbox = create_empty_sandbox();
    sandbox.enable_git();

    HooksGenerator::new(
        &load_git(sandbox.path()),
        &VcsConfig::default(),
        sandbox.path(),
    )
    .generate()
    .await
    .unwrap();

    assert!(!sandbox.path().join(".moon/hooks").exists());
}

#[tokio::test]
async fn doesnt_generate_when_no_commands() {
    let sandbox = create_empty_sandbox();
    sandbox.enable_git();

    HooksGenerator::new(
        &load_git(sandbox.path()),
        &VcsConfig {
            hooks: FxHashMap::from_iter([
                ("pre-commit".into(), vec![]),
                ("post-push".into(), vec![]),
            ]),
            ..VcsConfig::default()
        },
        sandbox.path(),
    )
    .generate()
    .await
    .unwrap();

    assert!(!sandbox.path().join(".moon/hooks").exists());
}

#[tokio::test]
async fn cleans_up_hooks() {
    let sandbox = create_empty_sandbox();
    sandbox.enable_git();

    run_generator(sandbox.path()).await;

    let pre_commit = sandbox.path().join(".git/hooks/pre-commit");
    let post_push = sandbox.path().join(".git/hooks/post-push");
    let local_hooks = sandbox.path().join(".moon/hooks");

    assert!(pre_commit.exists());
    assert!(post_push.exists());
    assert!(local_hooks.exists());

    clean_generator(sandbox.path()).await;

    assert!(!pre_commit.exists());
    assert!(!post_push.exists());
    assert!(!local_hooks.exists());
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn doesnt_create_outside_repo_root() {
        let sandbox = create_empty_sandbox();
        sandbox.enable_git();
        sandbox.run_git(|git| {
            git.args(["config", "--local", "core.hooksPath", "/tmp/git_hooks"]);
        });

        run_generator(sandbox.path()).await;

        let pre_commit = sandbox.path().join(".git/hooks/pre-commit");
        let post_push = sandbox.path().join(".git/hooks/post-push");

        assert!(pre_commit.exists());
        assert!(post_push.exists());
        assert!(!PathBuf::from("/tmp/git_hooks").exists());
    }

    #[tokio::test]
    async fn creates_local_hook_files() {
        let sandbox = create_empty_sandbox();
        sandbox.enable_git();

        run_generator(sandbox.path()).await;

        let pre_commit = sandbox.path().join(".moon/hooks/pre-commit.sh");
        let post_push = sandbox.path().join(".moon/hooks/post-push.sh");

        assert!(pre_commit.exists());
        assert!(post_push.exists());

        assert_snapshot!(fs::read_to_string(pre_commit).unwrap());
        assert_snapshot!(fs::read_to_string(post_push).unwrap());

        let pre_commit = sandbox.path().join(".git/hooks/pre-commit");
        let post_push = sandbox.path().join(".git/hooks/post-push");

        assert!(pre_commit.exists());
        assert!(post_push.exists());

        assert!(
            fs::read_to_string(pre_commit)
                .unwrap()
                .contains("./.moon/hooks/pre-commit.sh $1 $2 $3")
        );
        assert!(
            fs::read_to_string(post_push)
                .unwrap()
                .contains("./.moon/hooks/post-push.sh $1 $2 $3")
        );
    }

    #[tokio::test]
    async fn creates_hook_files_with_trailing_newline() {
        let sandbox = create_empty_sandbox();
        sandbox.enable_git();

        run_generator(sandbox.path()).await;

        let pre_commit = sandbox.path().join(".moon/hooks/pre-commit.sh");

        assert!(pre_commit.exists());
        assert_eq!(
            fs::read_to_string(pre_commit).unwrap().chars().last(),
            Some('\n')
        )
    }

    #[tokio::test]
    async fn links_git_hooks() {
        let sandbox = create_empty_sandbox();
        sandbox.enable_git();

        run_generator(sandbox.path()).await;

        let pre_commit = sandbox.path().join(".git/hooks/pre-commit");
        let post_push = sandbox.path().join(".git/hooks/post-push");

        assert!(pre_commit.exists());
        assert!(post_push.exists());

        assert_snapshot!(fs::read_to_string(pre_commit).unwrap());
        assert_snapshot!(fs::read_to_string(post_push).unwrap());
    }

    #[tokio::test]
    async fn doesnt_links_git_hooks_if_no_root() {
        let sandbox = create_empty_sandbox();

        run_generator(sandbox.path()).await;

        let pre_commit = sandbox.path().join(".git/hooks/pre-commit");
        let post_push = sandbox.path().join(".git/hooks/post-push");

        assert!(!pre_commit.exists());
        assert!(!post_push.exists());
    }

    #[tokio::test]
    async fn supports_git_worktrees() {
        let sandbox = create_empty_sandbox();
        sandbox.enable_git();

        sandbox.run_git(|cmd| {
            cmd.args(["worktree", "add", "tree"]);
        });

        run_generator(&sandbox.path().join("tree")).await;

        let pre_commit = sandbox.path().join("tree/.moon/hooks/pre-commit.sh");
        let post_push = sandbox.path().join("tree/.moon/hooks/post-push.sh");

        assert!(pre_commit.exists());
        assert!(post_push.exists());

        let pre_commit = sandbox.path().join(".git/worktrees/tree/hooks/pre-commit");
        let post_push = sandbox.path().join(".git/worktrees/tree/hooks/post-push");

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

        let pre_commit = sandbox.path().join(".git/hooks/pre-commit");
        let post_push = sandbox.path().join(".git/hooks/post-push");

        assert!(pre_commit.exists());
        assert!(post_push.exists());

        assert!(
            fs::read_to_string(pre_commit)
                .unwrap()
                .contains(".\\.moon\\hooks\\pre-commit.ps1")
        );
        assert!(
            fs::read_to_string(post_push)
                .unwrap()
                .contains(".\\.moon\\hooks\\post-push.ps1")
        );
    }

    #[tokio::test]
    async fn creates_local_hook_files_as_bash() {
        let sandbox = create_empty_sandbox();
        sandbox.enable_git();

        let mut config = create_config();
        config.hook_format = VcsHookFormat::Bash;

        HooksGenerator::new(&load_git(sandbox.path()), &config, sandbox.path())
            .generate()
            .await
            .unwrap();

        let pre_commit = sandbox.path().join(".moon/hooks/pre-commit.sh");
        let post_push = sandbox.path().join(".moon/hooks/post-push.sh");

        assert!(pre_commit.exists());
        assert!(post_push.exists());

        assert_snapshot!(clean_powershell(fs::read_to_string(pre_commit).unwrap()));
        assert_snapshot!(clean_powershell(fs::read_to_string(post_push).unwrap()));

        let pre_commit = sandbox.path().join(".git/hooks/pre-commit");
        let post_push = sandbox.path().join(".git/hooks/post-push");

        assert!(pre_commit.exists());
        assert!(post_push.exists());

        assert!(
            fs::read_to_string(pre_commit)
                .unwrap()
                .contains("./.moon/hooks/pre-commit.sh $1 $2 $3")
        );
        assert!(
            fs::read_to_string(post_push)
                .unwrap()
                .contains("./.moon/hooks/post-push.sh $1 $2 $3")
        );
    }

    #[tokio::test]
    async fn links_git_hooks() {
        let sandbox = create_empty_sandbox();
        sandbox.enable_git();

        run_generator(sandbox.path()).await;

        let pre_commit = sandbox.path().join(".git/hooks/pre-commit");
        let post_push = sandbox.path().join(".git/hooks/post-push");

        assert!(pre_commit.exists());
        assert!(post_push.exists());

        assert_snapshot!(clean_powershell(fs::read_to_string(pre_commit).unwrap()));
        assert_snapshot!(clean_powershell(fs::read_to_string(post_push).unwrap()));
    }
}
