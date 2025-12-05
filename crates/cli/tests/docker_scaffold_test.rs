use moon_test_utils2::create_moon_sandbox;
use std::fs;

mod docker_scaffold {
    use super::*;

    mod configs_skeleton {
        use super::*;

        #[test]
        fn runs_toolchain_funcs() {
            let sandbox = create_moon_sandbox("docker");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("docker").arg("scaffold").arg("scaffold");
                })
                .success();

            let docker = sandbox.path().join(".moon/docker/configs");

            assert!(docker.join("scaffold/from-configs-phase").exists());
            assert!(docker.join("from-configs-phase").exists());

            assert!(!docker.join("scaffold/from-sources-phase").exists());
            assert!(!docker.join("from-sources-phase").exists());
        }

        #[test]
        fn copies_toolchain_files() {
            let sandbox = create_moon_sandbox("docker");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("docker").arg("scaffold").arg("scaffold");
                })
                .success();

            let docker = sandbox.path().join(".moon/docker/configs");

            // From all projects
            assert!(docker.join("dockerManifest.json").exists());
            assert!(docker.join("dep/tc.cfg").exists());
            assert!(docker.join("prune/tc.cfg").exists());
            assert!(docker.join("scaffold/tc.cfg").exists());
            assert!(docker.join("tc.root.cfg").exists());
            assert!(docker.join("tc.lock").exists());
        }

        #[test]
        fn copies_moon_files() {
            let sandbox = create_moon_sandbox("docker");

            // Test inherited configs
            fs::create_dir(sandbox.path().join(".moon/tasks")).unwrap();
            fs::write(sandbox.path().join(".moon/tasks/test.yml"), "{}").unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("docker").arg("scaffold").arg("scaffold");
                })
                .debug();

            let docker = sandbox.path().join(".moon/docker/configs");

            // From all projects
            assert!(docker.join(".moon/tasks/test.yml").exists());
            assert!(docker.join(".moon/toolchains.yml").exists());
            assert!(docker.join(".moon/workspace.yml").exists());
            assert!(docker.join("dep/moon.yml").exists());
            assert!(docker.join("prune/moon.yml").exists());
            assert!(docker.join("scaffold/moon.yml").exists());
        }

        #[test]
        fn doesnt_copy_source_files() {
            let sandbox = create_moon_sandbox("docker");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("docker").arg("scaffold").arg("scaffold");
                })
                .success();

            let docker = sandbox.path().join(".moon/docker/configs");

            assert!(!docker.join("dep/file.txt").exists());
            assert!(!docker.join("scaffold/file.txt").exists());
        }
    }

    mod sources_skeleton {
        use super::*;

        #[test]
        fn runs_toolchain_funcs() {
            let sandbox = create_moon_sandbox("docker");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("docker").arg("scaffold").arg("scaffold");
                })
                .debug();

            let docker = sandbox.path().join(".moon/docker/sources");

            assert!(!docker.join("scaffold/from-configs-phase").exists());
            assert!(!docker.join("from-configs-phase").exists());

            assert!(docker.join("scaffold/from-sources-phase").exists());
            assert!(docker.join("from-sources-phase").exists());
        }

        #[test]
        fn only_copies_source_files_for_focused_projects() {
            let sandbox = create_moon_sandbox("docker");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("docker").arg("scaffold").arg("scaffold");
                })
                .success();

            let docker = sandbox.path().join(".moon/docker/sources");

            assert!(docker.join("dep/file.txt").exists());
            assert!(!docker.join("prune/file.txt").exists());
            assert!(docker.join("scaffold/file.txt").exists());
        }

        #[test]
        fn can_copy_multiple_projects() {
            let sandbox = create_moon_sandbox("docker");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("docker").arg("scaffold").arg("dep").arg("prune");
                })
                .success();

            let docker = sandbox.path().join(".moon/docker/sources");

            assert!(docker.join("dep/file.txt").exists());
            assert!(docker.join("prune/file.txt").exists());
            assert!(!docker.join("scaffold/file.txt").exists());
        }

        #[test]
        fn doesnt_copy_vendor_files() {
            let sandbox = create_moon_sandbox("docker");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("docker").arg("scaffold").arg("prune");
                })
                .success();

            let docker = sandbox.path().join(".moon/docker/sources");

            assert!(!docker.join("prune/vendor").exists());
        }
    }
}
