mod utils;

use moon_test_utils2::{MoonSandbox, create_empty_moon_sandbox};
use rustc_hash::FxHashMap;
use utils::create_projects_sandbox;

mod sync_codeowners {
    use super::*;

    #[test]
    fn creates_codeowners_file() {
        let sandbox = create_empty_moon_sandbox();
        let file = sandbox.path().join(".github/CODEOWNERS");

        assert!(!file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("sync").arg("codeowners");
            })
            .success();

        assert!(file.exists());
    }

    #[test]
    fn removes_codeowners_file() {
        let sandbox = create_empty_moon_sandbox();
        let file = sandbox.path().join(".github/CODEOWNERS");

        assert!(!file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("sync").arg("codeowners");
            })
            .success();

        assert!(file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("sync").arg("codeowners").arg("--clean");
            })
            .success();

        assert!(!file.exists());
    }
}

mod sync_config_schemas {
    use super::*;

    #[test]
    fn creates_schemas_dir() {
        let sandbox = create_empty_moon_sandbox();
        let dir = sandbox.path().join(".moon/cache/schemas");

        assert!(!dir.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("sync").arg("config-schemas");
            })
            .success();

        assert!(dir.exists());
    }
}

mod sync_hooks {
    use super::*;

    fn create_hooks_sandbox() -> MoonSandbox {
        let sandbox = create_empty_moon_sandbox();

        sandbox.update_workspace_config(|config| {
            config.vcs.get_or_insert_default().hooks = Some(FxHashMap::from_iter([
                (
                    "pre-commit".into(),
                    vec!["moon run :lint".into(), "some-command".into()],
                ),
                ("post-push".into(), vec!["moon check --all".into()]),
            ]));
        });

        sandbox.enable_git();
        sandbox
    }

    #[test]
    fn creates_hook_files() {
        let sandbox = create_hooks_sandbox();
        let dir = sandbox.path().join(".moon/hooks");

        assert!(!dir.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("sync").arg("hooks");
            })
            .success();

        assert!(dir.exists());

        assert!(dir.join("pre-commit").exists());
        assert!(dir.join("post-push").exists());

        if cfg!(windows) {
            assert!(dir.join("pre-commit.ps1").exists());
            assert!(dir.join("post-push.ps1").exists());
        }
    }

    #[test]
    fn removes_hook_files() {
        let sandbox = create_hooks_sandbox();
        let dir = sandbox.path().join(".moon/hooks");

        assert!(!dir.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("sync").arg("hooks");
            })
            .success();

        assert!(dir.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("sync").arg("hooks").arg("--clean");
            })
            .success();

        assert!(!dir.exists());
    }
}

mod sync_projects {
    use super::*;

    #[test]
    fn syncs_all_projects() {
        let sandbox = create_projects_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("sync").arg("projects");
            })
            .success();
    }

    #[test]
    fn runs_legacy_sync_command() {
        let sandbox = create_projects_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("sync");
            })
            .success();
    }
}
