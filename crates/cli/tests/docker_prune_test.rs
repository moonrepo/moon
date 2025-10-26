use moon_test_utils2::{MoonSandbox, create_moon_sandbox, predicates};
use starbase_utils::fs;

mod docker_prune {
    use super::*;

    fn create_workdir(sandbox: &MoonSandbox) -> std::path::PathBuf {
        sandbox
            .run_bin(|cmd| {
                cmd.arg("docker").arg("scaffold").arg("prune");
            })
            .success();

        fs::copy_dir_all(
            sandbox.path().join(".moon/docker/configs"),
            sandbox.path().join(".moon/docker/configs"),
            sandbox.path().join("work"),
        )
        .unwrap();

        fs::copy_dir_all(
            sandbox.path().join(".moon/docker/sources"),
            sandbox.path().join(".moon/docker/sources"),
            sandbox.path().join("work"),
        )
        .unwrap();

        sandbox.path().join("work")
    }

    #[test]
    fn errors_for_missing_manifest() {
        let sandbox = create_moon_sandbox("docker");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("docker").arg("prune");
            })
            .failure()
            .stderr(predicates::str::contains(
                "Unable to continue, docker manifest missing.",
            ));
    }

    #[test]
    fn removes_vendor_dirs() {
        let sandbox = create_moon_sandbox("docker");
        let workdir = create_workdir(&sandbox);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("docker").arg("prune").current_dir(&workdir);
            })
            .success();

        assert!(!workdir.join("prune/vendor").exists());
    }
}
