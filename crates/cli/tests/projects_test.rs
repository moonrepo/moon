mod utils;

use moon_test_utils2::create_empty_moon_sandbox;
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

    #[test]
    fn many_projects() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("projects");
        });

        assert_snapshot!(assert.output());
    }
}
