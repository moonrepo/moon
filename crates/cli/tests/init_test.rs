use moon_test_utils2::{create_empty_sandbox, predicates::prelude::*};
use std::fs;

mod init {
    use super::*;

    #[test]
    fn creates_files_in_dest() {
        let sandbox = create_empty_sandbox();
        let workspace_config = sandbox.path().join(".moon").join("workspace.yml");
        let gitignore = sandbox.path().join(".gitignore");

        assert!(!workspace_config.exists());
        assert!(!gitignore.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("init").arg("--yes").arg(sandbox.path());
            })
            .success()
            .code(0)
            .stdout(predicate::str::contains("Successfully initialized moon in"));

        assert!(workspace_config.exists());
        assert!(gitignore.exists());

        assert_eq!(
            fs::read_to_string(gitignore).unwrap(),
            "\n# moon\n.moon/cache\n.moon/docker\n"
        );
    }

    #[test]
    fn appends_existing_gitignore_file() {
        let sandbox = create_empty_sandbox();

        sandbox.create_file(".gitignore", "*.js\n*.log");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("init").arg("--yes").arg(sandbox.path());
            })
            .success();

        assert_eq!(
            fs::read_to_string(sandbox.path().join(".gitignore")).unwrap(),
            "*.js\n*.log\n# moon\n.moon/cache\n.moon/docker\n"
        );
    }

    #[test]
    fn overwrites_existing_config_if_force_passed() {
        let sandbox = create_empty_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("init").arg("--yes").arg(sandbox.path());
            })
            .success();

        // Run again
        sandbox
            .run_bin(|cmd| {
                cmd.arg("init")
                    .arg("--yes")
                    .arg(sandbox.path())
                    .arg("--force");
            })
            .success()
            .code(0)
            .stdout(predicate::str::contains("Successfully initialized moon in"));
    }

    mod vcs {
        use super::*;

        #[test]
        fn detects_git() {
            let sandbox = create_empty_sandbox();
            sandbox.enable_git();

            let workspace_config = sandbox.path().join(".moon").join("workspace.yml");

            sandbox.run_git(|cmd| {
                cmd.args(["checkout", "-b", "sandbox-test"]);
            });

            sandbox.run_bin(|cmd| {
                cmd.arg("init").arg("--yes").arg(sandbox.path());
            });

            let content = fs::read_to_string(workspace_config).unwrap();

            assert!(predicate::str::contains("client: \"git\"").eval(&content));
            // TODO fix in command
            // assert!(predicate::str::contains("defaultBranch: \"sandbox-test\"").eval(&content));
        }
    }
}
