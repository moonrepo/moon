mod utils;

use moon_test_utils2::predicates::prelude::*;
use utils::create_pipeline_sandbox;

mod check {
    use super::*;

    #[test]
    fn runs_tasks_in_one_project() {
        let sandbox = create_pipeline_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("check").arg("check");
        });

        assert.success().stdout(
            predicate::str::contains("check:build").and(predicate::str::contains("check:test")),
        );
    }

    #[test]
    fn runs_tasks_in_one_project_using_cwd() {
        let sandbox = create_pipeline_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.current_dir(sandbox.path().join("check"))
                .arg("check")
                .arg("--closest");
        });

        assert.success().stdout(
            predicate::str::contains("check:build").and(predicate::str::contains("check:test")),
        );
    }

    #[test]
    fn runs_tasks_in_many_projects() {
        let sandbox = create_pipeline_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("check").arg("check").arg("shared");
        });

        // Fails because of `shared:willFail`
        assert.failure().stdout(
            predicate::str::contains("check:build")
                .and(predicate::str::contains("check:test"))
                .and(predicate::str::contains("shared:base"))
                .and(predicate::str::contains("shared:willFail")),
        );
    }

    #[test]
    fn doesnt_run_internal_tasks() {
        let sandbox = create_pipeline_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("check").arg("check");
        });

        assert
            .success()
            .stdout(predicate::str::contains("check:internal").not());
    }
}
