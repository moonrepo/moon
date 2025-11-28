use moon_test_utils2::{create_moon_sandbox, predicates::prelude::*};
use starbase_sandbox::assert_snapshot;

mod template {
    use super::*;

    #[test]
    fn unknown_template() {
        let sandbox = create_moon_sandbox("generator");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("template").arg("unknown");
            })
            .failure()
            .code(1)
            .stderr(predicate::str::contains(
                "No template with the name unknown could be found",
            ));
    }

    #[test]
    fn renders_task() {
        let sandbox = create_moon_sandbox("generator");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("template").arg("dest");
        });

        assert_snapshot!(assert.output_standardized());
    }

    #[test]
    fn can_output_json() {
        let sandbox = create_moon_sandbox("generator");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("template").arg("vars").arg("--json");
            })
            .success()
            .stdout(predicate::str::starts_with("{"));
    }
}
