use moon_test_utils2::{create_moon_sandbox, predicates};

mod docker_setup {
    use super::*;

    #[test]
    fn errors_for_missing_manifest() {
        let sandbox = create_moon_sandbox("docker");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("docker").arg("setup");
            })
            .failure()
            .stderr(predicates::str::contains(
                "Unable to continue, docker manifest missing.",
            ));
    }
}
