use moon_test_utils::{create_sandbox, predicates::prelude::*};
use std::fs;

mod init_node {
    use super::*;

    #[test]
    fn infers_version_from_nvm() {
        let sandbox = create_sandbox("init-sandbox");
        let root = sandbox.path().to_path_buf();
        let config = root.join(".moon").join("toolchain.yml");

        sandbox.create_file(".nvmrc", "1.2.3");

        sandbox.run_moon(|cmd| {
            cmd.arg("init").arg("--yes").arg(root);
        });

        let content = fs::read_to_string(config).unwrap();

        assert!(predicate::str::contains("version: '1.2.3'").eval(&content));
    }

    #[test]
    fn infers_version_from_nodenv() {
        let sandbox = create_sandbox("init-sandbox");
        let root = sandbox.path().to_path_buf();
        let config = root.join(".moon").join("toolchain.yml");

        sandbox.create_file(".node-version", "1.2.3");

        sandbox.run_moon(|cmd| {
            cmd.arg("init").arg("--yes").arg(root);
        });

        let content = fs::read_to_string(config).unwrap();

        assert!(predicate::str::contains("version: '1.2.3'").eval(&content));
    }

    #[test]
    fn infers_globs_from_workspaces() {
        let sandbox = create_sandbox("init-sandbox");
        let root = sandbox.path().to_path_buf();
        let config = root.join(".moon").join("workspace.yml");

        sandbox.create_file("packages/foo/README", "Hello");
        sandbox.create_file("app/README", "World");
        sandbox.create_file("package.json", r#"{"workspaces": ["packages/*", "app"] }"#);

        sandbox.run_moon(|cmd| {
            cmd.arg("init").arg("--yes").arg(root);
        });

        let content = fs::read_to_string(config).unwrap();

        assert!(predicate::str::contains("projects:\n  - 'app'").eval(&content));
    }

    #[test]
    fn infers_globs_from_workspaces_expanded() {
        let sandbox = create_sandbox("init-sandbox");
        let root = sandbox.path().to_path_buf();
        let config = root.join(".moon").join("workspace.yml");

        sandbox.create_file("packages/bar/README", "Hello");
        sandbox.create_file("app/README", "World");
        sandbox.create_file(
            "package.json",
            r#"{"workspaces": { "packages": ["packages/*", "app"] }}"#,
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("init").arg("--yes").arg(root);
        });

        let content = fs::read_to_string(config).unwrap();

        assert!(predicate::str::contains("projects:\n  - 'app'").eval(&content));
    }

    mod package_manager {
        use super::*;

        #[test]
        fn infers_npm() {
            let sandbox = create_sandbox("init-sandbox");
            let root = sandbox.path().to_path_buf();
            let config = root.join(".moon").join("toolchain.yml");

            sandbox.create_file("package-lock.json", "");

            sandbox.run_moon(|cmd| {
                cmd.arg("init").arg("--yes").arg(root);
            });

            let content = fs::read_to_string(config).unwrap();

            assert!(predicate::str::contains("packageManager: 'npm'").eval(&content));
        }

        #[test]
        fn infers_npm_from_package() {
            let sandbox = create_sandbox("init-sandbox");
            let root = sandbox.path().to_path_buf();
            let config = root.join(".moon").join("toolchain.yml");

            sandbox.create_file("package.json", r#"{"packageManager":"npm@4.5.6"}"#);

            sandbox.run_moon(|cmd| {
                cmd.arg("init").arg("--yes").arg(root);
            });

            let content = fs::read_to_string(config).unwrap();

            assert!(predicate::str::contains("packageManager: 'npm'").eval(&content));
            assert!(predicate::str::contains("npm:\n    version: '4.5.6'").eval(&content));
        }

        #[test]
        fn infers_pnpm() {
            let sandbox = create_sandbox("init-sandbox");
            let root = sandbox.path().to_path_buf();
            let config = root.join(".moon").join("toolchain.yml");

            sandbox.create_file("pnpm-lock.yaml", "");

            sandbox.run_moon(|cmd| {
                cmd.arg("init").arg("--yes").arg(root);
            });

            let content = fs::read_to_string(config).unwrap();

            assert!(predicate::str::contains("packageManager: 'pnpm'").eval(&content));
        }

        #[test]
        fn infers_pnpm_from_package() {
            let sandbox = create_sandbox("init-sandbox");
            let root = sandbox.path().to_path_buf();
            let config = root.join(".moon").join("toolchain.yml");

            sandbox.create_file("package.json", r#"{"packageManager":"pnpm@4.5.6"}"#);

            sandbox.run_moon(|cmd| {
                cmd.arg("init").arg("--yes").arg(root);
            });

            let content = fs::read_to_string(config).unwrap();

            assert!(predicate::str::contains("packageManager: 'pnpm'").eval(&content));
            assert!(predicate::str::contains("pnpm:\n    version: '4.5.6'").eval(&content));
        }

        #[test]
        fn infers_yarn() {
            let sandbox = create_sandbox("init-sandbox");
            let root = sandbox.path().to_path_buf();
            let config = root.join(".moon").join("toolchain.yml");

            sandbox.create_file("yarn.lock", "");

            sandbox.run_moon(|cmd| {
                cmd.arg("init").arg("--yes").arg(root);
            });

            let content = fs::read_to_string(config).unwrap();

            assert!(predicate::str::contains("packageManager: 'yarn'").eval(&content));
        }

        #[test]
        fn infers_yarn_from_package() {
            let sandbox = create_sandbox("init-sandbox");
            let root = sandbox.path().to_path_buf();
            let config = root.join(".moon").join("toolchain.yml");

            sandbox.create_file("package.json", r#"{"packageManager":"yarn@4.5.6"}"#);

            sandbox.run_moon(|cmd| {
                cmd.arg("init").arg("--yes").arg(root);
            });

            let content = fs::read_to_string(config).unwrap();

            assert!(predicate::str::contains("packageManager: 'yarn'").eval(&content));
            assert!(predicate::str::contains("yarn:\n    version: '4.5.6'").eval(&content));
        }
    }
}
