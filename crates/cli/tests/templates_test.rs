use moon_test_utils2::{create_empty_moon_sandbox, create_moon_sandbox, predicates::prelude::*};
use starbase_sandbox::assert_snapshot;

mod templates {
    use super::*;

    #[test]
    fn no_templates() {
        let sandbox = create_empty_moon_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("templates");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn no_templates_with_filter() {
        let sandbox = create_moon_sandbox("generator");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("templates").arg("--filter").arg("unknown");
        });

        assert_snapshot!(assert.output());
    }

    #[cfg(unix)]
    #[test]
    fn many_templates() {
        let sandbox = create_moon_sandbox("generator");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("templates");
        });

        assert_snapshot!(assert.output());
    }

    #[cfg(unix)]
    #[test]
    fn can_filter() {
        let sandbox = create_moon_sandbox("generator");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("templates").arg("--filter").arg("vars");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn can_output_json() {
        let sandbox = create_moon_sandbox("generator");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("templates").arg("--json");
            })
            .success()
            .stdout(predicate::str::starts_with("{"));
    }
}
