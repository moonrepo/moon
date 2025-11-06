mod utils;

use moon_test_utils2::{create_empty_moon_sandbox, predicates::prelude::*};
use starbase_sandbox::assert_snapshot;
use utils::create_projects_sandbox;

mod projects {
    use super::*;

    #[test]
    fn no_projects() {
        let sandbox = create_empty_moon_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("projects");
        });

        assert_snapshot!(assert.output());
    }

    #[cfg(unix)]
    #[test]
    fn many_projects() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("projects");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn can_output_json() {
        let sandbox = create_projects_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("projects").arg("--json");
            })
            .success()
            .stdout(predicate::str::starts_with("["));
    }
}
